//! Mid-function hooks with full register context
//!
//! Allows hooking at arbitrary addresses within functions with access to all CPU registers.

use parking_lot::RwLock;
use slotmap::{new_key_type, SlotMap};
use std::ptr::NonNull;
use std::sync::LazyLock;

use super::context::MidHookContext;
use super::inline::HookError;
use super::trampoline::alloc_trampoline_sized;

new_key_type! {
    /// Handle for a mid-function hook
    pub struct MidHookKey;
}

/// Callback type for mid-function hooks
pub type MidHookCallback = Box<dyn Fn(&mut MidHookContext) + Send + Sync>;

/// Storage for a mid-function hook
struct MidHookEntry {
    /// Target address being hooked
    target: *const u8,

    /// Trampoline containing:
    /// 1. Save registers
    /// 2. Call Rust callback
    /// 3. Restore registers
    /// 4. Execute original bytes
    /// 5. Jump back
    /// (stored to keep allocation alive)
    #[allow(dead_code)]
    trampoline: NonNull<u8>,

    /// Size of trampoline
    #[allow(dead_code)]
    trampoline_size: usize,

    /// Original bytes that were overwritten
    original_bytes: Vec<u8>,

    /// Callback function (stored to keep closure alive)
    #[allow(dead_code)]
    callback: MidHookCallback,

    /// Raw callback pointer for the trampoline (stored to prevent dangling pointer)
    #[allow(dead_code)]
    callback_ptr: *const (),

    /// Whether currently active
    enabled: bool,

    /// Debug name
    name: String,
}

unsafe impl Send for MidHookEntry {}
unsafe impl Sync for MidHookEntry {}

/// Global mid-hook registry
static MID_HOOKS: LazyLock<RwLock<SlotMap<MidHookKey, MidHookEntry>>> =
    LazyLock::new(|| RwLock::new(SlotMap::with_key()));

/// Minimum bytes needed at hook site for a JMP rel32
const MIN_HOOK_SIZE: usize = 5;

/// Size of the context-saving trampoline stub
const STUB_SIZE: usize = 1024;

/// FFI callback wrapper that the trampoline calls
///
/// # Safety
/// This is called from assembly with a pointer to the context on the stack
#[no_mangle]
unsafe extern "C" fn mid_hook_callback_wrapper(context: *mut MidHookContext, callback: *const ()) {
    if context.is_null() || callback.is_null() {
        return;
    }

    // Cast callback back to Rust closure
    let callback_ref = &*(callback as *const MidHookCallback);
    callback_ref(&mut *context);
}

/// Create a mid-function hook at an arbitrary address
///
/// # Safety
/// - `target` must be a valid code address
/// - The hook site must have at least 5 bytes of instructions that can be relocated
///
/// # Arguments
/// * `name` - Debug name for the hook
/// * `target` - Address to hook
/// * `callback` - Function called with register context
pub unsafe fn create_mid_hook<F>(
    name: &str,
    target: *const u8,
    callback: F,
) -> Result<MidHookKey, HookError>
where
    F: Fn(&mut MidHookContext) + Send + Sync + 'static,
{
    use iced_x86::{Decoder, DecoderOptions, Instruction};

    tracing::debug!("Creating mid-hook '{}' at {:x}", name, target as usize);

    // Decode instructions at target to find safe cut point
    let mut decoder = Decoder::with_ip(
        64,
        std::slice::from_raw_parts(target, 32),
        target as u64,
        DecoderOptions::NONE,
    );

    let mut instructions: Vec<Instruction> = Vec::new();
    let mut total_size = 0usize;

    while total_size < MIN_HOOK_SIZE {
        let instr = decoder.decode();
        if instr.is_invalid() {
            return Err(HookError::InvalidAddress(target as usize));
        }
        total_size += instr.len();
        instructions.push(instr);
    }

    tracing::debug!(
        "Hook site: {} bytes, {} instructions",
        total_size,
        instructions.len()
    );

    // Allocate trampoline near target
    let trampoline = alloc_trampoline_sized(target, STUB_SIZE).ok_or(
        HookError::MemoryProtection("Failed to allocate trampoline".into()),
    )?;

    // Box the callback and get a stable pointer
    let callback_box: MidHookCallback = Box::new(callback);
    let callback_ptr = &*callback_box as *const _ as *const ();

    // Build the trampoline
    let stub_code = build_mid_hook_stub(
        callback_ptr,
        trampoline.as_ptr() as u64,
        &instructions,
        target as u64 + total_size as u64,
    )?;

    // Copy stub to trampoline
    std::ptr::copy_nonoverlapping(stub_code.as_ptr(), trampoline.as_ptr(), stub_code.len());

    // Save original bytes
    let original_bytes = std::slice::from_raw_parts(target, total_size).to_vec();

    // Make target writable
    region::protect(target, total_size, region::Protection::READ_WRITE_EXECUTE)
        .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

    // Write JMP to trampoline
    let target_mut = target as *mut u8;
    *target_mut = 0xE9; // JMP rel32
    let rel_offset = (trampoline.as_ptr() as i64 - (target as i64 + 5)) as i32;
    std::ptr::copy_nonoverlapping(&rel_offset as *const i32 as *const u8, target_mut.add(1), 4);

    // Fill remaining bytes with NOPs
    for i in 5..total_size {
        *target_mut.add(i) = 0x90;
    }

    // Restore protection
    region::protect(target, total_size, region::Protection::READ_EXECUTE)
        .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

    let entry = MidHookEntry {
        target,
        trampoline,
        trampoline_size: stub_code.len(),
        original_bytes,
        callback: callback_box,
        callback_ptr,
        enabled: true,
        name: name.to_string(),
    };

    let key = MID_HOOKS.write().insert(entry);

    tracing::info!("Created mid-hook '{}' at {:x}", name, target as usize);

    Ok(key)
}

