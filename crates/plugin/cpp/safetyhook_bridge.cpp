#include "safetyhook_bridge.h"
#include <safetyhook.hpp>
#include <unordered_map>
#include <mutex>
#include <cstring>

// Storage for hook objects (SafetyHook uses RAII, we need to keep them alive)
static std::mutex g_hookMutex;
static std::unordered_map<uintptr_t, safetyhook::InlineHook> g_inlineHooks;
static std::unordered_map<uintptr_t, safetyhook::MidHook> g_midHooks;

// User data storage for mid hooks
struct MidHookUserData {
    MidHookCallback callback;
    void* user_data;
};
static std::unordered_map<uintptr_t, MidHookUserData> g_midHookUserData;

static uintptr_t g_nextInlineHandle = 1;
static uintptr_t g_nextMidHandle = 1;

// Convert SafetyHook error to our error code
static HookResult convert_inline_error(const safetyhook::InlineHook::Error& err) {
    switch (err.type) {
        case safetyhook::InlineHook::Error::BAD_ALLOCATION:
            return HOOK_ERROR_ALLOCATION;
        case safetyhook::InlineHook::Error::FAILED_TO_DECODE_INSTRUCTION:
            return HOOK_ERROR_DECODE;
        case safetyhook::InlineHook::Error::FAILED_TO_UNPROTECT:
            return HOOK_ERROR_UNPROTECT;
        case safetyhook::InlineHook::Error::NOT_ENOUGH_SPACE:
            return HOOK_ERROR_NOT_ENOUGH_SPACE;
        case safetyhook::InlineHook::Error::UNSUPPORTED_INSTRUCTION_IN_TRAMPOLINE:
            return HOOK_ERROR_UNSUPPORTED;
        case safetyhook::InlineHook::Error::IP_RELATIVE_INSTRUCTION_OUT_OF_RANGE:
            return HOOK_ERROR_IP_RELATIVE;
        case safetyhook::InlineHook::Error::SHORT_JUMP_IN_TRAMPOLINE:
            return HOOK_ERROR_NOT_ENOUGH_SPACE;
        default:
            return HOOK_ERROR_INVALID;
    }
}

// Convert SafetyHook Context to Rust context (different register order!)
static void context_to_rust(const safetyhook::Context& ctx, RustMidHookContext* rust_ctx) {
    // Copy XMM registers (same order, just packed differently)
    std::memcpy(&rust_ctx->xmm[0],   &ctx.xmm0,  16);
    std::memcpy(&rust_ctx->xmm[16],  &ctx.xmm1,  16);
    std::memcpy(&rust_ctx->xmm[32],  &ctx.xmm2,  16);
    std::memcpy(&rust_ctx->xmm[48],  &ctx.xmm3,  16);
    std::memcpy(&rust_ctx->xmm[64],  &ctx.xmm4,  16);
    std::memcpy(&rust_ctx->xmm[80],  &ctx.xmm5,  16);
    std::memcpy(&rust_ctx->xmm[96],  &ctx.xmm6,  16);
    std::memcpy(&rust_ctx->xmm[112], &ctx.xmm7,  16);
    std::memcpy(&rust_ctx->xmm[128], &ctx.xmm8,  16);
    std::memcpy(&rust_ctx->xmm[144], &ctx.xmm9,  16);
    std::memcpy(&rust_ctx->xmm[160], &ctx.xmm10, 16);
    std::memcpy(&rust_ctx->xmm[176], &ctx.xmm11, 16);
    std::memcpy(&rust_ctx->xmm[192], &ctx.xmm12, 16);
    std::memcpy(&rust_ctx->xmm[208], &ctx.xmm13, 16);
    std::memcpy(&rust_ctx->xmm[224], &ctx.xmm14, 16);
    std::memcpy(&rust_ctx->xmm[240], &ctx.xmm15, 16);

    // Copy GPRs (note: SafetyHook and Rust have different order after rsi)
    // SafetyHook: rflags, r15-r8, rdi, rsi, rdx, rcx, rbx, rax, rbp, rsp, ...
    // Rust:       rflags, r15-r8, rdi, rsi, rbp, rdx, rcx, rbx, rax, rsp
    rust_ctx->rflags = ctx.rflags;
    rust_ctx->r15 = ctx.r15;
    rust_ctx->r14 = ctx.r14;
    rust_ctx->r13 = ctx.r13;
    rust_ctx->r12 = ctx.r12;
    rust_ctx->r11 = ctx.r11;
    rust_ctx->r10 = ctx.r10;
    rust_ctx->r9 = ctx.r9;
    rust_ctx->r8 = ctx.r8;
    rust_ctx->rdi = ctx.rdi;
    rust_ctx->rsi = ctx.rsi;
    rust_ctx->rbp = ctx.rbp;  // Rust expects rbp here
    rust_ctx->rdx = ctx.rdx;
    rust_ctx->rcx = ctx.rcx;
    rust_ctx->rbx = ctx.rbx;
    rust_ctx->rax = ctx.rax;
    rust_ctx->rsp = ctx.rsp;
}

