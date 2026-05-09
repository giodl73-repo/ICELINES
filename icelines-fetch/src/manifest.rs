//! Phase Foster — sharded manifest for `~/.icelines/data/`.
//!
//! One JSON shard per `DataKind` — `bios.json`, `stats.json`, … —
//! plus a `version.json` carrying the writer's schema version and
//! the minimum reader version that can interpret the on-disk shape.
//! Each entry maps a `DataKey` (Season / SeasonType / Game / Date /
//! Player / Global) to a `Freshness` + the file path the bytes live
//! at on disk.
//!
//! Mutability + persistence: the manifest is **mutable** and persisted
//! immediately on append (lazy-fetch writes through). Concurrent
//! writers serialize via a marker-file lock at `manifest/.lock`; the
//! lock pattern matches `fetch_lock.rs` to keep one cross-platform
//! locking idiom in this crate.
//!
//! Sharding by kind is a perf optimization — a `query leaders` run
//! deserializes ~50 entries (bios + stats), not the full 50k that
//! would include every per-game boxscore once Foster.3 lands.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use icelines_core::freshness::Freshness;
use icelines_core::identity::{GameId, PlayerId};
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use serde::{Deserialize, Serialize};

use crate::atomic_write::{write_bytes_atomic, write_json_atomic};
use crate::fetch_lock;

/// On-disk shape version. Bump when the JSON shape changes in a way
/// older readers cannot interpret. `MIN_READER_VERSION` is what
/// today's writer marks as the floor for any future reader; bump it
/// only on incompatible changes.
pub const SCHEMA_VERSION: u32 = 1;
pub const MIN_READER_VERSION: u32 = 1;
pub const MAX_SUPPORTED_VERSION: u32 = 1;

/// Kinds of data the manifest tracks. `#[non_exhaustive]` so adding
/// e.g. `Standings` later doesn't break downstream `match` blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum DataKind {
    Bios,
    Stats,
    GoalieStats,
    Transactions,
    Boxscore,
    CareerHistory,
    Schedule,
    Score,
    PlayoffBracket,
}

impl DataKind {
    /// Filename for this kind's shard (`bios.json` / `stats.json` /
    /// …). Lowercased + snake_cased.
    pub fn shard_filename(self) -> &'static str {
        match self {
            Self::Bios => "bios.json",
            Self::Stats => "stats.json",
            Self::GoalieStats => "goalie_stats.json",
            Self::Transactions => "transactions.json",
            Self::Boxscore => "boxscores.json",
            Self::CareerHistory => "career_history.json",
            Self::Schedule => "schedule.json",
            Self::Score => "score.json",
            Self::PlayoffBracket => "playoff_bracket.json",
        }
    }

    pub fn all() -> &'static [DataKind] {
        &[
            Self::Bios,
            Self::Stats,
            Self::GoalieStats,
            Self::Transactions,
            Self::Boxscore,
            Self::CareerHistory,
            Self::Schedule,
            Self::Score,
            Self::PlayoffBracket,
        ]
    }
}

/// Coordinate for an entry within a shard. `Global` fits the
/// career_history single-blob form (one file, one entry).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataKey {
    Season(Season),
    SeasonType(Season, SeasonType),
    Game(GameId),
    /// `YYYY-MM-DD`. Parked as a string to avoid pulling
    /// `chrono::NaiveDate` into the public API surface yet — Foster.1
    /// formalizes the date axis.
    Date(String),
    Player(PlayerId),
    Global,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub key: DataKey,
    pub path: PathBuf,
    #[serde(flatten)]
    pub freshness: Freshness,
}

/// On-disk shape per shard file. `#[serde(default)]` on `datasets`
/// plus `flatten` on `_extra` preserves unknown top-level keys on
/// round-trip — forward-compat for fields a future writer adds.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ShardFile {
    #[serde(default)]
    pub datasets: Vec<ManifestEntry>,
    #[serde(flatten)]
    pub _extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionFile {
    pub schema_version: u32,
    pub min_reader_version: u32,
    #[serde(flatten)]
    pub _extra: serde_json::Map<String, serde_json::Value>,
}

