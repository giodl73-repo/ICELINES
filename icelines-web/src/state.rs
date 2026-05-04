//! `WebState` — shared application state passed to every handler.
//!
//! Per the spec's "Concurrency & state" section
//! (`design/specs/web-dashboard.md`), the web server does NOT share
//! `App` with a running TUI process. They are different processes; the
//! only shared state is `~/.icelines/config.toml` and
//! `~/.icelines/icelines.db`.
//!
//! ## King.1.1 shape — empty
//!
//! Today this struct holds no fields — King.1's stub `/` handler doesn't
//! need state. The struct exists so handler signatures and the router
//! API are stable; later sub-phases add fields without changing the
//! `Router::with_state` plumbing.
//!
//! ## Target shape (post-King.1.2 .. King.9)
//!
//! ```ignore
//! pub struct WebState {
//!     pub repo: Arc<RwLock<StatsRepository>>,
//!     pub config: Arc<RwLock<Config>>,
//!     pub fantasy_db: Arc<FantasyDb>,
//!     pub group_db: Arc<GroupDb>,
//!     pub cache: Arc<ResponseCache>,
//! }
//! ```
//!
//! Whether the repository is `Arc<RwLock<StatsRepository>>` or sits
//! behind a `LocalSet`/actor pattern is the King.1.2 decision; the
//! choice gets recorded in the King.1 plan file's "Outcomes" block.
//! `Config` lives in `icelines-cli` today and may move to
//! `icelines-core` if King.6 needs it shared without inverting the
//! crate dependency chain.

use std::sync::Arc;

use icelines_core::stats_repository::StatsRepository;
use tokio::sync::RwLock;

use crate::config::WebConfig;

/// Shared state handed to every handler via axum's
/// `State<WebState>` extractor.
///
/// Cloning a `WebState` clones the inner `Arc`s — cheap, no deep copy.
/// Per the spec's "Lock discipline" subsection, reads take
/// `repo.read().await` (concurrent, non-blocking) and writes take
/// `repo.write().await` (exclusive, brief). Lazy career fan-outs build
/// a temp repo, then take the write lock only for the LRU swap — so the
/// 50ms fan-out NEVER blocks readers.
#[derive(Clone, Default)]
pub struct WebState {
    /// `StatsRepository` behind a tokio RwLock so multi-threaded axum
    /// handlers can read concurrently. Phase Hart's `!Send + !Sync`
    /// soft-lint is bypassed via `icelines-core`'s `send-sync` feature
    /// (King.1.2 decision). The borrow checker still prevents
    /// concurrent mutation through `&` shared access.
    pub repo: Arc<RwLock<StatsRepository>>,

    /// Active-season slice for the sticky header rendered on every
    /// page. King.1.x patch (broadcast finding): advanced from the
    /// originally-planned King.6 to King.1.4 so the askama base
    /// template can render the active label without re-architecture.
    /// King.6's PATCH `/api/v1/active-season` mutates this through
    /// `RwLock::write`.
    pub config: Arc<RwLock<WebConfig>>,
    // Future fields land alongside the routes that need them:
    //   pub fantasy_db: Arc<FantasyDb>,                 // King.9
    //   pub group_db: Arc<GroupDb>,                     // King.8
    //   pub cache: Arc<moka::sync::Cache<_, _>>,        // King.2
}

impl WebState {
    /// Construct a WebState with an empty repository and default
    /// (current-season, regular) config. King.1.5 wires this from
    /// `commands::serve::run`, populating the repo via the existing
    /// `stats_loader::load_into_repo` path and overriding `config`
    /// with the user's `~/.icelines/config.toml` values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from an already-built repository (handy for tests
    /// and for the future serve driver that loads a fixture).
    pub fn with_repo(repo: StatsRepository) -> Self {
        Self {
            repo: Arc::new(RwLock::new(repo)),
            config: Arc::new(RwLock::new(WebConfig::default())),
        }
    }

