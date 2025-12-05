//! FNV-1a hash functions for schema field lookup
//!
//! Source 2's schema system uses FNV-1a hashes as keys for fast lookup.

/// FNV-1a 32-bit hash (compile-time capable)
pub const fn fnv1a_32(data: &[u8]) -> u32 {
    const FNV_OFFSET_BASIS: u32 = 0x811c9dc5;
    const FNV_PRIME: u32 = 0x01000193;

    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u32;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// FNV-1a 64-bit hash (compile-time capable)
pub const fn fnv1a_64(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;

    let mut hash = FNV_OFFSET_BASIS;
    let mut i = 0;
    while i < data.len() {
        hash ^= data[i] as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
        i += 1;
    }
    hash
}

/// Combined class+field hash for cache key
///
/// Uses 32-bit hashes for class and field, combined into a 64-bit key.
/// This provides efficient lookup while avoiding hash collisions between
/// different class/field combinations.
pub const fn combined_hash(class_name: &[u8], field_name: &[u8]) -> u64 {
    let class_hash = fnv1a_32(class_name);
    let field_hash = fnv1a_32(field_name);
    ((class_hash as u64) << 32) | (field_hash as u64)
}

/// Hash a string at runtime
#[inline]
pub fn hash_str(s: &str) -> u32 {
    fnv1a_32(s.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_32_empty() {
        // Empty string should return offset basis
        assert_eq!(fnv1a_32(b""), 0x811c9dc5);
    }

    #[test]
    fn test_fnv1a_32_basic() {
        // Known test vectors
        assert_eq!(fnv1a_32(b"a"), 0xe40c292c);
        assert_eq!(fnv1a_32(b"foobar"), 0xbf9cf968);
    }

    #[test]
    fn test_combined_hash_unique() {
        // Different class/field combinations should produce different hashes
        let hash1 = combined_hash(b"CBaseEntity", b"m_iHealth");
        let hash2 = combined_hash(b"CBaseEntity", b"m_iTeamNum");
        let hash3 = combined_hash(b"CCSPlayerPawn", b"m_iHealth");

        assert_ne!(hash1, hash2);
        assert_ne!(hash1, hash3);
        assert_ne!(hash2, hash3);
    }

    #[test]
    fn test_const_evaluation() {
        // Verify these can be computed at compile time
        const HASH: u32 = fnv1a_32(b"test");
        const COMBINED: u64 = combined_hash(b"class", b"field");
        assert!(HASH != 0);
        assert!(COMBINED != 0);
    }
}
