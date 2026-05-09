//! Phase Foster.0.7 — capability matrix coverage.
//!
//! 24 tests pinning the typed config schema:
//! - 18 mode-honored (5 capabilities × 3 modes happy path + 3 shifts negative)
//! - 6 interaction tests (truth tables, sync section round-trips)
//!
//! These are integration tests against the `Config` API directly,
//! not subprocess shell-out — `~/.icelines/config.toml` lives in the
//! user's home so isolating it across test threads is fragile. The
//! L2 surface (`icelines config get/set/list/reset`) is a thin
//! shell over these methods; persona tests in Foster.6 cover the
//! subprocess surface.

use icelines_cli::config::{
    BannerMode, Capability, CapabilityError, CapabilityMatrix, CapabilityMode, Config,
    SeasonTransition, SyncConfig, SyncPolicy,
};

fn fresh() -> Config {
    Config {
        csv_path: None,
        cache_dir: std::path::PathBuf::new(),
        season: None,
        live: None,
        dashboards: None,
        reports: Default::default(),
        sync: SyncConfig::default(),
    }
}

// ── Mode-honored (18) ───────────────────────────────────────────────────────
//
// For every capability EXCEPT shifts, all three modes must round-trip
// through `set_key` / `get_key`. Shifts has its own three-case block
// (off OK, favorites + league rejected with literal error).

#[test]
fn l1_foster07_stats_off() {
    let mut c = fresh();
    c.set_key("sync.capabilities.stats", "off").unwrap();
    assert_eq!(c.get_key("sync.capabilities.stats").unwrap(), "off");
}

#[test]
fn l1_foster07_stats_favorites() {
    let mut c = fresh();
    c.set_key("sync.capabilities.stats", "favorites").unwrap();
    assert_eq!(c.get_key("sync.capabilities.stats").unwrap(), "favorites");
}

#[test]
fn l1_foster07_stats_league() {
    let mut c = fresh();
    c.set_key("sync.capabilities.stats", "league").unwrap();
    assert_eq!(c.get_key("sync.capabilities.stats").unwrap(), "league");
}

#[test]
fn l1_foster07_scores_schedule_off() {
    let mut c = fresh();
    c.set_key("sync.capabilities.scores_schedule", "off")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.scores_schedule").unwrap(),
        "off"
    );
}

#[test]
fn l1_foster07_scores_schedule_favorites() {
    let mut c = fresh();
    c.set_key("sync.capabilities.scores_schedule", "favorites")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.scores_schedule").unwrap(),
        "favorites"
    );
}

#[test]
fn l1_foster07_scores_schedule_league() {
    let mut c = fresh();
    c.set_key("sync.capabilities.scores_schedule", "league")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.scores_schedule").unwrap(),
        "league"
    );
}

#[test]
fn l1_foster07_transactions_off() {
    let mut c = fresh();
    c.set_key("sync.capabilities.transactions", "off").unwrap();
    assert_eq!(c.get_key("sync.capabilities.transactions").unwrap(), "off");
}

#[test]
fn l1_foster07_transactions_favorites() {
    let mut c = fresh();
    c.set_key("sync.capabilities.transactions", "favorites")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.transactions").unwrap(),
        "favorites"
    );
}

#[test]
fn l1_foster07_transactions_league() {
    let mut c = fresh();
    c.set_key("sync.capabilities.transactions", "league")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.transactions").unwrap(),
        "league"
    );
}

#[test]
fn l1_foster07_boxscores_off() {
    let mut c = fresh();
    c.set_key("sync.capabilities.boxscores", "off").unwrap();
    assert_eq!(c.get_key("sync.capabilities.boxscores").unwrap(), "off");
}

#[test]
fn l1_foster07_boxscores_favorites() {
    let mut c = fresh();
    c.set_key("sync.capabilities.boxscores", "favorites")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.boxscores").unwrap(),
        "favorites"
    );
}

#[test]
fn l1_foster07_boxscores_league() {
    let mut c = fresh();
    c.set_key("sync.capabilities.boxscores", "league").unwrap();
    assert_eq!(c.get_key("sync.capabilities.boxscores").unwrap(), "league");
}

#[test]
fn l1_foster07_career_history_off() {
    let mut c = fresh();
    c.set_key("sync.capabilities.career_history", "off")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.career_history").unwrap(),
        "off"
    );
}

#[test]
fn l1_foster07_career_history_favorites() {
    let mut c = fresh();
    c.set_key("sync.capabilities.career_history", "favorites")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.career_history").unwrap(),
        "favorites"
    );
}

#[test]
fn l1_foster07_career_history_league() {
    let mut c = fresh();
    c.set_key("sync.capabilities.career_history", "league")
        .unwrap();
    assert_eq!(
        c.get_key("sync.capabilities.career_history").unwrap(),
        "league"
    );
}

#[test]
fn l1_foster07_shifts_off_succeeds() {
    let mut c = fresh();
    c.set_key("sync.capabilities.shifts", "off").unwrap();
    assert_eq!(c.get_key("sync.capabilities.shifts").unwrap(), "off");
}

