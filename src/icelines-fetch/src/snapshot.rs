//! Snapshot-based cache for IceLines NHL data.
//!
//! Replaces the simple TTL file cache with named, sealed, integrity-hashed
//! snapshots that maintain a three-tier provenance chain:
//!   Tier 1 (Rosters) → Tier 2 (Stats) → Tier 3 (Derived, future)
//!
//! Each snapshot is immutable after sealing, integrity-verified on every read,
//! and linked to its parent snapshot via a provenance key chain.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

// ── Errors ────────────────────────────────────────────────────────────────────

#[derive(Debug, Error)]
pub enum SnapshotError {
    #[error("snapshot not found: {name}")]
    NotFound { name: String },

    #[error("snapshot '{name}' is not sealed — run `icelines fetch` to complete it")]
    NotSealed { name: String },

    #[error("integrity violation in '{file}': expected {expected}, got {got}")]
    IntegrityViolation {
        file: String,
        expected: String,
        got: String,
    },

    #[error(
        "stats snapshot '{name}' requires parent rosters snapshot '{parent}' which is missing"
    )]
    MissingParent { name: String, parent: String },

    #[error("no active snapshot — run `icelines fetch` first")]
    NoActiveSnapshot,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotTier {
    Rosters,   // Tier 1 — 32 team rosters + headshots
    Stats,     // Tier 2 — skater bios + season stats
    Positions, // Tier 2b — boxscore-derived position eligibility
    Derived,   // Tier 3 — computed scores, depth charts (Phase 3)
}

impl SnapshotTier {
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Rosters => "rosters",
            Self::Stats => "stats",
            Self::Positions => "positions",
            Self::Derived => "derived",
        }
    }
}

/// Entry in the top-level manifest index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotEntry {
    pub name: String,
    pub season: String,
    pub tier: SnapshotTier,
    pub date: String,               // YYYY-MM-DD
    pub created_at: String,         // RFC3339
    pub parent_key: Option<String>, // name of parent snapshot (tier chain)
    pub file_count: usize,
    pub sealed: bool,
}

/// Top-level index: ~/.icelines/snapshots/index.json
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub snapshots: Vec<SnapshotEntry>,
    pub active: Option<String>,
}

/// Per-snapshot metadata: {snapshot_dir}/snapshot.json
#[derive(Debug, Serialize, Deserialize)]
pub struct SnapshotMeta {
    pub name: String,
    pub season: String,
    pub tier: SnapshotTier,
    pub created_at: String,
    pub parent_key: Option<String>,
    /// SHA-256 hex digest per file (path relative to snapshot_dir)
    pub integrity: HashMap<String, String>,
    pub metadata: HashMap<String, String>,
    pub sealed: bool,
}

// ── SnapshotStore ─────────────────────────────────────────────────────────────

pub struct SnapshotStore {
    root: PathBuf, // ~/.icelines/snapshots/
}