/// Build the mid-hook trampoline stub
#[cfg(unix)]
fn build_mid_hook_stub(
    callback_ptr: *const (),
    trampoline_base: u64,
    original_instructions: &[iced_x86::Instruction],
    return_address: u64,
) -> Result<Vec<u8>, HookError> {
    let mut code = Vec::with_capacity(STUB_SIZE);

    // System V AMD64 ABI trampoline:
    // 1. Save all GPRs to build MidHookContext
    // 2. Save XMM registers
    // 3. Call Rust callback wrapper
    // 4. Restore all registers
    // 5. Execute relocated original instructions
    // 6. JMP back to original code

    // Push all GPRs (in reverse order of MidHookContext fields for easy access)
    code.extend_from_slice(&[
        0x50, // push rax
        0x53, // push rbx
        0x51, // push rcx
        0x52, // push rdx
        0x55, // push rbp
        0x56, // push rsi
        0x57, // push rdi
        0x41, 0x50, // push r8
        0x41, 0x51, // push r9
        0x41, 0x52, // push r10
        0x41, 0x53, // push r11
        0x41, 0x54, // push r12
        0x41, 0x55, // push r13
        0x41, 0x56, // push r14
        0x41, 0x57, // push r15
        0x9C, // pushfq (RFLAGS)
    ]);

    // Allocate space for XMM registers (256 bytes, aligned)
    // sub rsp, 256
    code.extend_from_slice(&[0x48, 0x81, 0xEC, 0x00, 0x01, 0x00, 0x00]);

    // Save XMM0-15 using movups (unaligned, safer)
    for i in 0..8 {
        // movups [rsp + i*16], xmmi
        code.extend_from_slice(&[0x0F, 0x11, 0x44 + (i / 2) * 8, 0x24, (i * 16) as u8]);
    }
    // XMM8-15 need REX.R prefix
    for i in 0..8 {
        // movups [rsp + (i+8)*16], xmm(i+8)
        let offset = ((i + 8) * 16) as u8;
        if offset < 128 {
            code.extend_from_slice(&[0x44, 0x0F, 0x11, 0x44, 0x24, offset]);
        } else {
            code.extend_from_slice(&[0x44, 0x0F, 0x11, 0x84, 0x24, offset, 0x00, 0x00, 0x00]);
        }
    }

    // Now RSP points to MidHookContext (xmm array, then rflags, then GPRs)
    // RDI = context pointer (first arg) = RSP
    // mov rdi, rsp
    code.extend_from_slice(&[0x48, 0x89, 0xE7]);

    // RSI = callback pointer (second arg)
    // mov rsi, callback_ptr
    code.extend_from_slice(&[0x48, 0xBE]);
    code.extend_from_slice(&(callback_ptr as u64).to_le_bytes());

    // Align stack to 16 bytes before call
    // Save current RSP
    // mov rbp, rsp
    code.extend_from_slice(&[0x48, 0x89, 0xE5]);
    // and rsp, -16
    code.extend_from_slice(&[0x48, 0x83, 0xE4, 0xF0]);

    // Call the callback wrapper
    // mov rax, mid_hook_callback_wrapper
    code.extend_from_slice(&[0x48, 0xB8]);
    code.extend_from_slice(&(mid_hook_callback_wrapper as u64).to_le_bytes());
    // call rax
    code.extend_from_slice(&[0xFF, 0xD0]);

    // Restore RSP
    // mov rsp, rbp
    code.extend_from_slice(&[0x48, 0x89, 0xEC]);

    // Restore XMM0-15
    for i in 0..8 {
        // movups xmmi, [rsp + i*16]
        code.extend_from_slice(&[0x0F, 0x10, 0x44 + (i / 2) * 8, 0x24, (i * 16) as u8]);
    }
    for i in 0..8 {
        let offset = ((i + 8) * 16) as u8;
        if offset < 128 {
            code.extend_from_slice(&[0x44, 0x0F, 0x10, 0x44, 0x24, offset]);
        } else {
            code.extend_from_slice(&[0x44, 0x0F, 0x10, 0x84, 0x24, offset, 0x00, 0x00, 0x00]);
        }
    }

    // Deallocate XMM space
    // add rsp, 256
    code.extend_from_slice(&[0x48, 0x81, 0xC4, 0x00, 0x01, 0x00, 0x00]);

    // Restore RFLAGS and GPRs
    code.extend_from_slice(&[
        0x9D, // popfq
        0x41, 0x5F, // pop r15
        0x41, 0x5E, // pop r14
        0x41, 0x5D, // pop r13
        0x41, 0x5C, // pop r12
        0x41, 0x5B, // pop r11
        0x41, 0x5A, // pop r10
        0x41, 0x59, // pop r9
        0x41, 0x58, // pop r8
        0x5F, // pop rdi
        0x5E, // pop rsi
        0x5D, // pop rbp
        0x5A, // pop rdx
        0x59, // pop rcx
        0x5B, // pop rbx
        0x58, // pop rax
    ]);

    // Relocate and append original instructions
    let current_ip = trampoline_base + code.len() as u64;
    let relocated = relocate_instructions(original_instructions, current_ip)?;
    code.extend_from_slice(&relocated);

    // JMP back to original function (after hooked bytes)
    // jmp rel32
    code.push(0xE9);
    let jmp_offset =
        (return_address as i64 - (trampoline_base as i64 + code.len() as i64 + 4)) as i32;
    code.extend_from_slice(&jmp_offset.to_le_bytes());

    Ok(code)
}

