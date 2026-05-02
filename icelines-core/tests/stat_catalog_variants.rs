//! Phase Lindsay L.2.3 — cross-product `read()` test (BENCH-R2 L2-B22).
//!
//! Exercises every `StatId × variant` cell — 107 stats × 6 variants =
//! 642 reads — to catch:
//!   - panics from a missing match arm (since `read()` is total)
//!   - silent zeros where the catalog should have returned `None`
//!     (era gate / DI-11 trade guard / MIN_GP guard / position gate)
//!   - missing position-applicability gating (FaceoffWinPct on D, etc.)
//!
//! The assertions are deliberately coarse — this catches structural
//! errors (panic, type mismatch, swapped Some/None at category-gate
//! boundaries). Per-stat numeric correctness lives in the `stats_catalog`
//! module's L0 tests.

use icelines_core::fixtures::stat_catalog_variants;
use icelines_core::stats_catalog::{StatCategory, StatId};
use icelines_core::stats_repository::PlayerView;

/// Cross-product smoke: every (variant, StatId) pair calls `read()`
/// without panicking. 642 reads total. The test counts how many
/// returned `Some` per variant for visibility.
#[test]
fn l1_lindsay_stat_catalog_variants_no_panics_across_cross_product() {
    let mut totals = Vec::new();
    for (name, builder) in stat_catalog_variants::all() {
        let (identity, stats) = builder();
        let view = PlayerView { identity: &identity, stats: &stats, contract: None };
        let mut some_count = 0usize;
        for sid in StatId::all() {
            // The read() call: if it panics, the test fails. We don't
            // assert on the value — just that it returns without
            // panicking (read() is total per DI-07).
            if sid.read(&view).is_some() {
                some_count += 1;
            }
        }
        totals.push((*name, some_count));
    }
    // Sanity: each variant has at least SOME data populated. Even
    // skater_pre_2005 has Scoring + Pim. low_gp has scoring.
    for (name, count) in &totals {
        assert!(
            *count > 5,
            "variant {name} has only {count} non-None stats — \
             that's likely a bug; expected ≥6 (Goals + Assists + Points + \
             Pim + Shots + ShootingPct at minimum)"
        );
    }
}

/// Era gate: `skater_pre_2005` returns `None` for all realtime stats
/// (Hits/Blocks/Takeaways/Giveaways/MissedShots and per-60 derivatives).
/// Realtime data wasn't loaded for that fixture; era gate codifies the
/// expected absence.
#[test]
fn l1_lindsay_pre_2005_realtime_stats_are_none() {
    let (identity, stats) = stat_catalog_variants::skater_pre_2005();
    let view = PlayerView { identity: &identity, stats: &stats, contract: None };
    for sid in [
        StatId::Hits, StatId::BlockedShots, StatId::Takeaways,
        StatId::Giveaways, StatId::MissedShots,
        StatId::HitsPer60, StatId::BlockedShotsPer60,
        StatId::TakeawaysPer60, StatId::GiveawaysPer60,
    ] {
        assert!(
            sid.read(&view).is_none(),
            "{sid:?} should be None on skater_pre_2005 (no realtime data)"
        );
    }
    // Sanity: Scoring stats DO read.
    assert!(StatId::Goals.read(&view).is_some());
    assert!(StatId::Points.read(&view).is_some());
}

/// DI-11 — `traded_multistint` short-circuits OnIceGoals reads to None.
#[test]
fn l1_lindsay_traded_multistint_di11_blocks_on_ice_goals() {
    let (identity, stats) = stat_catalog_variants::traded_multistint();
    let view = PlayerView { identity: &identity, stats: &stats, contract: None };
    assert!(view.was_traded_in_window());
    for sid in StatId::all().iter().filter(|s| s.category() == StatCategory::OnIceGoals) {
        assert!(
            sid.read(&view).is_none(),
            "{sid:?} (OnIceGoals) should be None when traded mid-window"
        );
    }
    // EvenStrengthTimeOnIcePerGame is in TimeOnIce, NOT OnIceGoals — exempt.
    assert!(
        StatId::EvenStrengthTimeOnIcePerGame.read(&view).is_some(),
        "TimeOnIce category exempt from DI-11"
    );
}

/// MIN_GP gate — `low_gp` (GP=5 < MIN_GP=10) returns None for derived
/// per-game / per-82 stats.
#[test]
fn l1_lindsay_low_gp_min_gp_gate_blocks_derived() {
    let (identity, stats) = stat_catalog_variants::low_gp();
    let view = PlayerView { identity: &identity, stats: &stats, contract: None };
    for sid in [
        StatId::PointsPerGame, StatId::GoalsPerGame, StatId::AssistsPerGame,
        StatId::Pace82, StatId::GoalsPer82, StatId::AssistsPer82,
    ] {
        assert!(
            sid.read(&view).is_none(),
            "{sid:?} should be None below MIN_GP (low_gp variant)"
        );
    }
    // Sanity: raw counts still readable.
    assert_eq!(StatId::Goals.read(&view), Some(1.0));
    assert_eq!(StatId::Assists.read(&view), Some(1.0));
}

/// Goalie variant — Goalie category populates; skater stats hidden
/// via `applies_to(_, is_goalie=true)` (the L.2.4 filter layer
/// enforces this; for raw `read()` they may return Some from Scoring,
/// but all-goalies queries route through `applies_to` first).
#[test]
fn l1_lindsay_goalie_category_populates_for_goalie_variant() {
    let (identity, stats) = stat_catalog_variants::goalie();
    let view = PlayerView { identity: &identity, stats: &stats, contract: None };
    assert!(view.is_goalie());
    for sid in [
        StatId::Wins, StatId::Losses, StatId::Saves, StatId::ShotsAgainst,
        StatId::SavePct, StatId::Gaa, StatId::Shutouts,
        StatId::EvSavePct, StatId::ShSavePct,
        StatId::QualityStarts, StatId::QualityStartPct,
        StatId::RegulationWins, StatId::RegulationLosses,
    ] {
        assert!(
            sid.read(&view).is_some(),
            "{sid:?} should be populated for goalie variant"
        );
    }
}

/// Position-applicability — FaceoffWinPct gates to Center.
#[test]
fn l1_lindsay_faceoff_applies_to_center_only() {
    use icelines_core::model::Position;
    assert!(StatId::FaceoffWinPct.applies_to(Position::Center, false));
    assert!(!StatId::FaceoffWinPct.applies_to(Position::LeftWing, false));
    assert!(!StatId::FaceoffWinPct.applies_to(Position::RightWing, false));
    assert!(!StatId::FaceoffWinPct.applies_to(Position::Defense, false));
    // Goalie hidden — stats hidden on goalies regardless of category.
    assert!(!StatId::FaceoffWinPct.applies_to(Position::Center, true));
}

/// Cross-product cardinality: every (variant, StatId) pair has a
/// well-defined return. 107 × 6 = 642 cells. The point of this test
/// is the iteration count — read() must be total over the entire
/// catalog × variant cross-product.
#[test]
fn l1_lindsay_cross_product_cardinality_is_642() {
    let mut total_cells = 0usize;
    for (_, builder) in stat_catalog_variants::all() {
        let (identity, stats) = builder();
        let view = PlayerView { identity: &identity, stats: &stats, contract: None };
        for sid in StatId::all() {
            // Force the read — discard the value.
            let _ = sid.read(&view);
            total_cells += 1;
        }
    }
    assert_eq!(total_cells, 107 * 6, "expected 642 cross-product cells");
}
