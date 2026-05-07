//! Phase Art Ross A.0 — `DataProvider` trait + `EvalCtx`.
//!
//! The dependency-inversion seam between `icelines-query` (which
//! owns the IR + parser + planner) and `icelines-fetch` (which
//! owns DataStore + NHL API). Per the layering rule (CLAUDE.md):
//! query is a lower crate than fetch, so query cannot reach into
//! fetch directly. `DataProvider` is owned here; the impl lives
//! in `icelines-fetch::query_provider` and is injected by the
//! surface (CLI / web / TUI) at call time.

use std::marker::PhantomData;

use crate::plan::StrictMode;

/// Where the planner asks the surface to ensure data is local.
/// Surfaces yield `FetchEvent`s as work progresses; the library
/// never writes to stdout/stderr.
pub trait DataProvider {
    /// Ensure the data described in `req` is available locally.
    /// Implementations stream `FetchEvent`s via `events`. Returns
    /// `Ok(())` once everything is ensured; `Err(...)` if a fetch
    /// fails or the user passed `--no-fetch` and data is missing.
    fn ensure(
        &self,
        req: &PlanRequirement,
        events: &mut dyn FnMut(FetchEvent),
    ) -> Result<(), FetchError>;

    /// A.2 — fetch per-game stat lines for one player in one
    /// season. Returns the lines sorted ascending by date.
    /// Returns an empty Vec when the player has no boxscore data
    /// in this season (or the season isn't eligible — pre-2021-22
    /// where Foster +3 boxscore persistence doesn't cover).
    ///
    /// Implementations should consult the `BoxscoreIndex` first;
    /// fall back to lazy boxscore parse if the index hasn't been
    /// built yet for this season.
    fn fetch_game_lines(
        &self,
        player_id: u32,
        season: u32,
    ) -> Vec<crate::sliding_window::GameStatLine> {
        // Default: no data. Implementations override.
        let _ = (player_id, season);
        Vec::new()
    }
}

/// What data the plan needs to run. Computed by
/// `QueryPlan::requirements()` (planner) and consumed by
/// `DataProvider::ensure` (surface).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlanRequirement {
    pub seasons_needed: Vec<u32>,
    pub reports_needed: Vec<&'static str>,
    pub boxscore_seasons_needed: Vec<u32>,
    pub boxscore_date_range: Option<DateRange>,
    pub career_pids_needed: Vec<u32>,
    pub eligible_for_strict: StrictEligibility,
}

/// Inclusive date range (`start..=end`), serialized as ISO-8601
/// strings so the IR is wire-stable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DateRange {
    pub start: String,
    pub end: String,
}

/// Whether the plan can satisfy each `StrictMode` level.
/// Computed during `requirements()`; checked between
/// requirements computation and the first fetch (R12 — strict
/// rejects pre-materialize).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrictEligibility {
    pub all_seasons_have_boxscores: bool,
    pub all_pids_have_career_history: bool,
    /// Seasons that would emit `[fallback: <season>]` markers.
    pub fallback_seasons: Vec<u32>,
}

impl StrictEligibility {
    /// True iff the plan satisfies the given strict mode without
    /// producing any partial markers. The check fires BEFORE any
    /// fetch — strict-violating plans error out before network I/O.
    pub fn satisfies(&self, mode: StrictMode) -> bool {
        match mode {
            StrictMode::Off => true,
            StrictMode::RejectPartialSeasons => self.fallback_seasons.is_empty(),
            StrictMode::RejectPartialWindows => true, // checked at materialize
            StrictMode::RejectAll => self.fallback_seasons.is_empty(),
        }
    }
}

/// Streaming progress events from `DataProvider::ensure`. The
/// surface renders these — CLI prints stderr banners; web pushes
/// SSE; TUI updates a sync banner widget. The library never writes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchEvent {
    Started { units: u32, label: String },
    Progress { done: u32, total: u32 },
    Complete,
    Failed { reason: String },
}

#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum FetchError {
    #[error("data missing for {what} and --no-fetch is set")]
    DataMissingNoFetch { what: String },

    #[error("fetch failed: {reason}")]
    FetchFailed { reason: String },

    #[error("strict mode {mode:?} rejects this plan: {reason}")]
    StrictRejected { mode: StrictMode, reason: String },
}

// ── EvalCtx ─────────────────────────────────────────────────────

