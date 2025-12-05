//! CPU context for mid-function hooks
//!
//! Provides access to all x86_64 registers when executing mid-function hooks.

/// XMM register (128-bit SIMD)
#[repr(C, align(16))]
#[derive(Clone, Copy)]
pub struct Xmm {
    pub data: [u8; 16],
}

impl Xmm {
    /// Interpret as 4 single-precision floats
    pub fn as_f32x4(&self) -> [f32; 4] {
        let bytes: [[u8; 4]; 4] = [
            self.data[0..4].try_into().unwrap(),
            self.data[4..8].try_into().unwrap(),
            self.data[8..12].try_into().unwrap(),
            self.data[12..16].try_into().unwrap(),
        ];
        bytes.map(f32::from_le_bytes)
    }

    /// Interpret as 2 double-precision floats
    pub fn as_f64x2(&self) -> [f64; 2] {
        let bytes: [[u8; 8]; 2] = [
            self.data[0..8].try_into().unwrap(),
            self.data[8..16].try_into().unwrap(),
        ];
        bytes.map(f64::from_le_bytes)
    }

    /// Interpret as 2 64-bit integers
    pub fn as_u64x2(&self) -> [u64; 2] {
        let bytes: [[u8; 8]; 2] = [
            self.data[0..8].try_into().unwrap(),
            self.data[8..16].try_into().unwrap(),
        ];
        bytes.map(u64::from_le_bytes)
    }

    /// Set from 4 single-precision floats
    pub fn set_f32x4(&mut self, values: [f32; 4]) {
        for (i, v) in values.iter().enumerate() {
            self.data[i * 4..(i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
    }

    /// Set from 2 double-precision floats
    pub fn set_f64x2(&mut self, values: [f64; 2]) {
        for (i, v) in values.iter().enumerate() {
            self.data[i * 8..(i + 1) * 8].copy_from_slice(&v.to_le_bytes());
        }
    }
}

impl Default for Xmm {
    fn default() -> Self {
        Self { data: [0u8; 16] }
    }
}

impl std::fmt::Debug for Xmm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Xmm({:02x?})", &self.data[..])
    }
}

/// Full CPU context for x86_64 mid-function hooks
///
/// Layout matches the assembly stub's push order for direct memory mapping.
/// Modifications to this structure are reflected when the hook returns.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct MidHookContext {
    // XMM registers (saved first, 256 bytes total)
    pub xmm: [Xmm; 16],

    // RFLAGS (pushed before GPRs)
    pub rflags: u64,

    // General purpose registers (in push order)
    pub r15: u64,
    pub r14: u64,
    pub r13: u64,
    pub r12: u64,
    pub r11: u64,
    pub r10: u64,
    pub r9: u64,
    pub r8: u64,
    pub rdi: u64,
    pub rsi: u64,
    pub rbp: u64,
    pub rdx: u64,
    pub rcx: u64,
    pub rbx: u64,
    pub rax: u64,

    // Stack pointer (read-only, modification undefined)
    pub rsp: u64,
}

impl MidHookContext {
    /// Get the return address (on stack at RSP)
    pub fn return_address(&self) -> u64 {
        unsafe { *(self.rsp as *const u64) }
    }

    /// Get argument by index (System V AMD64 ABI)
    /// Arguments: RDI, RSI, RDX, RCX, R8, R9, then stack
    #[cfg(unix)]
    pub fn arg(&self, index: usize) -> u64 {
        match index {
            0 => self.rdi,
            1 => self.rsi,
            2 => self.rdx,
            3 => self.rcx,
            4 => self.r8,
            5 => self.r9,
            n => {
                // Stack arguments start at RSP + 8 (after return address)
                let stack_index = n - 6;
                unsafe { *((self.rsp as *const u64).add(1 + stack_index)) }
            }
        }
    }

    /// Get argument by index (Windows x64 ABI)
    /// Arguments: RCX, RDX, R8, R9, then stack
    #[cfg(windows)]
    pub fn arg(&self, index: usize) -> u64 {
        match index {
            0 => self.rcx,
            1 => self.rdx,
            2 => self.r8,
            3 => self.r9,
            n => {
                // Stack arguments start at RSP + 40 (shadow space + return)
                let stack_index = n - 4;
                unsafe { *((self.rsp as *const u64).add(5 + stack_index)) }
            }
        }
    }

    /// Set argument by index (System V AMD64 ABI)
    #[cfg(unix)]
    pub fn set_arg(&mut self, index: usize, value: u64) {
        match index {
            0 => self.rdi = value,
            1 => self.rsi = value,
            2 => self.rdx = value,
            3 => self.rcx = value,
            4 => self.r8 = value,
            5 => self.r9 = value,
            _ => tracing::warn!("Cannot set stack argument {} via context", index),
        }
    }

    /// Set argument by index (Windows x64 ABI)
    #[cfg(windows)]
    pub fn set_arg(&mut self, index: usize, value: u64) {
        match index {
            0 => self.rcx = value,
            1 => self.rdx = value,
            2 => self.r8 = value,
            3 => self.r9 = value,
            _ => tracing::warn!("Cannot set stack argument {} via context", index),
        }
    }

    /// Get float argument from XMM register (System V: XMM0-7, Windows: XMM0-3)
    pub fn float_arg(&self, index: usize) -> f64 {
        if index < 16 {
            self.xmm[index].as_f64x2()[0]
        } else {
            0.0
        }
    }

    /// Set float argument in XMM register
    pub fn set_float_arg(&mut self, index: usize, value: f64) {
        if index < 16 {
            self.xmm[index].set_f64x2([value, 0.0]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xmm_f32_conversions() {
        let mut xmm = Xmm::default();

        xmm.set_f32x4([1.0, 2.0, 3.0, 4.0]);
        let floats = xmm.as_f32x4();
        assert_eq!(floats, [1.0, 2.0, 3.0, 4.0]);
    }

    #[test]
    fn test_xmm_f64_conversions() {
        let mut xmm = Xmm::default();

        xmm.set_f64x2([1.5, 2.5]);
        let doubles = xmm.as_f64x2();
        assert!((doubles[0] - 1.5).abs() < 0.001);
        assert!((doubles[1] - 2.5).abs() < 0.001);
    }

    #[test]
    fn test_xmm_u64_conversions() {
        let mut xmm = Xmm::default();

        xmm.set_f64x2([0.0, 0.0]);
        // Set raw bytes for u64 values
        let val1: u64 = 0x1234567890ABCDEF;
        let val2: u64 = 0xFEDCBA0987654321;
        xmm.data[0..8].copy_from_slice(&val1.to_le_bytes());
        xmm.data[8..16].copy_from_slice(&val2.to_le_bytes());

        let ints = xmm.as_u64x2();
        assert_eq!(ints[0], val1);
        assert_eq!(ints[1], val2);
    }
}
