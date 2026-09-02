//! Fantasy scoring scheme engine.
//!
//! A scheme assigns a weight to each statistical category, enabling
//! fantasy-context rankings alongside (not instead of) pure hockey metrics.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Weights ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SkaterWeights {
    pub goals: f32,
    pub assists: f32,
    pub pp_goals: f32,   // power play goals (bonus on top of goals)
    pub pp_assists: f32, // power play assists (bonus on top of assists)
    pub sh_goals: f32,   // shorthanded goals bonus
    pub sh_assists: f32, // shorthanded assists bonus
    pub gwg: f32,        // game-winning goals
    pub ot_goals: f32,   // overtime goals
    pub hits: f32,
    pub blocks: f32,
    pub shots_on_goal: f32,
    pub plus_minus: f32,
    pub takeaways: f32,
    pub giveaways: f32, // typically negative weight
    pub faceoff_wins: f32,
}

impl Default for SkaterWeights {
    fn default() -> Self {
        Self {
            goals: 0.0,
            assists: 0.0,
            pp_goals: 0.0,
            pp_assists: 0.0,
            sh_goals: 0.0,
            sh_assists: 0.0,
            gwg: 0.0,
            ot_goals: 0.0,
            hits: 0.0,
            blocks: 0.0,
            shots_on_goal: 0.0,
            plus_minus: 0.0,
            takeaways: 0.0,
            giveaways: 0.0,
            faceoff_wins: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GoalieWeights {
    pub wins: f32,
    pub losses: f32,
    pub saves: f32,
    pub goals_against: f32, // typically negative
    pub shutouts: f32,
    pub save_pct: f32,
}

impl Default for GoalieWeights {
    fn default() -> Self {
        Self {
            wins: 0.0,
            losses: 0.0,
            saves: 0.0,
            goals_against: 0.0,
            shutouts: 0.0,
            save_pct: 0.0,
        }
    }
}

// ── Scheme ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchemeSource {
    Yahoo,
    Espn,
    Cbs,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scheme {
    pub name: String,
    pub description: String,
    pub source: SchemeSource,
    pub skater: SkaterWeights,
    pub goalie: GoalieWeights,
}

impl Scheme {
    /// Yahoo standard points league.
    pub fn yahoo_standard() -> Self {
        Self {
            name: "yahoo-standard".into(),
            description: "Yahoo Fantasy Hockey standard points league".into(),
            source: SchemeSource::Yahoo,
            skater: SkaterWeights {
                goals: 3.0,
                assists: 2.0,
                pp_goals: 1.0,
                pp_assists: 0.5,
                sh_goals: 1.0,
                sh_assists: 0.5,
                gwg: 0.5,
                hits: 0.5,
                blocks: 0.5,
                ..Default::default()
            },
            goalie: GoalieWeights {
                wins: 5.0,
                losses: -2.0,
                saves: 0.15,
                goals_against: -1.0,
                shutouts: 4.0,
                ..Default::default()
            },
        }
    }

    /// ESPN standard scoring.
    pub fn espn_standard() -> Self {
        Self {
            name: "espn-standard".into(),
            description: "ESPN Fantasy Hockey standard points league".into(),
            source: SchemeSource::Espn,
            skater: SkaterWeights {
                goals: 6.0,
                assists: 4.0,
                pp_goals: 2.0,
                pp_assists: 2.0,
                sh_goals: 3.0,
                sh_assists: 3.0,
                gwg: 1.0,
                hits: 1.0,
                blocks: 1.0,
                shots_on_goal: 1.0,
                plus_minus: 2.0,
                ..Default::default()
            },
            goalie: GoalieWeights {
                wins: 5.0,
                saves: 0.2,
                goals_against: -2.0,
                shutouts: 5.0,
                ..Default::default()
            },
        }
    }

    /// Sample Multicategory 2025-26 Yahoo points-league scoring, recovered from the
    /// exported workbook's exact player-stat and fantasy-point totals.
    pub fn sample_multicategory() -> Self {
        Self {
            name: "sample-multicategory".into(),
            description: "Sample Multicategory Yahoo points scoring (2025-26)".into(),
            source: SchemeSource::Custom,
            skater: SkaterWeights {
                goals: 3.25,
                assists: 2.25,
                pp_goals: 3.0,
                pp_assists: 2.0,
                sh_goals: 1.0,
                sh_assists: 1.0,
                gwg: 1.0,
                hits: 0.5,
                blocks: 0.5,
                ..Default::default()
            },
            goalie: GoalieWeights {
                wins: 3.0,
                losses: -0.5,
                saves: 0.2,
                goals_against: -0.25,
                shutouts: 3.0,
                ..Default::default()
            },
        }
    }

    /// Pure hockey points — goals + assists only, no bonuses.
    pub fn simple_pts() -> Self {
        Self {
            name: "simple-pts".into(),
            description: "Pure hockey points (G+A), no fantasy bonuses".into(),
            source: SchemeSource::Custom,
            skater: SkaterWeights {
                goals: 1.0,
                assists: 1.0,
                ..Default::default()
            },
            goalie: GoalieWeights::default(),
        }
    }

    pub fn all_builtins() -> Vec<Self> {
        vec![
            Self::yahoo_standard(),
            Self::espn_standard(),
            Self::sample_multicategory(),
            Self::simple_pts(),
        ]
    }

    pub fn builtin_named(name: &str) -> Option<Self> {
        let needle = name.trim().to_ascii_lowercase();
        Self::all_builtins()
            .into_iter()
            .find(|scheme| scheme.name.eq_ignore_ascii_case(&needle))
    }

    pub fn skater_category_keys(&self) -> Vec<&'static str> {
        let weights = &self.skater;
        [
            (weights.goals, "goals"),
            (weights.assists, "assists"),
            (weights.pp_goals, "pp_goals"),
            (weights.pp_assists, "pp_assists"),
            (weights.sh_goals, "sh_goals"),
            (weights.sh_assists, "sh_assists"),
            (weights.gwg, "gwg"),
            (weights.ot_goals, "ot_goals"),
            (weights.hits, "hits"),
            (weights.blocks, "blocks"),
            (weights.shots_on_goal, "shots"),
            (weights.plus_minus, "plus_minus"),
            (weights.takeaways, "takeaways"),
            (weights.giveaways, "giveaways"),
            (weights.faceoff_wins, "faceoff_wins"),
        ]
        .into_iter()
        .filter_map(|(weight, key)| (weight.abs() > f32::EPSILON).then_some(key))
        .collect()
    }
}

// ── Fantasy score ─────────────────────────────────────────────────────────────

/// Per-stat contribution to the total fantasy score.
pub type Breakdown = HashMap<&'static str, f32>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScore {
    pub total: f32,
    pub per_game: f32,
    pub gp: u32,
    pub breakdown: HashMap<String, f32>,
}

/// Skater stats input for fantasy scoring.
#[derive(Debug, Clone, Default)]
pub struct SkaterStats {
    pub goals: u32,
    pub assists: u32,
    pub pp_goals: u32,
    pub pp_assists: u32,
    pub sh_goals: u32,
    pub sh_assists: u32,
    pub gwg: u32,
    pub ot_goals: u32,
    pub hits: u32,
    pub blocks: u32,
    pub shots_on_goal: u32,
    pub plus_minus: i32,
    pub takeaways: u32,
    pub giveaways: u32,
    pub faceoff_wins: u32,
}

pub const MIN_GP_SCHEME: u32 = 10;

/// Minimum GP for a goalie to qualify for fantasy scoring. Goalies
/// accumulate counting stats (saves, wins) much faster than skaters,
/// but a one-game appearance shouldn't anchor a lopsided weekly score.
/// 5 GP is the typical NHL fantasy convention for goalie eligibility.
pub const MIN_GP_GOALIE_SCHEME: u32 = 5;

/// Pure-data shape consumed by `compute_goalie_fantasy_score`. Matches
/// the goalie counting stats fantasy schemes care about; the caller
/// builds it from `icelines_core::model::GoalieSeasonStats` (or any
/// equivalent source).
#[derive(Debug, Clone, Default)]
pub struct GoalieScoreStats {
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub saves: u32,
    pub goals_against: u32,
    pub shutouts: u32,
    pub save_pct: f32, // 0.0..=1.0
}

/// Compute fantasy score for a skater. Returns None if gp < MIN_GP_SCHEME.
///
/// DI-22: FantasyScore is always None when gp < MIN_GP_SCHEME.
/// DI-23: breakdown values sum to within 0.001 of total.
pub fn compute_fantasy_score(
    stats: &SkaterStats,
    weights: &SkaterWeights,
    gp: u32,
) -> Option<FantasyScore> {
    if gp < MIN_GP_SCHEME {
        return None;
    }

    let entries: &[(f32, &str)] = &[
        (weights.goals * stats.goals as f32, "goals"),
        (weights.assists * stats.assists as f32, "assists"),
        (weights.pp_goals * stats.pp_goals as f32, "pp_goals"),
        (weights.pp_assists * stats.pp_assists as f32, "pp_assists"),
        (weights.sh_goals * stats.sh_goals as f32, "sh_goals"),
        (weights.sh_assists * stats.sh_assists as f32, "sh_assists"),
        (weights.gwg * stats.gwg as f32, "gwg"),
        (weights.ot_goals * stats.ot_goals as f32, "ot_goals"),
        (weights.hits * stats.hits as f32, "hits"),
        (weights.blocks * stats.blocks as f32, "blocks"),
        (
            weights.shots_on_goal * stats.shots_on_goal as f32,
            "shots_on_goal",
        ),
        (weights.plus_minus * stats.plus_minus as f32, "plus_minus"),
        (weights.takeaways * stats.takeaways as f32, "takeaways"),
        (weights.giveaways * stats.giveaways as f32, "giveaways"),
        (
            weights.faceoff_wins * stats.faceoff_wins as f32,
            "faceoff_wins",
        ),
    ];

    let total: f32 = entries.iter().map(|(v, _)| v).sum();
    let breakdown: HashMap<String, f32> = entries
        .iter()
        .filter(|(v, _)| v.abs() > 0.001)
        .map(|(v, k)| (k.to_string(), *v))
        .collect();

    Some(FantasyScore {
        total,
        per_game: total / gp as f32,
        gp,
        breakdown,
    })
}

/// Compute fantasy score for a goalie. Returns None when the goalie
/// hasn't played enough games (`gp < MIN_GP_GOALIE_SCHEME`) so weekly
/// scoring isn't anchored by a single appearance. Phase G.6.
///
/// Matches the standard Yahoo-style scoring set used for goalies:
/// W (positive), L (negative), Saves (small per-save reward), GA
/// (negative), SO (large bonus), and SV% (rate-stat bonus).
pub fn compute_goalie_fantasy_score(
    stats: &GoalieScoreStats,
    weights: &GoalieWeights,
    gp: u32,
) -> Option<FantasyScore> {
    if gp < MIN_GP_GOALIE_SCHEME {
        return None;
    }

    let entries: &[(f32, &str)] = &[
        (weights.wins * stats.wins as f32, "wins"),
        (weights.losses * stats.losses as f32, "losses"),
        (weights.saves * stats.saves as f32, "saves"),
        (
            weights.goals_against * stats.goals_against as f32,
            "goals_against",
        ),
        (weights.shutouts * stats.shutouts as f32, "shutouts"),
        // SV% is a rate stat — multiply by GP so the contribution scales
        // with playing time. A backup with one .950 game shouldn't out-
        // earn a starter at .920 for the season.
        (weights.save_pct * stats.save_pct * gp as f32, "save_pct"),
    ];

    let total: f32 = entries.iter().map(|(v, _)| v).sum();
    let breakdown: HashMap<String, f32> = entries
        .iter()
        .filter(|(v, _)| v.abs() > 0.001)
        .map(|(v, k)| (k.to_string(), *v))
        .collect();

    Some(FantasyScore {
        total,
        per_game: total / gp as f32,
        gp,
        breakdown,
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn beniers_stats() -> SkaterStats {
        // Matty Beniers 2025-26: 20G, 30A, 82GP
        // PPG=6, PPA=5, SHG=0, SHA=0, GWG=1, HIT=31, BLK=69
        SkaterStats {
            goals: 20,
            assists: 30,
            pp_goals: 6,
            pp_assists: 5,
            sh_goals: 0,
            sh_assists: 0,
            gwg: 1,
            hits: 31,
            blocks: 69,
            ..Default::default()
        }
    }

    #[test]
    fn l0_scheme_yahoo_standard_beniers() {
        // G=3, A=2, PPG=1, PPA=0.5, GWG=0.5, HIT=0.5, BLK=0.5
        // 20×3 + 30×2 + 6×1 + 5×0.5 + 1×0.5 + 31×0.5 + 69×0.5
        //  =60  +  60  +  6  + 2.5   + 0.5   + 15.5  + 34.5  = 179.0
        let s =
            compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
        assert!(
            (s.total - 179.0).abs() < 0.001,
            "expected 179.0, got {}",
            s.total
        );
    }

    #[test]
    fn l0_sample_multicategory_reproduces_exported_mackinnon_total() {
        let stats = SkaterStats {
            goals: 35,
            assists: 39,
            pp_goals: 7,
            pp_assists: 11,
            gwg: 5,
            hits: 34,
            blocks: 23,
            ..Default::default()
        };
        let score =
            compute_fantasy_score(&stats, &Scheme::sample_multicategory().skater, 80).unwrap();
        assert!((score.total - 278.0).abs() < 0.001);
    }

    #[test]
    fn l0_scheme_breakdown_sums_to_total() {
        // DI-23: breakdown values must sum to total within 0.001
        let s =
            compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
        let sum: f32 = s.breakdown.values().sum();
        assert!(
            (sum - s.total).abs() < 0.001,
            "breakdown sum {sum} != total {}",
            s.total
        );
    }

    #[test]
    fn l0_scheme_none_below_min_gp() {
        // DI-22: None when gp < MIN_GP_SCHEME
        assert!(
            compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 9).is_none()
        );
        assert!(
            compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 0).is_none()
        );
    }

    #[test]
    fn l0_scheme_some_at_min_gp() {
        assert!(compute_fantasy_score(
            &beniers_stats(),
            &Scheme::yahoo_standard().skater,
            MIN_GP_SCHEME
        )
        .is_some());
    }

    #[test]
    fn l0_scheme_simple_pts_is_just_goals_assists() {
        // simple-pts: G=1, A=1 → 20+30 = 50
        let s = compute_fantasy_score(&beniers_stats(), &Scheme::simple_pts().skater, 82).unwrap();
        assert!(
            (s.total - 50.0).abs() < 0.001,
            "expected 50.0, got {}",
            s.total
        );
    }

    #[test]
    fn l0_scheme_builtins_count() {
        assert_eq!(Scheme::all_builtins().len(), 4);
    }

    #[test]
    fn l0_scheme_per_game_is_total_div_gp() {
        let s =
            compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
        // 179.0 / 82 = 2.182...
        assert!((s.per_game - s.total / 82.0).abs() < 0.001);
    }

    // ── Goalie scoring (Phase G.6) ─────────────────────────────────────────

    fn hellebuyck_24_25() -> GoalieScoreStats {
        // Connor Hellebuyck 24-25: 47W, 12L, 8 SO, 1539 saves, 125 GA, .9249
        GoalieScoreStats {
            games_played: 63,
            wins: 47,
            losses: 12,
            saves: 1539,
            goals_against: 125,
            shutouts: 8,
            save_pct: 0.92487,
        }
    }

    #[test]
    fn l0_goalie_yahoo_standard_arithmetic() {
        // Yahoo standard goalie weights: W=5, L=-2, Saves=0.15, GA=-1, SO=4
        // Hellebuyck: 47*5 + 12*-2 + 1539*0.15 + 125*-1 + 8*4
        //           = 235 - 24 + 230.85 - 125 + 32 = 348.85 (no SV% bonus
        //           in yahoo-standard — save_pct weight is 0.0)
        let s =
            compute_goalie_fantasy_score(&hellebuyck_24_25(), &Scheme::yahoo_standard().goalie, 63)
                .unwrap();
        let expected = 235.0 - 24.0 + 230.85 - 125.0 + 32.0;
        assert!(
            (s.total - expected).abs() < 0.01,
            "expected ≈{expected}, got {}",
            s.total
        );
    }

    #[test]
    fn l0_goalie_below_min_gp_returns_none() {
        // GP below MIN_GP_GOALIE_SCHEME (5) → no score.
        assert!(compute_goalie_fantasy_score(
            &hellebuyck_24_25(),
            &Scheme::yahoo_standard().goalie,
            4,
        )
        .is_none());
        // Exactly at the threshold returns Some.
        assert!(compute_goalie_fantasy_score(
            &hellebuyck_24_25(),
            &Scheme::yahoo_standard().goalie,
            MIN_GP_GOALIE_SCHEME,
        )
        .is_some());
    }

    #[test]
    fn l0_goalie_breakdown_omits_zero_categories() {
        // Yahoo standard has save_pct weight 0.0 → no save_pct entry in breakdown.
        let s =
            compute_goalie_fantasy_score(&hellebuyck_24_25(), &Scheme::yahoo_standard().goalie, 63)
                .unwrap();
        assert!(
            !s.breakdown.contains_key("save_pct"),
            "save_pct=0 weight should not appear in breakdown"
        );
        // Real categories DO appear.
        for k in &["wins", "losses", "saves", "goals_against", "shutouts"] {
            assert!(s.breakdown.contains_key(*k), "missing breakdown key: {k}");
        }
    }

    #[test]
    fn l0_goalie_save_pct_weight_scales_by_gp() {
        // A scheme that pays 100 per save_pct point per game.
        let weights = GoalieWeights {
            save_pct: 100.0,
            ..Default::default()
        };
        // .92 SV% × 50 GP × 100 = 4600
        let stats = GoalieScoreStats {
            games_played: 50,
            wins: 25,
            losses: 20,
            saves: 1400,
            goals_against: 100,
            shutouts: 3,
            save_pct: 0.92,
        };
        let s = compute_goalie_fantasy_score(&stats, &weights, 50).unwrap();
        let expected = 0.92 * 50.0 * 100.0;
        assert!(
            (s.total - expected).abs() < 0.01,
            "expected {}, got {}",
            expected,
            s.total
        );
    }
}
