//! Phase Foster — shared atomic-write helper.
//!
//! `CareerHistoryStore::save` (Calder) and `Manifest::save_shard`
//! (Foster.0.3) both need the same `tmp + rename` dance: write to
//! `<path>.tmp`, then `rename(<path>.tmp, <path>)` so a crash mid-
//! write never leaves a corrupt blob in place. Extracted here so the
//! invariant has one implementation rather than two divergent copies.
//!
//! `rename` is atomic on POSIX; on Windows `rename` is also atomic
//! when the target is on the same volume (which is always the case
//! here — both files share a parent directory).

use std::path::Path;

/// Write `bytes` to `path` atomically. Creates the parent directory
/// if missing. Cleans up the `.tmp` sidecar on rename success.
pub fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut tmp = path.to_path_buf();
    let mut name = tmp.file_name().map(|n| n.to_owned()).unwrap_or_default();
    name.push(".tmp");
    tmp.set_file_name(name);
    std::fs::write(&tmp, bytes)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Serialize `value` as compact JSON and write atomically.
pub fn write_json_atomic<T: serde::Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    write_bytes_atomic(path, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_foster03_atomic_write_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("foo.json");
        write_bytes_atomic(&path, b"hello").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello");
        assert!(
            !dir.path().join("foo.json.tmp").exists(),
            "tmp sidecar leaked"
        );
    }

    #[test]
    fn l0_foster03_atomic_write_creates_parent_dir() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("a").join("b").join("c.json");
        write_bytes_atomic(&nested, b"x").unwrap();
        assert!(nested.exists());
    }
}
