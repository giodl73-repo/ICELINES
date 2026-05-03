//! Content-addressed chunk store (Phase 8h).
//!
//! Stores arbitrary byte blobs keyed by their SHA-256 hash. Files are sharded
//! by the first two hex characters to keep any one directory small (<10k
//! entries even at full bundled-skater scale).
//!
//! Layout:
//!   {root}/chunks/
//!     ab/
//!       ab1f5c2c…d7e2.json
//!     92/
//!       92b1de03…4e7a.json
//!     ...
//!
//! Writes are atomic: blob is written to `<hash>.json.tmp`, then renamed to
//! `<hash>.json`. Identical content yields the same path, so concurrent writes
//! converge.
//!
//! Reads verify content by re-hashing — this catches both bit-rot and
//! manual edits to the chunks directory.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

use crate::error::FetchError;

/// A content-addressed blob store.
#[derive(Debug, Clone)]
pub struct ChunkStore {
    root: PathBuf,
}

impl ChunkStore {
    /// Open (or create on first use) a chunk store rooted at `path`.
    /// `path` is typically `~/.icelines/snapshots/chunks/` but any directory
    /// works — used for in-memory tests against tempdirs.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }

    /// Hex-encoded SHA-256 of `data`. Always lowercase, always 64 chars.
    pub fn hash(data: &[u8]) -> String {
        let mut h = Sha256::new();
        h.update(data);
        format!("{:x}", h.finalize())
    }

    /// Filesystem path for a chunk hash. Does not check existence.
    pub fn path_for(&self, hash: &str) -> PathBuf {
        let prefix = hash.get(..2).unwrap_or("00");
        self.root.join(prefix).join(hash)
    }

    /// True if the chunk is present on disk.
    pub fn exists(&self, hash: &str) -> bool {
        self.path_for(hash).exists()
    }

    /// Insert `data` and return its hash. If the chunk already exists, this
    /// is a fast no-op (the existing bytes are not re-verified — callers
    /// should call `verify_all` periodically for that).
    pub fn put(&self, data: &[u8]) -> Result<String, FetchError> {
        let hash = Self::hash(data);
        let path = self.path_for(&hash);
        if path.exists() {
            return Ok(hash);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Atomic rename — never expose a half-written chunk.
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, data)?;
        std::fs::rename(&tmp, &path)?;
        Ok(hash)
    }

    /// Read a chunk by hash. Returns `MissingChunk` if absent;
    /// `IntegrityViolation` if on-disk bytes hash to something different.
    pub fn get(&self, hash: &str) -> Result<Vec<u8>, FetchError> {
        let path = self.path_for(hash);
        let data = std::fs::read(&path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                FetchError::MissingChunk {
                    hash: hash.to_owned(),
                }
            } else {
                FetchError::Io(e)
            }
        })?;
        let actual = Self::hash(&data);
        if actual != hash {
            return Err(FetchError::IntegrityViolation {
                expected: hash.to_owned(),
                actual,
            });
        }
        Ok(data)
    }

    /// Remove a chunk. Idempotent: missing chunk is not an error.
    pub fn delete(&self, hash: &str) -> Result<(), FetchError> {
        let path = self.path_for(hash);
        match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(FetchError::Io(e)),
        }
    }

    /// Iterate every chunk in the store as (hash, path). Walks every shard
    /// directory; useful for GC and integrity sweeps.
    pub fn iter_chunks(&self) -> Result<Vec<(String, PathBuf)>, FetchError> {
        let mut out = Vec::new();
        let shards = match std::fs::read_dir(&self.root) {
            Ok(d) => d,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
            Err(e) => return Err(FetchError::Io(e)),
        };
        for shard in shards {
            let shard = shard?;
            if !shard.file_type()?.is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(shard.path())? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("tmp") {
                    continue; // skip in-flight writes
                }
                if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                    if name.len() == 64 && name.chars().all(|c| c.is_ascii_hexdigit()) {
                        out.push((name.to_owned(), path));
                    }
                }
            }
        }
        Ok(out)
    }

    /// Root directory for the store — used by tests and GC reporters.
    pub fn root(&self) -> &Path {
        &self.root
    }
}

#[cfg(test)]
mod tests {
    //! L0 tests for the content-addressed chunk store (Phase 8h.1).
    use super::*;
    use tempfile::tempdir;

    fn store() -> (ChunkStore, tempfile::TempDir) {
        let dir = tempdir().expect("tempdir");
        let cs = ChunkStore::new(dir.path().to_path_buf());
        (cs, dir)
    }

