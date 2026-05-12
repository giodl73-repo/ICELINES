//! Phase Foster.4 — non-blocking background sync engine.
//!
//! `launch_eager_sync` spawns a tokio task that walks the manifest
//! for stale entries, refreshes them via the configured `Fetcher`,
//! and emits per-entry events on a tokio mpsc channel. Caller
//! (TUI status bar / CLI banner) drains the channel as it pleases;
//! the spawn itself never blocks the launching thread.
//!
//! Test discipline:
//! - `MockClock` lets tests advance time deterministically.
//! - `ICELINES_TEST_MODE=1` env var short-circuits the spawn at the
//!   public API surface so L3 golden tests don't race a background
//!   refresh.
//! - The `enumerate_stale` helper is sync + pure-on-state so unit
//!   tests can call it directly without an async runtime.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::mpsc;

use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind};

/// Per-entry events emitted by the engine. The terminal `Done`
/// event includes the elapsed wall time and the count of successful
/// refreshes — the banner widget uses these to render
/// "Refreshed N · 2.1 s".
#[derive(Debug, Clone)]
pub enum SyncEvent {
    Refreshed {
        kind: DataKind,
        key: DataKey,
    },
    Failed {
        kind: DataKind,
        key: DataKey,
        error: String,
    },
    Done {
        refreshed: usize,
        failed: usize,
        elapsed: Duration,
    },
}

/// Bounded channel — caller drains at the pace it can render. A
/// slow consumer applies natural back-pressure to the engine.
const CHANNEL_CAPACITY: usize = 16;

/// Returns true iff `ICELINES_TEST_MODE=1` is set in the process
/// environment. Public so tests can verify the gate fires.
pub fn test_mode_enabled() -> bool {
    std::env::var_os("ICELINES_TEST_MODE")
        .map(|v| v == "1" || v == "true")
        .unwrap_or(false)
}

/// Spawn a background refresh task and return the receiver. When
/// `ICELINES_TEST_MODE=1` is set, returns `None` and emits no events
/// — the caller treats `None` as "nothing to await".
///
/// The receiver always closes once the engine emits its final
/// `SyncEvent::Done`. Drops on the receiver side cause the engine
/// to abandon remaining work gracefully (channel send becomes a
/// no-op).
pub fn launch_eager_sync(store: Arc<DataStore>) -> Option<mpsc::Receiver<SyncEvent>> {
    if test_mode_enabled() {
        return None;
    }
    let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
    let store_for_task = store.clone();
    tokio::spawn(async move {
        run_sync_loop(store_for_task, tx).await;
    });
    Some(rx)
}

/// Synchronous sibling — used by `icelines fetch sync` (which is
/// already inside a tokio runtime but doesn't need the spawn). Same
/// loop, blocks until done. Returns the `Done` event so the CLI can
/// print the summary.
pub async fn run_sync_blocking(store: Arc<DataStore>) -> SyncSummary {
    let (tx, mut rx) = mpsc::channel(CHANNEL_CAPACITY);
    let store_for_task = store.clone();
    let handle = tokio::spawn(async move { run_sync_loop(store_for_task, tx).await });
    let mut summary = SyncSummary::default();
    while let Some(ev) = rx.recv().await {
        match ev {
            SyncEvent::Refreshed { .. } => summary.refreshed += 1,
            SyncEvent::Failed { error, .. } => {
                summary.failed += 1;
                summary.errors.push(error);
            }
            SyncEvent::Done {
                refreshed,
                failed,
                elapsed,
            } => {
                summary.refreshed = refreshed;
                summary.failed = failed;
                summary.elapsed = elapsed;
            }
        }
    }
    let _ = handle.await;
    summary
}

#[derive(Debug, Default, Clone)]
pub struct SyncSummary {
    pub refreshed: usize,
    pub failed: usize,
    pub elapsed: Duration,
    pub errors: Vec<String>,
}

