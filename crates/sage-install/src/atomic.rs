//! Atomic file writes: temp-file + rename.
//!
//! A crash mid-write must never leave a half-written `.claude/`
//! config file on disk. We write to a sibling temp file with a random
//! suffix, then `fs::rename` it over the target. The kernel guarantees
//! `rename` is atomic within a single VFS scheme.

use astrid_sdk::prelude::*;

/// Suffix length appended to the temp filename. 4 bytes -> 8 hex
/// chars; cheap and large enough to avoid collisions across the few
/// files the installer writes per principal.
const RANDOM_SUFFIX_LEN: usize = 4;

/// Write `bytes` to `path` atomically.
///
/// 1. Derive a sibling temp path `.<basename>.tmp.<hex>` where `<hex>`
///    is 8 hex chars sourced from the host CSPRNG.
/// 2. Write to the temp path.
/// 3. Rename onto `path`.
///
/// On rename failure the temp file is best-effort removed so it
/// doesn't accumulate.
pub(crate) fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), SysError> {
    let temp = temp_sibling(path)?;
    fs::write(&temp, bytes)?;
    if let Err(e) = fs::rename(&temp, path) {
        // Best-effort cleanup. Ignore the cleanup result because the
        // user-visible failure is the original rename, not the unlink.
        let _ = fs::remove_file(&temp);
        return Err(e);
    }
    Ok(())
}

/// Best-effort removal of the temp sibling for `path`. Used by the
/// failure path of the install handler to scrub partial writes; never
/// returns an error because we may be cleaning up files that never
/// existed.
pub(crate) fn cleanup_temp(path: &str) {
    let prefix = match temp_prefix(path) {
        Some(p) => p,
        None => return,
    };
    let parent = match parent_dir(path) {
        Some(p) => p,
        None => return,
    };
    let Ok(entries) = fs::read_dir(&parent) else {
        return;
    };
    for entry in entries {
        if entry.file_name().starts_with(&prefix) {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Compute the parent directory portion of `path`.
fn parent_dir(path: &str) -> Option<String> {
    let idx = path.rfind('/')?;
    Some(path[..idx].to_string())
}

/// Compute `.<basename>.tmp.` — the prefix every temp sibling for
/// `path` shares.
fn temp_prefix(path: &str) -> Option<String> {
    let basename = match path.rfind('/') {
        Some(idx) => path.get(idx + 1..)?,
        None => path,
    };
    if basename.is_empty() {
        return None;
    }
    Some(format!(".{basename}.tmp."))
}

/// Build a fresh `.<basename>.tmp.<rand>` path under the same parent
/// as `path`.
fn temp_sibling(path: &str) -> Result<String, SysError> {
    let prefix = temp_prefix(path)
        .ok_or_else(|| SysError::ApiError(format!("invalid target path: '{path}'")))?;
    let suffix = random_hex_suffix()?;
    let dir = parent_dir(path)
        .ok_or_else(|| SysError::ApiError(format!("target path has no parent dir: '{path}'")))?;
    Ok(format!("{dir}/{prefix}{suffix}"))
}

/// Produce a hex string for the temp-file suffix from host random
/// bytes. The host CSPRNG is the entropy source (capsules have no
/// other one — see `runtime::random_bytes`).
fn random_hex_suffix() -> Result<String, SysError> {
    let bytes = runtime::random_bytes(RANDOM_SUFFIX_LEN)?;
    Ok(hex_encode(&bytes))
}

/// Lowercase-hex encode a byte slice. Extracted so it can be unit-tested
/// without going through the host CSPRNG.
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // ------------------------------------------------------------------
    // parent_dir — extract the parent slice of a VFS path.
    // ------------------------------------------------------------------

    #[test]
    fn parent_dir_of_nested_path() {
        assert_eq!(
            parent_dir("home://.claude/settings.local.json"),
            Some("home://.claude".to_string()),
        );
    }

    #[test]
    fn parent_dir_of_root_relative() {
        assert_eq!(parent_dir("/foo/bar"), Some("/foo".to_string()));
    }

    #[test]
    fn parent_dir_returns_none_for_basename_only() {
        // No '/' at all -> we can't infer a parent.
        assert_eq!(parent_dir("settings.local.json"), None);
    }

    // ------------------------------------------------------------------
    // temp_prefix — `.<basename>.tmp.` prefix for sibling temp files.
    // ------------------------------------------------------------------

    #[test]
    fn temp_prefix_for_typical_path() {
        assert_eq!(
            temp_prefix("home://.claude/settings.local.json"),
            Some(".settings.local.json.tmp.".to_string()),
        );
    }

    #[test]
    fn temp_prefix_for_basename_only() {
        assert_eq!(
            temp_prefix("settings.local.json"),
            Some(".settings.local.json.tmp.".to_string()),
        );
    }

    #[test]
    fn temp_prefix_rejects_trailing_slash() {
        // A path ending in '/' has no basename to anchor a temp file on.
        assert_eq!(temp_prefix("home://.claude/"), None);
    }

    // ------------------------------------------------------------------
    // hex_encode — deterministic, independent of host CSPRNG.
    // ------------------------------------------------------------------

    #[test]
    fn hex_encode_known_vectors() {
        assert_eq!(hex_encode(&[]), "");
        assert_eq!(hex_encode(&[0x00]), "00");
        assert_eq!(hex_encode(&[0xff]), "ff");
        assert_eq!(hex_encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn hex_encode_emits_two_chars_per_byte() {
        let bytes: Vec<u8> = (0u8..=15).collect();
        let s = hex_encode(&bytes);
        assert_eq!(s.len(), bytes.len() * 2);
        assert!(
            s.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
        );
    }
}
