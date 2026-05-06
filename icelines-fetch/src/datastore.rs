//! Phase Foster — `DataStore`, the unified bytes-and-manifest layer.
//!
//! One front door for "give me the bios for season S" / "give me the
//! stats for (S, T)" / "give me the boxscore for game G" — handles
//! the manifest lookup, falls back to the embedded bundle, and lazy-
//! fetches over the network when allowed and missing.
//!
//! Layering rule (FORGE H3): DataStore returns parsed-but-uncached
//! domain types; `StatsRepository` remains the session-cached layer
//! and calls DataStore on misses. DataStore must never grow its own
//! per-process LRU.
//!
//! Network injection: the `Fetcher` trait abstracts the lazy-fetch
//! path so tests inject a `MockFetcher` and the production wiring
//! plugs in a `NhlApiFetcher` that delegates to `NhlApiClient`. Real
//! NHL wiring lands in a follow-up sub-step; the trait + the routing
//! are what ship in F.0.4.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use icelines_core::career_history::CareerHistory;
use icelines_core::freshness::{Clock, FetchSource, Freshness, SystemClock, Ttl};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;

use crate::atomic_write::write_json_atomic;
use crate::bundled;
use crate::manifest::{DataKey, DataKind, ManifestEntry, ManifestError, ManifestSet};
use crate::schema::{SkaterBio, SkaterStats};

/// Errors surfaced by `DataStore`. Distinct from `ManifestError`
/// (which is the on-disk storage layer's error) — DataError is what
/// callers see.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DataError {
    #[error(
        "data not installed for {kind:?} / {key:?} — \
         run `icelines setup` or `icelines fetch …`"
    )]
    NotInstalled { kind: DataKind, key: DataKey },

    #[error("manifest entry exists but file missing on disk: {}", path.display())]
    BackingFileMissing { path: PathBuf },

    #[error("network error fetching {url}: {detail}")]
    Network { url: String, detail: String },

    #[error("HTTP {status} from {url}")]
    Http5xx { url: String, status: u16 },

    #[error("schema drift parsing {url}: {detail}")]
    SchemaDrift { url: String, detail: String },

    #[error("io error at {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("corrupt JSON in {}: {source}", path.display())]
    Corrupt {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },

    #[error("manifest error: {0}")]
    Manifest(#[from] ManifestError),
}

/// Lazy-fetch hook — pluggable so production wires `NhlApiClient`
/// while tests inject deterministic responses without touching the
/// network. Methods return parsed domain types; the DataStore writes
/// the bytes to disk + the manifest entry as one logical commit.
pub trait Fetcher: Send + Sync {
    fn fetch_bios(&self, season: Season) -> Result<Vec<SkaterBio>, DataError>;
    fn fetch_stats(
        &self,
        season: Season,
        season_type: SeasonType,
    ) -> Result<Vec<SkaterStats>, DataError>;
    fn fetch_career_history(&self, pid: PlayerId) -> Result<CareerHistory, DataError>;
}

/// Default fetcher — always fails. Used when the caller hasn't wired
/// a real `NhlApiFetcher` yet (Foster.0.4 ships the routing; the
/// real NHL wiring is a follow-up). Returns `NotInstalled` so the
/// caller path matches the "live_feeds=false" branch.
#[derive(Debug, Default)]
pub struct NoopFetcher;

impl Fetcher for NoopFetcher {
    fn fetch_bios(&self, season: Season) -> Result<Vec<SkaterBio>, DataError> {
        Err(DataError::NotInstalled {
            kind: DataKind::Bios,
            key: DataKey::Season(season),
        })
    }
    fn fetch_stats(
        &self,
        season: Season,
        season_type: SeasonType,
    ) -> Result<Vec<SkaterStats>, DataError> {
        Err(DataError::NotInstalled {
            kind: DataKind::Stats,
            key: DataKey::SeasonType(season, season_type),
        })
    }
    fn fetch_career_history(&self, pid: PlayerId) -> Result<CareerHistory, DataError> {
        Err(DataError::NotInstalled {
            kind: DataKind::CareerHistory,
            key: DataKey::Player(pid),
        })
    }
}

pub struct DataStore {
    root: PathBuf,
    manifest: ManifestSet,
    clock: Arc<dyn Clock>,
    fetcher: Arc<dyn Fetcher>,
    live_feeds: bool,
    test_mode: bool,
}

