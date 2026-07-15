//! Snapshot-based cache for IceLines NHL data.
//!
//! Replaces the simple TTL file cache with named, sealed, integrity-hashed
//! snapshots that maintain a three-tier provenance chain:
//!   Tier 1 (Rosters) → Tier 2 (Stats) → Tier 3 (Derived, future)
//!
//! Each snapshot is immutable after sealing, integrity-verified on every read,
//! and linked to its parent snapshot via a provenance key chain.

use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::path::{Path, PathBuf};

use icelines_core::season_stats::SeasonType;
use icelines_core::stats_catalog::ReportKind;
use serde::{de::DeserializeOwned, Deserialize, Deserializer, Serialize, Serializer};
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
    Realtime,  // Tier 2c — NHL realtime stats (hits, blocks, giveaways, takeaways)
    MoneyPuck, // Tier 3b — MoneyPuck xG, CF%, FF%, xGF%
    Contracts, // Tier 3c — NHL contract data (expiry type/year/salary)
    Derived,   // Tier 3 — computed scores, depth charts (Phase 3)
}

impl SnapshotTier {
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Rosters => "rosters",
            Self::Stats => "stats",
            Self::Positions => "positions",
            Self::Realtime => "realtime",
            Self::MoneyPuck => "moneypuck",
            Self::Contracts => "contracts",
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

/// Phase 8h: per-snapshot mapping of `player_id → chunk_hash` for each
/// stats report stored in the content-addressed `ChunkStore`. Lives at
/// `{snapshot_dir}/chunked.json` when a snapshot is chunked.
///
/// **Phase Lindsay L.1.2 (v=2)** — refactored to a unified
/// `reports: BTreeMap<ReportKind, BTreeMap<SeasonType, ...>>` map so new
/// Tier-1 reports (timeonice, goalsForAgainst, goalie-advanced,
/// goalie-savesByStrength, goalie-bios) get a uniform key without
/// multiplying flat fields. Custom `Deserialize` accepts both v=1 (flat
/// `bios`/`stats`/`playoff_bios`/`playoff_stats` fields per Hart.6.2) and
/// v=2 (nested) shapes; new writes always emit v=2. v=3+ manifests fail
/// `Deserialize` with a `RepoVersionUnknown`-shaped error which the
/// loader (L.1.3 / DI-28) re-wraps before surfacing.
///
/// `BTreeMap` (not `HashMap`) backs `reports` so iteration order is
/// deterministic — required for snapshot diffing, GC walks, and the
/// `iter_reports()` consumer pattern.
///
/// Backward-compat accessors (`bios()`, `stats()`, `playoff_bios()`,
/// `playoff_stats()`) read from the `reports` map under the
/// `SkaterBios`/`SkaterSummary` keys so existing call sites keep working
/// with a single character delta (`cm.bios.iter()` → `cm.bios().iter()`).
/// Per-(report kind, season type) chunk-hash storage. Lindsay L.1.2
/// unified-key shape replacing the v=1 flat fields. Outer `BTreeMap`
/// keyed by `ReportKind` for deterministic iteration; inner `BTreeMap`
/// keyed by `SeasonType` so each report can have separate regular /
/// playoff slots; innermost `HashMap` keyed by `player_id` is the
/// existing chunk-hash mapping.
pub type ChunkedReports = BTreeMap<ReportKind, BTreeMap<SeasonType, HashMap<u32, String>>>;

#[derive(Debug, Clone, PartialEq)]
pub struct ChunkedManifest {
    /// Schema version. v=1 = flat fields; v=2 = nested `reports` map.
    /// New writes always emit v=2. Lindsay L.1.2 bumps from 1.
    /// **In-memory invariant**: this field always equals `MAX_VERSION`
    /// (v=2). Both `Default::default()` and `Deserialize` produce that
    /// value; serialization always emits the constant. v=1 only exists
    /// on disk in legacy snapshots and is promoted to v=2 on read.
    pub version: u8,
    /// Per-(report kind, season type) chunk-hash map. The unified
    /// storage; flat-field accessors below read into this.
    pub reports: ChunkedReports,
}

impl Default for ChunkedManifest {
    fn default() -> Self {
        // In-memory invariant: every ChunkedManifest has version=v2 once
        // constructed. Default produces an empty Lindsay-aware manifest;
        // the MAX_VERSION stamp matches what `Deserialize` returns + what
        // `Serialize` emits — `eq` round-trips with no surprise.
        Self {
            version: Self::MAX_VERSION,
            reports: BTreeMap::new(),
        }
    }
}

impl ChunkedManifest {
    /// Highest manifest schema version this binary understands. `Deserialize`
    /// errors on any input with `version > MAX_VERSION` — the loader
    /// surfaces this as `LoadError::RepoVersionUnknown` (L.1.3 / DI-28).
    pub const MAX_VERSION: u8 = 2;

    /// Lookup: chunk-hash map for one (report kind, season type) pair.
    /// Returns `None` when the snapshot didn't write that report.
    pub fn report(&self, kind: ReportKind, st: SeasonType) -> Option<&HashMap<u32, String>> {
        self.reports.get(&kind).and_then(|m| m.get(&st))
    }

    /// Set chunk-hash map for one (kind, st). Overwrites any prior entry.
    pub fn set_report(&mut self, kind: ReportKind, st: SeasonType, m: HashMap<u32, String>) {
        self.reports.entry(kind).or_default().insert(st, m);
    }

    /// Iterate over every (kind, season-type, chunk-hash-map) entry.
    /// Used by GC walks, `verify_layout`, and `delete` to enumerate every
    /// referenced chunk regardless of which report holds it.
    pub fn iter_reports(
        &self,
    ) -> impl Iterator<Item = (ReportKind, SeasonType, &HashMap<u32, String>)> {
        self.reports
            .iter()
            .flat_map(|(kind, by_st)| by_st.iter().map(move |(st, m)| (*kind, *st, m)))
    }

    /// Backward-compat accessor — regular-season skater bios chunk-hashes.
    /// Empty map when the snapshot wrote no regular-season skater bios.
    pub fn bios(&self) -> &HashMap<u32, String> {
        self.report(ReportKind::SkaterBios, SeasonType::Regular)
            .unwrap_or(empty_chunk_map())
    }

    /// Backward-compat accessor — regular-season skater summary chunk-hashes.
    /// Note: pre-Lindsay this map ALSO contained realtime + goalsForAgainst
    /// fields merged into each chunk. Lindsay L.1.4 splits those out into
    /// per-report files; the SkaterSummary chunk shrinks to summary-only
    /// content over time. The accessor name stays for API stability.
    pub fn stats(&self) -> &HashMap<u32, String> {
        self.report(ReportKind::SkaterSummary, SeasonType::Regular)
            .unwrap_or(empty_chunk_map())
    }

    /// Backward-compat accessor — playoff skater bios chunk-hashes.
    /// `None` when the snapshot wrote no playoff data (Hart.6.2 semantic).
    pub fn playoff_bios(&self) -> Option<&HashMap<u32, String>> {
        self.report(ReportKind::SkaterBios, SeasonType::Playoff)
    }

    /// Backward-compat accessor — playoff skater summary chunk-hashes.
    pub fn playoff_stats(&self) -> Option<&HashMap<u32, String>> {
        self.report(ReportKind::SkaterSummary, SeasonType::Playoff)
    }
}

/// Empty-map fallback for accessors that need a `&HashMap` even when
/// the underlying entry is absent. `OnceLock` so we never allocate
/// repeatedly on the hot path.
fn empty_chunk_map() -> &'static HashMap<u32, String> {
    use std::sync::OnceLock;
    static E: OnceLock<HashMap<u32, String>> = OnceLock::new();
    E.get_or_init(HashMap::new)
}

// ── ChunkedManifest serde impls ─────────────────────────────────────────────
//
// Custom impls so we can:
//   - emit v=2 nested shape on every write (`Serialize`),
//   - accept BOTH v=1 flat shape and v=2 nested shape on read
//     (`Deserialize`),
//   - error cleanly on v>2 with a string the loader can pattern-match
//     into `LoadError::RepoVersionUnknown` at `load_window` time
//     (L.1.3 / DI-28).

impl Serialize for ChunkedManifest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct as _;
        // Always emit v=2 (Lindsay-aware writer).
        let mut s = serializer.serialize_struct("ChunkedManifest", 2)?;
        s.serialize_field("version", &Self::MAX_VERSION)?;
        s.serialize_field("reports", &self.reports)?;
        s.end()
    }
}

impl<'de> Deserialize<'de> for ChunkedManifest {
    /// Accept v=1 (Hart.6.2 flat shape) and v=2 (Lindsay nested shape).
    /// Promote v=1 flat fields into the unified `reports` map under
    /// `SkaterBios` / `SkaterSummary` keys. Reject `version > MAX_VERSION`
    /// with a descriptive error — the loader's DI-28 path re-wraps.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error as _;

        // Helper struct accepts both shapes simultaneously; every field
        // is `#[serde(default)]` so unknown shapes fall through to None.
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            version: u8,
            // v=1 flat shape (Hart.6.2)
            #[serde(default)]
            bios: Option<HashMap<u32, String>>,
            #[serde(default)]
            stats: Option<HashMap<u32, String>>,
            #[serde(default)]
            playoff_bios: Option<HashMap<u32, String>>,
            #[serde(default)]
            playoff_stats: Option<HashMap<u32, String>>,
            // v=2 nested shape (Lindsay)
            #[serde(default)]
            reports: Option<ChunkedReports>,
        }

        let raw = Raw::deserialize(deserializer)?;

        if raw.version > ChunkedManifest::MAX_VERSION {
            return Err(D::Error::custom(format!(
                "ChunkedManifest version {} > supported {} \
                 (RepoVersionUnknown — upgrade icelines)",
                raw.version,
                ChunkedManifest::MAX_VERSION,
            )));
        }

        // Start from v=2 storage (empty if absent). Then promote any v=1
        // flat fields. Promotion is additive: if BOTH flat fields and
        // `reports` were somehow present (corrupted hand-edited file),
        // the flat fields win for their slot — pre-Lindsay writers can
        // only emit flat, so this is the safer recovery direction.
        let mut reports: BTreeMap<ReportKind, BTreeMap<SeasonType, HashMap<u32, String>>> =
            raw.reports.unwrap_or_default();

        if let Some(m) = raw.bios {
            reports
                .entry(ReportKind::SkaterBios)
                .or_default()
                .insert(SeasonType::Regular, m);
        }
        if let Some(m) = raw.stats {
            reports
                .entry(ReportKind::SkaterSummary)
                .or_default()
                .insert(SeasonType::Regular, m);
        }
        if let Some(m) = raw.playoff_bios {
            reports
                .entry(ReportKind::SkaterBios)
                .or_default()
                .insert(SeasonType::Playoff, m);
        }
        if let Some(m) = raw.playoff_stats {
            reports
                .entry(ReportKind::SkaterSummary)
                .or_default()
                .insert(SeasonType::Playoff, m);
        }

        Ok(ChunkedManifest {
            // In-memory we always represent as the latest version we know.
            // Lifecycle: `Deserialize` reads → all fields land in `reports`
            // → `Serialize` emits v=2. There's no in-memory "v=1" state.
            version: ChunkedManifest::MAX_VERSION,
            reports,
        })
    }
}