/// Shifts → favorites is rejected with the **literal** error message
/// the spec pins (BENCH H3). Multi-line, includes the chosen value
/// and the "Allowed values today: off" trailer.
#[test]
fn l1_foster07_shifts_favorites_rejected_with_literal_error() {
    let mut c = fresh();
    let err = c
        .set_key("sync.capabilities.shifts", "favorites")
        .expect_err("must reject");
    let formatted = err.to_string();
    let expected = "capability `shifts` cannot be set to `favorites` yet —\n       per-shift parsing isn't implemented. Reserved for a future\n       phase. Allowed values today: off";
    assert_eq!(
        formatted, expected,
        "literal error string drift — BENCH H3 pins this exact wording"
    );
    assert!(
        matches!(err, CapabilityError::ShiftsLocked { .. }),
        "got: {err:?}"
    );
}

#[test]
fn l1_foster07_shifts_league_rejected_with_literal_error() {
    let mut c = fresh();
    let err = c
        .set_key("sync.capabilities.shifts", "league")
        .expect_err("must reject");
    let formatted = err.to_string();
    let expected = "capability `shifts` cannot be set to `league` yet —\n       per-shift parsing isn't implemented. Reserved for a future\n       phase. Allowed values today: off";
    assert_eq!(
        formatted, expected,
        "literal error string drift — BENCH H3 pins this exact wording"
    );
}

// ── Interaction tests (6) ───────────────────────────────────────────────────

/// I1 — transactions=favorites + boxscores=off: `allowed` returns the
/// expected truth table for a favorited entity vs a non-fav entity.
#[test]
fn l1_foster07_interaction_transactions_fav_boxscores_off() {
    let mut c = fresh();
    c.set_key("sync.capabilities.transactions", "favorites")
        .unwrap();
    c.set_key("sync.capabilities.boxscores", "off").unwrap();

    let m = c.sync.capabilities;
    assert!(
        m.allowed(Capability::Transactions, true),
        "fav transactions allowed when transactions=favorites"
    );
    assert!(
        !m.allowed(Capability::Transactions, false),
        "non-fav transactions not allowed when transactions=favorites"
    );
    assert!(
        !m.allowed(Capability::Boxscores, true),
        "boxscores blocked entirely when off"
    );
    assert!(!m.allowed(Capability::Boxscores, false));
}

/// I2 — shifts=off blocks every fetch_shifts attempt regardless of
/// favorite status (validates the typed `allowed` helper).
#[test]
fn l1_foster07_interaction_shifts_off_blocks_all() {
    let m = CapabilityMatrix::default();
    assert_eq!(m.shifts, CapabilityMode::Off);
    assert!(!m.allowed(Capability::Shifts, true));
    assert!(!m.allowed(Capability::Shifts, false));
}

/// I3 — career_history=favorites filters non-favorited lazy
/// fan-outs: only favorited PIDs come back true.
#[test]
fn l1_foster07_interaction_career_history_favorites_filters_lazy_fanout() {
    let m = CapabilityMatrix::default(); // career_history defaults to favorites
    assert_eq!(m.career_history, CapabilityMode::Favorites);
    assert!(
        m.allowed(Capability::CareerHistory, true),
        "fav PID allowed"
    );
    assert!(
        !m.allowed(Capability::CareerHistory, false),
        "non-fav PID skipped under favorites mode"
    );
}

/// I4 — `sync.policy=off` short-circuits Foster.4. Round-trips
/// through set_key + get_key + the typed enum.
#[test]
fn l1_foster07_interaction_sync_policy_off_short_circuit() {
    let mut c = fresh();
    c.set_key("sync.policy", "off").unwrap();
    assert_eq!(c.get_key("sync.policy").unwrap(), "off");
    assert_eq!(c.sync.policy, SyncPolicy::Off);
}

/// I5 — banner verbosity: summary vs silent round-trip; the typed
/// enum lets the sync engine branch without re-parsing the string.
#[test]
fn l1_foster07_interaction_banner_summary_vs_silent() {
    let mut c = fresh();
    c.set_key("sync.banner", "silent").unwrap();
    assert_eq!(c.sync.banner, BannerMode::Silent);
    c.set_key("sync.banner", "summary").unwrap();
    assert_eq!(c.sync.banner, BannerMode::Summary);
    c.set_key("sync.banner", "verbose").unwrap();
    assert_eq!(c.sync.banner, BannerMode::Verbose);
}

/// I6 — season_transition=prompt is the default and round-trips.
/// In test mode (`ICELINES_TEST_MODE=1`) the sync engine is
/// expected to short-circuit the prompt — covered by Foster.4 sync-
/// engine tests, not here. This test pins the typed value plumbing.
#[test]
fn l1_foster07_interaction_season_transition_prompt_default_round_trip() {
    let c = fresh();
    assert_eq!(c.sync.season_transition, SeasonTransition::Prompt);
    let mut c2 = fresh();
    c2.set_key("sync.season_transition", "ignore").unwrap();
    assert_eq!(c2.sync.season_transition, SeasonTransition::Ignore);
    assert_eq!(c2.get_key("sync.season_transition").unwrap(), "ignore");
}
