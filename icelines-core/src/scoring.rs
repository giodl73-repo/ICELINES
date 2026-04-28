use crate::model::{FitClass, PaceScore, Player, Position, MIN_GP};

/// Compute the pace-projected score for a skater.
/// Returns None if GP is zero or below MIN_GP.
///
/// Formula: pace_82 = (goals + assists) / gp * 82
///          sort_key = pace_82 + goals_per_82 * 0.001 (goals as tiebreaker)
pub fn compute_pace_score(goals: u32, assists: u32, gp: u32) -> Option<PaceScore> {
    if gp < MIN_GP {
        return None;
    }
    let gp_f = gp as f64;
    // pace_82 = (goals + assists) / gp * 82
    let pace_82 = (goals + assists) as f64 / gp_f * 82.0;
    // goals_per_82 = goals / gp * 82
    let goals_per_82 = goals as f64 / gp_f * 82.0;

    Some(PaceScore {
        pace_82,
        goals_per_82,
        raw_points: goals + assists,
        gp,
    })
}

/// Classify a player's fit relative to their line slot.
///
/// Thresholds are pace_82 values (points per 82 games):
///   Forwards: Elite ≥ 65 | Solid ≥ 40 | Buried ≥ 20 | Stretch < 20
///   Defense:  Elite ≥ 45 | Solid ≥ 28 | Buried ≥ 14 | Stretch < 14
///
/// Rationale: thresholds approximate ~80th/50th/20th league percentiles
/// for each position group. Calibrated against 2024-25 season distributions.
pub fn classify_fit(pace_82: f64, position: Position) -> FitClass {
    let (elite, solid, buried) = if position.is_forward() {
        (65.0_f64, 40.0_f64, 20.0_f64)
    } else {
        // Defense — lower scoring volume is expected
        (45.0_f64, 28.0_f64, 14.0_f64)
    };

    if pace_82 >= elite {
        FitClass::Elite
    } else if pace_82 >= solid {
        FitClass::Solid
    } else if pace_82 >= buried {
        FitClass::Buried
    } else {
        FitClass::Stretch
    }
}

