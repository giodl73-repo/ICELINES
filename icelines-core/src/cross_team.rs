//! Cross-team ranking metrics.
//!
//! For each player, computes their average line number across the other 31
//! NHL teams — answering: "what line would this player play if they were
//! on any other team?"
//!
//! This is the metric used by the web site for color-coding lineup cards.
//! It differs from the terminal `classify_fit()` which uses absolute pace
//! thresholds.

use crate::model::{Player, Position, TeamAbbr};
use std::collections::HashMap;

/// Per-player cross-team metrics.
#[derive(Debug, Clone)]
pub struct CrossTeamMetrics {
    pub player_nhl_id: Option<u32>,
    pub own_line: u8,        // rank on own team at their position (1-indexed)
    pub avg_other_line: f32, // average rank across the other 31 teams
    pub delta: f32,          // own_line - avg_other_line (positive = buried)
}

impl CrossTeamMetrics {
    /// Fit class based on relative cross-team ranking (web site model).
    /// Uses own_line vs avg_other_line, not absolute pace thresholds.
    pub fn web_fit_class(&self) -> WebFitClass {
        let own = self.own_line as f32;
        let avg = self.avg_other_line;
        if own - avg > 0.75 {
            WebFitClass::Buried // blue: could play higher elsewhere
        } else if avg <= own + 0.5 {
            WebFitClass::Elite // green: true caliber for this line
        } else if avg <= own + 1.25 {
            WebFitClass::Solid // yellow: ok but above their level
        } else {
            WebFitClass::Stretch // red: overextended
        }
    }
}

/// Web site fit classification (relative, cross-team).
/// Different from terminal FitClass which uses absolute pace thresholds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebFitClass {
    Elite,   // green  — avg ≤ own + 0.5
    Solid,   // yellow — avg ≤ own + 1.25
    Buried,  // blue   — avg < own - 0.75
    Stretch, // red    — avg > own + 1.25
}

impl WebFitClass {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Elite => "fit",
            Self::Solid => "solid",
            Self::Buried => "buried",
            Self::Stretch => "stretch",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Elite => "★",
            Self::Solid => "~",
            Self::Buried => "↑",
            Self::Stretch => "↓",
        }
    }
}

/// Build a map: (team, position) → sorted list of sort keys (desc).
fn build_pos_index(players: &[Player]) -> HashMap<(&TeamAbbr, Position), Vec<f64>> {
    let mut map: HashMap<(&TeamAbbr, Position), Vec<f64>> = HashMap::new();
    for p in players {
        if let Some(score) = p.pace_score {
            map.entry((&p.team, p.position))
                .or_default()
                .push(score.sort_key());
        }
    }
    // Sort descending so index = rank - 1
    for v in map.values_mut() {
        v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    }
    map
}

/// Rank of a sort_key among a sorted (desc) list. 1-indexed.
fn rank_in(sort_key: f64, sorted_desc: &[f64]) -> u8 {
    let rank = sorted_desc.iter().filter(|&&k| k > sort_key).count() + 1;
    rank.min(255) as u8
}

