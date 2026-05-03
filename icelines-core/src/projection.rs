//! Rest-of-season projection engine.
//!
//! Three modes of increasing sophistication:
//!   Pace       — raw PPG × remaining games
//!   Regressed  — α × current + (1-α) × career, α = min(GP/50, 1.0)
//!   Composite  — regressed × age_factor × schedule_factor (future)

use serde::{Deserialize, Serialize};

// ── Projection mode ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectionMode {
    Pace,
    Regressed,
    Composite,
}

impl std::str::FromStr for ProjectionMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "pace" => Ok(Self::Pace),
            "regressed" => Ok(Self::Regressed),
            "composite" => Ok(Self::Composite),
            other => Err(format!(
                "unknown mode '{other}' — use pace|regressed|composite"
            )),
        }
    }
}

// ── Projection result ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectionResult {
    pub mode: ProjectionMode,
    pub current_ppg: f64,        // points per game this season
    pub career_ppg: Option<f64>, // career average (None if not available)
    pub alpha: f64,              // regressed blend weight (0–1)
    pub age_factor: f64,         // age curve multiplier (1.0 at peak)
    pub remaining_games: u32,
    pub projected_points: f64, // main output
    pub low_band: f64,         // projected_points - 1σ
    pub high_band: f64,        // projected_points + 1σ
}

impl ProjectionResult {
    pub fn confidence_band_width(&self) -> f64 {
        self.high_band - self.low_band
    }
}

// ── Alpha — regression weight ─────────────────────────────────────────────────

/// Regression weight: α = min(GP / 50, 1.0)
/// At 50+ GP the current season dominates; at 10 GP it's heavily regressed.
pub fn compute_alpha(gp: u32) -> f64 {
    (gp as f64 / 50.0).min(1.0)
}

// ── Age factor ────────────────────────────────────────────────────────────────

/// Age curve: peaks at age 26–27 (factor = 1.0).
/// Pre-peak: small discount for development uncertainty.
/// Post-peak: ~2% decline per year after 27.
///
/// Age factor table:
///   ≤20: 0.88    21: 0.92    22: 0.95    23: 0.97
///   24: 0.99    25: 1.00    26: 1.00    27: 1.00
///   28: 0.98    29: 0.96    30: 0.94    31: 0.92
///   32: 0.90    33: 0.87    34: 0.84    35: 0.80
///   ≥36: 0.76
pub fn age_factor(age: u8) -> f64 {
    match age {
        0..=20 => 0.88,
        21 => 0.92,
        22 => 0.95,
        23 => 0.97,
        24 => 0.99,
        25..=27 => 1.00,
        28 => 0.98,
        29 => 0.96,
        30 => 0.94,
        31 => 0.92,
        32 => 0.90,
        33 => 0.87,
        34 => 0.84,
        35 => 0.80,
        _ => 0.76,
    }
}

// ── Per-game standard deviation (for confidence band) ─────────────────────────

/// Estimate σ of per-game point production from sample size.
/// Smaller GP → wider band (more uncertainty).
/// Uses empirical approximation: σ ≈ 0.65 / sqrt(GP)
pub fn per_game_sigma(ppg: f64, gp: u32) -> f64 {
    if gp == 0 {
        return ppg;
    }
    // Typical NHL skater point variance ≈ 0.65 per game
    0.65 / (gp as f64).sqrt()
}

// ── Main projection function ──────────────────────────────────────────────────