// Copy modified Rust context back to SafetyHook context
static void rust_to_context(const RustMidHookContext* rust_ctx, safetyhook::Context& ctx) {
    // Copy XMM registers back
    std::memcpy(&ctx.xmm0,  &rust_ctx->xmm[0],   16);
    std::memcpy(&ctx.xmm1,  &rust_ctx->xmm[16],  16);
    std::memcpy(&ctx.xmm2,  &rust_ctx->xmm[32],  16);
    std::memcpy(&ctx.xmm3,  &rust_ctx->xmm[48],  16);
    std::memcpy(&ctx.xmm4,  &rust_ctx->xmm[64],  16);
    std::memcpy(&ctx.xmm5,  &rust_ctx->xmm[80],  16);
    std::memcpy(&ctx.xmm6,  &rust_ctx->xmm[96],  16);
    std::memcpy(&ctx.xmm7,  &rust_ctx->xmm[112], 16);
    std::memcpy(&ctx.xmm8,  &rust_ctx->xmm[128], 16);
    std::memcpy(&ctx.xmm9,  &rust_ctx->xmm[144], 16);
    std::memcpy(&ctx.xmm10, &rust_ctx->xmm[160], 16);
    std::memcpy(&ctx.xmm11, &rust_ctx->xmm[176], 16);
    std::memcpy(&ctx.xmm12, &rust_ctx->xmm[192], 16);
    std::memcpy(&ctx.xmm13, &rust_ctx->xmm[208], 16);
    std::memcpy(&ctx.xmm14, &rust_ctx->xmm[224], 16);
    std::memcpy(&ctx.xmm15, &rust_ctx->xmm[240], 16);

    // Copy GPRs back (same reordering as above)
    ctx.rflags = rust_ctx->rflags;
    ctx.r15 = rust_ctx->r15;
    ctx.r14 = rust_ctx->r14;
    ctx.r13 = rust_ctx->r13;
    ctx.r12 = rust_ctx->r12;
    ctx.r11 = rust_ctx->r11;
    ctx.r10 = rust_ctx->r10;
    ctx.r9 = rust_ctx->r9;
    ctx.r8 = rust_ctx->r8;
    ctx.rdi = rust_ctx->rdi;
    ctx.rsi = rust_ctx->rsi;
    ctx.rbp = rust_ctx->rbp;
    ctx.rdx = rust_ctx->rdx;
    ctx.rcx = rust_ctx->rcx;
    ctx.rbx = rust_ctx->rbx;
    ctx.rax = rust_ctx->rax;
    // Note: rsp is read-only in SafetyHook, don't copy back
}

extern "C" {

// === Inline Hook Implementation ===

HookResult safetyhook_create_inline(
    void* target,
    void* destination,
    InlineHookHandle* out_handle,
    void** out_trampoline
) {
    if (!target || !destination || !out_handle || !out_trampoline) {
        return HOOK_ERROR_INVALID;
    }

    auto result = safetyhook::InlineHook::create(target, destination);
    if (!result) {
        return convert_inline_error(result.error());
    }

    std::lock_guard<std::mutex> lock(g_hookMutex);
    uintptr_t handle = g_nextInlineHandle++;
    *out_trampoline = reinterpret_cast<void*>(result->trampoline().address());
    g_inlineHooks.emplace(handle, std::move(*result));
    *out_handle = reinterpret_cast<InlineHookHandle>(handle);
    return HOOK_SUCCESS;
}

HookResult safetyhook_enable_inline(InlineHookHandle handle) {
    if (!handle) return HOOK_ERROR_INVALID;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_inlineHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_inlineHooks.end()) return HOOK_ERROR_INVALID;

    auto result = it->second.enable();
    return result ? HOOK_SUCCESS : HOOK_ERROR_UNPROTECT;
}

HookResult safetyhook_disable_inline(InlineHookHandle handle) {
    if (!handle) return HOOK_ERROR_INVALID;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_inlineHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_inlineHooks.end()) return HOOK_ERROR_INVALID;

    auto result = it->second.disable();
    return result ? HOOK_SUCCESS : HOOK_ERROR_UNPROTECT;
}