impl Default for VersionFile {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            min_reader_version: MIN_READER_VERSION,
            _extra: serde_json::Map::new(),
        }
    }
}

/// One in-memory shard — the writable map, plus the full original
/// JSON object so unknown top-level keys round-trip on save.
#[derive(Debug)]
struct Shard {
    entries: RwLock<HashMap<DataKey, ManifestEntry>>,
    extra: RwLock<serde_json::Map<String, serde_json::Value>>,
}

impl Shard {
    fn empty() -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            extra: RwLock::new(serde_json::Map::new()),
        }
    }

    fn from_file(file: ShardFile) -> Self {
        let mut map = HashMap::with_capacity(file.datasets.len());
        for entry in file.datasets {
            map.insert(entry.key.clone(), entry);
        }
        Self {
            entries: RwLock::new(map),
            extra: RwLock::new(file._extra),
        }
    }

    fn snapshot(&self) -> ShardFile {
        let entries = self.entries.read().expect("shard poisoned");
        let mut datasets: Vec<_> = entries.values().cloned().collect();
        datasets.sort_by(|a, b| format!("{:?}", a.key).cmp(&format!("{:?}", b.key)));
        ShardFile {
            datasets,
            _extra: self.extra.read().expect("shard poisoned").clone(),
        }
    }
}

/// The set of shards. Operations look up the right shard by
/// `DataKind`, then operate on its in-memory map. Persistence is
/// per-shard: only the shard you mutated is rewritten.
#[derive(Debug)]
pub struct ManifestSet {
    root: PathBuf,
    shards: HashMap<DataKind, Shard>,
}

impl ManifestSet {
    /// Manifest dir — typically `~/.icelines/data/manifest/`.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Open or create the manifest at `root` (e.g. `~/.icelines/data/manifest/`).
    /// Reads `version.json`, refuses if `min_reader_version >
    /// MAX_SUPPORTED_VERSION`. Reads each shard file if present;
    /// missing shards become empty maps. Unknown shards (files we
    /// don't recognize) are left untouched on disk and ignored.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, ManifestError> {
        let root: PathBuf = root.into();
        std::fs::create_dir_all(&root).map_err(|e| ManifestError::Io {
            path: root.clone(),
            source: e,
        })?;