/// Compute a rest-of-season projection.
///
/// Parameters:
///   current_ppg     — points per game this season (goals+assists / GP)
///   career_ppg      — career average points per game (None = use current_ppg)
///   gp              — games played this season
///   age             — player age as of season start
///   remaining_games — remaining regular season games for this player
///   mode            — which projection formula to use
pub fn compute_projection(
    current_ppg: f64,
    career_ppg: Option<f64>,
    gp: u32,
    age: u8,
    remaining_games: u32,
    mode: ProjectionMode,
) -> ProjectionResult {
    let alpha = compute_alpha(gp);
    let age_fac = age_factor(age);
    let career = career_ppg.unwrap_or(current_ppg);
    let sigma = per_game_sigma(current_ppg, gp);

    let effective_ppg = match mode {
        ProjectionMode::Pace => {
            // Raw current pace — no adjustment
            current_ppg
        }
        ProjectionMode::Regressed => {
            // Weighted blend toward career mean as sample grows
            // α × current + (1−α) × career
            alpha * current_ppg + (1.0 - alpha) * career
        }
        ProjectionMode::Composite => {
            // Regressed × age curve × schedule (schedule factor placeholder = 1.0)
            let regressed = alpha * current_ppg + (1.0 - alpha) * career;
            regressed * age_fac
        }
    };

    let projected = effective_ppg * remaining_games as f64;
    let band_width = sigma * remaining_games as f64;

    ProjectionResult {
        mode,
        current_ppg,
        career_ppg,
        alpha,
        age_factor: age_fac,
        remaining_games,
        projected_points: projected,
        low_band: (projected - band_width).max(0.0),
        high_band: projected + band_width,
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── compute_alpha ─────────────────────────────────────────────────────────

    #[test]
    fn l0_alpha_at_10_gp() {
        // α = 10/50 = 0.20
        let a = compute_alpha(10);
        assert!((a - 0.20).abs() < 0.001, "expected 0.20, got {a}");
    }

    #[test]
    fn l0_alpha_at_50_gp() {
        // α = 50/50 = 1.00 — fully current-season weighted
        let a = compute_alpha(50);
        assert!((a - 1.00).abs() < 0.001, "expected 1.00, got {a}");
    }

    #[test]
    fn l0_alpha_clamped_above_50() {
        // α = min(82/50, 1.0) = 1.00
        let a = compute_alpha(82);
        assert!((a - 1.00).abs() < 0.001, "alpha must be clamped at 1.0");
    }

    // ── age_factor ────────────────────────────────────────────────────────────

    #[test]
    fn l0_age_factor_peak_is_one() {
        // Age 25, 26, 27 → factor = 1.0
        for age in [25u8, 26, 27] {
            assert!(
                (age_factor(age) - 1.0).abs() < 0.001,
                "peak age {age} must have factor 1.0"
            );
        }
    }

    #[test]
    fn l0_age_factor_30_decline() {
        // Age 30 → 0.94 (two years past peak → ~6% decline)
        assert!(
            (age_factor(30) - 0.94).abs() < 0.001,
            "age 30 must have factor 0.94"
        );
    }

    #[test]
    fn l0_age_factor_35_significant_decline() {
        assert!(
            (age_factor(35) - 0.80).abs() < 0.001,
            "age 35 must have factor 0.80"
        );
    }

    #[test]
    fn l0_age_factor_always_positive() {
        for age in 0u8..=45 {
            assert!(age_factor(age) > 0.0, "age factor must always be positive");
        }
    }

    // ── compute_projection ────────────────────────────────────────────────────

    #[test]
    fn l0_pace_mode_is_ppg_times_remaining() {
        // pace: 1.68 ppg × 30 remaining = 50.4 pts
        let r = compute_projection(1.68, None, 82, 27, 30, ProjectionMode::Pace);
        assert!(
            (r.projected_points - 50.4).abs() < 0.001,
            "expected 50.4, got {}",
            r.projected_points
        );
    }

    #[test]
    fn l0_regressed_at_50gp_equals_pace() {
        // At GP=50, α=1.0, regressed = 1.0×current + 0.0×career = pace
        let pace = compute_projection(1.0, Some(0.8), 50, 27, 20, ProjectionMode::Pace);
        let regr = compute_projection(1.0, Some(0.8), 50, 27, 20, ProjectionMode::Regressed);
        assert!(
            (pace.projected_points - regr.projected_points).abs() < 0.001,
            "at GP=50 regressed must equal pace"
        );
    }

    #[test]
    fn l0_regressed_between_pace_and_career_low_gp() {
        // At GP=10, α=0.2. current=1.5, career=0.7
        // regressed = 0.2×1.5 + 0.8×0.7 = 0.3 + 0.56 = 0.86 ppg
        // projected = 0.86 × 20 = 17.2
        let r = compute_projection(1.5, Some(0.7), 10, 27, 20, ProjectionMode::Regressed);
        assert!(
            (r.projected_points - 17.2).abs() < 0.01,
            "expected ~17.2, got {}",
            r.projected_points
        );
        // Must be between pace (30.0) and career (14.0)
        let pace_pts = 1.5 * 20.0;
        let career_pts = 0.7 * 20.0;
        assert!(
            r.projected_points > career_pts && r.projected_points < pace_pts,
            "regressed must be between pace ({pace_pts}) and career ({career_pts})"
        );
    }

    #[test]
    fn l0_composite_applies_age_discount_post_peak() {
        // Age 35: factor = 0.80. Regressed at 50GP (α=1.0) = current.
        // composite = current × 0.80
        let current = 1.0;
        let remaining = 20;
        let regr = compute_projection(current, None, 50, 27, remaining, ProjectionMode::Regressed);
        let comp = compute_projection(current, None, 50, 35, remaining, ProjectionMode::Composite);
        let expected_ratio = 0.80;
        let actual_ratio = comp.projected_points / regr.projected_points;
        assert!(
            (actual_ratio - expected_ratio).abs() < 0.001,
            "composite at age 35 must be 0.80× of regressed, got ratio {actual_ratio}"
        );
    }

    #[test]
    fn l0_confidence_band_wider_at_low_gp() {
        let high_gp = compute_projection(1.0, None, 82, 27, 20, ProjectionMode::Pace);
        let low_gp = compute_projection(1.0, None, 10, 27, 20, ProjectionMode::Pace);
        assert!(
            low_gp.confidence_band_width() > high_gp.confidence_band_width(),
            "smaller sample must produce wider confidence band"
        );
    }

    // ── Proptest ──────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod proptests {
        use super::*;
        use proptest::prelude::*;

        proptest! {
            #[test]
            fn alpha_always_in_0_to_1(gp in 0u32..300) {
                let a = compute_alpha(gp);
                prop_assert!((0.0..=1.0).contains(&a), "alpha={a} out of [0,1]");
            }

            #[test]
            fn age_factor_always_positive(age in 0u8..=45) {
                prop_assert!(age_factor(age) > 0.0);
            }

            #[test]
            fn regressed_between_pace_and_career(
                cur  in 0.1f64..3.0,
                car  in 0.1f64..3.0,
                gp   in 10u32..82,
                rem  in 1u32..41,
            ) {
                let r = compute_projection(cur, Some(car), gp, 27, rem, ProjectionMode::Regressed);
                let pace_pts   = cur * rem as f64;
                let career_pts = car * rem as f64;
                let lo = pace_pts.min(career_pts);
                let hi = pace_pts.max(career_pts);
                prop_assert!(
                    r.projected_points >= lo - 0.001 && r.projected_points <= hi + 0.001,
                    "regressed ({}) not between pace ({pace_pts}) and career ({career_pts})",
                    r.projected_points
                );
            }
        }
    }
}
