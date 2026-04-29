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
    Rosters,    // Tier 1 — 32 team rosters + headshots
    Stats,      // Tier 2 — skater bios + season stats
    Positions,  // Tier 2b — boxscore-derived position eligibility
    Realtime,   // Tier 2c — NHL realtime stats (hits, blocks, giveaways, takeaways)
    MoneyPuck,  // Tier 3b — MoneyPuck xG, CF%, FF%, xGF%
    Contracts,  // Tier 3c — NHL contract data (expiry type/year/salary)
    Derived,    // Tier 3 — computed scores, depth charts (Phase 3)
}

impl SnapshotTier {
    pub fn dir_name(&self) -> &'static str {
        match self {
            Self::Rosters   => "rosters",
            Self::Stats     => "stats",
            Self::Positions => "positions",
            Self::Realtime  => "realtime",
            Self::MoneyPuck => "moneypuck",
            Self::Contracts => "contracts",
            Self::Derived   => "derived",
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

/// Phase 8h: per-snapshot mapping of player_id → chunk_hash for a Stats
/// tier stored in the content-addressed `ChunkStore`. Lives at
/// `{snapshot_dir}/chunked.json` when a snapshot is chunked.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ChunkedManifest {
    pub version: u8,                         // schema version, currently 1
    /// player_id (as JSON string key) → chunk hash for the bios record.
    pub bios:    HashMap<u32, String>,
    /// player_id → chunk hash for the stats record.
    pub stats:   HashMap<u32, String>,
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
    pub removed:     u32,
    pub bytes_freed: u64,
    pub dry_run:     bool,
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
            let hashes: Vec<String> = cm.bios.values().chain(cm.stats.values()).cloned().collect();
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
    /// Refs: every hash referenced by the manifest is incremented in
    /// `chunkrefs.json` so `delete` + `gc_chunks` can prune later.
    pub fn write_chunked_stats(
        &self,
        snapshot_name: &str,
        bios:  &[crate::schema::SkaterBio],
        stats: &[crate::schema::SkaterStats],
    ) -> Result<ChunkedManifest, SnapshotError> {
        let store = self.chunk_store();
        let mut manifest = ChunkedManifest { version: 1, bios: HashMap::new(), stats: HashMap::new() };
        let mut all_hashes: Vec<String> = Vec::with_capacity(bios.len() + stats.len());

        // Each chunk is the canonical JSON of one record.
        for b in bios {
            let bytes = serde_json::to_vec(b)?;
            let hash = store.put(&bytes).map_err(io_to_snapshot)?;
            manifest.bios.insert(b.player_id, hash.clone());
            all_hashes.push(hash);
        }
        for s in stats {
            let bytes = serde_json::to_vec(s)?;
            let hash = store.put(&bytes).map_err(io_to_snapshot)?;
            manifest.stats.insert(s.player_id, hash.clone());
            all_hashes.push(hash);
        }

        self.write_chunked_manifest(snapshot_name, &manifest)?;
        self.inc_refs(&all_hashes)?;
        Ok(manifest)
    }

    /// Read a chunked Stats tier back into bios + stats arrays. Errors if
    /// the snapshot has no `chunked.json` (i.e. it was written with the
    /// legacy `write_file` path).
    pub fn read_chunked_stats(
        &self,
        snapshot_name: &str,
    ) -> Result<(Vec<crate::schema::SkaterBio>, Vec<crate::schema::SkaterStats>), SnapshotError> {
        let cm = self.load_chunked_manifest(snapshot_name)?;
        let store = self.chunk_store();

        let mut bios: Vec<crate::schema::SkaterBio> = Vec::with_capacity(cm.bios.len());
        for (_, hash) in cm.bios.iter() {
            let bytes = store.get(hash).map_err(io_to_snapshot)?;
            bios.push(serde_json::from_slice(&bytes)?);
        }
        let mut stats: Vec<crate::schema::SkaterStats> = Vec::with_capacity(cm.stats.len());
        for (_, hash) in cm.stats.iter() {
            let bytes = store.get(hash).map_err(io_to_snapshot)?;
            stats.push(serde_json::from_slice(&bytes)?);
        }
        Ok((bios, stats))
    }

    /// True if the named snapshot has a chunked manifest on disk.
    pub fn is_chunked(&self, snapshot_name: &str) -> bool {
        self.snapshot_dir(snapshot_name).join("chunked.json").exists()
    }

    fn chunked_manifest_path(&self, snapshot_name: &str) -> PathBuf {
        self.snapshot_dir(snapshot_name).join("chunked.json")
    }

    fn load_chunked_manifest(&self, snapshot_name: &str)
        -> Result<ChunkedManifest, SnapshotError>
    {
        let p = self.chunked_manifest_path(snapshot_name);
        if !p.exists() {
            return Err(SnapshotError::NotFound { name: format!("{snapshot_name}/chunked.json") });
        }
        let raw = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn write_chunked_manifest(&self, snapshot_name: &str, cm: &ChunkedManifest)
        -> Result<(), SnapshotError>
    {
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
        if !p.exists() { return Ok(ChunkRefs::default()); }
        let raw = std::fs::read_to_string(&p)?;
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    fn save_refs(&self, refs: &ChunkRefs) -> Result<(), SnapshotError> {
        let p = self.refs_path();
        if let Some(parent) = p.parent() { std::fs::create_dir_all(parent)?; }
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
                if *c == 0 { refs.counts.remove(h); }
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
            if !self.is_chunked(&entry.name) { continue; }
            if let Ok(cm) = self.load_chunked_manifest(&entry.name) {
                for h in cm.bios.values().chain(cm.stats.values()) {
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
            if refs.counts.contains_key(&hash) { continue; }
            // Zero-ref chunk → sweep
            if let Ok(meta) = std::fs::metadata(&path) {
                bytes_freed += meta.len();
            }
            removed += 1;
            if !dry_run {
                store.delete(&hash).map_err(io_to_snapshot)?;
            }
        }
        Ok(GcReport { removed, bytes_freed, dry_run })
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
        let bios:  Vec<crate::schema::SkaterBio>   =
            self.read(snapshot_name, &SnapshotTier::Stats, "bios.json")?;
        let stats: Vec<crate::schema::SkaterStats> =
            self.read(snapshot_name, &SnapshotTier::Stats, "stats.json")?;
        self.write_chunked_stats(snapshot_name, &bios, &stats)
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

/// Bridge a `FetchError` from the chunkstore back into `SnapshotError` so
/// the SnapshotStore's public surface stays homogeneous. The chunkstore
/// uses FetchError for consistency with the rest of icelines-fetch.
fn io_to_snapshot(e: crate::error::FetchError) -> SnapshotError {
    use crate::error::FetchError;
    match e {
        FetchError::Io(inner)         => SnapshotError::Io(inner),
        FetchError::MissingChunk { hash } => SnapshotError::NotFound { name: hash },
        FetchError::IntegrityViolation { expected, actual } =>
            SnapshotError::IntegrityViolation { file: "chunk".to_owned(), expected, got: actual },
        other => SnapshotError::Io(std::io::Error::other(other.to_string())),
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
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

    // ── Chunked storage (Phase 8h) ───────────────────────────────────────────

    fn fixture_bio(id: u32, name: &str) -> crate::schema::SkaterBio {
        crate::schema::SkaterBio {
            player_id:           id,
            skater_full_name:    name.to_owned(),
            current_team_abbrev: Some("EDM".to_owned()),
            position_code:       "C".to_owned(),
            games_played:        50,
            goals:               20,
            assists:             30,
            points:              50,
            shoots_catches:      Some("L".to_owned()),
            birth_date:          Some("1997-01-13".to_owned()),
            birth_country:       Some("CAN".to_owned()),
            nationality_code:    Some("CAN".to_owned()),
            birth_city:          Some("Edmonton".to_owned()),
            birth_state_province_code: Some("AB".to_owned()),
            height:              Some(73),
            weight:              Some(193),
            draft_year:          Some(2015),
            draft_round:         Some(1),
            draft_overall:       Some(1),
            first_season_for_game_type: Some(20152016),
            is_in_hall_of_fame_yn: Some("N".to_owned()),
            last_name:           name.split_whitespace().last().unwrap_or(name).to_owned(),
        }
    }

    fn fixture_stats(id: u32, goals: u32) -> crate::schema::SkaterStats {
        crate::schema::SkaterStats {
            player_id:           id,
            games_played:        50,
            goals,
            assists:             30,
            points:              goals + 30,
            points_per_game:     1.0,
            pp_goals:            5, pp_points: 10,
            sh_goals:            0, sh_points: 0,
            game_winning_goals:  3, ot_goals: 1,
            shots:               150, shooting_pctg: None,
            plus_minus:          5,
            time_on_ice_per_game: None,
            faceoff_win_pct:     None,
        }
    }

    #[test]
    fn l0_chunked_snapshot_write_then_read_roundtrip() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        let bios  = vec![fixture_bio(1, "A One"), fixture_bio(2, "B Two")];
        let stats = vec![fixture_stats(1, 20), fixture_stats(2, 30)];

        let manifest = store.write_chunked_stats("a", &bios, &stats).unwrap();
        assert_eq!(manifest.bios.len(), 2);
        assert_eq!(manifest.stats.len(), 2);
        assert!(store.is_chunked("a"));

        let (got_bios, got_stats) = store.read_chunked_stats("a").unwrap();
        assert_eq!(got_bios.len(), 2);
        assert_eq!(got_stats.len(), 2);
        // Players present (unordered — HashMap iteration)
        assert!(got_bios.iter().any(|b| b.player_id == 1));
        assert!(got_stats.iter().any(|s| s.player_id == 2 && s.goals == 30));
    }

    #[test]
    fn l0_chunked_snapshot_dedup_two_snapshots_share_unchanged_chunks() {
        // Snapshot A: 3 players. Snapshot B: same 3 players, but player 2's
        // stats line changed. Players 1 and 3's chunks should be reused.
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        store.create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26").unwrap();

        let bios = vec![fixture_bio(1, "A"), fixture_bio(2, "B"), fixture_bio(3, "C")];
        let stats_a = vec![fixture_stats(1, 10), fixture_stats(2, 20), fixture_stats(3, 30)];
        let stats_b = vec![fixture_stats(1, 10), fixture_stats(2, 25), fixture_stats(3, 30)];

        let m_a = store.write_chunked_stats("a", &bios, &stats_a).unwrap();
        let m_b = store.write_chunked_stats("b", &bios, &stats_b).unwrap();

        // Bio chunks: identical for all 3 players
        assert_eq!(m_a.bios[&1], m_b.bios[&1]);
        assert_eq!(m_a.bios[&2], m_b.bios[&2]);
        assert_eq!(m_a.bios[&3], m_b.bios[&3]);
        // Stats chunks: 1 + 3 unchanged, 2 differs
        assert_eq!(m_a.stats[&1], m_b.stats[&1]);
        assert_ne!(m_a.stats[&2], m_b.stats[&2], "player 2 stats changed → new chunk");
        assert_eq!(m_a.stats[&3], m_b.stats[&3]);

        // Total unique chunks on disk: 3 bios + 3 stats_a + 1 stats_b = 7
        let on_disk = store.chunk_store().iter_chunks().unwrap();
        assert_eq!(on_disk.len(), 7,
            "expected 7 unique chunks (3 shared bios + 4 distinct stats), got {}",
            on_disk.len());
    }

    #[test]
    fn l0_chunked_snapshot_inc_refs_on_write() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_chunked_stats("a", &bios, &stats).unwrap();
        let refs = store.load_refs().unwrap();
        // One bio chunk + one stats chunk, each at refcount 1
        assert_eq!(refs.counts.len(), 2);
        for c in refs.counts.values() { assert_eq!(*c, 1); }
    }

    #[test]
    fn l0_chunked_snapshot_dec_refs_on_delete() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        store.create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26").unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_chunked_stats("a", &bios, &stats).unwrap();
        store.write_chunked_stats("b", &bios, &stats).unwrap();
        store.seal("a").unwrap();    // 'a' is now active

        // Both snapshots reference the same 2 chunks → refcount 2 each.
        let refs = store.load_refs().unwrap();
        assert_eq!(refs.counts.values().copied().collect::<Vec<_>>(), vec![2, 2]
            .into_iter().take(refs.counts.len()).collect::<Vec<_>>(),
            "all hashes should have refcount 2");

        // Delete b (non-active): chunks drop to refcount 1, not removed.
        store.delete("b").unwrap();
        let refs = store.load_refs().unwrap();
        assert_eq!(refs.counts.len(), 2);
        for c in refs.counts.values() { assert_eq!(*c, 1); }
    }

    #[test]
    fn l0_chunked_snapshot_is_chunked_distinguishes_layouts() {
        let (_dir, store) = store();
        store.create("legacy", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        store.create("chunked", "20252026", SnapshotTier::Stats, None, "2026-04-26").unwrap();

        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_chunked_stats("chunked", &bios, &stats).unwrap();

        assert!(!store.is_chunked("legacy"), "legacy snapshot must not look chunked");
        assert!(store.is_chunked("chunked"), "chunked snapshot must report true");
    }

    #[test]
    fn l0_chunked_snapshot_read_legacy_returns_not_found() {
        // A snapshot that was never chunked has no chunked.json — read errors.
        let (_dir, store) = store();
        store.create("legacy", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        let err = store.read_chunked_stats("legacy").unwrap_err();
        assert!(matches!(err, SnapshotError::NotFound { .. }));
    }

    // ── GC + migration (Phase 8h.4) ──────────────────────────────────────────

    #[test]
    fn l0_gc_dry_run_reports_zero_ref_chunks_without_deleting() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_chunked_stats("a", &bios, &stats).unwrap();

        // Manually drop a stray chunk that nothing references.
        let stray_hash = store.chunk_store().put(b"unreferenced bytes").unwrap();
        assert!(store.chunk_store().exists(&stray_hash));

        let report = store.gc_chunks(true).unwrap();
        assert!(report.dry_run);
        assert_eq!(report.removed, 1, "exactly one zero-ref chunk should be reported");
        assert!(report.bytes_freed > 0);
        // Dry-run did NOT remove
        assert!(store.chunk_store().exists(&stray_hash), "dry_run must not delete");
    }

    #[test]
    fn l0_gc_real_run_sweeps_zero_ref_chunks() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_chunked_stats("a", &bios, &stats).unwrap();
        let stray_hash = store.chunk_store().put(b"unreferenced").unwrap();

        let report = store.gc_chunks(false).unwrap();
        assert!(!report.dry_run);
        assert_eq!(report.removed, 1);
        assert!(!store.chunk_store().exists(&stray_hash), "stray chunk must be swept");

        // Referenced chunks are preserved
        let cm = store.load_chunked_manifest("a").unwrap();
        for h in cm.bios.values().chain(cm.stats.values()) {
            assert!(store.chunk_store().exists(h),
                "referenced chunk {h} must survive GC");
        }
    }

    #[test]
    fn l0_recompute_refs_rebuilds_from_manifests() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        store.create("b", "20252026", SnapshotTier::Stats, None, "2026-04-26").unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_chunked_stats("a", &bios, &stats).unwrap();
        store.write_chunked_stats("b", &bios, &stats).unwrap();

        // Manually corrupt the refs file
        std::fs::write(store.refs_path(), "{}").unwrap();
        let recomputed = store.recompute_refs().unwrap();
        // Two snapshots × two chunks each, all shared → 2 entries at refcount 2
        assert_eq!(recomputed.counts.len(), 2);
        for c in recomputed.counts.values() { assert_eq!(*c, 2); }
    }

    #[test]
    fn l0_rebuild_chunked_idempotent_on_already_chunked() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        let m1 = store.write_chunked_stats("a", &bios, &stats).unwrap();
        let m2 = store.rebuild_chunked("a").unwrap();
        assert_eq!(m1.bios, m2.bios);
        assert_eq!(m1.stats, m2.stats);
        // Refs not double-incremented
        let refs = store.load_refs().unwrap();
        for c in refs.counts.values() { assert_eq!(*c, 1); }
    }

    #[test]
    fn l0_rebuild_chunked_migrates_legacy_layout() {
        let (_dir, store) = store();
        store.create("a", "20252026", SnapshotTier::Stats, None, "2026-04-25").unwrap();
        // Write the legacy file-per-tier layout
        let bios = vec![fixture_bio(1, "A")];
        let stats = vec![fixture_stats(1, 10)];
        store.write_file("a", &SnapshotTier::Stats, "bios.json",
            &serde_json::to_vec(&bios).unwrap()).unwrap();
        store.write_file("a", &SnapshotTier::Stats, "stats.json",
            &serde_json::to_vec(&stats).unwrap()).unwrap();
        store.seal("a").unwrap();
        assert!(!store.is_chunked("a"), "starts as legacy");

        // Migrate
        let cm = store.rebuild_chunked("a").unwrap();
        assert!(store.is_chunked("a"), "now chunked");
        assert_eq!(cm.bios.len(), 1);
        assert_eq!(cm.stats.len(), 1);

        // Read-back works
        let (got_bios, got_stats) = store.read_chunked_stats("a").unwrap();
        assert_eq!(got_bios.len(), 1);
        assert_eq!(got_stats[0].goals, 10);
    }
}
