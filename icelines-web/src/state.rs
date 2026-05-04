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
    // Future fields:
    //   pub config: Arc<RwLock<Config>>,                // King.6
    //   pub fantasy_db: Arc<FantasyDb>,                 // King.9
    //   pub group_db: Arc<GroupDb>,                     // King.8
    //   pub cache: Arc<moka::sync::Cache<_, _>>,        // King.2
}

impl WebState {
    /// Construct a WebState with an empty repository. King.1.5 wires
    /// this from `commands::serve::run`, populating the repo via the
    /// existing `stats_loader::load_into_repo` path.
    pub fn new() -> Self {
        Self::default()
    }

    /// Construct from an already-built repository (handy for tests
    /// and for the future serve driver that loads a fixture).
    pub fn with_repo(repo: StatsRepository) -> Self {
        Self {
            repo: Arc::new(RwLock::new(repo)),
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
}