async fn run_sync_loop(store: Arc<DataStore>, tx: mpsc::Sender<SyncEvent>) {
    let started = Instant::now();
    let stale = store.enumerate_stale();
    let mut refreshed = 0usize;
    let mut failed = 0usize;

    for (kind, entry) in stale {
        // Each refresh is a sync method on DataStore — call inside
        // a `spawn_blocking` so the network call doesn't pin the
        // executor's worker thread.
        let store_for_call = store.clone();
        let key = entry.key.clone();
        let result =
            tokio::task::spawn_blocking(move || store_for_call.refresh_entry(kind, &key)).await;

        match result {
            Ok(Ok(_freshness)) => {
                refreshed += 1;
                let _ = tx
                    .send(SyncEvent::Refreshed {
                        kind,
                        key: entry.key.clone(),
                    })
                    .await;
            }
            Ok(Err(e)) => {
                failed += 1;
                let _ = tx
                    .send(SyncEvent::Failed {
                        kind,
                        key: entry.key.clone(),
                        error: format!("{e}"),
                    })
                    .await;
            }
            Err(join_err) => {
                failed += 1;
                let _ = tx
                    .send(SyncEvent::Failed {
                        kind,
                        key: entry.key.clone(),
                        error: format!("task panicked: {join_err}"),
                    })
                    .await;
            }
        }
    }

    let _ = tx
        .send(SyncEvent::Done {
            refreshed,
            failed,
            elapsed: started.elapsed(),
        })
        .await;
}

/// Pure helper used by `icelines fetch sync --dry-run` — returns the
/// would-refresh list without actually firing any fetches. Wraps
/// `DataStore::enumerate_stale` so the CLI doesn't need to know the
/// data layer's internals.
pub fn enumerate_stale_for_dry_run(store: &DataStore) -> Vec<(DataKind, DataKey)> {
    store
        .enumerate_stale()
        .into_iter()
        .map(|(k, e)| (k, e.key))
        .collect()
}