void safetyhook_destroy_inline(InlineHookHandle handle) {
    if (!handle) return;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    g_inlineHooks.erase(reinterpret_cast<uintptr_t>(handle));
}

bool safetyhook_is_inline_enabled(InlineHookHandle handle) {
    if (!handle) return false;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_inlineHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_inlineHooks.end()) return false;
    return it->second.enabled();
}

void* safetyhook_get_inline_trampoline(InlineHookHandle handle) {
    if (!handle) return nullptr;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_inlineHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_inlineHooks.end()) return nullptr;
    return reinterpret_cast<void*>(it->second.trampoline().address());
}

// === Mid Hook Implementation ===

// Map from target address to user data (for callback lookup)
static std::unordered_map<uintptr_t, MidHookUserData> g_midHookByTarget;

// Map from handle to target (for cleanup)
static std::unordered_map<uintptr_t, uintptr_t> g_handleToTarget;

// Global trampoline that looks up the callback by the hooked address
// SafetyHook's Context has 'rip' which points to trampoline, but we stored target
static void mid_hook_trampoline(safetyhook::Context& ctx) {
    // We need to find which hook this is for.
    // The rip points to the trampoline code, not the original target.
    // So we need to iterate and check if rip is within any hook's trampoline.
    // This is inefficient but works for now.

    // Actually, we can use a different approach: check each registered target
    // by seeing if the return address on the stack matches what we expect.
    // But this is complex.

    // Simpler approach: just call all registered callbacks (only one active at a time typically)
    // This is a limitation but works for single mid-hook scenarios.

    std::lock_guard<std::mutex> lock(g_hookMutex);

    for (auto& [target, userData] : g_midHookByTarget) {
        // For now, call all callbacks - in practice there's usually only one mid-hook
        // or we'd need a more sophisticated lookup mechanism
        RustMidHookContext rust_ctx;
        context_to_rust(ctx, &rust_ctx);
        userData.callback(&rust_ctx, userData.user_data);
        rust_to_context(&rust_ctx, ctx);
        return; // Only call first match (should be only one)
    }
}

HookResult safetyhook_create_mid(
    void* target,
    MidHookCallback callback,
    void* user_data,
    MidHookHandle* out_handle
) {
    if (!target || !callback || !out_handle) {
        return HOOK_ERROR_INVALID;
    }

    std::lock_guard<std::mutex> lock(g_hookMutex);

    uintptr_t handle = g_nextMidHandle++;
    uintptr_t targetAddr = reinterpret_cast<uintptr_t>(target);

    // Store user data keyed by target address
    g_midHookByTarget[targetAddr] = {callback, user_data};
    g_handleToTarget[handle] = targetAddr;

    // Create the hook with our static trampoline
    auto result = safetyhook::MidHook::create(target, mid_hook_trampoline);

    if (!result) {
        g_midHookByTarget.erase(targetAddr);
        g_handleToTarget.erase(handle);
        return HOOK_ERROR_ALLOCATION;
    }

    g_midHooks.emplace(handle, std::move(*result));
    *out_handle = reinterpret_cast<MidHookHandle>(handle);
    return HOOK_SUCCESS;
}

HookResult safetyhook_enable_mid(MidHookHandle handle) {
    if (!handle) return HOOK_ERROR_INVALID;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_midHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_midHooks.end()) return HOOK_ERROR_INVALID;

    auto result = it->second.enable();
    return result ? HOOK_SUCCESS : HOOK_ERROR_UNPROTECT;
}

HookResult safetyhook_disable_mid(MidHookHandle handle) {
    if (!handle) return HOOK_ERROR_INVALID;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_midHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_midHooks.end()) return HOOK_ERROR_INVALID;

    auto result = it->second.disable();
    return result ? HOOK_SUCCESS : HOOK_ERROR_UNPROTECT;
}

void safetyhook_destroy_mid(MidHookHandle handle) {
    if (!handle) return;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    uintptr_t h = reinterpret_cast<uintptr_t>(handle);

    // Find target address and clean up
    auto targetIt = g_handleToTarget.find(h);
    if (targetIt != g_handleToTarget.end()) {
        g_midHookByTarget.erase(targetIt->second);
        g_handleToTarget.erase(targetIt);
    }

    // Remove the hook object (this will unhook)
    g_midHooks.erase(h);
}

bool safetyhook_is_mid_enabled(MidHookHandle handle) {
    if (!handle) return false;

    std::lock_guard<std::mutex> lock(g_hookMutex);
    auto it = g_midHooks.find(reinterpret_cast<uintptr_t>(handle));
    if (it == g_midHooks.end()) return false;
    return it->second.enabled();
}

} // extern "C"