        let version_path = root.join("version.json");
        let version: VersionFile = match std::fs::read(&version_path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| ManifestError::Corrupt {
                path: version_path.clone(),
                source: e,
            })?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => VersionFile::default(),
            Err(e) => {
                return Err(ManifestError::Io {
                    path: version_path,
                    source: e,
                })
            }
        };

        if version.min_reader_version > MAX_SUPPORTED_VERSION {
            return Err(ManifestError::VersionTooNew {
                found: version.schema_version,
                min_supported: version.min_reader_version,
                our_version: MAX_SUPPORTED_VERSION,
            });
        }

        let mut shards = HashMap::with_capacity(DataKind::all().len());
        for &kind in DataKind::all() {
            let path = root.join(kind.shard_filename());
            let shard = match std::fs::read(&path) {
                Ok(bytes) => {
                    let file: ShardFile =
                        serde_json::from_slice(&bytes).map_err(|e| ManifestError::Corrupt {
                            path: path.clone(),
                            source: e,
                        })?;
                    Shard::from_file(file)
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Shard::empty(),
                Err(e) => return Err(ManifestError::Io { path, source: e }),
            };
            shards.insert(kind, shard);
        }

        // First-open seed: write version.json so future runs see a
        // populated manifest dir.
        if !version_path.exists() {
            write_json_atomic(&version_path, &VersionFile::default()).map_err(|e| {
                ManifestError::Io {
                    path: version_path.clone(),
                    source: e,
                }
            })?;
        }

        Ok(Self { root, shards })
    }

    pub fn get(&self, kind: DataKind, key: &DataKey) -> Option<ManifestEntry> {
        self.shards
            .get(&kind)?
            .entries
            .read()
            .ok()?
            .get(key)
            .cloned()
    }

    pub fn list(&self, kind: DataKind) -> Vec<ManifestEntry> {
        let Some(shard) = self.shards.get(&kind) else {
            return Vec::new();
        };
        let mut entries: Vec<_> = shard
            .entries
            .read()
            .expect("shard poisoned")
            .values()
            .cloned()
            .collect();
        entries.sort_by(|a, b| format!("{:?}", a.key).cmp(&format!("{:?}", b.key)));
        entries
    }

    /// Append or replace an entry, then persist the affected shard.
    /// Holds the cross-process lock for the duration of the write.
    pub fn upsert(&self, kind: DataKind, entry: ManifestEntry) -> Result<(), ManifestError> {
        let _guard = self.lock()?;
        let shard = self
            .shards
            .get(&kind)
            .ok_or(ManifestError::UnknownKind(format!("{kind:?}")))?;
        {
            let mut map = shard.entries.write().expect("shard poisoned");
            map.insert(entry.key.clone(), entry);
        }
        self.write_shard(kind)?;
        Ok(())
    }

    /// Remove an entry, then persist the affected shard.
    pub fn remove(&self, kind: DataKind, key: &DataKey) -> Result<bool, ManifestError> {
        let _guard = self.lock()?;
        let shard = self
            .shards
            .get(&kind)
            .ok_or(ManifestError::UnknownKind(format!("{kind:?}")))?;
        let removed = {
            let mut map = shard.entries.write().expect("shard poisoned");
            map.remove(key).is_some()
        };
        if removed {
            self.write_shard(kind)?;
        }
        Ok(removed)
    }

    fn write_shard(&self, kind: DataKind) -> Result<(), ManifestError> {
        let shard = self
            .shards
            .get(&kind)
            .ok_or(ManifestError::UnknownKind(format!("{kind:?}")))?;
        let snap = shard.snapshot();
        let path = self.root.join(kind.shard_filename());
        let bytes = serde_json::to_vec_pretty(&snap).map_err(|e| ManifestError::Corrupt {
            path: path.clone(),
            source: e,
        })?;
        write_bytes_atomic(&path, &bytes).map_err(|e| ManifestError::Io { path, source: e })?;
        Ok(())
    }

    fn lock(&self) -> Result<fetch_lock::FetchLockGuard, ManifestError> {
        fetch_lock::acquire(&self.root, std::time::Duration::from_secs(5)).map_err(|e| {
            ManifestError::LockFailed(format!("manifest lock at {}: {e}", self.root.display()))
        })
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ManifestError {
    #[error("manifest version {found} requires reader v{min_supported} (we are v{our_version})")]
    VersionTooNew {
        found: u32,
        min_supported: u32,
        our_version: u32,
    },

    #[error("corrupt JSON in {}: {source}", path.display())]
    Corrupt {
        path: PathBuf,
        source: serde_json::Error,
    },

    #[error("io error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("unknown manifest kind '{0}'")]
    UnknownKind(String),

    #[error("manifest lock failed: {0}")]
    LockFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use icelines_core::freshness::{FetchSource, Ttl};
    use std::time::Duration;

    fn fresh_now() -> Freshness {
        Freshness {
            fetched_at: Utc::now(),
            source: FetchSource::Setup,
            ttl: Ttl::After(Duration::from_secs(86400)),
        }
    }

    fn entry_for(key: DataKey) -> ManifestEntry {
        ManifestEntry {
            path: PathBuf::from(format!("data/dummy/{key:?}.json")),
            key,
            freshness: fresh_now(),
        }
    }

    #[test]
    fn l0_foster03_open_empty_creates_version_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("manifest");
        let m = ManifestSet::open(&root).expect("open empty");
        assert!(root.join("version.json").exists(), "version.json seeded");
        assert_eq!(m.list(DataKind::Bios).len(), 0);
    }

    #[test]
    fn l0_foster03_upsert_then_get_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let m = ManifestSet::open(dir.path()).unwrap();
        let key = DataKey::Season(Season(20252026));
        m.upsert(DataKind::Bios, entry_for(key.clone())).unwrap();
        let got = m.get(DataKind::Bios, &key).expect("get hit");
        assert_eq!(got.key, key);
    }

    #[test]
    fn l0_foster03_persistence_across_open() {
        let dir = tempfile::tempdir().unwrap();
        let key = DataKey::Season(Season(20252026));
        {
            let m = ManifestSet::open(dir.path()).unwrap();
            m.upsert(DataKind::Bios, entry_for(key.clone())).unwrap();
        }
        let m2 = ManifestSet::open(dir.path()).unwrap();
        assert!(
            m2.get(DataKind::Bios, &key).is_some(),
            "persisted across reopen"
        );
    }

    #[test]
    fn l0_foster03_remove_returns_true_then_false() {
        let dir = tempfile::tempdir().unwrap();
        let m = ManifestSet::open(dir.path()).unwrap();
        let key = DataKey::Player(PlayerId(8478402));
        m.upsert(DataKind::CareerHistory, entry_for(key.clone()))
            .unwrap();
        assert!(m.remove(DataKind::CareerHistory, &key).unwrap());
        assert!(!m.remove(DataKind::CareerHistory, &key).unwrap());
    }

    #[test]
    fn l0_foster03_atomic_save_no_tmp_leak() {
        let dir = tempfile::tempdir().unwrap();
        let m = ManifestSet::open(dir.path()).unwrap();
        let key = DataKey::Season(Season(20252026));
        m.upsert(DataKind::Stats, entry_for(key)).unwrap();
        let tmp = dir.path().join("stats.json.tmp");
        assert!(!tmp.exists(), "no tmp sidecar after save");
    }

    #[test]
    fn l0_foster03_version_too_new_refuses() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        // Hand-write a version.json with min_reader_version above what
        // we support; open must refuse.
        let v = serde_json::json!({
            "schema_version": 99,
            "min_reader_version": 99,
        });
        std::fs::write(dir.path().join("version.json"), v.to_string()).unwrap();
        let err = ManifestSet::open(dir.path()).expect_err("must refuse");
        assert!(matches!(err, ManifestError::VersionTooNew { .. }));
    }

    #[test]
    fn l0_foster03_unknown_top_level_keys_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        // Plant a shard file with an unknown top-level key.
        std::fs::create_dir_all(dir.path()).unwrap();
        let extra = serde_json::json!({
            "datasets": [],
            "future_field": {"x": 1, "y": "hi"},
        });
        std::fs::write(dir.path().join("bios.json"), extra.to_string()).unwrap();

        let m = ManifestSet::open(dir.path()).unwrap();
        // Mutate the shard to force a re-write.
        m.upsert(DataKind::Bios, entry_for(DataKey::Season(Season(20252026))))
            .unwrap();

        let after: serde_json::Value =
            serde_json::from_slice(&std::fs::read(dir.path().join("bios.json")).unwrap()).unwrap();
        assert_eq!(
            after.get("future_field"),
            Some(&serde_json::json!({"x": 1, "y": "hi"})),
            "unknown top-level key preserved on rewrite"
        );
    }

    #[test]
    fn l0_foster03_corrupt_json_surfaces_corrupt_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join("bios.json"), "{not valid").unwrap();
        let err = ManifestSet::open(dir.path()).expect_err("must error");
        assert!(matches!(err, ManifestError::Corrupt { .. }));
    }

    #[test]
    fn l0_foster03_distinct_kinds_isolated() {
        let dir = tempfile::tempdir().unwrap();
        let m = ManifestSet::open(dir.path()).unwrap();
        let key = DataKey::Season(Season(20252026));
        m.upsert(DataKind::Bios, entry_for(key.clone())).unwrap();
        // Same key under a different kind is independent.
        assert!(m.get(DataKind::Stats, &key).is_none());
        assert!(m.get(DataKind::Bios, &key).is_some());
    }
}