    /// Construct with both repo and explicit config. The serve driver
    /// (King.1.5) uses this to inject the user's active-season choice.
    pub fn with_repo_and_config(repo: StatsRepository, config: WebConfig) -> Self {
        Self {
            repo: Arc::new(RwLock::new(repo)),
            config: Arc::new(RwLock::new(config)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// l0_web_state_is_send_sync
    /// — axum requires the router state to be `Clone + Send + Sync +
    ///   'static`. Today's empty `WebState` is trivially Send+Sync.
    ///   King.1.2 may add the repo and must preserve this property —
    ///   if this test fails after a field is added, the new field is
    ///   not Send+Sync and the spec's concurrency-model decision needs
    ///   revisiting.
    #[test]
    fn l0_web_state_is_send_sync() {
        fn assert_send_sync<T: Send + Sync + 'static>() {}
        assert_send_sync::<WebState>();
    }

    /// l0_web_state_is_clone
    /// — axum's State extractor calls `Clone::clone` per request. The
    ///   struct must remain cheap to clone (Arc bumps only) as fields
    ///   are added. If a non-Arc field lands, this test still passes
    ///   but the per-request cost grows — reviewers should catch it.
    #[test]
    fn l0_web_state_is_clone() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<WebState>();
    }

    /// l0_repo_clone_bumps_arc_strong_count
    /// — King.1.2 fence: cloning WebState should NOT clone the
    ///   `StatsRepository` itself (which is expensive — millions of
    ///   stat rows). It should only bump the `Arc` strong count.
    ///   If a future refactor accidentally swaps Arc<RwLock> for an
    ///   owned RwLock, the per-request cost explodes and this test
    ///   catches it.
    #[test]
    fn l0_repo_clone_bumps_arc_strong_count() {
        let state = WebState::new();
        let strong_before = std::sync::Arc::strong_count(&state.repo);
        let _clone = state.clone();
        let strong_after = std::sync::Arc::strong_count(&state.repo);
        assert_eq!(
            strong_after,
            strong_before + 1,
            "WebState::clone must bump the repo Arc by exactly one (cheap clone, no deep copy)"
        );
    }

    /// l0_repo_concurrent_reads_dont_block_each_other
    /// — Spec's "Lock discipline" rule: reads take `repo.read()`
    ///   (concurrent, non-blocking). Two simultaneous readers must
    ///   coexist. If the type is ever changed to `Mutex<>` (which
    ///   serializes reads), this fails.
    #[tokio::test]
    async fn l0_repo_concurrent_reads_dont_block_each_other() {
        let state = WebState::new();
        let r1 = state.repo.read().await;
        // A second read should also succeed; if RwLock is misused
        // (downgraded to Mutex), this would deadlock the test.
        let r2 = state.repo.read().await;
        // Both guards live, both can read.
        assert_eq!(r1.resident_windows(), 0);
        assert_eq!(r2.resident_windows(), 0);
    }

    /// l0_repo_concurrent_reads_dont_block_across_tasks
    /// — King.1.x patch (bench review): the previous test ran two
    ///   `.read().await` calls sequentially on a single task, which
    ///   only proves API shape. This version spawns two concurrent
    ///   tasks each holding a read guard and asserts both make
    ///   progress within a tight bound — proving non-blocking
    ///   semantics even under interleaved scheduling. If the type is
    ///   downgraded to a serializing primitive (`Mutex`), the second
    ///   task would block on the first's guard release and the test
    ///   would still complete (eventually) but the timing assertion
    ///   would catch the regression.
    #[tokio::test]
    async fn l0_repo_concurrent_reads_dont_block_across_tasks() {
        use tokio::sync::Barrier;

        let state = WebState::new();
        // Barrier ensures both tasks have actually acquired their read
        // guards before either releases — proves true parallelism.
        let barrier = std::sync::Arc::new(Barrier::new(2));

        let s1 = state.clone();
        let b1 = barrier.clone();
        let t1 = tokio::spawn(async move {
            let _guard = s1.repo.read().await;
            b1.wait().await; // wait for t2 to also hold its guard
            std::time::Instant::now()
        });

        let s2 = state.clone();
        let b2 = barrier.clone();
        let t2 = tokio::spawn(async move {
            let _guard = s2.repo.read().await;
            b2.wait().await; // wait for t1 to also hold its guard
            std::time::Instant::now()
        });

        // Both must complete; under Mutex this barrier would
        // deadlock (only one guard at a time → second task can't
        // proceed). Wrapping in a generous timeout converts a true
        // deadlock from CI hang to a clear test failure.
        let result =
            tokio::time::timeout(std::time::Duration::from_secs(5), futures_join(t1, t2)).await;
        assert!(
            result.is_ok(),
            "concurrent read tasks deadlocked — RwLock semantics may have regressed to Mutex"
        );
    }

    /// Tiny helper: join two `JoinHandle`s without pulling the
    /// `futures` crate (we'd rather not bloat icelines-web's deps for
    /// one helper).
    async fn futures_join<T>(
        a: tokio::task::JoinHandle<T>,
        b: tokio::task::JoinHandle<T>,
    ) -> (
        Result<T, tokio::task::JoinError>,
        Result<T, tokio::task::JoinError>,
    ) {
        tokio::join!(a, b)
    }
}