/// Phase 8h: refcount table tracking how many chunked snapshots reference
/// each chunk. Lives at `{root}/chunkrefs.json`. Used by `delete` and
/// `gc_chunks` to identify unreferenced chunks safely.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ChunkRefs {
    pub counts: HashMap<String, u32>,
}

/// Phase 8h: result of a `gc_chunks` invocation. Reported back to the user
/// so the CLI can print "swept N chunks, freed M KB".
#[derive(Debug, Clone, Copy)]
pub struct GcReport {
    pub removed: u32,
    pub bytes_freed: u64,
    pub dry_run: bool,
}

/// Phase 8f.2: result of a `prune` invocation.
#[derive(Debug, Clone)]
pub struct PruneReport {
    pub planned: u32, // count that *would* be deleted
    pub deleted: u32, // actually removed (== planned unless dry_run)
    pub dry_run: bool,
    pub names: Vec<String>, // names slated for deletion, sorted
}

/// Phase 8f.3: result of `diff(a, b)`. All `player_id` lists are sorted.
#[derive(Debug, Clone)]
pub struct DiffReport {
    pub a_name: String,
    pub b_name: String,
    pub added: Vec<u32>,         // in B but not A
    pub removed: Vec<u32>,       // in A but not B
    pub changed_bios: Vec<u32>,  // bio hash differs
    pub changed_stats: Vec<u32>, // stats hash differs
}

