//! Background player loading for the TUI.
//!
//! Loads all players from bundled/snapshot data without blocking the event loop.
//!
//! Hart.5c.6 Phase A: introduces the mpsc-based repo loader
//! (`spawn_repo_load` + `RepoLoadResult`) alongside the existing
//! `Arc<Mutex<LoadInner>>` path. The new shape is required because
//! `LoadOutcome` (containing `StatsRepository`) is `!Send`, so the
//! background task must run on a `tokio::task::spawn_local` inside a
//! `LocalSet`. Subsequent phases migrate consumers off the legacy path
//! and delete it.

use std::sync::{Arc, Mutex};
use icelines_core::model::{Goalie, Player, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::Transaction;
use icelines_fetch::stats_loader::LoadOutcome;
// Re-export for ergonomic import elsewhere in the TUI; canonical home is
// icelines-fetch::stats_loader (KEEL v4 — pure logic shared across surfaces).
pub use icelines_fetch::stats_loader::format_missing_sources;
use tokio::sync::mpsc;

/// Per-load transactions bundle. Empty/default when the snapshot is
/// missing — UI renders the empty legend card in that case.
#[derive(Debug, Clone, Default)]
pub struct TransactionsLoad {
    pub rows:        Vec<Transaction>,
    pub fetched_at:  String,
    pub stale:       bool,
}

/// Shared loading state readable from the event loop.
#[derive(Debug, Clone)]
pub struct LoadState {
    inner: Arc<Mutex<LoadInner>>,
}

#[derive(Debug)]
struct LoadInner {
    pub players:      Vec<Player>,
    pub goalies:      Vec<Goalie>,
    pub transactions: TransactionsLoad,
    pub loading:      bool,
    pub error:        Option<String>,
}

impl LoadState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(LoadInner {
                players:      Vec::new(),
                goalies:      Vec::new(),
                transactions: TransactionsLoad::default(),
                loading:      true,
                error:        None,
            })),
        }
    }

    #[allow(dead_code)]
    pub fn is_loading(&self) -> bool {
        self.inner.lock().map(|g| g.loading).unwrap_or(false)
    }

    pub fn take_players(&self) -> Option<Vec<Player>> {
        let mut g = self.inner.lock().ok()?;
        if !g.loading && g.error.is_none() && !g.players.is_empty() {
            Some(std::mem::take(&mut g.players))
        } else {
            None
        }
    }

    /// Pop the loaded goalie pool. Phase G.3.
    pub fn take_goalies(&self) -> Option<Vec<Goalie>> {
        let mut g = self.inner.lock().ok()?;
        if !g.loading && g.error.is_none() && !g.goalies.is_empty() {
            Some(std::mem::take(&mut g.goalies))
        } else {
            None
        }
    }

    /// Pop the loaded transactions bundle. Phase T.5. Returns Some even
    /// when `rows.is_empty()` so the UI can distinguish "still loading"
    /// from "loaded, empty" (and render the legend card for the latter).
    pub fn take_transactions(&self) -> Option<TransactionsLoad> {
        let mut g = self.inner.lock().ok()?;
        if !g.loading && g.error.is_none() {
            Some(std::mem::take(&mut g.transactions))
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn error(&self) -> Option<String> {
        self.inner.lock().ok().and_then(|g| g.error.clone())
    }

    fn set_done(&self, players: Vec<Player>, goalies: Vec<Goalie>, transactions: TransactionsLoad) {
        if let Ok(mut g) = self.inner.lock() {
            g.players      = players;
            g.goalies      = goalies;
            g.transactions = transactions;
            g.loading      = false;
        }
    }

    fn set_error(&self, msg: String) {
        if let Ok(mut g) = self.inner.lock() {
            g.error   = Some(msg);
            g.loading = false;
        }
    }
}

/// Spawn a tokio task that loads players + goalies + transactions in
/// parallel and stores them in `state`. Phase T.5 extends the goalies
/// pattern with a third bundle.
pub fn spawn_loader(state: LoadState) {
    tokio::spawn(async move {
        let players = crate::commands::players::load_all_players();
        let goalies = crate::commands::players::load_all_goalies();
        let transactions = load_transactions_bundle();

        match players {
            Err(e) => { state.set_error(e.to_string()); }
            Ok(p) => {
                let g = goalies.unwrap_or_default();
                state.set_done(p, g, transactions);
            }
        }
    });
}

// ── Hart.5c.6 mpsc repo loader ────────────────────────────────────────────
//
// `RepoLoadResult` is the payload sent across the channel from the
// background load task (running on `LocalSet`) to the TUI event loop.
// `LoadOutcome` carries `StatsRepository` which is `!Send` — that's why
// this whole chain runs single-threaded via `spawn_local`.

/// One-shot result delivered by the spawned repo load.
pub type RepoLoadResult = Result<LoadOutcome, String>;

/// Spawn a `spawn_local` task that calls `load_into_repo(season, ty, store)`
/// and forwards the result over the returned receiver. The receiver is
/// `try_recv`-polled from the App's per-tick `poll_repo_load`.
///
/// **Must be called inside a `tokio::task::LocalSet`** — `spawn_local`
/// panics otherwise. The TUI bootstrap pins this requirement.
pub fn spawn_repo_load(
    season: Season,
    season_type: SeasonType,
    snapshot_dir: std::path::PathBuf,
) -> mpsc::UnboundedReceiver<RepoLoadResult> {
    let (tx, rx) = mpsc::unbounded_channel::<RepoLoadResult>();
    tokio::task::spawn_local(async move {
        // Run the synchronous loader on the local task. It's I/O-bound
        // (disk + JSON parse) but at N≈1000 the latency is ~50ms cold,
        // well within the TUI's 100ms event poll.
        let store = icelines_fetch::snapshot::SnapshotStore::new(snapshot_dir);
        let result = icelines_fetch::stats_loader::load_into_repo(season, season_type, &store)
            .map_err(|e| e.to_string());
        // App may have moved on (rare); ignore send failures.
        let _ = tx.send(result);
    });
    rx
}

// `format_missing_sources` lives in icelines-fetch::stats_loader (canonical
// home — pure logic shared across all four surfaces). Re-exported above for
// import ergonomics inside the TUI module tree.

/// Best-effort transactions load. Failure to find a snapshot for the
/// current season is normal (legend card path) — we never bail.
fn load_transactions_bundle() -> TransactionsLoad {
    use icelines_fetch::{
        bundled::load_transactions_with_fallback,
        snapshot::{SnapshotMetaFlags, SnapshotStore},
    };

    let cfg = match crate::config::Config::load() {
        Ok(c) => c,
        Err(_) => return TransactionsLoad::default(),
    };
    let snapshots_root = cfg.snapshot_dir();
    let store = SnapshotStore::new(snapshots_root.clone());
    let season = cfg.season_str();

    let envelope = load_transactions_with_fallback(&season, &store);
    let flags = SnapshotMetaFlags::load(&snapshots_root, &season);
    match envelope {
        Ok(env) => TransactionsLoad {
            rows:       env.rows,
            fetched_at: env.fetched_at,
            stale:      flags.transactions_stale,
        },
        Err(_) => TransactionsLoad {
            rows:       Vec::new(),
            fetched_at: String::new(),
            stale:      flags.transactions_stale,
        },
    }
}

// ── Season install state ──────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum InstallPhase {
    Idle,
    Downloading(String), // season id
    Done(String, u64),   // season id, KB
    Error(String, String), // season id, message
}

#[derive(Debug, Clone)]
pub struct InstallState {
    inner: Arc<Mutex<InstallPhase>>,
}

impl InstallState {
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(InstallPhase::Idle)) }
    }

    pub fn phase(&self) -> InstallPhase {
        self.inner.lock().map(|g| g.clone()).unwrap_or(InstallPhase::Idle)
    }

    fn set(&self, phase: InstallPhase) {
        if let Ok(mut g) = self.inner.lock() { *g = phase; }
    }

    /// Drive a specific phase from a test. Bypasses the spawn_install pipeline
    /// so render assertions can target each branch deterministically.
    #[cfg(test)]
    pub fn force_phase(&self, phase: InstallPhase) {
        self.set(phase);
    }
}

/// Spawn a background season install. Updates `state` with progress.
pub fn spawn_install(season: String, state: InstallState) {
    let state2 = state.clone();
    state.set(InstallPhase::Downloading(season.clone()));
    tokio::spawn(async move {
        match crate::commands::data::install_season_tui(&season).await {
            Ok(kb)  => state2.set(InstallPhase::Done(season, kb)),
            Err(e)  => state2.set(InstallPhase::Error(season, e.to_string())),
        }
    });
}