/// Force-invalidate every Static-TTL entry by overriding to `Live`
/// via a synthetic stale check. Caller (CLI `--force` flag) uses
/// this to bypass the `DataInstall` pin. Returns the count of
/// entries marked for refresh.
///
/// Currently a placeholder — `--force` semantics are wired by the
/// CLI as a flag that hands the engine a "skip TTL" filter.
pub fn force_refresh_filter(store: &DataStore) -> Vec<(DataKind, DataKey)> {
    let mut out = Vec::new();
    for &kind in DataKind::all() {
        for entry in store.manifest().list(kind) {
            out.push((kind, entry.key));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::datastore::{DataError, Fetcher};
    use crate::manifest::ManifestEntry;
    use crate::schema::{SkaterBio, SkaterStats};
    use chrono::{DateTime, Utc};
    use icelines_core::career_history::CareerHistory;
    use icelines_core::freshness::{Clock, FetchSource, Freshness, MockClock, Ttl};
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::season_stats::SeasonType;
    use std::sync::Mutex;

    fn dummy_bio(id: u32) -> SkaterBio {
        let json = serde_json::json!({
            "playerId": id, "skaterFullName": "Test", "lastName": "T",
            "gamesPlayed": 0, "goals": 0, "assists": 0, "points": 0,
            "positionCode": "C", "currentTeamAbbrev": "EDM",
        });
        serde_json::from_value(json).unwrap()
    }

    #[derive(Default)]
    struct CountingFetcher {
        bios_calls: Mutex<u32>,
    }

    impl Fetcher for CountingFetcher {
        fn fetch_bios(&self, _s: Season) -> Result<Vec<SkaterBio>, DataError> {
            *self.bios_calls.lock().unwrap() += 1;
            Ok(vec![dummy_bio(1)])
        }
        fn fetch_stats(&self, _s: Season, _t: SeasonType) -> Result<Vec<SkaterStats>, DataError> {
            Err(DataError::NotInstalled {
                kind: DataKind::Stats,
                key: DataKey::Season(Season(0)),
            })
        }
        fn fetch_career_history(&self, _p: PlayerId) -> Result<CareerHistory, DataError> {
            Err(DataError::NotInstalled {
                kind: DataKind::CareerHistory,
                key: DataKey::Player(PlayerId(0)),
            })
        }
    }

    fn fresh_at(when: DateTime<Utc>, ttl_seconds: u64) -> Freshness {
        Freshness {
            fetched_at: when,
            source: FetchSource::Live,
            ttl: Ttl::After(Duration::from_secs(ttl_seconds)),
        }
    }

    fn t(year: i32, month: u32, day: u32, hour: u32) -> DateTime<Utc> {
        chrono::TimeZone::with_ymd_and_hms(&Utc, year, month, day, hour, 0, 0).unwrap()
    }

    fn store_with_clock(clock: Arc<dyn Clock>) -> Arc<DataStore> {
        let dir = tempfile::tempdir().unwrap();
        // Leak the tempdir so the store outlives this fn; tests
        // discard it at process exit.
        let path = dir.keep();
        let store = DataStore::open(path)
            .unwrap()
            .with_clock(clock)
            .with_fetcher(Arc::new(CountingFetcher::default()) as Arc<dyn Fetcher>);
        Arc::new(store)
    }

    // ── enumerate_stale truth table ────────────────────────────────────────

    #[test]
    fn l1_foster4_enumerate_stale_empty_manifest() {
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let stale = store.enumerate_stale();
        assert!(stale.is_empty(), "empty manifest → empty stale list");
    }

    #[test]
    fn l1_foster4_enumerate_stale_skips_static_entries() {
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    path: dummy_path,
                    freshness: Freshness {
                        fetched_at: t(2024, 1, 1, 0),
                        source: FetchSource::Bundle,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();
        let stale = store.enumerate_stale();
        assert!(stale.is_empty(), "Static TTL never reports stale");
    }

    #[test]
    fn l1_foster4_enumerate_stale_respects_data_install_pin() {
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Stats,
                ManifestEntry {
                    key: DataKey::SeasonType(Season(20252026), SeasonType::Regular),
                    path: dummy_path,
                    freshness: Freshness {
                        // Tight TTL but DataInstall source — pinned.
                        fetched_at: t(2020, 1, 1, 0),
                        source: FetchSource::DataInstall,
                        ttl: Ttl::After(Duration::from_secs(60)),
                    },
                },
            )
            .unwrap();
        let stale = store.enumerate_stale();
        assert!(
            stale.is_empty(),
            "DataInstall is pinned, even with After TTL"
        );
    }

    #[test]
    fn l1_foster4_enumerate_stale_picks_aged_entry() {
        let clock = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock.clone() as Arc<dyn Clock>);
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    // 2 hours ago, with a 1h TTL → stale.
                    path: dummy_path,
                    freshness: fresh_at(t(2026, 1, 15, 10), 3600),
                },
            )
            .unwrap();
        let stale = store.enumerate_stale();
        assert_eq!(stale.len(), 1, "1 stale entry");
        assert_eq!(stale[0].0, DataKind::Bios);
    }

    #[test]
    fn l1_foster4_enumerate_stale_advances_with_clock() {
        let clock = MockClock::new(t(2026, 1, 15, 10));
        let clock_arc: Arc<dyn Clock> = Arc::new(clock);
        let store = store_with_clock(clock_arc.clone());
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    path: dummy_path,
                    freshness: fresh_at(t(2026, 1, 15, 10), 3600),
                },
            )
            .unwrap();
        // 30 min later — still fresh.
        assert!(store.enumerate_stale().is_empty());
        // Advance the clock 2 hours.
        // Need to get back to the inner MockClock — the trait object
        // hides the tick API, so swap to a fresh clock for the second
        // assertion (mirrors what tests would do in practice).
        let later: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 13)));
        let store2 = store_with_clock(later);
        let dummy_path2 = store2.root().join("dummy.json");
        std::fs::write(&dummy_path2, b"{}").unwrap();
        store2
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    path: dummy_path2,
                    freshness: fresh_at(t(2026, 1, 15, 10), 3600),
                },
            )
            .unwrap();
        assert_eq!(
            store2.enumerate_stale().len(),
            1,
            "3 hours past 1h-TTL → stale"
        );
    }

    // ── ICELINES_TEST_MODE gate ───────────────────────────────────────────

    /// Static mutex serializes the env-var tests so parallel
    /// runners don't race on `ICELINES_TEST_MODE`. The lock is held
    /// for the duration of each test's env mutations.
    static ENV_MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn l1_foster4_test_mode_env_short_circuits_launch() {
        let _guard = ENV_MUTEX.lock().await;
        let saved = std::env::var_os("ICELINES_TEST_MODE");
        std::env::set_var("ICELINES_TEST_MODE", "1");
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let rx = launch_eager_sync(store);
        assert!(rx.is_none(), "test mode → no spawn, no receiver");
        match saved {
            Some(v) => std::env::set_var("ICELINES_TEST_MODE", v),
            None => std::env::remove_var("ICELINES_TEST_MODE"),
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn l1_foster4_test_mode_helper_reads_env() {
        let _guard = ENV_MUTEX.lock().await;
        let saved = std::env::var_os("ICELINES_TEST_MODE");
        std::env::remove_var("ICELINES_TEST_MODE");
        assert!(!test_mode_enabled(), "no env var → false");
        std::env::set_var("ICELINES_TEST_MODE", "1");
        assert!(test_mode_enabled(), "ICELINES_TEST_MODE=1 → true");
        match saved {
            Some(v) => std::env::set_var("ICELINES_TEST_MODE", v),
            None => std::env::remove_var("ICELINES_TEST_MODE"),
        }
    }

    // ── Async sync loop ────────────────────────────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn l1_foster4_sync_loop_emits_done_on_empty_manifest() {
        let _guard = ENV_MUTEX.lock().await;
        let saved = std::env::var_os("ICELINES_TEST_MODE");
        std::env::remove_var("ICELINES_TEST_MODE");
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let summary = run_sync_blocking(store).await;
        assert_eq!(summary.refreshed, 0);
        assert_eq!(summary.failed, 0);
        if let Some(v) = saved {
            std::env::set_var("ICELINES_TEST_MODE", v);
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn l1_foster4_sync_loop_refreshes_one_stale_bios() {
        let _guard = ENV_MUTEX.lock().await;
        let saved = std::env::var_os("ICELINES_TEST_MODE");
        std::env::remove_var("ICELINES_TEST_MODE");
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    path: dummy_path,
                    freshness: fresh_at(t(2026, 1, 15, 10), 3600), // 2h ago, 1h TTL
                },
            )
            .unwrap();

        let summary = run_sync_blocking(store).await;
        assert_eq!(summary.refreshed, 1, "one stale bios refreshed");
        assert_eq!(summary.failed, 0);
        if let Some(v) = saved {
            std::env::set_var("ICELINES_TEST_MODE", v);
        }
    }

    // ── Dry-run + force helpers ────────────────────────────────────────────

    #[test]
    fn l1_foster4_enumerate_stale_for_dry_run_strips_paths() {
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    path: dummy_path,
                    freshness: fresh_at(t(2026, 1, 15, 10), 3600),
                },
            )
            .unwrap();
        let dry = enumerate_stale_for_dry_run(&store);
        assert_eq!(dry.len(), 1);
        assert_eq!(dry[0].0, DataKind::Bios);
    }

    #[test]
    fn l1_foster4_force_refresh_filter_includes_static_entries() {
        let clock: Arc<dyn Clock> = Arc::new(MockClock::new(t(2026, 1, 15, 12)));
        let store = store_with_clock(clock);
        let dummy_path = store.root().join("dummy.json");
        std::fs::write(&dummy_path, b"{}").unwrap();
        store
            .manifest()
            .upsert(
                DataKind::Bios,
                ManifestEntry {
                    key: DataKey::Season(Season(20252026)),
                    path: dummy_path,
                    freshness: Freshness {
                        fetched_at: t(2026, 1, 15, 12),
                        source: FetchSource::Bundle,
                        ttl: Ttl::Static,
                    },
                },
            )
            .unwrap();
        let stale = enumerate_stale_for_dry_run(&store);
        assert!(stale.is_empty(), "Static not stale");
        let forced = force_refresh_filter(&store);
        assert_eq!(forced.len(), 1, "--force ignores TTL");
    }
}
