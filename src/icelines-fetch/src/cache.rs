use std::path::PathBuf;
use std::time::{Duration, SystemTime};
use serde::{de::DeserializeOwned, Serialize};
use crate::error::FetchError;

/// File-based cache under `~/.icelines/cache/`.
/// Each entry is a JSON file. TTL is checked on read; expired entries are treated as misses.
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_owned());
        PathBuf::from(home).join(".icelines").join("cache")
    }

    fn path(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }

    /// Read a cached value. Returns None if missing or expired.
    pub fn get<T: DeserializeOwned>(&self, key: &str, ttl: Duration) -> Option<T> {
        let p = self.path(key);
        if !p.exists() { return None; }

        // Check age
        if let Ok(meta) = p.metadata() {
            if let Ok(modified) = meta.modified() {
                if SystemTime::now().duration_since(modified).unwrap_or(ttl) >= ttl {
                    return None; // expired
                }
            }
        }

        let bytes = std::fs::read(&p).ok()?;
        serde_json::from_slice(&bytes).ok()
    }

    /// Write a value to cache. Creates parent directories as needed.
    pub fn put<T: Serialize>(&self, key: &str, value: &T) -> Result<(), FetchError> {
        let p = self.path(key);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| FetchError::Cache(format!("create dir {}: {e}", parent.display())))?;
        }
        let json = serde_json::to_vec_pretty(value)
            .map_err(|e| FetchError::Cache(format!("serialize {key}: {e}")))?;
        std::fs::write(&p, json)
            .map_err(|e| FetchError::Cache(format!("write {}: {e}", p.display())))
    }

    /// Remove a single cache entry.
    pub fn invalidate(&self, key: &str) {
        let _ = std::fs::remove_file(self.path(key));
    }

    /// Remove all entries under a subdirectory key prefix.
    pub fn invalidate_prefix(&self, prefix: &str) {
        let p = self.root.join(prefix);
        let _ = std::fs::remove_dir_all(p);
    }
}

/// Standard TTLs used across the fetch layer.
pub mod ttl {
    use std::time::Duration;
    pub const ROSTER:    Duration = Duration::from_secs(48 * 3600);   // 48h
    pub const STATS:     Duration = Duration::from_secs(24 * 3600);   // 24h
    pub const BOXSCORE:  Duration = Duration::from_secs(365 * 24 * 3600); // completed games: permanent
    pub const SCHEDULE:  Duration = Duration::from_secs(6 * 3600);    // 6h
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn cache_round_trip() {
        let dir = TempDir::new().unwrap();
        let cache = Cache::new(dir.path());
        cache.put("test/key.json", &42u32).unwrap();
        let v: Option<u32> = cache.get("test/key.json", Duration::from_secs(3600));
        assert_eq!(v, Some(42));
    }

    #[test]
    fn cache_miss_returns_none() {
        let dir = TempDir::new().unwrap();
        let cache = Cache::new(dir.path());
        let v: Option<u32> = cache.get("nonexistent.json", Duration::from_secs(3600));
        assert!(v.is_none());
    }

    #[test]
    fn cache_invalidate_removes_entry() {
        let dir = TempDir::new().unwrap();
        let cache = Cache::new(dir.path());
        cache.put("item.json", &"hello").unwrap();
        cache.invalidate("item.json");
        let v: Option<String> = cache.get("item.json", Duration::from_secs(3600));
        assert!(v.is_none());
    }
}