impl DataStore {
    /// Open a DataStore rooted at `~/.icelines/data/`. Creates the
    /// directory and the manifest dir if missing. Defaults to
    /// `SystemClock`, `NoopFetcher`, `live_feeds=true`,
    /// `test_mode=false`.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, DataError> {
        let root: PathBuf = root.into();
        std::fs::create_dir_all(&root).map_err(|e| DataError::Io {
            path: root.clone(),
            source: e,
        })?;
        let manifest = ManifestSet::open(root.join("manifest"))?;
        Ok(Self {
            root,
            manifest,
            clock: Arc::new(SystemClock),
            fetcher: Arc::new(NoopFetcher),
            live_feeds: true,
            test_mode: false,
        })
    }

    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    pub fn with_fetcher(mut self, fetcher: Arc<dyn Fetcher>) -> Self {
        self.fetcher = fetcher;
        self
    }

    pub fn with_live_feeds(mut self, live: bool) -> Self {
        self.live_feeds = live;
        self
    }

    pub fn with_test_mode(mut self, test_mode: bool) -> Self {
        self.test_mode = test_mode;
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn manifest(&self) -> &ManifestSet {
        &self.manifest
    }

    /// Read order:
    /// 1. Manifest hit → load from disk.
    /// 2. Bundle hit → return embedded bytes (no manifest write).
    /// 3. live_feeds + !test_mode → lazy-fetch via Fetcher, persist,
    ///    return.
    /// 4. Otherwise → `NotInstalled`.
    pub fn load_bios(&self, season: Season) -> Result<Vec<SkaterBio>, DataError> {
        let kind = DataKind::Bios;
        let key = DataKey::Season(season);

        if let Some(entry) = self.manifest.get(kind, &key) {
            return self.read_json::<Vec<SkaterBio>>(&entry.path);
        }

        if let Some(bios) = bundled::get_bios(&season.as_str()) {
            return Ok(bios);
        }

        if self.live_feeds && !self.test_mode {
            self.lazy_fetch_banner(kind, &key);
            let bios = self.fetcher.fetch_bios(season)?;
            self.persist_bios(season, &bios)?;
            return Ok(bios);
        }

        Err(DataError::NotInstalled { kind, key })
    }

    pub fn load_stats(
        &self,
        season: Season,
        season_type: SeasonType,
    ) -> Result<Vec<SkaterStats>, DataError> {
        let kind = DataKind::Stats;
        let key = DataKey::SeasonType(season, season_type);

        if let Some(entry) = self.manifest.get(kind, &key) {
            return self.read_json::<Vec<SkaterStats>>(&entry.path);
        }

        // Bundle only carries regular-season stats today; playoff
        // stats live under separate `playoff-stats.json` files in
        // installed bundles. F.0.4 reads the regular-season slice;
        // playoff routing is a Foster.0 follow-up once
        // `bundled::get_playoff_stats` exists.
        if matches!(season_type, SeasonType::Regular) {
            if let Some(stats) = bundled::get_stats(&season.as_str()) {
                return Ok(stats);
            }
        }

        if self.live_feeds && !self.test_mode {
            self.lazy_fetch_banner(kind, &key);
            let stats = self.fetcher.fetch_stats(season, season_type)?;
            self.persist_stats(season, season_type, &stats)?;
            return Ok(stats);
        }

        Err(DataError::NotInstalled { kind, key })
    }

    /// Career history is per-player; returns `None` when the player
    /// isn't in the local store (newly-favorited rookies, etc.) and
    /// lazy-fetch is disabled or fails. Mirrors the spec's
    /// `Option<CareerHistory>` shape.
    pub fn load_career_history(&self, pid: PlayerId) -> Option<CareerHistory> {
        let kind = DataKind::CareerHistory;
        let key = DataKey::Player(pid);

        if let Some(entry) = self.manifest.get(kind, &key) {
            return self.read_json::<CareerHistory>(&entry.path).ok();
        }

        if self.live_feeds && !self.test_mode {
            self.lazy_fetch_banner(kind, &key);
            if let Ok(ch) = self.fetcher.fetch_career_history(pid) {
                let _ = self.persist_career_history(pid, &ch);
                return Some(ch);
            }
        }
        None
    }

    /// Synthesizes a Bundle freshness record for bundle hits
    /// (Static TTL, never stale). Manifest hits return their stored
    /// freshness directly. Misses return `None`.
    pub fn freshness(&self, kind: DataKind, key: &DataKey) -> Option<Freshness> {
        if let Some(entry) = self.manifest.get(kind, key) {
            return Some(entry.freshness);
        }
        // Bundle-served bios/stats: synthesize Bundle source.
        if matches!(kind, DataKind::Bios | DataKind::Stats) {
            if let DataKey::Season(s) | DataKey::SeasonType(s, _) = key {
                if bundled::BUNDLED_SEASONS.contains(&s.as_str().as_str()) {
                    return Some(Freshness {
                        fetched_at: chrono::DateTime::<Utc>::from_timestamp(0, 0)
                            .unwrap_or_else(Utc::now),
                        source: FetchSource::Bundle,
                        ttl: Ttl::Static,
                    });
                }
            }
        }
        None
    }

    /// Union of seasons available via manifest + bundle, deduped.
    pub fn list_seasons(&self, kind: DataKind) -> Vec<Season> {
        use std::collections::BTreeSet;
        let mut set: BTreeSet<Season> = BTreeSet::new();
        for entry in self.manifest.list(kind) {
            match entry.key {
                DataKey::Season(s) | DataKey::SeasonType(s, _) => {
                    set.insert(s);
                }
                _ => {}
            }
        }
        if matches!(kind, DataKind::Bios | DataKind::Stats) {
            for s in bundled::BUNDLED_SEASONS {
                if let Ok(n) = s.parse::<u32>() {
                    set.insert(Season(n));
                }
            }
        }
        set.into_iter().collect()
    }

    fn lazy_fetch_banner(&self, kind: DataKind, key: &DataKey) {
        // One-line stderr nudge so the user knows a network call is
        // happening (TAPE H2). Suppressed under test_mode (gated
        // higher up).
        let _ = self.clock.now();
        eprintln!(
            "icelines: fetching {kind:?} / {key:?} from NHL API…"
        );
    }

    fn persist_bios(&self, season: Season, bios: &[SkaterBio]) -> Result<(), DataError> {
        let path = self.bios_path(season);
        write_json_atomic(&path, &bios).map_err(|e| DataError::Io {
            path: path.clone(),
            source: e,
        })?;
        let entry = ManifestEntry {
            key: DataKey::Season(season),
            path: path.clone(),
            freshness: Freshness {
                fetched_at: self.clock.now(),
                source: FetchSource::Live,
                ttl: Ttl::After(std::time::Duration::from_secs(86400)),
            },
        };
        self.manifest.upsert(DataKind::Bios, entry)?;
        Ok(())
    }

    fn persist_stats(
        &self,
        season: Season,
        season_type: SeasonType,
        stats: &[SkaterStats],
    ) -> Result<(), DataError> {
        let path = self.stats_path(season, season_type);
        write_json_atomic(&path, &stats).map_err(|e| DataError::Io {
            path: path.clone(),
            source: e,
        })?;
        let entry = ManifestEntry {
            key: DataKey::SeasonType(season, season_type),
            path: path.clone(),
            freshness: Freshness {
                fetched_at: self.clock.now(),
                source: FetchSource::Live,
                ttl: Ttl::After(std::time::Duration::from_secs(86400)),
            },
        };
        self.manifest.upsert(DataKind::Stats, entry)?;
        Ok(())
    }

    fn persist_career_history(
        &self,
        pid: PlayerId,
        ch: &CareerHistory,
    ) -> Result<(), DataError> {
        let path = self.career_history_path(pid);
        write_json_atomic(&path, ch).map_err(|e| DataError::Io {
            path: path.clone(),
            source: e,
        })?;
        let entry = ManifestEntry {
            key: DataKey::Player(pid),
            path: path.clone(),
            freshness: Freshness {
                fetched_at: self.clock.now(),
                source: FetchSource::Live,
                ttl: Ttl::After(std::time::Duration::from_secs(7 * 86400)),
            },
        };
        self.manifest.upsert(DataKind::CareerHistory, entry)?;
        Ok(())
    }

    fn read_json<T: serde::de::DeserializeOwned>(&self, path: &Path) -> Result<T, DataError> {
        let bytes = std::fs::read(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                DataError::BackingFileMissing {
                    path: path.to_path_buf(),
                }
            } else {
                DataError::Io {
                    path: path.to_path_buf(),
                    source: e,
                }
            }
        })?;
        serde_json::from_slice(&bytes).map_err(|e| DataError::Corrupt {
            path: path.to_path_buf(),
            source: e,
        })
    }

    fn bios_path(&self, season: Season) -> PathBuf {
        self.root
            .join("seasons")
            .join(season.as_str())
            .join("bios.json")
    }

    fn stats_path(&self, season: Season, season_type: SeasonType) -> PathBuf {
        let file = match season_type {
            SeasonType::Regular => "stats.json",
            SeasonType::Playoff => "playoff-stats.json",
        };
        self.root
            .join("seasons")
            .join(season.as_str())
            .join(file)
    }

    fn career_history_path(&self, pid: PlayerId) -> PathBuf {
        self.root
            .join("career_history")
            .join(format!("{pid}.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Test fetcher with configurable per-call results so a single
    /// test can assert on routing without crossing the network.
    #[derive(Default)]
    struct MockFetcher {
        bios_response: Mutex<Option<Result<Vec<SkaterBio>, DataError>>>,
        bios_calls: Mutex<u32>,
        stats_calls: Mutex<u32>,
    }

    impl MockFetcher {
        fn set_bios(&self, r: Result<Vec<SkaterBio>, DataError>) {
            *self.bios_response.lock().unwrap() = Some(r);
        }
        fn bios_call_count(&self) -> u32 {
            *self.bios_calls.lock().unwrap()
        }
    }

    impl Fetcher for MockFetcher {
        fn fetch_bios(&self, season: Season) -> Result<Vec<SkaterBio>, DataError> {
            *self.bios_calls.lock().unwrap() += 1;
            self.bios_response.lock().unwrap().take().unwrap_or(Err(
                DataError::NotInstalled {
                    kind: DataKind::Bios,
                    key: DataKey::Season(season),
                },
            ))
        }
        fn fetch_stats(
            &self,
            season: Season,
            season_type: SeasonType,
        ) -> Result<Vec<SkaterStats>, DataError> {
            *self.stats_calls.lock().unwrap() += 1;
            Err(DataError::NotInstalled {
                kind: DataKind::Stats,
                key: DataKey::SeasonType(season, season_type),
            })
        }
        fn fetch_career_history(&self, pid: PlayerId) -> Result<CareerHistory, DataError> {
            Err(DataError::NotInstalled {
                kind: DataKind::CareerHistory,
                key: DataKey::Player(pid),
            })
        }
    }

    fn dummy_bio(id: u32) -> SkaterBio {
        // Round-trip via JSON so test bios match the camelCase wire
        // shape consumers persist.
        let json = serde_json::json!({
            "playerId": id,
            "skaterFullName": "Test Player",
            "lastName": "Player",
            "gamesPlayed": 0,
            "goals": 0,
            "assists": 0,
            "points": 0,
            "positionCode": "C",
            "currentTeamAbbrev": "EDM",
        });
        serde_json::from_value(json).expect("dummy SkaterBio")
    }

    #[test]
    fn l1_foster04_bundled_bios_hit_no_manifest_write() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap();
        // Current season is bundled.
        let s = Season(20252026);
        let bios = store.load_bios(s).expect("bundled hit");
        assert!(!bios.is_empty(), "bundle yields >0 bios");
        // Bundle hits do NOT write manifest entries.
        assert!(
            store.manifest().get(DataKind::Bios, &DataKey::Season(s)).is_none(),
            "bundle hit must not write manifest entry"
        );
    }

    #[test]
    fn l1_foster04_manifest_wins_over_bundle() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap();
        let s = Season(20252026);
        // Plant a manifest entry pointing at our hand-written bios
        // file. The DataStore must read THIS file, not the bundle.
        let custom_path = dir.path().join("custom-bios.json");
        let custom = vec![dummy_bio(99999)];
        write_json_atomic(&custom_path, &custom).unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(s),
                    path: custom_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::Manual,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();

        let bios = store.load_bios(s).unwrap();
        assert_eq!(bios.len(), 1);
        assert_eq!(bios[0].player_id, 99999, "manifest path won");
    }

    #[test]
    fn l1_foster04_offline_unbundled_returns_not_installed() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap().with_live_feeds(false);
        // 1980-81 is NOT in the bundle; manifest is empty.
        let s = Season(19801981);
        let err = store.load_bios(s).expect_err("must fail");
        assert!(matches!(err, DataError::NotInstalled { .. }));
    }

    #[test]
    fn l1_foster04_lazy_fetch_persists_and_writes_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = Arc::new(MockFetcher::default());
        fetcher.set_bios(Ok(vec![dummy_bio(8478402), dummy_bio(8475670)]));
        let store = DataStore::open(dir.path())
            .unwrap()
            .with_fetcher(fetcher.clone() as Arc<dyn Fetcher>);
        let s = Season(19801981);
        let bios = store.load_bios(s).expect("lazy fetch ok");
        assert_eq!(bios.len(), 2);
        assert_eq!(fetcher.bios_call_count(), 1, "fetcher hit once");
        // Disk + manifest entry persisted.
        assert!(
            store.manifest().get(DataKind::Bios, &DataKey::Season(s)).is_some(),
            "manifest entry written"
        );
        assert!(
            dir.path().join("seasons/19801981/bios.json").exists(),
            "bios.json on disk"
        );
        // Re-load is served from manifest, no second fetch.
        let _ = store.load_bios(s).unwrap();
        assert_eq!(fetcher.bios_call_count(), 1, "no second fetch");
    }

    #[test]
    fn l1_foster04_test_mode_bypasses_lazy_fetch() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = Arc::new(MockFetcher::default());
        fetcher.set_bios(Ok(vec![dummy_bio(1)]));
        let store = DataStore::open(dir.path())
            .unwrap()
            .with_fetcher(fetcher.clone() as Arc<dyn Fetcher>)
            .with_test_mode(true);
        let s = Season(19801981);
        let err = store.load_bios(s).expect_err("test_mode blocks fetch");
        assert!(matches!(err, DataError::NotInstalled { .. }));
        assert_eq!(fetcher.bios_call_count(), 0, "fetcher not called");
    }

    #[test]
    fn l1_foster04_lazy_fetch_network_error_propagates() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = Arc::new(MockFetcher::default());
        fetcher.set_bios(Err(DataError::Network {
            url: "https://api.nhle.com/...".into(),
            detail: "connection refused".into(),
        }));
        let store = DataStore::open(dir.path())
            .unwrap()
            .with_fetcher(fetcher as Arc<dyn Fetcher>);
        let s = Season(19801981);
        let err = store.load_bios(s).expect_err("network err propagates");
        assert!(matches!(err, DataError::Network { .. }));
    }

    #[test]
    fn l1_foster04_lazy_fetch_5xx_propagates() {
        let dir = tempfile::tempdir().unwrap();
        let fetcher = Arc::new(MockFetcher::default());
        fetcher.set_bios(Err(DataError::Http5xx {
            url: "https://api.nhle.com/...".into(),
            status: 503,
        }));
        let store = DataStore::open(dir.path())
            .unwrap()
            .with_fetcher(fetcher as Arc<dyn Fetcher>);
        let s = Season(19801981);
        let err = store.load_bios(s).expect_err("5xx propagates");
        assert!(matches!(err, DataError::Http5xx { status: 503, .. }));
    }

    #[test]
    fn l1_foster04_freshness_synthesizes_bundle_for_bundled_season() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap();
        let s = Season(20252026);
        let f = store
            .freshness(DataKind::Bios, &DataKey::Season(s))
            .expect("synthesized");
        assert_eq!(f.source, FetchSource::Bundle);
        assert!(matches!(f.ttl, Ttl::Static));
    }

    #[test]
    fn l1_foster04_freshness_returns_none_for_unbundled_unmanifested() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap();
        let s = Season(19801981);
        assert!(store.freshness(DataKind::Bios, &DataKey::Season(s)).is_none());
    }

    #[test]
    fn l1_foster04_list_seasons_unions_bundle_and_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let store = DataStore::open(dir.path()).unwrap();
        // Add an out-of-bundle season via manifest.
        let custom_path = dir.path().join("ancient.json");
        write_json_atomic(&custom_path, &Vec::<SkaterBio>::new()).unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(19801981)),
                    path: custom_path,
                    freshness: Freshness {
                        fetched_at: Utc::now(),
                        source: FetchSource::DataInstall,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();

        let seasons = store.list_seasons(DataKind::Bios);
        assert!(seasons.contains(&Season(19801981)), "manifest season present");
        assert!(seasons.contains(&Season(20252026)), "bundle season present");
    }
}
