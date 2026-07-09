//! Atomic file writes for generated Codex config.

use astrid_sdk::prelude::*;

const RANDOM_SUFFIX_LEN: usize = 4;

pub(crate) fn write_atomic(path: &str, bytes: &[u8]) -> Result<(), SysError> {
    let temp = temp_sibling(path)?;
    fs::write(&temp, bytes)?;
    if let Err(err) = fs::rename(&temp, path) {
        let _ = fs::remove_file(&temp);
        return Err(err);
    }
    Ok(())
}

fn temp_sibling(path: &str) -> Result<String, SysError> {
    let parent = parent_dir(path)
        .ok_or_else(|| SysError::ApiError(format!("target path has no parent dir: '{path}'")))?;
    let basename = path.rsplit_once('/').map(|(_, name)| name).unwrap_or(path);
    if basename.is_empty() {
        return Err(SysError::ApiError(format!("invalid target path: '{path}'")));
    }
    Ok(format!("{parent}/.{basename}.tmp.{}", random_hex_suffix()?))
}

fn parent_dir(path: &str) -> Option<String> {
    let idx = path.rfind('/')?;
    Some(path[..idx].to_string())
}

fn random_hex_suffix() -> Result<String, SysError> {
    runtime::random_bytes(RANDOM_SUFFIX_LEN).map(|bytes| hex_encode(&bytes))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}