#[cfg(windows)]
fn build_mid_hook_stub(
    callback_ptr: *const (),
    trampoline_base: u64,
    original_instructions: &[iced_x86::Instruction],
    return_address: u64,
) -> Result<Vec<u8>, HookError> {
    let mut code = Vec::with_capacity(STUB_SIZE);

    // Windows x64 ABI trampoline
    // Similar to Unix but uses RCX, RDX for first two args

    // Push all GPRs
    code.extend_from_slice(&[
        0x50, // push rax
        0x53, // push rbx
        0x51, // push rcx
        0x52, // push rdx
        0x55, // push rbp
        0x56, // push rsi
        0x57, // push rdi
        0x41, 0x50, // push r8
        0x41, 0x51, // push r9
        0x41, 0x52, // push r10
        0x41, 0x53, // push r11
        0x41, 0x54, // push r12
        0x41, 0x55, // push r13
        0x41, 0x56, // push r14
        0x41, 0x57, // push r15
        0x9C, // pushfq
    ]);

    // Allocate XMM space
    code.extend_from_slice(&[0x48, 0x81, 0xEC, 0x00, 0x01, 0x00, 0x00]);

    // Save XMM registers (same as Unix)
    for i in 0..8 {
        code.extend_from_slice(&[0x0F, 0x11, 0x44 + (i / 2) * 8, 0x24, (i * 16) as u8]);
    }
    for i in 0..8 {
        let offset = ((i + 8) * 16) as u8;
        if offset < 128 {
            code.extend_from_slice(&[0x44, 0x0F, 0x11, 0x44, 0x24, offset]);
        } else {
            code.extend_from_slice(&[0x44, 0x0F, 0x11, 0x84, 0x24, offset, 0x00, 0x00, 0x00]);
        }
    }

    // RCX = context pointer (first arg) = RSP
    code.extend_from_slice(&[0x48, 0x89, 0xE1]);

    // RDX = callback pointer (second arg)
    code.extend_from_slice(&[0x48, 0xBA]);
    code.extend_from_slice(&(callback_ptr as u64).to_le_bytes());

    // Save RSP and align
    code.extend_from_slice(&[0x48, 0x89, 0xE5]);
    code.extend_from_slice(&[0x48, 0x83, 0xE4, 0xF0]);

    // Allocate shadow space (32 bytes)
    code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x20]);

    // Call wrapper
    code.extend_from_slice(&[0x48, 0xB8]);
    code.extend_from_slice(&(mid_hook_callback_wrapper as u64).to_le_bytes());
    code.extend_from_slice(&[0xFF, 0xD0]);

    // Deallocate shadow space
    code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x20]);

    // Restore RSP
    code.extend_from_slice(&[0x48, 0x89, 0xEC]);

    // Restore XMM registers
    for i in 0..8 {
        code.extend_from_slice(&[0x0F, 0x10, 0x44 + (i / 2) * 8, 0x24, (i * 16) as u8]);
    }
    for i in 0..8 {
        let offset = ((i + 8) * 16) as u8;
        if offset < 128 {
            code.extend_from_slice(&[0x44, 0x0F, 0x10, 0x44, 0x24, offset]);
        } else {
            code.extend_from_slice(&[0x44, 0x0F, 0x10, 0x84, 0x24, offset, 0x00, 0x00, 0x00]);
        }
    }

    // Deallocate XMM space
    code.extend_from_slice(&[0x48, 0x81, 0xC4, 0x00, 0x01, 0x00, 0x00]);

    // Restore GPRs
    code.extend_from_slice(&[
        0x9D, 0x41, 0x5F, 0x41, 0x5E, 0x41, 0x5D, 0x41, 0x5C, 0x41, 0x5B, 0x41, 0x5A, 0x41, 0x59,
        0x41, 0x58, 0x5F, 0x5E, 0x5D, 0x5A, 0x59, 0x5B, 0x58,
    ]);

    // Relocate original instructions
    let current_ip = trampoline_base + code.len() as u64;
    let relocated = relocate_instructions(original_instructions, current_ip)?;
    code.extend_from_slice(&relocated);

    // JMP back
    code.push(0xE9);
    let jmp_offset =
        (return_address as i64 - (trampoline_base as i64 + code.len() as i64 + 4)) as i32;
    code.extend_from_slice(&jmp_offset.to_le_bytes());

    Ok(code)
}

