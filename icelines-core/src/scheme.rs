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
            Self::simple_pts(),
        ]
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
        assert_eq!(Scheme::all_builtins().len(), 3);
    }

    #[test]
    fn l0_scheme_per_game_is_total_div_gp() {
        let s =
            compute_fantasy_score(&beniers_stats(), &Scheme::yahoo_standard().skater, 82).unwrap();
        // 179.0 / 82 = 2.182...
        assert!((s.per_game - s.total / 82.0).abs() < 0.001);
    }
}