/// The execution context handed to `materialize` and `execute`.
/// `EvalCtx` is `!Send` because it holds a `&StatsRepository`
/// reference (which is `!Send` per Hart). Async callers must run
/// it via `spawn_local`, never `tokio::spawn`. The compile_fail
/// doctest below pins this.
///
/// ```compile_fail
/// use icelines_query::data_provider::EvalCtx;
/// fn assert_send<T: Send>() {}
/// fn require_send_eval_ctx() {
///     assert_send::<EvalCtx<'_>>();
/// }
/// ```
pub struct EvalCtx<'a> {
    pub provider: &'a dyn DataProvider,
    pub strict: StrictMode,
    pub no_fetch: bool,
    /// Active season-id for the query. Sliding-window calendar
    /// atoms (`g.last30d`) measure from `today`; season-typed
    /// atoms (`AT age<=22`) compute age relative to `season`.
    pub season: u32,
    /// Anchor date for calendar windows. Set by the surface
    /// (CLI uses `Utc::now().date_naive()`; tests inject via
    /// `MockClock`). Defaults to today.
    pub today: chrono::NaiveDate,
    /// Marker forcing `!Send` so async accidents (`tokio::spawn`)
    /// fail at compile time, not at runtime.
    _not_send: PhantomData<*const ()>,
}

impl<'a> EvalCtx<'a> {
    pub fn new(provider: &'a dyn DataProvider, strict: StrictMode, no_fetch: bool) -> Self {
        Self {
            provider,
            strict,
            no_fetch,
            season: icelines_core::CURRENT_SEASON,
            today: chrono::Utc::now().date_naive(),
            _not_send: PhantomData,
        }
    }

    /// Override the anchor date. Used by tests with `MockClock` and
    /// by `--date YYYY-MM-DD` (when wired in A.5).
    pub fn with_today(mut self, today: chrono::NaiveDate) -> Self {
        self.today = today;
        self
    }

    /// Override the active season. Used when `--season YYYYZZZZ`
    /// is set on a query subcommand.
    pub fn with_season(mut self, season: u32) -> Self {
        self.season = season;
        self
    }

    /// Run the strict-eligibility gate against this context's
    /// `StrictMode`. Called between `requirements()` and the first
    /// `provider.ensure()` (R12).
    pub fn strict_check(&self, eligibility: &StrictEligibility) -> Result<(), FetchError> {
        if !eligibility.satisfies(self.strict) {
            let reason = if !eligibility.fallback_seasons.is_empty() {
                format!(
                    "fallback seasons would be used: {:?}",
                    eligibility.fallback_seasons
                )
            } else {
                "plan would produce partial answers".to_string()
            };
            return Err(FetchError::StrictRejected {
                mode: self.strict,
                reason,
            });
        }
        Ok(())
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// A no-op DataProvider for unit-test purposes.
    struct MockProvider;
    impl DataProvider for MockProvider {
        fn ensure(
            &self,
            _req: &PlanRequirement,
            _events: &mut dyn FnMut(FetchEvent),
        ) -> Result<(), FetchError> {
            Ok(())
        }
    }

    #[test]
    fn l0_strict_off_always_satisfies() {
        let elig = StrictEligibility {
            fallback_seasons: vec![19891990],
            ..Default::default()
        };
        assert!(elig.satisfies(StrictMode::Off));
    }

    #[test]
    fn l0_strict_reject_partial_seasons_blocks_fallback() {
        let elig = StrictEligibility {
            fallback_seasons: vec![19891990],
            ..Default::default()
        };
        assert!(!elig.satisfies(StrictMode::RejectPartialSeasons));
        assert!(!elig.satisfies(StrictMode::RejectAll));
    }

    #[test]
    fn l0_strict_passes_when_no_fallbacks() {
        let elig = StrictEligibility {
            fallback_seasons: vec![],
            ..Default::default()
        };
        assert!(elig.satisfies(StrictMode::RejectPartialSeasons));
        assert!(elig.satisfies(StrictMode::RejectAll));
    }

    #[test]
    fn l0_eval_ctx_strict_check_errors_on_violation() {
        let provider = MockProvider;
        let ctx = EvalCtx::new(&provider, StrictMode::RejectAll, false);
        let elig = StrictEligibility {
            fallback_seasons: vec![19891990],
            ..Default::default()
        };
        let err = ctx.strict_check(&elig).unwrap_err();
        assert!(matches!(err, FetchError::StrictRejected { .. }));
    }

    #[test]
    fn l0_eval_ctx_strict_check_passes_on_clean_plan() {
        let provider = MockProvider;
        let ctx = EvalCtx::new(&provider, StrictMode::RejectAll, false);
        let elig = StrictEligibility::default();
        assert!(ctx.strict_check(&elig).is_ok());
    }
}