impl DiffReport {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty()
            && self.removed.is_empty()
            && self.changed_bios.is_empty()
            && self.changed_stats.is_empty()
    }
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

    /// Read access to the on-disk root. Used by callers that need to
    /// pass the path to lower-level helpers (e.g. `SnapshotMetaFlags::load`).
    pub fn root(&self) -> &Path {
        &self.root
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
        let path = dir.join(filename);
        // Phase T.0: route through the shared atomic writer — gives us the
        // .bak preservation contract for free, and keeps the rename pattern
        // in one place.
        atomic_write_bytes(&path, data)?;

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

    /// Hart.6.9 — season-filtered variant of `find_snapshot_for_tier`.
    /// Walks the parent chain skipping snapshots whose `meta.season`
    /// doesn't match `requested_season`. Returns `NotFound` if no
    /// snapshot in the chain is for the requested season.
    ///
    /// Without this, querying a historical season while the active
    /// snapshot is current-season returns wrong-season data and trips
    /// the Hart.6.4 SeasonIdMismatch fence.
    pub fn find_snapshot_for_tier_and_season(
        &self,
        tier: &SnapshotTier,
        requested_season: &str,
    ) -> Result<String, SnapshotError> {
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
            if tier_dir.exists() && meta.sealed && meta.season == requested_season {
                return Ok(name);
            }
            match meta.parent_key {
                Some(parent) => name = parent,
                None => {
                    return Err(SnapshotError::NotFound {
                        name: format!(
                        "{} data for season {requested_season} not found in chain from '{active}'",
                        tier.dir_name()
                    ),
                    })
                }
            }
        }
    }

    /// Read a tier file from the latest snapshot in the chain whose
    /// `meta.season` matches `requested_season`. Hart.6.9 — paired
    /// with `find_snapshot_for_tier_and_season`.
    pub fn read_tier_for_season<T: serde::de::DeserializeOwned>(
        &self,
        tier: &SnapshotTier,
        filename: &str,
        requested_season: &str,
    ) -> Result<T, SnapshotError> {
        let name = self.find_snapshot_for_tier_and_season(tier, requested_season)?;
        self.read(&name, tier, filename)
    }

    /// Find any sealed snapshot for the given season whose directory
    /// contains the requested tier's data — irrespective of the
    /// active/parent chain.
    ///
    /// `find_snapshot_for_tier_and_season` only walks the parent chain
    /// from the active snapshot. That works when the user fetched in
    /// dependency order, but a side-fetch (e.g. `fetch transactions`
    /// for one season followed by `fetch realtime` for another) leaves
    /// the realtime data orphaned from the active chain. The web
    /// dashboard hits this every time: the active is wherever the last
    /// CLI command landed, but the loader is asked for a specific
    /// season's realtime block.
    ///
    /// This scans the full manifest list and returns the most recent
    /// (lexicographic name desc) sealed snapshot for `requested_season`
    /// that has the tier directory on disk. Use this for tiers like
    /// `Realtime` that are commonly fetched out-of-band from the
    /// rosters→stats chain.
    pub fn find_any_snapshot_with_tier_for_season(
        &self,
        tier: &SnapshotTier,
        requested_season: &str,
    ) -> Result<String, SnapshotError> {
        let manifest = self.load_manifest()?;
        let mut candidates: Vec<&SnapshotEntry> = manifest
            .snapshots
            .iter()
            .filter(|e| e.season == requested_season && e.sealed)
            .filter(|e| self.snapshot_dir(&e.name).join(tier.dir_name()).exists())
            .collect();
        candidates.sort_by(|a, b| b.name.cmp(&a.name));
        match candidates.first() {
            Some(e) => Ok(e.name.clone()),
            None => Err(SnapshotError::NotFound {
                name: format!(
                    "no sealed snapshot for season {requested_season} contains \
                     {tier_dir}/* on disk",
                    tier_dir = tier.dir_name(),
                ),
            }),
        }
    }

    /// Season-aware tier read that scans the full manifest list, not
    /// just the active snapshot's parent chain. See
    /// `find_any_snapshot_with_tier_for_season`.
    pub fn read_tier_any_for_season<T: serde::de::DeserializeOwned>(
        &self,
        tier: &SnapshotTier,
        filename: &str,
        requested_season: &str,
    ) -> Result<T, SnapshotError> {
        let name = self.find_any_snapshot_with_tier_for_season(tier, requested_season)?;
        self.read(&name, tier, filename)
    }

    /// Read a specific tier file from the newest sealed snapshot for a season
    /// that actually contains that file.
    ///
    /// Side-fetch snapshots can share a tier directory without carrying every
    /// file in that tier (for example goalie or transaction snapshots under
    /// `stats/`). Filtering on the file avoids selecting one of those partial
    /// snapshots when a caller needs `bios.json` or `stats.json`.
    pub fn read_tier_file_any_for_season<T: serde::de::DeserializeOwned>(
        &self,
        tier: &SnapshotTier,
        filename: &str,
        requested_season: &str,
    ) -> Result<T, SnapshotError> {
        let manifest = self.load_manifest()?;
        let mut candidates: Vec<&SnapshotEntry> = manifest
            .snapshots
            .iter()
            .filter(|entry| entry.season == requested_season && entry.sealed)
            .filter(|entry| {
                self.snapshot_dir(&entry.name)
                    .join(tier.dir_name())
                    .join(filename)
                    .is_file()
            })
            .collect();
        candidates.sort_by(|a, b| b.name.cmp(&a.name));
        let snapshot = candidates.first().ok_or_else(|| SnapshotError::NotFound {
            name: format!(
                "no sealed snapshot for season {requested_season} contains {}/{}",
                tier.dir_name(),
                filename
            ),
        })?;
        self.read(&snapshot.name, tier, filename)
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

        // Legacy file-per-tier integrity: walk meta.integrity and re-hash
        // each tracked file. Chunked snapshots have an empty integrity map
        // (the filename IS the hash), so this loop is a no-op for them.
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

        // Phase 8h: chunked layout integrity. Walks chunked.json and reads
        // every referenced chunk through ChunkStore::get, which re-hashes
        // the bytes against the expected hash (the filename). Catches both
        // missing chunks and bit-rot of chunk files.
        if self.is_chunked(name) {
            let cm = self.load_chunked_manifest(name)?;
            let store = self.chunk_store();
            for (player_id, hash) in cm.bios().iter().chain(cm.stats().iter()) {
                match store.get(hash) {
                    Ok(_) => {}
                    Err(crate::error::FetchError::MissingChunk { hash }) => {
                        failures.push(format!(
                            "MISSING CHUNK: bios/stats for player {player_id} → {hash}"
                        ));
                    }
                    Err(crate::error::FetchError::IntegrityViolation { expected, actual }) => {
                        failures.push(format!(
                            "CORRUPT CHUNK: player {player_id} (expected {expected}, got {actual})"
                        ));
                    }
                    Err(other) => {
                        failures.push(format!("CHUNK ERROR: player {player_id} → {other}"));
                    }
                }
            }
        }

        Ok(failures)
    }

    /// Compare two chunked snapshots and report player-level changes.
    /// Phase 8f.3: leverages the chunked layout's content-addressing —
    /// `bios`/`stats` hashes are byte-identical iff the records are. Set
    /// comparison gives an O(n) exact diff with no JSON deep-walking.
    ///
    /// Both snapshots must be chunked. Legacy snapshots can be migrated
    /// via `snapshot rebuild --chunked <name>` first.
    pub fn diff(&self, a: &str, b: &str) -> Result<DiffReport, SnapshotError> {
        if !self.is_chunked(a) || !self.is_chunked(b) {
            return Err(SnapshotError::Io(std::io::Error::other(format!(
                "snapshot diff requires both snapshots to be chunked. \
                 Run `icelines snapshot rebuild --chunked <name>` first. \
                 a={a} chunked={a_chk}, b={b} chunked={b_chk}",
                a_chk = self.is_chunked(a),
                b_chk = self.is_chunked(b),
            ))));
        }
        let cm_a = self.load_chunked_manifest(a)?;
        let cm_b = self.load_chunked_manifest(b)?;

        use std::collections::HashSet;
        let ids_a: HashSet<u32> = cm_a.bios().keys().copied().collect();
        let ids_b: HashSet<u32> = cm_b.bios().keys().copied().collect();

        let mut added: Vec<u32> = ids_b.difference(&ids_a).copied().collect();
        let mut removed: Vec<u32> = ids_a.difference(&ids_b).copied().collect();
        added.sort();
        removed.sort();

        let mut changed_bios: Vec<u32> = Vec::new();
        let mut changed_stats: Vec<u32> = Vec::new();
        for id in ids_a.intersection(&ids_b) {
            if cm_a.bios().get(id) != cm_b.bios().get(id) {
                changed_bios.push(*id);
            }
            if cm_a.stats().get(id) != cm_b.stats().get(id) {
                changed_stats.push(*id);
            }
        }
        changed_bios.sort();
        changed_stats.sort();

        Ok(DiffReport {
            a_name: a.to_owned(),
            b_name: b.to_owned(),
            added,
            removed,
            changed_bios,
            changed_stats,
        })
    }

    /// Prune sealed snapshots, keeping the newest `keep` per tier and
    /// deleting older ones. Returns a `PruneReport`. The active snapshot is
    /// always preserved regardless of count. Drafts (unsealed) are ignored.
    /// With `dry_run=true`, computes what would be deleted without touching
    /// disk.
    ///
    /// Useful for daily-cron workflows: pair with `gc_chunks` to keep
    /// long-running snapshot storage bounded.
    pub fn prune(&self, keep: usize, dry_run: bool) -> Result<PruneReport, SnapshotError> {
        let manifest = self.load_manifest()?;
        let active = manifest.active.clone();

        // Group sealed snapshots by tier, sorted newest-first by created_at
        // (stable tie-break by name to keep things deterministic).
        let mut by_tier: HashMap<&'static str, Vec<&SnapshotEntry>> = HashMap::new();
        for entry in &manifest.snapshots {
            if !entry.sealed {
                continue;
            }
            by_tier
                .entry(entry.tier.dir_name())
                .or_default()
                .push(entry);
        }
        for entries in by_tier.values_mut() {
            entries.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(b.name.cmp(&a.name)));
        }

        let mut to_delete: Vec<String> = Vec::new();
        for entries in by_tier.values() {
            for entry in entries.iter().skip(keep) {
                if active.as_deref() == Some(entry.name.as_str()) {
                    continue;
                }
                to_delete.push(entry.name.clone());
            }
        }
        // Stable order across runs.
        to_delete.sort();

        let mut deleted = 0u32;
        if !dry_run {
            for name in &to_delete {
                self.delete(name)?;
                deleted += 1;
            }
        }
        Ok(PruneReport {
            planned: to_delete.len() as u32,
            deleted,
            dry_run,
            names: to_delete,
        })
    }

    /// Delete a snapshot (cannot delete the active one).
    /// If the snapshot is chunked, decrements ref counts for its chunks.
    /// (GC of zero-ref chunks happens on `gc_chunks` — `delete` does not
    /// physically remove chunk files.)
    pub fn delete(&self, name: &str) -> Result<(), SnapshotError> {
        let manifest = self.load_manifest()?;
        if manifest.active.as_deref() == Some(name) {
            return Err(SnapshotError::Io(std::io::Error::other(
                "cannot delete active snapshot — use `icelines snapshot use` to switch first",
            )));
        }
        // If chunked, decrement refs before removing the directory.
        if let Ok(cm) = self.load_chunked_manifest(name) {
            let hashes: Vec<String> = cm
                .bios()
                .values()
                .chain(cm.stats().values())
                .cloned()
                .collect();
            self.dec_refs(&hashes)?;
        }
        let dir = self.snapshot_dir(name);
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        let mut manifest = manifest;
        manifest.snapshots.retain(|e| e.name != name);
        self.save_manifest(&manifest)
    }

    // ── Chunked storage (Phase 8h) ────────────────────────────────────────────

    /// Get the global content-addressed chunk store at `{root}/chunks/`.
    pub fn chunk_store(&self) -> crate::chunkstore::ChunkStore {
        crate::chunkstore::ChunkStore::new(self.root.join("chunks"))
    }

    /// Write a chunked Stats tier: each player's bio + stats record becomes
    /// its own content-addressed chunk; `{snapshot}/chunked.json` holds the
    /// player_id → chunk_hash mapping. Subsequent snapshots that share
    /// unchanged player records re-use existing chunks (storage dedup).
    ///
    /// Hart.6.2: `season_type` selects which manifest fields populate.
    /// Regular writes to `bios`/`stats`; Playoff writes to
    /// `playoff_bios`/`playoff_stats`. Calls preserve the manifest's
    /// other-type fields if they already exist on disk so callers can
    /// write regular and playoff in any order without clobbering.
    ///
    /// Refs: every hash referenced by the manifest is incremented in
    /// `chunkrefs.json` so `delete` + `gc_chunks` can prune later.
    pub fn write_chunked_stats(
        &self,
        snapshot_name: &str,
        season_type: icelines_core::season_stats::SeasonType,
        bios: &[crate::schema::SkaterBio],
        stats: &[crate::schema::SkaterStats],
    ) -> Result<ChunkedManifest, SnapshotError> {
        let store = self.chunk_store();
        // Preserve the other-type fields if a manifest already exists on disk
        // (e.g. user fetched regular first, now fetching playoff for the same
        // snapshot). load_chunked_manifest returns NotFound when absent.
        let mut manifest = self
            .load_chunked_manifest(snapshot_name)
            .unwrap_or_default();
        // Lindsay L.1.2: writes always emit v=2 (the in-memory rep is
        // unconditionally v=2; setting the field is redundant but
        // explicit-is-better-than-implicit for future readers).
        manifest.version = ChunkedManifest::MAX_VERSION;

        let mut written_bios: HashMap<u32, String> = HashMap::with_capacity(bios.len());
        let mut written_stats: HashMap<u32, String> = HashMap::with_capacity(stats.len());
        let mut all_hashes: Vec<String> = Vec::with_capacity(bios.len() + stats.len());

        // Each chunk is the canonical JSON of one record.
        for b in bios {
            let bytes = serde_json::to_vec(b)?;
            let hash = store.put(&bytes).map_err(io_to_snapshot)?;
            written_bios.insert(b.player_id, hash.clone());
            all_hashes.push(hash);
        }
        for s in stats {
            let bytes = serde_json::to_vec(s)?;
            let hash = store.put(&bytes).map_err(io_to_snapshot)?;
            written_stats.insert(s.player_id, hash.clone());
            all_hashes.push(hash);
        }

        // Lindsay L.1.2: store via the unified `reports` map. The
        // accessors still expose `bios()`/`stats()`/`playoff_*` so
        // existing readers don't need to know the new shape.
        manifest.set_report(ReportKind::SkaterBios, season_type, written_bios);
        manifest.set_report(ReportKind::SkaterSummary, season_type, written_stats);

        self.write_chunked_manifest(snapshot_name, &manifest)?;
        self.inc_refs(&all_hashes)?;
        Ok(manifest)
    }

    /// Read a chunked Stats tier back into bios + stats arrays. Errors if
    /// the snapshot has no `chunked.json` (i.e. it was written with the
    /// legacy `write_file` path).
    ///
    /// Hart.6.2: `season_type` selects which manifest fields to read.
    /// For Playoff, returns `NotFound` if the snapshot was written with
    /// only regular-season data (`playoff_bios` / `playoff_stats` are None).
    pub fn read_chunked_stats(
        &self,
        snapshot_name: &str,
        season_type: icelines_core::season_stats::SeasonType,
    ) -> Result<
        (
            Vec<crate::schema::SkaterBio>,
            Vec<crate::schema::SkaterStats>,
        ),
        SnapshotError,
    > {
        let cm = self.load_chunked_manifest(snapshot_name)?;
        let store = self.chunk_store();

        let (bios_idx, stats_idx) = match season_type {
            SeasonType::Regular => (cm.bios(), cm.stats()),
            SeasonType::Playoff => {
                let pb = cm.playoff_bios().ok_or_else(|| SnapshotError::NotFound {
                    name: format!("{snapshot_name}/chunked.json: no playoff_bios"),
                })?;
                let ps = cm.playoff_stats().ok_or_else(|| SnapshotError::NotFound {
                    name: format!("{snapshot_name}/chunked.json: no playoff_stats"),
                })?;
                (pb, ps)
            }
        };

        let mut bios: Vec<crate::schema::SkaterBio> = Vec::with_capacity(bios_idx.len());
        for hash in bios_idx.values() {
            let bytes = store.get(hash).map_err(io_to_snapshot)?;
            bios.push(serde_json::from_slice(&bytes)?);
        }
        let mut stats: Vec<crate::schema::SkaterStats> = Vec::with_capacity(stats_idx.len());
        for hash in stats_idx.values() {
            let bytes = store.get(hash).map_err(io_to_snapshot)?;
            stats.push(serde_json::from_slice(&bytes)?);
        }
        Ok((bios, stats))
    }

    /// True if the named snapshot has a chunked manifest on disk.
    pub fn is_chunked(&self, snapshot_name: &str) -> bool {
        self.snapshot_dir(snapshot_name)
            .join("chunked.json")
            .exists()
    }

    fn chunked_manifest_path(&self, snapshot_name: &str) -> PathBuf {
        self.snapshot_dir(snapshot_name).join("chunked.json")
    }

    fn load_chunked_manifest(&self, snapshot_name: &str) -> Result<ChunkedManifest, SnapshotError> {
        let p = self.chunked_manifest_path(snapshot_name);
        if !p.exists() {
            return Err(SnapshotError::NotFound {
                name: format!("{snapshot_name}/chunked.json"),
            });
        }
        let raw = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn write_chunked_manifest(
        &self,
        snapshot_name: &str,
        cm: &ChunkedManifest,
    ) -> Result<(), SnapshotError> {
        let p = self.chunked_manifest_path(snapshot_name);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = p.with_extension("tmp");
        let json = serde_json::to_string_pretty(cm)?;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
        drop(f);
        std::fs::rename(&tmp, &p)?;
        Ok(())
    }

    // ── Refcount table (chunkrefs.json) ──────────────────────────────────────

    fn refs_path(&self) -> PathBuf {
        self.root.join("chunkrefs.json")
    }

    /// Load the refcount table; missing file → empty map (recoverable).
    pub fn load_refs(&self) -> Result<ChunkRefs, SnapshotError> {
        let p = self.refs_path();
        if !p.exists() {
            return Ok(ChunkRefs::default());
        }
        let raw = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn save_refs(&self, refs: &ChunkRefs) -> Result<(), SnapshotError> {
        let p = self.refs_path();
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = p.with_extension("tmp");
        let json = serde_json::to_string_pretty(refs)?;
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(json.as_bytes())?;
        f.flush()?;
        drop(f);
        std::fs::rename(&tmp, &p)?;
        Ok(())
    }

    /// Increment refcount for each hash. Idempotent in the sense that the
    /// refs file is written once at the end of the call.
    pub fn inc_refs(&self, hashes: &[String]) -> Result<(), SnapshotError> {
        let mut refs = self.load_refs()?;
        for h in hashes {
            *refs.counts.entry(h.clone()).or_insert(0) += 1;
        }
        self.save_refs(&refs)
    }

    /// Decrement refcount for each hash, removing entries that reach 0.
    pub fn dec_refs(&self, hashes: &[String]) -> Result<(), SnapshotError> {
        let mut refs = self.load_refs()?;
        for h in hashes {
            if let Some(c) = refs.counts.get_mut(h) {
                *c = c.saturating_sub(1);
                if *c == 0 {
                    refs.counts.remove(h);
                }
            }
        }
        self.save_refs(&refs)
    }

    /// Rebuild `chunkrefs.json` from scratch by walking every chunked
    /// snapshot's manifest. Recovery path when the refcount file is lost
    /// or suspected corrupt.
    pub fn recompute_refs(&self) -> Result<ChunkRefs, SnapshotError> {
        let manifest = self.load_manifest()?;
        let mut refs = ChunkRefs::default();
        for entry in &manifest.snapshots {
            if !self.is_chunked(&entry.name) {
                continue;
            }
            if let Ok(cm) = self.load_chunked_manifest(&entry.name) {
                for h in cm.bios().values().chain(cm.stats().values()) {
                    *refs.counts.entry(h.clone()).or_insert(0) += 1;
                }
            }
        }
        self.save_refs(&refs)?;
        Ok(refs)
    }

    /// Garbage-collect zero-ref chunks. With `dry_run=true`, computes the
    /// list of removable chunks without touching disk and reports their
    /// total size. Returns `(chunks_removed, bytes_freed)`.
    ///
    /// A chunk is "zero-ref" if it appears in the global chunk store but
    /// is not referenced by any sealed-or-draft chunked manifest. The
    /// authoritative source of truth is the chunked manifests themselves;
    /// `chunkrefs.json` is treated as a hint and recomputed if missing.
    pub fn gc_chunks(&self, dry_run: bool) -> Result<GcReport, SnapshotError> {
        // Always recompute refs from manifests — protects against drift.
        let refs = self.recompute_refs()?;
        let store = self.chunk_store();
        let on_disk = store.iter_chunks().map_err(io_to_snapshot)?;

        let mut removed = 0u32;
        let mut bytes_freed: u64 = 0;
        for (hash, path) in on_disk {
            if refs.counts.contains_key(&hash) {
                continue;
            }
            // Zero-ref chunk → sweep
            if let Ok(meta) = std::fs::metadata(&path) {
                bytes_freed += meta.len();
            }
            removed += 1;
            if !dry_run {
                store.delete(&hash).map_err(io_to_snapshot)?;
            }
        }
        Ok(GcReport {
            removed,
            bytes_freed,
            dry_run,
        })
    }

    /// One-shot migration of a legacy snapshot (with `stats/bios.json` and
    /// `stats/stats.json` files) into the chunked layout. Idempotent: if the
    /// snapshot is already chunked, returns the existing manifest unchanged.
    ///
    /// The legacy files are NOT deleted by this method — leave them for
    /// fallback during the migration window. A future cleanup pass can
    /// remove them.
    pub fn rebuild_chunked(&self, snapshot_name: &str) -> Result<ChunkedManifest, SnapshotError> {
        if self.is_chunked(snapshot_name) {
            return self.load_chunked_manifest(snapshot_name);
        }
        // Read the legacy files via the same on-disk paths used by `read`.
        let bios: Vec<crate::schema::SkaterBio> =
            self.read(snapshot_name, &SnapshotTier::Stats, "bios.json")?;
        let stats: Vec<crate::schema::SkaterStats> =
            self.read(snapshot_name, &SnapshotTier::Stats, "stats.json")?;
        self.write_chunked_stats(
            snapshot_name,
            icelines_core::season_stats::SeasonType::Regular,
            &bios,
            &stats,
        )
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    pub fn load_meta(&self, name: &str) -> Result<SnapshotMeta, SnapshotError> {
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

/// Bridge a `FetchError` from the chunkstore back into `SnapshotError` so
/// the SnapshotStore's public surface stays homogeneous. The chunkstore
/// uses FetchError for consistency with the rest of icelines-fetch.
fn io_to_snapshot(e: crate::error::FetchError) -> SnapshotError {
    use crate::error::FetchError;
    match e {
        FetchError::Io(inner) => SnapshotError::Io(inner),
        FetchError::MissingChunk { hash } => SnapshotError::NotFound { name: hash },
        FetchError::IntegrityViolation { expected, actual } => SnapshotError::IntegrityViolation {
            file: "chunk".to_owned(),
            expected,
            got: actual,
        },
        other => SnapshotError::Io(std::io::Error::other(other.to_string())),
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

// ── Per-season meta flags (Phase T.3) ─────────────────────────────────────────
//
// Lives at `~/.icelines/snapshots/{season}/_meta.json`. Tracks last-fetch
// status for tiers that can degrade silently (transactions, future
// network-only sources). Future tiers add fields with `#[serde(default)]`
// so old binaries can read forward-compatible files.

/// Status flags surfaced in the CLI ("snapshot is N days stale") and TUI
/// (red [STALE] prefix). Persisted alongside the canonical snapshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SnapshotMetaFlags {
    /// True when the most-recent transactions fetch failed. Cleared on
    /// the next successful fetch. The CLI / TUI WARN until the flag clears.
    pub transactions_stale: bool,
    /// Last error message — surfaced in the WARN line. None when the
    /// last fetch succeeded.
    pub transactions_last_error: Option<String>,
    /// Wall-clock of the most-recent transactions fetch attempt
    /// (success or failure). Used by the TUI staleness check.
    pub transactions_fetched_at: Option<String>,

    // ── Phase Hart.3 — schema versioning ──────────────────────────────
    //
    // Older meta files don't have these — `#[serde(default)]` at the
    // struct level fills with 0, which the loader interprets as
    // "pre-Hart bundle" and accepts (the loader uses MAX_KNOWN_BUNDLE
    // and MAX_KNOWN_REPO consts to decide whether to error out).
    //
    /// Bundled-JSON file format version. Loader validates: equal = OK;
    /// `incoming > MAX_KNOWN_BUNDLE` = error with upgrade message;
    /// `incoming < MAX_KNOWN_BUNDLE` = run a migrator on read.
    pub bundle_schema_version: u32,
    /// In-memory `StatsRepository` model version. Bumps on every
    /// breaking change to the model. Phase Hart starts at 1.
    pub repository_version: u32,
}

impl SnapshotMetaFlags {
    /// Current bundled-JSON file format version. Bumps whenever a
    /// non-Option field is added to a bundled type. Hart.3 starts at 1.
    /// Keep in sync with `stats_loader::MAX_KNOWN_BUNDLE_SCHEMA`.
    ///
    /// **Lindsay L.1** keeps this at 1 — Lindsay introduces NEW per-report
    /// files (`timeonice.json`, etc.), not new fields on existing bundled
    /// types. The schema-version bump is only for `repository_version`.
    pub const CURRENT_BUNDLE_SCHEMA_VERSION: u32 = 1;

    /// Current in-memory `StatsRepository` model version. Bumps on every
    /// breaking change to the `icelines-core` model. Phase Hart started at 1.
    /// Keep in sync with `stats_loader::MAX_KNOWN_REPO_VERSION`.
    ///
    /// **Lindsay L.1.3 (DI-28)** bumps to 2: `SeasonStats` gains five
    /// typed Tier-1 substructs (`time_on_ice`, `goals_for_against`,
    /// `goalie_advanced`, `goalie_saves_by_strength`, `goalie_bios`) and
    /// `ChunkedManifest` refactors to a unified per-report key map (v=2).
    /// Old binaries (`MAX_KNOWN_REPO_VERSION = 1`) opening a v=2 stamped
    /// snapshot error cleanly with `LoadError::RepoVersionUnknown` at the
    /// `load_into_repo` boundary — DI-28 requires the failure point be at
    /// load time, not at `repo_swap` after the fact.
    pub const CURRENT_REPOSITORY_VERSION: u32 = 2;

    /// Read the flags file at the given root for the given season.
    /// Missing file → default flags (no stale, no error). Corrupt file
    /// falls through to `.bak` recovery via [`read_json_with_bak_fallback`].
    pub fn load(snapshots_root: &Path, season: &str) -> Self {
        let path = Self::path(snapshots_root, season);
        if !path.exists() {
            return Self::default();
        }
        read_json_with_bak_fallback::<SnapshotMetaFlags>(&path).unwrap_or_default()
    }

    /// Write atomically. Best-effort: callers do not propagate failure
    /// because losing the meta file is recoverable on the next fetch.
    ///
    /// Hart.3.3: stamps both version fields with the current binary's
    /// `CURRENT_*_VERSION` constants on every save. Without this, every
    /// `_meta.json` on disk would stay at version 0 forever and the
    /// loader's version gate would never see a real value in
    /// production. The stamp is idempotent (loaders accept any value
    /// `<= MAX_KNOWN`).
    pub fn save(&self, snapshots_root: &Path, season: &str) -> std::io::Result<()> {
        let mut stamped = self.clone();
        stamped.bundle_schema_version = Self::CURRENT_BUNDLE_SCHEMA_VERSION;
        stamped.repository_version = Self::CURRENT_REPOSITORY_VERSION;
        let path = Self::path(snapshots_root, season);
        atomic_write_json(&path, &stamped)
    }

    fn path(snapshots_root: &Path, season: &str) -> PathBuf {
        snapshots_root.join(season).join("_meta.json")
    }
}

// ── Atomic snapshot writer (Phase T.0) ────────────────────────────────────────
//
// Two-step durability: write to `path.tmp`, fsync, then `rename(.tmp, path)`.
// Any prior content at `path` is moved aside to `path.bak` first so a partial
// write or a corrupt downstream snapshot can recover via the backup file.
//
// Used by every snapshot tier and by Phase T (transactions). The .bak is
// best-effort — failure to back up does NOT abort the write, since a prior
// good copy is less important than landing the new bytes correctly.

/// Write `data` to `path` atomically. Any existing file at `path` is
/// preserved at `path.bak` before the rename, so a corrupt downstream
/// can recover via [`read_json_with_bak_fallback`].
///
/// Failure mode contract:
/// - If the write to `path.tmp` fails, the original `path` is left untouched.
/// - If the rename fails, both `path` and `path.tmp` may exist; caller can
///   inspect and recover.
/// - If the .bak copy fails, we proceed with the write anyway (a fresh good
///   copy is more valuable than yesterday's preserved one).
pub fn atomic_write_bytes(path: &Path, data: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Move-aside: best-effort copy of existing content to .bak.
    if path.exists() {
        let bak = bak_path(path);
        // Ignore copy errors — we'd rather land the new write than fail on
        // a missing backup. The .bak is a safety net, not a hard contract.
        let _ = std::fs::copy(path, &bak);
    }

    let tmp = tmp_path(path);
    let mut f = std::fs::File::create(&tmp)?;
    f.write_all(data)?;
    f.flush()?;
    f.sync_all()?;
    drop(f);
    std::fs::rename(&tmp, path)?;
    Ok(())
}

/// Serialize `value` as pretty JSON and atomically write it to `path`.
/// Uses [`atomic_write_bytes`] so the .tmp + .bak + rename contract holds.
pub fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write_bytes(path, &json)
}

/// Read JSON from `path`, falling back to `path.bak` if the primary file
/// is missing or fails to parse. Returns the *first* error if both fail.
///
/// Use for any tier-tier file written via [`atomic_write_json`] where a
/// half-written or human-truncated primary should not fail the load.
pub fn read_json_with_bak_fallback<T: DeserializeOwned>(path: &Path) -> std::io::Result<T> {
    let primary_err = match try_read_json::<T>(path) {
        Ok(value) => return Ok(value),
        Err(e) => e,
    };
    let bak = bak_path(path);
    if !bak.exists() {
        return Err(primary_err);
    }
    match try_read_json::<T>(&bak) {
        Ok(value) => Ok(value),
        Err(_) => Err(primary_err), // surface the primary error, more useful
    }
}

fn try_read_json<T: DeserializeOwned>(path: &Path) -> std::io::Result<T> {
    let raw = std::fs::read_to_string(path)?;
    serde_json::from_str(&raw).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".tmp");
    PathBuf::from(s)
}

fn bak_path(path: &Path) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(".bak");
    PathBuf::from(s)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn today_date() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
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

    // ── Phase T.0: atomic snapshot writer ─────────────────────────────────

    #[test]
    fn l0_atomic_write_json_helper_creates_file_no_tmp_left() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("hello.json");

        atomic_write_json(&path, &serde_json::json!({"k": "v"})).expect("write must succeed");

        assert!(path.exists(), "target file must exist after write");
        assert!(
            !tmp_path(&path).exists(),
            ".tmp file must be cleaned up by the rename"
        );
        let contents = std::fs::read_to_string(&path).unwrap();
        assert!(contents.contains("\"k\""), "JSON content must round-trip");
    }

    #[test]
    fn l0_atomic_write_creates_bak_when_target_exists() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        // First write — no .bak yet.
        atomic_write_json(&path, &serde_json::json!({"v": 1})).unwrap();
        assert!(
            !bak_path(&path).exists(),
            "no .bak on first write — nothing to back up"
        );

        // Second write — prior content moved to .bak.
        atomic_write_json(&path, &serde_json::json!({"v": 2})).unwrap();
        assert!(bak_path(&path).exists(), ".bak must exist after overwrite");

        let bak_contents = std::fs::read_to_string(bak_path(&path)).unwrap();
        assert!(
            bak_contents.contains("\"v\": 1"),
            ".bak must contain the prior content, got: {bak_contents}"
        );
        let curr_contents = std::fs::read_to_string(&path).unwrap();
        assert!(
            curr_contents.contains("\"v\": 2"),
            "primary must contain the new content, got: {curr_contents}"
        );
    }

    #[test]
    fn l0_atomic_write_failure_keeps_prior_intact() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        // Pre-populate target with known content.
        atomic_write_json(&path, &serde_json::json!({"v": "original"})).unwrap();

        // Force the .tmp open to fail by making `path.tmp` already exist as
        // a directory — `File::create` then errors with EISDIR / similar
        // ("Access is denied" on Windows). Either way the rename never
        // happens, so the original must still be readable.
        let tmp = tmp_path(&path);
        std::fs::create_dir_all(&tmp).unwrap();

        let result = atomic_write_json(&path, &serde_json::json!({"v": "new"}));
        assert!(
            result.is_err(),
            "write must fail when .tmp can't be created"
        );

        // Original untouched.
        let after = std::fs::read_to_string(&path).unwrap();
        assert!(
            after.contains("\"v\": \"original\""),
            "primary must be intact after failed write, got: {after}"
        );

        // Cleanup so tempdir drop doesn't trip on the directory-named .tmp.
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn l0_read_json_with_bak_fallback_uses_primary_when_valid() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");
        atomic_write_json(&path, &serde_json::json!({"who": "primary"})).unwrap();

        let v: serde_json::Value = read_json_with_bak_fallback(&path).expect("primary must load");
        assert_eq!(v["who"], "primary");
    }

    #[test]
    fn l0_read_json_with_bak_fallback_recovers_from_corrupt_primary() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        // Establish primary then overwrite (creates .bak).
        atomic_write_json(&path, &serde_json::json!({"v": 1})).unwrap();
        atomic_write_json(&path, &serde_json::json!({"v": 2})).unwrap();
        // Corrupt the primary.
        std::fs::write(&path, "{ this is not valid json").unwrap();

        // Loader falls through to .bak (which still has v: 1, the prior good
        // content moved aside on the second write).
        let v: serde_json::Value =
            read_json_with_bak_fallback(&path).expect("must recover from .bak");
        assert_eq!(
            v["v"], 1,
            "must serve the .bak content on primary corruption"
        );
    }

    #[test]
    fn l0_read_json_with_bak_fallback_errors_when_both_corrupt() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("data.json");

        atomic_write_json(&path, &serde_json::json!({"v": 1})).unwrap();
        atomic_write_json(&path, &serde_json::json!({"v": 2})).unwrap();
        // Corrupt both.
        std::fs::write(&path, "garbage primary").unwrap();
        std::fs::write(bak_path(&path), "garbage bak").unwrap();

        let result: std::io::Result<serde_json::Value> = read_json_with_bak_fallback(&path);
        assert!(
            result.is_err(),
            "both primary and bak corrupt → loader returns Err, no panic"
        );
    }

    #[test]
    fn l0_read_json_with_bak_fallback_errors_when_primary_missing_no_bak() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("does-not-exist.json");

        let result: std::io::Result<serde_json::Value> = read_json_with_bak_fallback(&path);
        assert!(
            result.is_err(),
            "missing file with no .bak → Err, not panic"
        );
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
    fn l0_snapshot_read_named_refuses_unsealed_snapshot() {
        let (_dir, store) = store();
        store
            .create(
                "draft",
                "20252026",
                SnapshotTier::Rosters,
                None,
                "2026-04-25",
            )
            .unwrap();
        store
            .write_file("draft", &SnapshotTier::Rosters, "ANA.json", b"{}")
            .unwrap();

        let result: Result<serde_json::Value, _> =
            store.read("draft", &SnapshotTier::Rosters, "ANA.json");

        assert!(matches!(
            result,
            Err(SnapshotError::NotSealed { name }) if name == "draft"
        ));
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
    fn l0_snapshot_read_catches_integrity_mismatch() {
        let (_dir, store) = store();
        store
            .create(
                "snap-read-integrity",
                "20252026",
                SnapshotTier::Rosters,
                None,
                "2026-04-25",
            )
            .unwrap();
        store
            .write_file(
                "snap-read-integrity",
                &SnapshotTier::Rosters,
                "ANA.json",
                br#"{"team":"ANA"}"#,
            )
            .unwrap();
        store.seal("snap-read-integrity").unwrap();

        let path = store.root.join("snap-read-integrity/rosters/ANA.json");
        std::fs::write(path, br#"{"team":"VGK"}"#).unwrap();

        let result: Result<serde_json::Value, _> =
            store.read("snap-read-integrity", &SnapshotTier::Rosters, "ANA.json");
        assert!(matches!(
            result,
            Err(SnapshotError::IntegrityViolation { file, .. }) if file == "rosters/ANA.json"
        ));
    }

    #[test]
    fn l0_snapshot_verify_reports_missing_integrity_file() {
        let (_dir, store) = store();
        store
            .create(
                "snap-missing-file",
                "20252026",
                SnapshotTier::Rosters,
                None,
                "2026-04-25",
            )
            .unwrap();
        store
            .write_file(
                "snap-missing-file",
                &SnapshotTier::Rosters,
                "ANA.json",
                br#"{"team":"ANA"}"#,
            )
            .unwrap();
        store.seal("snap-missing-file").unwrap();

        let path = store.root.join("snap-missing-file/rosters/ANA.json");
        std::fs::remove_file(path).unwrap();

        let failures = store.verify("snap-missing-file").unwrap();
        assert_eq!(failures, vec!["MISSING: rosters/ANA.json"]);
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

    // ── find_any_snapshot_with_tier_for_season (2026-05-04) ─────────────
    //
    // The active-snapshot-chain reader (`find_snapshot_for_tier`) only
    // walks parent_keys from the active pointer. That misses realtime
    // data when a user fetches realtime in a separate, non-chained
    // snapshot — common in practice because the CLI's `fetch
    // transactions` command leaves the active pointing at a transactions
    // snapshot that has no parent linking back to stats/realtime.
    //
    // The new full-list scanner finds any sealed snapshot for the
    // requested season that has the tier directory on disk, regardless
    // of the active chain.

    /// l0_find_any_snapshot_finds_orphaned_tier_data
    /// — fixture: two snapshots, neither in the active chain. The
    ///   stats one carries a `realtime/` subdir. Active is the
    ///   transactions one (no realtime). Old finder returns
    ///   NotFound; new finder returns the stats snapshot.
    #[test]
    fn l0_find_any_snapshot_finds_orphaned_tier_data() {
        let (_dir, store) = store();
        // 1) Stats snapshot for 25-26 with realtime data inside.
        store
            .create(
                "snap-stats",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-05-03",
            )
            .unwrap();
        store
            .write_file(
                "snap-stats",
                &SnapshotTier::Realtime,
                "realtime.json",
                b"[]",
            )
            .unwrap();
        store.seal("snap-stats").unwrap();

        // 2) Transactions snapshot — separate, no parent link to stats.
        //    Active will point at THIS one.
        store
            .create(
                "snap-tx",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-05-03",
            )
            .unwrap();
        store
            .write_file("snap-tx", &SnapshotTier::Stats, "transactions.json", b"[]")
            .unwrap();
        store.seal("snap-tx").unwrap(); // makes snap-tx active

        // The chain-walking reader can't reach the realtime data:
        let chain_err = store.find_snapshot_for_tier(&SnapshotTier::Realtime);
        assert!(chain_err.is_err(), "chain walker MUST NOT find orphan");

        // The full-list scanner does:
        let found = store
            .find_any_snapshot_with_tier_for_season(&SnapshotTier::Realtime, "20252026")
            .expect("full-list scanner must find the orphaned realtime snapshot");
        assert_eq!(found, "snap-stats");
    }

    /// l0_find_any_snapshot_filters_by_season
    /// — same tier on disk for two different seasons; the scanner must
    ///   return only the one matching `requested_season`.
    #[test]
    fn l0_find_any_snapshot_filters_by_season() {
        let (_dir, store) = store();
        for season in &["20242025", "20252026"] {
            let name = format!("snap-{season}");
            store
                .create(&name, season, SnapshotTier::Stats, None, "2026-05-03")
                .unwrap();
            store
                .write_file(&name, &SnapshotTier::Realtime, "realtime.json", b"[]")
                .unwrap();
            store.seal(&name).unwrap();
        }
        let found = store
            .find_any_snapshot_with_tier_for_season(&SnapshotTier::Realtime, "20242025")
            .unwrap();
        assert_eq!(found, "snap-20242025");
        let found = store
            .find_any_snapshot_with_tier_for_season(&SnapshotTier::Realtime, "20252026")
            .unwrap();
        assert_eq!(found, "snap-20252026");
    }

    /// l0_find_any_snapshot_skips_unsealed_candidates
    /// — half-fetched / aborted snapshots are unsealed. They MUST NOT
    ///   be returned because their integrity hashes aren't stamped yet
    ///   and reading would fail integrity verification anyway.
    #[test]
    fn l0_find_any_snapshot_skips_unsealed_candidates() {
        let (_dir, store) = store();
        store
            .create(
                "half-fetched",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-05-03",
            )
            .unwrap();
        store
            .write_file(
                "half-fetched",
                &SnapshotTier::Realtime,
                "realtime.json",
                b"[]",
            )
            .unwrap();
        // No seal — snapshot stays in-progress.
        let err = store
            .find_any_snapshot_with_tier_for_season(&SnapshotTier::Realtime, "20252026")
            .unwrap_err();
        match err {
            SnapshotError::NotFound { .. } => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    // ── Chunked storage (Phase 8h) ───────────────────────────────────────────

    fn fixture_bio(id: u32, name: &str) -> crate::schema::SkaterBio {
        crate::schema::SkaterBio {
            player_id: id,
            skater_full_name: name.to_owned(),
            current_team_abbrev: Some("EDM".to_owned()),
            position_code: "C".to_owned(),
            games_played: 50,
            goals: 20,
            assists: 30,
            points: 50,
            shoots_catches: Some("L".to_owned()),
            birth_date: Some("1997-01-13".to_owned()),
            birth_country: Some("CAN".to_owned()),
            nationality_code: Some("CAN".to_owned()),
            birth_city: Some("Edmonton".to_owned()),
            birth_state_province_code: Some("AB".to_owned()),
            height: Some(73),
            weight: Some(193),
            draft_year: Some(2015),
            draft_round: Some(1),
            draft_overall: Some(1),
            first_season_for_game_type: Some(20152016),
            is_in_hall_of_fame_yn: Some("N".to_owned()),
            last_name: name.split_whitespace().last().unwrap_or(name).to_owned(),
            season_id: None,
        }
    }

    fn fixture_stats(id: u32, goals: u32) -> crate::schema::SkaterStats {
        crate::schema::SkaterStats {
            player_id: id,
            games_played: 50,
            goals,
            assists: 30,
            points: goals + 30,
            points_per_game: 1.0,
            pp_goals: 5,
            pp_points: 10,
            sh_goals: 0,
            sh_points: 0,
            game_winning_goals: 3,
            ot_goals: 1,
            shots: 150,
            shooting_pctg: None,
            plus_minus: 5,
            time_on_ice_per_game: None,
            faceoff_win_pct: None,
            season_id: None,
            team_abbrevs: None,
        }
    }

    #[test]
    fn l0_chunked_snapshot_write_then_read_roundtrip() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A One"), fixture_bio(2, "B Two")];
        let stats = vec![fixture_stats(1, 20), fixture_stats(2, 30)];

        let manifest = store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        assert_eq!(manifest.bios().len(), 2);
        assert_eq!(manifest.stats().len(), 2);
        assert!(store.is_chunked("a"));

        let (got_bios, got_stats) = store
            .read_chunked_stats("a", icelines_core::season_stats::SeasonType::Regular)
            .unwrap();
        assert_eq!(got_bios.len(), 2);
        assert_eq!(got_stats.len(), 2);
        // Players present (unordered — HashMap iteration)
        assert!(got_bios.iter().any(|b| b.player_id == 1));
        assert!(got_stats.iter().any(|s| s.player_id == 2 && s.goals == 30));
    }

    // ── Hart.6.2 — chunked manifest playoff round-trip ──────────────────────

    /// Playoff write goes into `playoff_bios` / `playoff_stats` fields,
    /// not the regular `bios` / `stats` slots. Read with Playoff returns
    /// what was written.
    #[test]
    fn l0_hart6_2_chunked_playoff_write_lands_in_playoff_fields() {
        let (_dir, store) = store();
        store
            .create("a", "20242025", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();

        let bios = vec![fixture_bio(101, "Playoff A")];
        let stats = vec![fixture_stats(101, 50)];

        let manifest = store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Playoff,
                &bios,
                &stats,
            )
            .unwrap();
        // Playoff fields populated, regular stays empty.
        assert!(manifest.playoff_bios().is_some_and(|m| m.len() == 1));
        assert!(manifest.playoff_stats().is_some_and(|m| m.len() == 1));
        assert_eq!(manifest.bios().len(), 0);
        assert_eq!(manifest.stats().len(), 0);

        let (got_bios, got_stats) = store
            .read_chunked_stats("a", icelines_core::season_stats::SeasonType::Playoff)
            .unwrap();
        assert_eq!(got_bios.len(), 1);
        assert_eq!(got_bios[0].player_id, 101);
        assert_eq!(got_stats[0].goals, 50);
    }

    /// Reading Playoff from a snapshot that only holds Regular returns
    /// `NotFound` cleanly. Catches a regression where the read silently
    /// returns the regular data under a playoff query.
    #[test]
    fn l0_hart6_2_chunked_playoff_read_misses_when_only_regular_present() {
        let (_dir, store) = store();
        store
            .create("a", "20242025", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        let err = store
            .read_chunked_stats("a", icelines_core::season_stats::SeasonType::Playoff)
            .expect_err("playoff read must miss when only regular was written");
        assert!(matches!(err, SnapshotError::NotFound { .. }));
    }

    /// Writing Regular then Playoff into the same snapshot must
    /// preserve both — neither clobbers the other. Catches a bug
    /// where the second write would zero the first manifest's fields.
    #[test]
    fn l0_hart6_2_chunked_regular_then_playoff_preserves_both() {
        let (_dir, store) = store();
        store
            .create("a", "20242025", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();

        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &[fixture_bio(1, "Reg A")],
                &[fixture_stats(1, 10)],
            )
            .unwrap();

        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Playoff,
                &[fixture_bio(101, "Po A")],
                &[fixture_stats(101, 5)],
            )
            .unwrap();

        let (reg_b, _reg_s) = store
            .read_chunked_stats("a", icelines_core::season_stats::SeasonType::Regular)
            .unwrap();
        let (po_b, _po_s) = store
            .read_chunked_stats("a", icelines_core::season_stats::SeasonType::Playoff)
            .unwrap();
        assert_eq!(reg_b.len(), 1);
        assert_eq!(reg_b[0].player_id, 1);
        assert_eq!(po_b.len(), 1);
        assert_eq!(po_b[0].player_id, 101);
    }

    /// Pre-Hart.6 manifests deserialize cleanly: their JSON has no
    /// `playoff_bios` / `playoff_stats` keys, so #[serde(default)]
    /// gives None for both. Catches a forward-compat regression.
    #[test]
    fn l0_hart6_2_chunked_manifest_pre_hart6_json_deserializes_with_none_playoff() {
        let json = r#"{"version":1,"bios":{"1":"abc"},"stats":{"1":"def"}}"#;
        let m: ChunkedManifest = serde_json::from_str(json).expect("parse");
        assert_eq!(m.bios().len(), 1);
        assert!(m.playoff_bios().is_none());
        assert!(m.playoff_stats().is_none());
    }

    #[test]
    fn l0_file_specific_season_read_skips_partial_same_tier_snapshot() {
        let (_dir, store) = store();
        store
            .create(
                "20252026-2026-07-14-stats",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-07-14",
            )
            .unwrap();
        store
            .write_file(
                "20252026-2026-07-14-stats",
                &SnapshotTier::Stats,
                "bios.json",
                br#"[1,2]"#,
            )
            .unwrap();
        store.seal("20252026-2026-07-14-stats").unwrap();

        store
            .create(
                "20252026-2026-07-15-transactions",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-07-15",
            )
            .unwrap();
        store
            .write_file(
                "20252026-2026-07-15-transactions",
                &SnapshotTier::Stats,
                "transactions.json",
                br#"[]"#,
            )
            .unwrap();
        store.seal("20252026-2026-07-15-transactions").unwrap();

        let bios: Vec<u32> = store
            .read_tier_file_any_for_season(&SnapshotTier::Stats, "bios.json", "20252026")
            .unwrap();
        assert_eq!(bios, vec![1, 2]);
    }

    #[test]
    fn l0_chunked_snapshot_dedup_two_snapshots_share_unchanged_chunks() {
        // Snapshot A: 3 players. Snapshot B: same 3 players, but player 2's
        // stats line changed. Players 1 and 3's chunks should be reused.
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        store
            .create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26")
            .unwrap();

        let bios = vec![
            fixture_bio(1, "A"),
            fixture_bio(2, "B"),
            fixture_bio(3, "C"),
        ];
        let stats_a = vec![
            fixture_stats(1, 10),
            fixture_stats(2, 20),
            fixture_stats(3, 30),
        ];
        let stats_b = vec![
            fixture_stats(1, 10),
            fixture_stats(2, 25),
            fixture_stats(3, 30),
        ];

        let m_a = store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats_a,
            )
            .unwrap();
        let m_b = store
            .write_chunked_stats(
                "b",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats_b,
            )
            .unwrap();

        // Bio chunks: identical for all 3 players
        assert_eq!(m_a.bios()[&1], m_b.bios()[&1]);
        assert_eq!(m_a.bios()[&2], m_b.bios()[&2]);
        assert_eq!(m_a.bios()[&3], m_b.bios()[&3]);
        // Stats chunks: 1 + 3 unchanged, 2 differs
        assert_eq!(m_a.stats()[&1], m_b.stats()[&1]);
        assert_ne!(
            m_a.stats()[&2],
            m_b.stats()[&2],
            "player 2 stats changed → new chunk"
        );
        assert_eq!(m_a.stats()[&3], m_b.stats()[&3]);

        // Total unique chunks on disk: 3 bios + 3 stats_a + 1 stats_b = 7
        let on_disk = store.chunk_store().iter_chunks().unwrap();
        assert_eq!(
            on_disk.len(),
            7,
            "expected 7 unique chunks (3 shared bios + 4 distinct stats), got {}",
            on_disk.len()
        );
    }

    #[test]
    fn l0_chunked_snapshot_inc_refs_on_write() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        let refs = store.load_refs().unwrap();
        // One bio chunk + one stats chunk, each at refcount 1
        assert_eq!(refs.counts.len(), 2);
        for c in refs.counts.values() {
            assert_eq!(*c, 1);
        }
    }

    #[test]
    fn l0_chunked_snapshot_dec_refs_on_delete() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        store
            .create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        store
            .write_chunked_stats(
                "b",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        store.seal("a").unwrap(); // 'a' is now active

        // Both snapshots reference the same 2 chunks → refcount 2 each.
        let refs = store.load_refs().unwrap();
        assert_eq!(
            refs.counts.values().copied().collect::<Vec<_>>(),
            vec![2, 2]
                .into_iter()
                .take(refs.counts.len())
                .collect::<Vec<_>>(),
            "all hashes should have refcount 2"
        );

        // Delete b (non-active): chunks drop to refcount 1, not removed.
        store.delete("b").unwrap();
        let refs = store.load_refs().unwrap();
        assert_eq!(refs.counts.len(), 2);
        for c in refs.counts.values() {
            assert_eq!(*c, 1);
        }
    }

    #[test]
    fn l0_chunked_snapshot_is_chunked_distinguishes_layouts() {
        let (_dir, store) = store();
        store
            .create(
                "legacy",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-04-25",
            )
            .unwrap();
        store
            .create(
                "chunked",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-04-26",
            )
            .unwrap();

        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "chunked",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        assert!(
            !store.is_chunked("legacy"),
            "legacy snapshot must not look chunked"
        );
        assert!(
            store.is_chunked("chunked"),
            "chunked snapshot must report true"
        );
    }

    #[test]
    fn l0_chunked_snapshot_read_legacy_returns_not_found() {
        // A snapshot that was never chunked has no chunked.json — read errors.
        let (_dir, store) = store();
        store
            .create(
                "legacy",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-04-25",
            )
            .unwrap();
        let err = store
            .read_chunked_stats("legacy", icelines_core::season_stats::SeasonType::Regular)
            .unwrap_err();
        assert!(matches!(err, SnapshotError::NotFound { .. }));
    }

    // ── GC + migration (Phase 8h.4) ──────────────────────────────────────────

    #[test]
    fn l0_gc_dry_run_reports_zero_ref_chunks_without_deleting() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        // Manually drop a stray chunk that nothing references.
        let stray_hash = store.chunk_store().put(b"unreferenced bytes").unwrap();
        assert!(store.chunk_store().exists(&stray_hash));

        let report = store.gc_chunks(true).unwrap();
        assert!(report.dry_run);
        assert_eq!(
            report.removed, 1,
            "exactly one zero-ref chunk should be reported"
        );
        assert!(report.bytes_freed > 0);
        // Dry-run did NOT remove
        assert!(
            store.chunk_store().exists(&stray_hash),
            "dry_run must not delete"
        );
    }

    #[test]
    fn l0_gc_real_run_sweeps_zero_ref_chunks() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        let stray_hash = store.chunk_store().put(b"unreferenced").unwrap();

        let report = store.gc_chunks(false).unwrap();
        assert!(!report.dry_run);
        assert_eq!(report.removed, 1);
        assert!(
            !store.chunk_store().exists(&stray_hash),
            "stray chunk must be swept"
        );

        // Referenced chunks are preserved
        let cm = store.load_chunked_manifest("a").unwrap();
        for h in cm.bios().values().chain(cm.stats().values()) {
            assert!(
                store.chunk_store().exists(h),
                "referenced chunk {h} must survive GC"
            );
        }
    }

    #[test]
    fn l0_recompute_refs_rebuilds_from_manifests() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        store
            .create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        store
            .write_chunked_stats(
                "b",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        // Manually corrupt the refs file
        std::fs::write(store.refs_path(), "{}").unwrap();
        let recomputed = store.recompute_refs().unwrap();
        // Two snapshots × two chunks each, all shared → 2 entries at refcount 2
        assert_eq!(recomputed.counts.len(), 2);
        for c in recomputed.counts.values() {
            assert_eq!(*c, 2);
        }
    }

    #[test]
    fn l0_rebuild_chunked_idempotent_on_already_chunked() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        let m1 = store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        let m2 = store.rebuild_chunked("a").unwrap();
        assert_eq!(m1.bios(), m2.bios());
        assert_eq!(m1.stats(), m2.stats());
        // Refs not double-incremented
        let refs = store.load_refs().unwrap();
        for c in refs.counts.values() {
            assert_eq!(*c, 1);
        }
    }

    // ── Prune (Phase 8f.2) ───────────────────────────────────────────────────

    fn create_sealed_dated(store: &SnapshotStore, name: &str, date: &str) {
        store
            .create(name, "20252026", SnapshotTier::Stats, None, date)
            .unwrap();
        store.seal(name).unwrap();
    }

    #[test]
    fn l0_prune_dry_run_lists_planned_deletions_without_touching_disk() {
        let (_dir, store) = store();
        // Five Stats snapshots dated newest-first; one of them stays active.
        create_sealed_dated(&store, "stats-2026-04-25", "2026-04-25");
        create_sealed_dated(&store, "stats-2026-04-26", "2026-04-26");
        create_sealed_dated(&store, "stats-2026-04-27", "2026-04-27");
        create_sealed_dated(&store, "stats-2026-04-28", "2026-04-28");
        create_sealed_dated(&store, "stats-2026-04-29", "2026-04-29");
        // Sealing in order makes the last one active. Keep newest 2.

        let report = store.prune(2, true).unwrap();
        assert!(report.dry_run);
        // Newest 2 kept (28, 29); one of those (29) is active and excluded
        // from any deletion logic regardless. Three older ones planned.
        assert_eq!(
            report.planned, 3,
            "expected 3 to prune, got {}",
            report.planned
        );
        assert_eq!(report.deleted, 0, "dry_run must not actually delete");
        // All 5 still present
        assert_eq!(store.list().unwrap().len(), 5);
    }

    #[test]
    fn l0_prune_real_run_deletes_oldest_keeps_newest() {
        let (_dir, store) = store();
        for date in &[
            "2026-04-25",
            "2026-04-26",
            "2026-04-27",
            "2026-04-28",
            "2026-04-29",
        ] {
            create_sealed_dated(&store, &format!("stats-{date}"), date);
        }
        let report = store.prune(2, false).unwrap();
        assert_eq!(report.deleted, 3);

        let remaining: Vec<String> = store.list().unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(remaining.len(), 2);
        assert!(remaining.contains(&"stats-2026-04-29".to_owned()));
        assert!(remaining.contains(&"stats-2026-04-28".to_owned()));
    }

    #[test]
    fn l0_prune_never_deletes_active() {
        let (_dir, store) = store();
        create_sealed_dated(&store, "stats-2026-04-25", "2026-04-25");
        create_sealed_dated(&store, "stats-2026-04-26", "2026-04-26");
        create_sealed_dated(&store, "stats-2026-04-27", "2026-04-27");
        // Force the OLDEST to be active — pinning it.
        store.set_active("stats-2026-04-25").unwrap();

        // Keep only 1 — that would delete the two newer ones, but the active
        // (2026-04-25) must survive because prune skips the active.
        let report = store.prune(1, false).unwrap();
        let names: Vec<String> = store.list().unwrap().into_iter().map(|e| e.name).collect();
        assert!(
            names.contains(&"stats-2026-04-25".to_owned()),
            "active snapshot must always survive prune"
        );
        // We told prune to keep 1 newest; with 2026-04-27 newest, that's kept.
        // 2026-04-26 is the only one prune is allowed to delete (active is excluded).
        assert!(report.deleted >= 1);
    }

    #[test]
    fn l0_prune_keep_more_than_count_is_noop() {
        let (_dir, store) = store();
        create_sealed_dated(&store, "stats-2026-04-25", "2026-04-25");
        create_sealed_dated(&store, "stats-2026-04-26", "2026-04-26");
        let report = store.prune(10, false).unwrap();
        assert_eq!(report.deleted, 0);
        assert_eq!(store.list().unwrap().len(), 2);
    }

    #[test]
    fn l0_prune_skips_drafts() {
        let (_dir, store) = store();
        // Create one sealed and one draft (un-sealed).
        create_sealed_dated(&store, "stats-2026-04-25", "2026-04-25");
        store
            .create(
                "stats-2026-04-26-draft",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-04-26",
            )
            .unwrap();
        // Don't seal the draft.

        let report = store.prune(0, true).unwrap();
        // Only the sealed one is countable; the draft is invisible to prune.
        assert!(
            !report.names.contains(&"stats-2026-04-26-draft".to_owned()),
            "drafts must be excluded from prune candidates"
        );
    }

    // ── Diff (Phase 8f.3) ───────────────────────────────────────────────────

    #[test]
    fn l0_diff_identical_chunked_snapshots_returns_empty() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        store
            .create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26")
            .unwrap();
        let bios = vec![fixture_bio(1, "A"), fixture_bio(2, "B")];
        let stats = vec![fixture_stats(1, 10), fixture_stats(2, 20)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();
        store
            .write_chunked_stats(
                "b",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        let diff = store.diff("a", "b").unwrap();
        assert!(
            diff.is_empty(),
            "identical content must yield empty diff: {diff:?}"
        );
    }

    #[test]
    fn l0_diff_detects_added_and_removed_players() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        store
            .create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26")
            .unwrap();
        // a has {1, 2}, b has {2, 3}: removed=[1], added=[3]
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &[fixture_bio(1, "A"), fixture_bio(2, "B")],
                &[fixture_stats(1, 10), fixture_stats(2, 20)],
            )
            .unwrap();
        store
            .write_chunked_stats(
                "b",
                icelines_core::season_stats::SeasonType::Regular,
                &[fixture_bio(2, "B"), fixture_bio(3, "C")],
                &[fixture_stats(2, 20), fixture_stats(3, 30)],
            )
            .unwrap();

        let diff = store.diff("a", "b").unwrap();
        assert_eq!(diff.removed, vec![1], "player 1 in A only");
        assert_eq!(diff.added, vec![3], "player 3 in B only");
        assert!(diff.changed_bios.is_empty());
        assert!(diff.changed_stats.is_empty());
    }

    #[test]
    fn l0_diff_detects_changed_stats_only() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        store
            .create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26")
            .unwrap();
        // Same bios; player 2's stats differ between A and B.
        let bios = vec![fixture_bio(1, "A"), fixture_bio(2, "B")];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &[fixture_stats(1, 10), fixture_stats(2, 20)],
            )
            .unwrap();
        store
            .write_chunked_stats(
                "b",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &[fixture_stats(1, 10), fixture_stats(2, 25)],
            )
            .unwrap();

        let diff = store.diff("a", "b").unwrap();
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
        assert!(diff.changed_bios.is_empty(), "bios are identical");
        assert_eq!(diff.changed_stats, vec![2]);
    }

    #[test]
    fn l0_diff_legacy_snapshot_errors_with_clear_message() {
        let (_dir, store) = store();
        store
            .create(
                "legacy",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-04-25",
            )
            .unwrap();
        store
            .create(
                "chunked",
                "20252026",
                SnapshotTier::Stats,
                None,
                "2026-04-26",
            )
            .unwrap();
        store
            .write_chunked_stats(
                "chunked",
                icelines_core::season_stats::SeasonType::Regular,
                &[fixture_bio(1, "A")],
                &[fixture_stats(1, 10)],
            )
            .unwrap();

        let err = store.diff("legacy", "chunked").unwrap_err().to_string();
        assert!(
            err.contains("requires both snapshots to be chunked"),
            "error must explain the requirement, got: {err}"
        );
        assert!(
            err.contains("rebuild --chunked"),
            "error must hint at migration command, got: {err}"
        );
    }

    #[test]
    fn l0_chunked_snapshot_verify_clean_returns_no_failures() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A"), fixture_bio(2, "B")];
        let stats = vec![fixture_stats(1, 10), fixture_stats(2, 20)];
        store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        let failures = store.verify("a").unwrap();
        assert!(
            failures.is_empty(),
            "clean chunked snapshot must verify, got: {failures:?}"
        );
    }

    #[test]
    fn l0_chunked_snapshot_verify_catches_corrupted_chunk() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        let cm = store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        // Corrupt the bio chunk: write garbage at its on-disk path.
        let bio_hash = cm.bios().values().next().unwrap();
        let bio_path = store.chunk_store().path_for(bio_hash);
        std::fs::write(&bio_path, b"tampered bytes").unwrap();

        let failures = store.verify("a").unwrap();
        assert_eq!(
            failures.len(),
            1,
            "exactly one failure expected, got {failures:?}"
        );
        assert!(
            failures[0].contains("CORRUPT CHUNK"),
            "must classify as CORRUPT CHUNK, got: {}",
            failures[0]
        );
        assert!(
            failures[0].contains(bio_hash),
            "must mention the offending hash, got: {}",
            failures[0]
        );
    }

    #[test]
    fn l0_chunked_snapshot_verify_catches_missing_chunk() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        let cm = store
            .write_chunked_stats(
                "a",
                icelines_core::season_stats::SeasonType::Regular,
                &bios,
                &stats,
            )
            .unwrap();

        // Remove the stats chunk from the global store.
        let stats_hash = cm.stats().values().next().unwrap();
        store.chunk_store().delete(stats_hash).unwrap();

        let failures = store.verify("a").unwrap();
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].contains("MISSING CHUNK"),
            "must classify as MISSING CHUNK, got: {}",
            failures[0]
        );
    }

    #[test]
    fn l0_rebuild_chunked_migrates_legacy_layout() {
        let (_dir, store) = store();
        store
            .create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25")
            .unwrap();
        // Write the legacy file-per-tier layout
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store
            .write_file(
                "a",
                &SnapshotTier::Stats,
                "bios.json",
                &serde_json::to_vec(&bios).unwrap(),
            )
            .unwrap();
        store
            .write_file(
                "a",
                &SnapshotTier::Stats,
                "stats.json",
                &serde_json::to_vec(&stats).unwrap(),
            )
            .unwrap();
        store.seal("a").unwrap();
        assert!(!store.is_chunked("a"), "starts as legacy");

        // Migrate
        let cm = store.rebuild_chunked("a").unwrap();
        assert!(store.is_chunked("a"), "now chunked");
        assert_eq!(cm.bios().len(), 1);
        assert_eq!(cm.stats().len(), 1);

        // Read-back works
        let (got_bios, got_stats) = store
            .read_chunked_stats("a", icelines_core::season_stats::SeasonType::Regular)
            .unwrap();
        assert_eq!(got_bios.len(), 1);
        assert_eq!(got_stats[0].goals, 10);
    }

    // ── Phase Lindsay L.1.2 — ChunkedManifest v=2 ──────────────────────

    /// L.1.2: a v=1 manifest (Hart.6.2 flat shape) loads and the data
    /// reaches the new `reports` map under `SkaterBios`/`SkaterSummary`
    /// keys. Backward-compat accessors (`bios()`, `stats()`,
    /// `playoff_bios()`, `playoff_stats()`) return the same data.
    #[test]
    fn l0_lindsay_chunked_manifest_v1_promotion_to_v2() {
        let v1_json = r#"{
            "version": 1,
            "bios": {"100": "h1", "200": "h2"},
            "stats": {"100": "s1", "200": "s2"},
            "playoff_bios": {"100": "ph1"},
            "playoff_stats": {"100": "ps1"}
        }"#;
        let m: ChunkedManifest = serde_json::from_str(v1_json).expect("v1 parses");

        // In-memory always represents as v=2 (the latest known shape).
        assert_eq!(m.version, ChunkedManifest::MAX_VERSION);

        // Flat-field data lands under the unified `reports` map.
        assert_eq!(m.bios().len(), 2);
        assert_eq!(m.stats().len(), 2);
        assert_eq!(m.bios().get(&100).unwrap(), "h1");
        assert_eq!(m.stats().get(&200).unwrap(), "s2");

        // Playoff fields promoted into SeasonType::Playoff slot.
        assert_eq!(m.playoff_bios().unwrap().len(), 1);
        assert_eq!(m.playoff_stats().unwrap().len(), 1);
        assert_eq!(m.playoff_bios().unwrap().get(&100).unwrap(), "ph1");
    }

    /// L.1.2: a v=1 manifest WITHOUT playoff fields promotes cleanly.
    /// Pre-Hart.6 manifests produced this shape — ensure backward-compat
    /// stays intact across the v=1 → v=2 schema bump.
    #[test]
    fn l0_lindsay_chunked_manifest_v1_no_playoff_promotion() {
        let v1_json = r#"{
            "version": 1,
            "bios": {"7": "abc"},
            "stats": {"7": "def"}
        }"#;
        let m: ChunkedManifest = serde_json::from_str(v1_json).expect("parse");
        assert_eq!(m.version, ChunkedManifest::MAX_VERSION);
        assert_eq!(m.bios().len(), 1);
        assert_eq!(m.stats().len(), 1);
        assert!(m.playoff_bios().is_none());
        assert!(m.playoff_stats().is_none());
    }

    /// L.1.2: a v=2 manifest round-trips byte-equivalently. Catches any
    /// drift between the custom Serialize emit shape and the custom
    /// Deserialize parse shape.
    #[test]
    fn l0_lindsay_chunked_manifest_v2_round_trip() {
        let mut m = ChunkedManifest::default();
        m.set_report(
            ReportKind::SkaterBios,
            SeasonType::Regular,
            HashMap::from([(100u32, "h1".to_owned())]),
        );
        m.set_report(
            ReportKind::SkaterSummary,
            SeasonType::Regular,
            HashMap::from([(100u32, "s1".to_owned())]),
        );
        // L.1-NEW Tier-1 report — exercises the unified key path that
        // pre-Lindsay had no field for.
        m.set_report(
            ReportKind::SkaterTimeOnIce,
            SeasonType::Regular,
            HashMap::from([(100u32, "toi1".to_owned())]),
        );
        m.set_report(
            ReportKind::GoalieBios,
            SeasonType::Playoff,
            HashMap::from([(8400000u32, "gb-pp".to_owned())]),
        );

        let json = serde_json::to_string(&m).unwrap();
        let back: ChunkedManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);

        // v=2 emit shape: nested `reports` object keyed by camelCase
        // ReportKind. Pin the wire format so we'd catch a casing slip
        // (snake_case → camelCase regression on rename_all).
        assert!(json.contains("\"version\":2"));
        assert!(json.contains("\"skaterBios\""));
        assert!(json.contains("\"skaterTimeOnIce\""));
        assert!(json.contains("\"goalieBios\""));
        assert!(json.contains("\"regular\""));
        assert!(json.contains("\"playoff\""));
    }

    /// L.1.2 / DI-28: a v=3 manifest fails Deserialize with a
    /// RepoVersionUnknown-shaped error. Loader (L.1.3) re-wraps this
    /// into `LoadError::RepoVersionUnknown` at `load_window` boundary.
    #[test]
    fn l0_lindsay_chunked_manifest_v3_rejected() {
        let v3_json = r#"{"version":3,"reports":{}}"#;
        let err = serde_json::from_str::<ChunkedManifest>(v3_json).expect_err("v3 should reject");
        let msg = err.to_string();
        assert!(
            msg.contains("RepoVersionUnknown") || msg.contains("version 3"),
            "expected RepoVersionUnknown-shaped error, got: {msg}",
        );
    }

    /// L.1.2: v=2 storage keeps the new Tier-1 reports separate from
    /// the legacy `bios()`/`stats()` accessors. Adding a `SkaterTimeOnIce`
    /// entry does NOT inflate `cm.bios()` — they're distinct keys.
    #[test]
    fn l0_lindsay_chunked_manifest_new_reports_dont_leak_into_legacy_accessors() {
        let mut m = ChunkedManifest::default();
        m.set_report(
            ReportKind::SkaterTimeOnIce,
            SeasonType::Regular,
            HashMap::from([(100u32, "toi1".to_owned())]),
        );
        assert_eq!(m.bios().len(), 0);
        assert_eq!(m.stats().len(), 0);
        assert_eq!(
            m.report(ReportKind::SkaterTimeOnIce, SeasonType::Regular)
                .unwrap()
                .len(),
            1,
        );
    }

    /// L.1.2: `iter_reports()` enumerates every (kind, st, map) entry
    /// in deterministic order. GC walks rely on this — `BTreeMap`
    /// iteration is sorted.
    #[test]
    fn l0_lindsay_chunked_manifest_iter_reports_deterministic() {
        let mut m = ChunkedManifest::default();
        // Insert in scrambled order; iter should emit sorted.
        m.set_report(
            ReportKind::SkaterTimeOnIce,
            SeasonType::Regular,
            HashMap::from([(1u32, "a".to_owned())]),
        );
        m.set_report(
            ReportKind::SkaterBios,
            SeasonType::Playoff,
            HashMap::from([(1u32, "b".to_owned())]),
        );
        m.set_report(
            ReportKind::SkaterBios,
            SeasonType::Regular,
            HashMap::from([(1u32, "c".to_owned())]),
        );

        let order: Vec<(ReportKind, SeasonType)> =
            m.iter_reports().map(|(k, s, _)| (k, s)).collect();

        // SkaterBios sorts before SkaterTimeOnIce (declaration order).
        // Within SkaterBios, Regular sorts before Playoff (declaration).
        assert_eq!(
            order,
            vec![
                (ReportKind::SkaterBios, SeasonType::Regular),
                (ReportKind::SkaterBios, SeasonType::Playoff),
                (ReportKind::SkaterTimeOnIce, SeasonType::Regular),
            ],
        );
    }
}
