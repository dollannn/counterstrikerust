//! Gamedata system for loading signatures and offsets from JSON
//!
//! Signatures are loaded from a gamedata.json file deployed with the plugin.
//! This allows updating signatures without recompiling.

use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;
use thiserror::Error;

/// Errors that can occur when loading gamedata
#[derive(Debug, Error)]
pub enum GamedataError {
    #[error("Failed to read gamedata file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse gamedata JSON: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Signature not found: {0}")]
    SignatureNotFound(String),

    #[error("Offset not found: {0}")]
    OffsetNotFound(String),

    #[error("Invalid signature format: {0}")]
    InvalidSignature(String),

    #[error("Failed to find signature in memory: {0}")]
    ScanFailed(String),
}

/// Platform-specific signature entry
#[derive(Debug, Deserialize)]
pub struct SignatureEntry {
    /// Library to scan (e.g., "server", "engine")
    #[serde(default = "default_library")]
    pub library: String,
    /// Windows signature pattern
    pub windows: Option<String>,
    /// Linux signature pattern
    pub linux: Option<String>,
}

fn default_library() -> String {
    "server".to_string()
}

/// Platform-specific offset entry
#[derive(Debug, Deserialize)]
pub struct OffsetEntry {
    /// Windows offset value
    pub windows: Option<i64>,
    /// Linux offset value
    pub linux: Option<i64>,
}

/// Combined gamedata entry that can be either signatures or offsets
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum GamedataEntry {
    Signatures { signatures: SignatureEntry },
    Offsets { offsets: OffsetEntry },
}

/// Loaded gamedata
#[derive(Debug, Default)]
pub struct Gamedata {
    signatures: HashMap<String, SignatureEntry>,
    offsets: HashMap<String, OffsetEntry>,
}

/// Global gamedata instance
static GAMEDATA: OnceLock<Gamedata> = OnceLock::new();

impl Gamedata {
    /// Load gamedata from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, GamedataError> {
        let content = std::fs::read_to_string(path)?;
        Self::load_from_str(&content)
    }

    /// Load gamedata from a JSON string
    pub fn load_from_str(json: &str) -> Result<Self, GamedataError> {
        let raw: HashMap<String, serde_json::Value> = serde_json::from_str(json)?;

        let mut gamedata = Gamedata::default();

        for (name, value) in raw {
            // Check if it has "signatures" key
            if value.get("signatures").is_some() {
                let entry: SignatureEntry = serde_json::from_value(value["signatures"].clone())?;
                gamedata.signatures.insert(name, entry);
            }
            // Check if it has "offsets" key
            else if value.get("offsets").is_some() {
                let entry: OffsetEntry = serde_json::from_value(value["offsets"].clone())?;
                gamedata.offsets.insert(name, entry);
            }
            // Assume it's a signature entry directly (CSS format)
            else if value.get("linux").is_some() || value.get("windows").is_some() {
                let entry: SignatureEntry = serde_json::from_value(value)?;
                gamedata.signatures.insert(name, entry);
            }
        }

        tracing::info!(
            "Loaded gamedata: {} signatures, {} offsets",
            gamedata.signatures.len(),
            gamedata.offsets.len()
        );

        Ok(gamedata)
    }

    /// Get a signature by name for the current platform
    pub fn get_signature(&self, name: &str) -> Result<&str, GamedataError> {
        let entry = self
            .signatures
            .get(name)
            .ok_or_else(|| GamedataError::SignatureNotFound(name.to_string()))?;

        #[cfg(target_os = "linux")]
        let sig = entry.linux.as_deref();

        #[cfg(target_os = "windows")]
        let sig = entry.windows.as_deref();

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let sig: Option<&str> = None;

        sig.ok_or_else(|| {
            GamedataError::SignatureNotFound(format!("{} (no signature for this platform)", name))
        })
    }

    /// Get an offset by name for the current platform
    pub fn get_offset(&self, name: &str) -> Result<i64, GamedataError> {
        let entry = self
            .offsets
            .get(name)
            .ok_or_else(|| GamedataError::OffsetNotFound(name.to_string()))?;

        #[cfg(target_os = "linux")]
        let offset = entry.linux;

        #[cfg(target_os = "windows")]
        let offset = entry.windows;

        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        let offset: Option<i64> = None;

        offset.ok_or_else(|| {
            GamedataError::OffsetNotFound(format!("{} (no offset for this platform)", name))
        })
    }

    /// Get the library name for a signature
    pub fn get_signature_library(&self, name: &str) -> Option<&str> {
        self.signatures.get(name).map(|e| e.library.as_str())
    }
}