/// Relocate instructions to a new address using iced-x86 BlockEncoder
fn relocate_instructions(
    instructions: &[iced_x86::Instruction],
    new_address: u64,
) -> Result<Vec<u8>, HookError> {
    use iced_x86::{BlockEncoder, BlockEncoderOptions, InstructionBlock};

    let block = InstructionBlock::new(instructions, new_address);

    let result = BlockEncoder::encode(64, block, BlockEncoderOptions::NONE)
        .map_err(|e| HookError::DetourCreation(format!("Relocation failed: {:?}", e)))?;

    Ok(result.code_buffer)
}

/// Disable a mid-function hook
pub fn disable_mid_hook(key: MidHookKey) -> Result<(), HookError> {
    let mut hooks = MID_HOOKS.write();
    let entry = hooks.get_mut(key).ok_or(HookError::NotFound)?;

    if !entry.enabled {
        return Ok(());
    }

    unsafe {
        // Restore original bytes
        region::protect(
            entry.target,
            entry.original_bytes.len(),
            region::Protection::READ_WRITE_EXECUTE,
        )
        .map_err(|e| HookError::MemoryProtection(e.to_string()))?;

        std::ptr::copy_nonoverlapping(
            entry.original_bytes.as_ptr(),
            entry.target as *mut u8,
            entry.original_bytes.len(),
        );

        region::protect(
            entry.target,
            entry.original_bytes.len(),
            region::Protection::READ_EXECUTE,
        )
        .map_err(|e| HookError::MemoryProtection(e.to_string()))?;
    }

    entry.enabled = false;
    tracing::info!("Disabled mid-hook '{}'", entry.name);

    Ok(())
}

/// Check if a mid-hook is enabled
pub fn is_mid_hook_enabled(key: MidHookKey) -> bool {
    MID_HOOKS
        .read()
        .get(key)
        .map(|e| e.enabled)
        .unwrap_or(false)
}

/// Remove a mid-function hook completely
pub fn remove_mid_hook(key: MidHookKey) -> Result<(), HookError> {
    disable_mid_hook(key)?;

    let mut hooks = MID_HOOKS.write();
    let entry = hooks.remove(key).ok_or(HookError::NotFound)?;

    // Note: Trampoline memory is not freed (would need deallocation tracking)

    tracing::info!("Removed mid-hook '{}'", entry.name);
    Ok(())
}
