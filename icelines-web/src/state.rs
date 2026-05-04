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

/// Shared state handed to every handler via axum's
/// `State<WebState>` extractor.
///
/// Cloning a `WebState` will clone the inner `Arc`s once fields land —
/// cheap, no deep copy. Today the empty struct is trivially `Copy`-like
/// at zero cost.
#[derive(Clone, Default)]
pub struct WebState {
    // Fields land here as later sub-phases need them:
    //   pub repo: Arc<RwLock<StatsRepository>>,        // King.1.2 / King.2
    //   pub config: Arc<RwLock<Config>>,                // King.6
    //   pub fantasy_db: Arc<FantasyDb>,                 // King.9
    //   pub group_db: Arc<GroupDb>,                     // King.8
    //   pub cache: Arc<moka::sync::Cache<_, _>>,        // King.2
    //
    // Pin the empty-now / fields-later contract here so reviewers
    // remember to add the L0 `Send + Sync` fence once any non-trivial
    // type joins.
}

impl WebState {
    /// Construct an empty WebState. King.1.5 wires this from
    /// `commands::serve::run`; King.1.2+ extends with real fields.
    pub fn new() -> Self {
        Self::default()
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
}