    #[test]
    fn l0_chunkstore_put_then_get_roundtrip() {
        let (cs, _d) = store();
        let data = b"{\"player_id\":8478402,\"goals\":35}";
        let hash = cs.put(data).expect("put");
        assert_eq!(hash.len(), 64);
        let got = cs.get(&hash).expect("get");
        assert_eq!(got, data);
    }

    #[test]
    fn l0_chunkstore_put_dedup_identical_content() {
        let (cs, _d) = store();
        let data = b"identical bytes";
        let h1 = cs.put(data).unwrap();
        let h2 = cs.put(data).unwrap();
        assert_eq!(h1, h2, "same content must produce same hash");
        // Only one file on disk
        let chunks = cs.iter_chunks().unwrap();
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn l0_chunkstore_distinct_content_distinct_hashes() {
        let (cs, _d) = store();
        let a = cs.put(b"one").unwrap();
        let b = cs.put(b"two").unwrap();
        assert_ne!(a, b);
        assert_eq!(cs.iter_chunks().unwrap().len(), 2);
    }

    #[test]
    fn l0_chunkstore_shard_layout_uses_first_two_hex_chars() {
        let (cs, dir) = store();
        let hash = cs.put(b"shard test").unwrap();
        let prefix = &hash[..2];
        // Path must be {root}/{prefix}/{hash}
        let expected = dir.path().join(prefix).join(&hash);
        assert!(
            expected.exists(),
            "chunk must be at sharded path {expected:?}"
        );
    }

    #[test]
    fn l0_chunkstore_get_missing_returns_missing_chunk_error() {
        let (cs, _d) = store();
        let bogus = "0".repeat(64);
        match cs.get(&bogus) {
            Err(FetchError::MissingChunk { hash }) => assert_eq!(hash, bogus),
            other => panic!("expected MissingChunk, got {other:?}"),
        }
    }

    #[test]
    fn l0_chunkstore_get_corrupted_returns_integrity_violation() {
        let (cs, _d) = store();
        let hash = cs.put(b"clean").unwrap();
        // Corrupt the file in place — write different bytes to the same path
        let path = cs.path_for(&hash);
        std::fs::write(&path, b"tampered!").expect("corrupt");
        match cs.get(&hash) {
            Err(FetchError::IntegrityViolation { expected, .. }) => {
                assert_eq!(expected, hash);
            }
            other => panic!("expected IntegrityViolation, got {other:?}"),
        }
    }

    #[test]
    fn l0_chunkstore_delete_is_idempotent() {
        let (cs, _d) = store();
        let h = cs.put(b"toremove").unwrap();
        assert!(cs.exists(&h));
        cs.delete(&h).unwrap();
        assert!(!cs.exists(&h));
        // Second delete must not error
        cs.delete(&h).unwrap();
        // Delete on a never-existed hash also fine
        cs.delete(&"f".repeat(64)).unwrap();
    }

    #[test]
    fn l0_chunkstore_exists_reports_correctly() {
        let (cs, _d) = store();
        let h = ChunkStore::hash(b"present");
        assert!(!cs.exists(&h));
        cs.put(b"present").unwrap();
        assert!(cs.exists(&h));
    }

    #[test]
    fn l0_chunkstore_iter_walks_every_shard() {
        let (cs, _d) = store();
        // Force a few different first-byte prefixes
        for i in 0..50u32 {
            cs.put(format!("blob {i}").as_bytes()).unwrap();
        }
        let all = cs.iter_chunks().unwrap();
        assert_eq!(all.len(), 50);
        // Every hash is 64 lowercase hex
        for (h, _) in &all {
            assert_eq!(h.len(), 64);
            assert!(h.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn l0_chunkstore_iter_skips_tmp_files() {
        let (cs, dir) = store();
        cs.put(b"real").unwrap();
        // Drop a stale .tmp file in a shard
        let stale = dir.path().join("ff").join("ffff.tmp");
        std::fs::create_dir_all(stale.parent().unwrap()).unwrap();
        std::fs::write(&stale, b"junk").unwrap();
        let all = cs.iter_chunks().unwrap();
        assert_eq!(all.len(), 1, "iter must skip .tmp files");
    }

    #[test]
    fn l0_chunkstore_iter_empty_when_root_missing() {
        let dir = tempdir().unwrap();
        let cs = ChunkStore::new(dir.path().join("never-created"));
        let all = cs.iter_chunks().unwrap();
        assert!(all.is_empty());
    }

    #[test]
    fn l0_chunkstore_hash_is_deterministic() {
        // Same input -> same hash, every call.
        let h1 = ChunkStore::hash(b"abc");
        let h2 = ChunkStore::hash(b"abc");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64);
        // Known SHA-256 of "abc"
        assert_eq!(
            h1,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