/// Sort a slice of Players by pace score descending.
/// Players with no pace score (below MIN_GP) sort to the end.
/// Within the same pace score, goals per 82 breaks the tie.
/// Final tiebreaker: nhl_id ascending (deterministic, avoids alphabetical bias).
pub fn sort_by_pace(players: &mut [Player]) {
    players.sort_by(|a, b| {
        let sa = a.pace_score.map(|s| s.sort_key()).unwrap_or(-1.0);
        let sb = b.pace_score.map(|s| s.sort_key()).unwrap_or(-1.0);
        sb.partial_cmp(&sa)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.nhl_id.cmp(&b.nhl_id))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_pace_score ────────────────────────────────────────────────────

    #[test]
    fn l0_scoring_pace_mcdavid_82gp() {
        // (50+90) / 82 * 82 = 140.000 exactly (full season, no projection)
        let s = compute_pace_score(50, 90, 82).unwrap();
        assert!(
            (s.pace_82 - 140.0).abs() < 0.001,
            "expected 140.000 got {}",
            s.pace_82
        );
    }

    #[test]
    fn l0_scoring_pace_mid_tier() {
        // (28+40) / 74 * 82 = 75.351...
        let s = compute_pace_score(28, 40, 74).unwrap();
        assert!(
            (s.pace_82 - 75.351).abs() < 0.001,
            "expected ~75.351 got {}",
            s.pace_82
        );
    }

    #[test]
    fn l0_scoring_pace_zero_gp_is_none() {
        // GP=0 → always None. Must not divide by zero.
        assert!(compute_pace_score(0, 0, 0).is_none());
    }

    #[test]
    fn l0_scoring_pace_below_min_gp_is_none() {
        // GP=9 is below MIN_GP=10 → None
        assert!(
            compute_pace_score(3, 6, 9).is_none(),
            "GP=9 should be below MIN_GP={MIN_GP}"
        );
    }

    #[test]
    fn l0_scoring_pace_at_exactly_min_gp_is_some() {
        // GP=10 is exactly MIN_GP → must return Some
        // (3+5) / 10 * 82 = 65.600
        let s = compute_pace_score(3, 5, 10).unwrap();
        assert!(
            (s.pace_82 - 65.600).abs() < 0.001,
            "expected 65.600 got {}",
            s.pace_82
        );
    }

    #[test]
    fn l0_scoring_goals_tiebreaker_in_sort_key() {
        let s1 = compute_pace_score(20, 60, 82).unwrap(); // same points, fewer goals
        let s2 = compute_pace_score(40, 40, 82).unwrap(); // same points, more goals
                                                          // s2 should have higher sort key (goals break the tie)
        assert!(
            s2.sort_key() > s1.sort_key(),
            "goals/82 tiebreaker: s2({}) should beat s1({})",
            s2.sort_key(),
            s1.sort_key()
        );
    }

    // ── classify_fit ─────────────────────────────────────────────────────────

    #[test]
    fn l0_classify_forward_below_elite_is_solid() {
        // 64.9 < 65.0 → Solid (just below Elite threshold)
        assert_eq!(classify_fit(64.9, Position::Center), FitClass::Solid);
    }

    #[test]
    fn l0_classify_forward_at_elite_threshold_is_elite() {
        // 65.0 >= 65.0 → Elite (inclusive boundary)
        assert_eq!(classify_fit(65.0, Position::Center), FitClass::Elite);
    }

    #[test]
    fn l0_classify_forward_above_elite_is_elite() {
        assert_eq!(classify_fit(140.0, Position::Center), FitClass::Elite);
    }

    #[test]
    fn l0_classify_forward_at_solid_threshold_is_solid() {
        // 40.0 >= 40.0 → Solid
        assert_eq!(classify_fit(40.0, Position::LeftWing), FitClass::Solid);
    }

    #[test]
    fn l0_classify_forward_at_buried_threshold_is_buried() {
        // 20.0 >= 20.0 → Buried
        assert_eq!(classify_fit(20.0, Position::RightWing), FitClass::Buried);
    }

    #[test]
    fn l0_classify_forward_below_buried_is_stretch() {
        // 19.9 < 20.0 → Stretch
        assert_eq!(classify_fit(19.9, Position::Center), FitClass::Stretch);
    }

    #[test]
    fn l0_classify_defense_uses_different_thresholds() {
        // 65.0 is Elite for forwards — must be Elite for defense too (≥45)
        assert_eq!(classify_fit(65.0, Position::Defense), FitClass::Elite);
        // But 40.0 is the Solid/Elite boundary for forwards, not defense
        // For defense: 40.0 >= 28.0 (solid) but < 45.0 (elite) → Solid
        assert_eq!(classify_fit(40.0, Position::Defense), FitClass::Solid);
        // 27.9 < 28.0 → Buried for defense
        assert_eq!(classify_fit(27.9, Position::Defense), FitClass::Buried);
        // 13.9 < 14.0 → Stretch for defense
        assert_eq!(classify_fit(13.9, Position::Defense), FitClass::Stretch);
    }

    #[test]
    fn l0_classify_defense_at_elite_threshold() {
        // 45.0 >= 45.0 → Elite for defense
        assert_eq!(classify_fit(45.0, Position::Defense), FitClass::Elite);
    }

    // ── Proptest: any pace above Elite threshold classifies as Elite ──────────

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn any_pace_above_fwd_elite_is_elite(pace in 65.0f64..500.0) {
                prop_assert_eq!(classify_fit(pace, Position::Center), FitClass::Elite);
            }

            #[test]
            fn any_pace_above_def_elite_is_elite(pace in 45.0f64..500.0) {
                prop_assert_eq!(classify_fit(pace, Position::Defense), FitClass::Elite);
            }

            #[test]
            fn any_pace_below_fwd_stretch_is_stretch(pace in -100.0f64..20.0) {
                prop_assert_eq!(classify_fit(pace, Position::Center), FitClass::Stretch);
            }
        }
    }
}