impl SnapshotStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_owned());
        PathBuf::from(home).join(".icelines").join("snapshots")
    }

    fn index_path(&self) -> PathBuf {
        self.root.join("index.json")
    }

    fn snapshot_dir(&self, name: &str) -> PathBuf {
        self.root.join(name)
    }

    fn meta_path(&self, name: &str) -> PathBuf {
        self.snapshot_dir(name).join("snapshot.json")
    }

    // ── Manifest ──────────────────────────────────────────────────────────────

    pub fn load_manifest(&self) -> Result<SnapshotManifest, SnapshotError> {
        let p = self.index_path();
        if !p.exists() {
            return Ok(SnapshotManifest::default());
        }
        let raw = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&raw)?)
    }

    /// Atomically update the manifest: write to .tmp then rename.
    fn save_manifest(&self, manifest: &SnapshotManifest) -> Result<(), SnapshotError> {
        std::fs::create_dir_all(&self.root)?;
        let tmp = self.index_path().with_extension("tmp");
        let json = serde_json::to_string_pretty(manifest)?;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
        drop(f);
        std::fs::rename(&tmp, self.index_path())?;
        Ok(())
    }

    // ── Snapshot lifecycle ────────────────────────────────────────────────────

    /// Create a new (unsealed) snapshot directory.
    pub fn create(
        &self,
        name: &str,
        season: &str,
        tier: SnapshotTier,
        parent_key: Option<String>,
        date: &str,
    ) -> Result<(), SnapshotError> {
        let dir = self.snapshot_dir(name);
        std::fs::create_dir_all(dir.join(tier.dir_name()))?;

        let meta = SnapshotMeta {
            name: name.to_owned(),
            season: season.to_owned(),
            tier,
            created_at: now_rfc3339(),
            parent_key,
            integrity: HashMap::new(),
            metadata: HashMap::new(),
            sealed: false,
        };
        self.write_meta(name, &meta)?;

        let mut manifest = self.load_manifest()?;
        manifest.snapshots.push(SnapshotEntry {
            name: name.to_owned(),
            season: season.to_owned(),
            tier: meta.tier.clone(),
            date: date.to_owned(),
            created_at: meta.created_at.clone(),
            parent_key: meta.parent_key.clone(),
            file_count: 0,
            sealed: false,
        });
        self.save_manifest(&manifest)
    }

    /// Write a data file into a snapshot tier directory. Returns the relative path.
    pub fn write_file(
        &self,
        snapshot_name: &str,
        tier: &SnapshotTier,
        filename: &str,
        data: &[u8],
    ) -> Result<(), SnapshotError> {
        let dir = self.snapshot_dir(snapshot_name).join(tier.dir_name());
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(filename);
        let tmp = path.with_extension("tmp");
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(data)?;
        f.flush()?;
        drop(f);
        std::fs::rename(&tmp, &path)?;

        // Update integrity hash
        let hex = sha256_hex(data);
        let rel = format!("{}/{}", tier.dir_name(), filename);
        let mut meta = self.load_meta(snapshot_name)?;
        meta.integrity.insert(rel, hex);
        self.write_meta(snapshot_name, &meta)?;
        Ok(())
    }

    /// Seal the snapshot — makes it immutable and sets it as active.
    pub fn seal(&self, name: &str) -> Result<(), SnapshotError> {
        let mut meta = self.load_meta(name)?;
        meta.sealed = true;
        self.write_meta(name, &meta)?;

        let mut manifest = self.load_manifest()?;
        if let Some(e) = manifest.snapshots.iter_mut().find(|e| e.name == name) {
            e.sealed = true;
            e.file_count = meta.integrity.len();
        }
        manifest.active = Some(name.to_owned());
        self.save_manifest(&manifest)
    }

    /// Walk the parent chain from the active snapshot to find one containing `tier`.
    /// Returns the snapshot name that owns the data for this tier.
    pub fn find_snapshot_for_tier(&self, tier: &SnapshotTier) -> Result<String, SnapshotError> {
        let manifest = self.load_manifest()?;
        let active = manifest
            .active
            .as_deref()
            .ok_or(SnapshotError::NoActiveSnapshot)?
            .to_owned();

        let mut name = active.clone();
        loop {
            let meta = self.load_meta(&name)?;
            let tier_dir = self.snapshot_dir(&name).join(tier.dir_name());
            if tier_dir.exists() && meta.sealed {
                return Ok(name);
            }
            match meta.parent_key {
                Some(parent) => name = parent,
                None => {
                    return Err(SnapshotError::NotFound {
                        name: format!(
                            "{} data not found in snapshot chain from '{active}'",
                            tier.dir_name()
                        ),
                    })
                }
            }
        }
    }

    // ── Reading ───────────────────────────────────────────────────────────────

    /// Read a file from the active snapshot, verifying integrity.
    pub fn read_active<T: serde::de::DeserializeOwned>(
        &self,
        tier: &SnapshotTier,
        filename: &str,
    ) -> Result<T, SnapshotError> {
        let manifest = self.load_manifest()?;
        let name = manifest
            .active
            .as_deref()
            .ok_or(SnapshotError::NoActiveSnapshot)?
            .to_owned();
        self.read(&name, tier, filename)
    }

    /// Read a file by finding the right snapshot for the tier in the parent chain.
    /// Use this when the active snapshot may be a higher tier (e.g. Stats)
    /// but you need Roster data from its parent.
    pub fn read_tier<T: serde::de::DeserializeOwned>(
        &self,
        tier: &SnapshotTier,
        filename: &str,
    ) -> Result<T, SnapshotError> {
        let name = self.find_snapshot_for_tier(tier)?;
        self.read(&name, tier, filename)
    }

    /// Read a file from a named snapshot, verifying integrity.
    pub fn read<T: serde::de::DeserializeOwned>(
        &self,
        snapshot_name: &str,
        tier: &SnapshotTier,
        filename: &str,
    ) -> Result<T, SnapshotError> {
        let meta = self.load_meta(snapshot_name)?;
        if !meta.sealed {
            return Err(SnapshotError::NotSealed {
                name: snapshot_name.to_owned(),
            });
        }

        let rel = format!("{}/{}", tier.dir_name(), filename);
        let path = self.snapshot_dir(snapshot_name).join(&rel);
        if !path.exists() {
            return Err(SnapshotError::NotFound { name: rel });
        }

        let data = std::fs::read(&path)?;

        // Verify integrity
        if let Some(expected) = meta.integrity.get(&rel) {
            let got = sha256_hex(&data);
            if got != *expected {
                return Err(SnapshotError::IntegrityViolation {
                    file: rel,
                    expected: expected.clone(),
                    got,
                });
            }
        }

        Ok(serde_json::from_slice(&data)?)
    }

    // ── Snapshot commands ─────────────────────────────────────────────────────

    /// List all snapshots.
    pub fn list(&self) -> Result<Vec<SnapshotEntry>, SnapshotError> {
        Ok(self.load_manifest()?.snapshots)
    }

    /// Set the active snapshot (must be sealed).
    pub fn set_active(&self, name: &str) -> Result<(), SnapshotError> {
        let meta = self.load_meta(name)?;
        if !meta.sealed {
            return Err(SnapshotError::NotSealed {
                name: name.to_owned(),
            });
        }
        let mut manifest = self.load_manifest()?;
        manifest.active = Some(name.to_owned());
        self.save_manifest(&manifest)
    }

    /// Verify integrity of all files in a named snapshot.
    pub fn verify(&self, name: &str) -> Result<Vec<String>, SnapshotError> {
        let meta = self.load_meta(name)?;
        let mut failures = Vec::new();
        for (rel, expected) in &meta.integrity {
            let path = self.snapshot_dir(name).join(rel);
            if !path.exists() {
                failures.push(format!("MISSING: {rel}"));
                continue;
            }
            let data = std::fs::read(&path)?;
            let got = sha256_hex(&data);
            if got != *expected {
                failures.push(format!("CORRUPT: {rel} (expected {expected}, got {got})"));
            }
        }
        Ok(failures)
    }

    /// Delete a snapshot (cannot delete the active one).
    pub fn delete(&self, name: &str) -> Result<(), SnapshotError> {
        let manifest = self.load_manifest()?;
        if manifest.active.as_deref() == Some(name) {
            return Err(SnapshotError::Io(std::io::Error::other(
                "cannot delete active snapshot — use `icelines snapshot use` to switch first",
            )));
        }
        let dir = self.snapshot_dir(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        let mut manifest = manifest;
        manifest.snapshots.retain(|e| e.name != name);
        self.save_manifest(&manifest)
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    fn load_meta(&self, name: &str) -> Result<SnapshotMeta, SnapshotError> {
        let p = self.meta_path(name);
        if !p.exists() {
            return Err(SnapshotError::NotFound {
                name: name.to_owned(),
            });
        }
        let raw = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn write_meta(&self, name: &str, meta: &SnapshotMeta) -> Result<(), SnapshotError> {
        let p = self.meta_path(name);
        let tmp = p.with_extension("tmp");
        let json = serde_json::to_string_pretty(meta)?;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
        drop(f);
        std::fs::rename(&tmp, &p)?;
        Ok(())
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

fn now_rfc3339() -> String {
    // std::time doesn't format RFC3339 — use a simple UTC approximation.
    // Replace with chrono in Phase 2 when it's a workspace dependency.
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Very basic: YYYY-MM-DDTHH:MM:SSZ from unix timestamp
    let s = secs;
    let sec = s % 60;
    let min = (s / 60) % 60;
    let hr = (s / 3600) % 24;
    let days = s / 86400;
    // Rough Gregorian (good enough for snapshot naming, not a calendar library)
    let year = 1970 + days / 365;
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;
    format!("{year:04}-{month:02}-{day:02}T{hr:02}:{min:02}:{sec:02}Z")
}

pub fn today_date() -> String {
    let s = now_rfc3339();
    s[..10].to_owned() // "YYYY-MM-DD"
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn store() -> (TempDir, SnapshotStore) {
        let dir = TempDir::new().unwrap();
        let store = SnapshotStore::new(dir.path());
        (dir, store)
    }

    #[test]
    fn l0_snapshot_create_and_seal() {
        let (_dir, store) = store();
        store
            .create(
                "20252026-2026-04-25",
                "20252026",
                SnapshotTier::Rosters,
                None,
                "2026-04-25",
            )
            .unwrap();
        let data = b"{\"forwards\":[],\"defensemen\":[],\"goalies\":[]}";
        store
            .write_file(
                "20252026-2026-04-25",
                &SnapshotTier::Rosters,
                "SEA.json",
                data,
            )
            .unwrap();
        store.seal("20252026-2026-04-25").unwrap();

        let manifest = store.load_manifest().unwrap();
        assert_eq!(manifest.active.as_deref(), Some("20252026-2026-04-25"));
        let entry = &manifest.snapshots[0];
        assert!(entry.sealed);
        assert_eq!(entry.file_count, 1);
    }

    #[test]
    fn l0_snapshot_integrity_verified_on_read() {
        let (_dir, store) = store();
        store
            .create("snap1", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let data = serde_json::to_vec(&serde_json::json!({"data":[],"total":0})).unwrap();
        store
            .write_file("snap1", &SnapshotTier::Stats, "bios.json", &data)
            .unwrap();
        store.seal("snap1").unwrap();

        // Read succeeds
        let v: serde_json::Value = store
            .read("snap1", &SnapshotTier::Stats, "bios.json")
            .unwrap();
        assert_eq!(v["total"], 0);
    }

    #[test]
    fn l0_snapshot_read_active_requires_seal() {
        let (_dir, store) = store();
        store
            .create(
                "snap2",
                "20252026",
                SnapshotTier::Rosters,
                None,
                "2026-04-25",
            )
            .unwrap();
        // Not sealed — read_active should fail
        let result: Result<serde_json::Value, _> =
            store.read_active(&SnapshotTier::Rosters, "ANA.json");
        // No active set yet
        assert!(matches!(result, Err(SnapshotError::NoActiveSnapshot)));
    }

    #[test]
    fn l0_snapshot_verify_catches_corruption() {
        let (_dir, store) = store();
        store
            .create(
                "snap3",
                "20252026",
                SnapshotTier::Rosters,
                None,
                "2026-04-25",
            )
            .unwrap();
        let data = b"original content";
        store
            .write_file("snap3", &SnapshotTier::Rosters, "ANA.json", data)
            .unwrap();
        store.seal("snap3").unwrap();

        // Corrupt the file directly
        let path = store.root.join("snap3/rosters/ANA.json");
        std::fs::write(path, b"corrupted!").unwrap();

        let failures = store.verify("snap3").unwrap();
        assert_eq!(failures.len(), 1);
        assert!(failures[0].contains("CORRUPT"));
    }

    #[test]
    fn l0_snapshot_manifest_atomic_write() {
        let (_dir, store) = store();
        // Two creates don't corrupt the manifest
        store
            .create("a", "20252026", SnapshotTier::Rosters, None, "2026-04-25")
            .unwrap();
        store
            .create(
                "b",
                "20252026",
                SnapshotTier::Stats,
                Some("a".into()),
                "2026-04-25",
            )
            .unwrap();
        let manifest = store.load_manifest().unwrap();
        assert_eq!(manifest.snapshots.len(), 2);
    }

    #[test]
    fn l0_snapshot_delete_non_active() {
        let (_dir, store) = store();
        store
            .create("old", "20252026", SnapshotTier::Rosters, None, "2026-04-24")
            .unwrap();
        store.seal("old").unwrap();
        store
            .create("new", "20252026", SnapshotTier::Rosters, None, "2026-04-25")
            .unwrap();
        store.seal("new").unwrap(); // new is now active
        store.delete("old").unwrap();
        let manifest = store.load_manifest().unwrap();
        assert_eq!(manifest.snapshots.len(), 1);
        assert_eq!(manifest.snapshots[0].name, "new");
    }
}