/// Compute cross-team metrics for every player in the dataset.
///
/// `players` must include all players from all 32 teams, with positions
/// already assigned and pace scores computed.
pub fn compute_all(players: &[Player]) -> Vec<CrossTeamMetrics> {
    let all_teams: Vec<&TeamAbbr> = {
        let mut teams: Vec<&TeamAbbr> = players.iter().map(|p| &p.team).collect();
        teams.sort_by(|a, b| a.0.cmp(&b.0));
        teams.dedup();
        teams
    };

    let pos_index = build_pos_index(players);

    players
        .iter()
        .map(|p| {
            let Some(score) = p.pace_score else {
                return CrossTeamMetrics {
                    player_nhl_id: p.nhl_id,
                    own_line: 255,
                    avg_other_line: 255.0,
                    delta: 0.0,
                };
            };

            let sort_key = score.sort_key();

            // Own rank on own team
            let own_sorted = pos_index
                .get(&(&p.team, p.position))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let own_line = rank_in(sort_key, own_sorted);

            // Average rank on each other team
            let other_ranks: Vec<f32> = all_teams
                .iter()
                .filter(|&&t| t != &p.team)
                .map(|t| {
                    let other_sorted = pos_index
                        .get(&(t, p.position))
                        .map(|v| v.as_slice())
                        .unwrap_or(&[]);
                    rank_in(sort_key, other_sorted) as f32
                })
                .collect();

            let avg_other_line = if other_ranks.is_empty() {
                own_line as f32
            } else {
                other_ranks.iter().sum::<f32>() / other_ranks.len() as f32
            };

            let delta = own_line as f32 - avg_other_line;

            CrossTeamMetrics {
                player_nhl_id: p.nhl_id,
                own_line,
                avg_other_line,
                delta,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{GpStatus, PaceScore};
    use crate::name::normalize_name;

    fn make_player(name: &str, team: &str, pos: Position, pace: f64) -> Player {
        Player {
            nhl_id: None,
            full_name: name.to_owned(),
            name_normalized: normalize_name(name),
            team: TeamAbbr(team.to_owned()),
            position: pos,
            eligible_pos: vec![pos],
            gp_status: GpStatus::Eligible(60),
            season_goals: 20,
            season_assists: 30,
            season_points: 50,
            pace_score: Some(PaceScore {
                pace_82: pace,
                goals_per_82: pace * 0.4,
                raw_points: 50,
                gp: 60,
            }),
            pp_goals: 0, pp_points: 0,
            sh_goals: 0, sh_points: 0,
            gwg: 0, ot_goals: 0,
            shots: 0, shooting_pct: None,
            plus_minus: 0,
            toi_per_game_sec: None,
            faceoff_win_pct: None,
            hits: 0, blocked_shots: 0, missed_shots: 0,
            giveaways: 0, takeaways: 0, pim: 0,
            xg: None, xg_per_60: None, cf_pct_5v5: None, ff_pct_5v5: None, xgf_pct_5v5: None,
            headshot_url: None,
            sweater_number: None,
            birth_date: None,
            birth_country: None,
            nationality_code: None,
            birth_city: None,
            birth_state_province: None,
            shoots_catches: None,
            height_in_inches: None,
            weight_lbs: None,
            draft_year: None,
            draft_round: None,
            draft_overall: None,
            rookie_season: None,
            contract_expiry_year: None,
            expiry_type: None,
            salary: None,
        }
    }

    #[test]
    fn l0_cross_team_rank_1_on_own_team() {
        // Top player on SEA should be rank 1 on their own team
        let players = vec![
            make_player("Elite", "SEA", Position::Center, 140.0),
            make_player("Mid", "SEA", Position::Center, 70.0),
            make_player("Depth", "SEA", Position::Center, 40.0),
        ];
        let metrics = compute_all(&players);
        let elite = metrics.iter().find(|m| m.avg_other_line < 2.0).unwrap();
        assert_eq!(elite.own_line, 1, "top player must be rank 1 on own team");
    }

    #[test]
    fn l0_cross_team_buried_detection() {
        // "Buried" is 3rd C on EDM (behind two elite players) but would be
        // #1 C on all other teams which have only weak/no centers.
        let players = vec![
            make_player("Star", "EDM", Position::Center, 140.0),
            make_player("Good", "EDM", Position::Center, 120.0),
            make_player("Buried", "EDM", Position::Center, 110.0),
            // Other teams' Cs are much weaker → Buried ranks #1 there
            make_player("SEA-C1", "SEA", Position::Center, 40.0),
            make_player("NYR-C1", "NYR", Position::Center, 38.0),
            make_player("TOR-C1", "TOR", Position::Center, 35.0),
        ];
        let metrics = compute_all(&players);
        let buried = metrics.iter().find(|m| m.own_line == 3).unwrap();
        // own_line=3 (3rd on EDM), avg_other_line≈1 (best C on SEA/NYR/TOR)
        // delta = 3 - ~1 = ~2 > 0.75 → buried
        assert!(
            buried.delta > 0.75,
            "delta={}, expected > 0.75",
            buried.delta
        );
        assert_eq!(buried.web_fit_class(), WebFitClass::Buried);
    }

    #[test]
    fn l0_web_fit_class_thresholds() {
        let m = |own: u8, avg: f32| CrossTeamMetrics {
            player_nhl_id: None,
            own_line: own,
            avg_other_line: avg,
            delta: own as f32 - avg,
        };
        // own=1, avg=1.3 → delta=-0.3, avg ≤ 1+0.5=1.5 → Elite
        assert_eq!(m(1, 1.3).web_fit_class(), WebFitClass::Elite);
        // own=1, avg=1.8 → avg ≤ 1+1.25=2.25 → Solid
        assert_eq!(m(1, 1.8).web_fit_class(), WebFitClass::Solid);
        // own=1, avg=2.5 → avg > 2.25 → Stretch
        assert_eq!(m(1, 2.5).web_fit_class(), WebFitClass::Stretch);
        // own=3, avg=1.5 → delta=1.5 > 0.75 → Buried
        assert_eq!(m(3, 1.5).web_fit_class(), WebFitClass::Buried);
    }
}