/// Initialize global gamedata from file
pub fn init_gamedata<P: AsRef<Path>>(path: P) -> Result<(), GamedataError> {
    let gd = Gamedata::load_from_file(path)?;
    GAMEDATA
        .set(gd)
        .map_err(|_| GamedataError::IoError(std::io::Error::other("Gamedata already initialized")))
}

/// Get the global gamedata instance
pub fn gamedata() -> Option<&'static Gamedata> {
    GAMEDATA.get()
}

/// Parse a signature pattern string into bytes
///
/// Supports:
/// - Hex bytes: "55 48 89 E5"
/// - Wildcards: "55 ? 89 E5" or "55 ?? 89 E5"
pub fn parse_signature(pattern: &str) -> Result<Vec<Option<u8>>, GamedataError> {
    let mut result = Vec::new();

    for part in pattern.split_whitespace() {
        if part == "?" || part == "??" {
            result.push(None); // Wildcard
        } else {
            let byte = u8::from_str_radix(part, 16).map_err(|_| {
                GamedataError::InvalidSignature(format!("Invalid hex byte: {}", part))
            })?;
            result.push(Some(byte));
        }
    }

    if result.is_empty() {
        return Err(GamedataError::InvalidSignature(
            "Empty signature pattern".to_string(),
        ));
    }

    Ok(result)
}

/// Scan memory for a signature pattern
///
/// # Safety
/// The memory region must be valid and readable.
pub unsafe fn scan_signature(
    start: *const u8,
    size: usize,
    pattern: &[Option<u8>],
) -> Option<*const u8> {
    if pattern.is_empty() || size < pattern.len() {
        return None;
    }

    let end = size - pattern.len();

    'outer: for offset in 0..=end {
        for (i, expected) in pattern.iter().enumerate() {
            if let Some(byte) = expected {
                let actual = *start.add(offset + i);
                if actual != *byte {
                    continue 'outer;
                }
            }
        }
        // All bytes matched
        return Some(start.add(offset));
    }

    None
}

/// Find a function address by signature name
///
/// # Arguments
/// * `name` - Signature name in gamedata
/// * `module_base` - Base address of the module to scan
/// * `module_size` - Size of the module
///
/// # Safety
/// Module memory must be valid and readable.
pub unsafe fn find_signature(
    name: &str,
    module_base: *const u8,
    module_size: usize,
) -> Result<*const u8, GamedataError> {
    let gd = gamedata()
        .ok_or_else(|| GamedataError::IoError(std::io::Error::other("Gamedata not initialized")))?;

    let sig_str = gd.get_signature(name)?;
    let pattern = parse_signature(sig_str)?;

    scan_signature(module_base, module_size, &pattern)
        .ok_or_else(|| GamedataError::ScanFailed(name.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_signature() {
        let pattern = parse_signature("55 48 89 E5").unwrap();
        assert_eq!(
            pattern,
            vec![Some(0x55), Some(0x48), Some(0x89), Some(0xE5)]
        );

        let pattern = parse_signature("55 ? 89 ??").unwrap();
        assert_eq!(pattern, vec![Some(0x55), None, Some(0x89), None]);
    }

    #[test]
    fn test_scan_signature() {
        let data = [0x00, 0x55, 0x48, 0x89, 0xE5, 0x00];
        let pattern = vec![Some(0x55), Some(0x48), Some(0x89), Some(0xE5)];

        unsafe {
            let result = scan_signature(data.as_ptr(), data.len(), &pattern);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), data.as_ptr().add(1));
        }
    }

    #[test]
    fn test_scan_signature_with_wildcard() {
        let data = [0x00, 0x55, 0xFF, 0x89, 0xE5, 0x00];
        let pattern = vec![Some(0x55), None, Some(0x89), Some(0xE5)];

        unsafe {
            let result = scan_signature(data.as_ptr(), data.len(), &pattern);
            assert!(result.is_some());
            assert_eq!(result.unwrap(), data.as_ptr().add(1));
        }
    }

    #[test]
    fn test_load_gamedata_css_format() {
        let json = r#"{
            "Host_Say": {
                "library": "server",
                "linux": "55 48 89 E5 41 57 49 89 F7",
                "windows": "44 89 4C 24 20"
            },
            "ClientPrint": {
                "library": "server",
                "linux": "55 48 8D 05 ? ? ? ?",
                "windows": "48 85 C9 0F 84"
            }
        }"#;

        let gd = Gamedata::load_from_str(json).unwrap();
        assert_eq!(gd.signatures.len(), 2);

        #[cfg(target_os = "linux")]
        {
            let sig = gd.get_signature("Host_Say").unwrap();
            assert!(sig.starts_with("55 48"));
        }
    }
}
