//! Cross-team ranking metrics.
//!
//! For each player, computes their average line number across the other 31
//! NHL teams — answering: "what line would this player play if they were
//! on any other team?"
//!
//! This is the metric used by the web site for color-coding lineup cards.
//! It differs from the terminal `classify_fit()` which uses absolute pace
//! thresholds.

use crate::model::Position;
use std::collections::HashMap;

/// Which metric to use when ranking players across teams.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringMode {
    Pace,    // pts/82 pace (default)
    Fantasy, // Yahoo-style: G×3 A×2 PPG×1 PPA×0.5 SHG×1 SHA×0.5 GWG×0.5 HIT×0.5 BLK×0.5
}

impl ScoringMode {
    pub fn label(self) -> &'static str {
        match self { Self::Pace => "Pts/82", Self::Fantasy => "FPts" }
    }
    pub fn toggle(self) -> Self {
        match self { Self::Pace => Self::Fantasy, Self::Fantasy => Self::Pace }
    }
}

/// Yahoo-style fantasy points for a skater. Realtime stats (hits,
/// blocked_shots) default to 0 when the view's realtime tier is None
/// (cold-start).
pub fn fantasy_score_view(v: &crate::stats_repository::PlayerView<'_>) -> f64 {
    let totals = &v.stats.totals;
    let pp_ast = (totals.pp_points as i32 - totals.pp_goals as i32).max(0) as f64;
    let sh_ast = (totals.sh_points as i32 - totals.sh_goals as i32).max(0) as f64;
    let hits = v.hits().unwrap_or(0);
    let blocks = v.blocked_shots().unwrap_or(0);
    totals.goals as f64        * 3.0
        + totals.assists as f64 * 2.0
        + totals.pp_goals as f64* 1.0
        + pp_ast                * 0.5
        + totals.sh_goals as f64* 1.0
        + sh_ast                * 0.5
        + totals.gwg as f64     * 0.5
        + hits as f64           * 0.5
        + blocks as f64         * 0.5
}

/// Aggregated strength for one team — top-4 per forward slot + top-6 D.
#[derive(Debug, Clone)]
pub struct TeamStrength {
    pub c_score:  f64,
    pub lw_score: f64,
    pub rw_score: f64,
    pub d_score:  f64,
    pub total:    f64,
    pub c_top:    String,
    pub lw_top:   String,
    pub rw_top:   String,
    pub d_top:    String,
}

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

/// Rank of a sort_key among a sorted (desc) list. 1-indexed.
fn rank_in(sort_key: f64, sorted_desc: &[f64]) -> u8 {
    let rank = sorted_desc.iter().filter(|&&k| k > sort_key).count() + 1;
    rank.min(255) as u8
}

/// Compute cross-team metrics for every player — Pace scoring (default).
pub fn compute_all_views(views: &[crate::stats_repository::PlayerView<'_>]) -> Vec<CrossTeamMetrics> {
    compute_all_views_with_mode(views, ScoringMode::Pace)
}

/// Hart.5b2c — PlayerView analog of `compute_all_with_mode`.
pub fn compute_all_views_with_mode(
    views: &[crate::stats_repository::PlayerView<'_>],
    mode: ScoringMode,
) -> Vec<CrossTeamMetrics> {
    use crate::stats_repository::PlayerView;

    // Build (team_str, pos) -> sorted scores desc.
    let mut pos_index: HashMap<(String, Position), Vec<f64>> = HashMap::new();
    for v in views {
        let score = match mode {
            ScoringMode::Pace => match v.pace_score() {
                Some(s) => s.sort_key(),
                None => continue,
            },
            ScoringMode::Fantasy => fantasy_score_view(v),
        };
        pos_index
            .entry((v.team_display().to_owned(), v.position()))
            .or_default()
            .push(score);
    }
    for vec in pos_index.values_mut() {
        vec.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    }

    let all_teams: Vec<String> = {
        let mut teams: Vec<String> = views.iter().map(|v| v.team_display().to_owned()).collect();
        teams.sort();
        teams.dedup();
        teams
    };

    views
        .iter()
        .map(|v: &PlayerView<'_>| {
            let sort_key = match mode {
                ScoringMode::Pace => match v.pace_score() {
                    Some(s) => s.sort_key(),
                    None => {
                        return CrossTeamMetrics {
                            player_nhl_id: Some(v.id().0),
                            own_line: 255,
                            avg_other_line: 255.0,
                            delta: 0.0,
                        }
                    }
                },
                ScoringMode::Fantasy => fantasy_score_view(v),
            };

            let own_team = v.team_display().to_owned();
            let own_sorted = pos_index
                .get(&(own_team.clone(), v.position()))
                .map(|v| v.as_slice())
                .unwrap_or(&[]);
            let own_line = rank_in(sort_key, own_sorted);

            let other_ranks: Vec<f32> = all_teams
                .iter()
                .filter(|t| **t != own_team)
                .map(|t| {
                    let other_sorted = pos_index
                        .get(&(t.clone(), v.position()))
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
                player_nhl_id: Some(v.id().0),
                own_line,
                avg_other_line,
                delta,
            }
        })
        .collect()
}

/// Hart.5b2c — PlayerView analog of `compute_team_strength`.
pub fn compute_team_strength_views(
    views: &[crate::stats_repository::PlayerView<'_>],
    mode: ScoringMode,
) -> HashMap<String, TeamStrength> {
    let mut groups: HashMap<(String, Position), Vec<(f64, String)>> = HashMap::new();
    for v in views {
        let score = match mode {
            ScoringMode::Fantasy => fantasy_score_view(v),
            ScoringMode::Pace => v.pace_82().unwrap_or(0.0),
        };
        groups
            .entry((v.team_display().to_owned(), v.position()))
            .or_default()
            .push((score, v.full_name().to_owned()));
    }
    for g in groups.values_mut() {
        g.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    }

    let all_teams: Vec<String> = groups
        .keys()
        .map(|(t, _)| t.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();

    let mut out = HashMap::new();
    for team in all_teams {
        let c = groups
            .get(&(team.clone(), Position::Center))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let lw = groups
            .get(&(team.clone(), Position::LeftWing))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let rw = groups
            .get(&(team.clone(), Position::RightWing))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        let d = groups
            .get(&(team.clone(), Position::Defense))
            .map(|v| v.as_slice())
            .unwrap_or(&[]);

        let c_score = c.iter().take(4).map(|(s, _)| s).sum();
        let lw_score = lw.iter().take(4).map(|(s, _)| s).sum();
        let rw_score = rw.iter().take(4).map(|(s, _)| s).sum();
        let d_score = d.iter().take(6).map(|(s, _)| s).sum();

        out.insert(
            team,
            TeamStrength {
                c_score,
                lw_score,
                rw_score,
                d_score,
                total: c_score + lw_score + rw_score + d_score,
                c_top: c
                    .first()
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| "—".to_owned()),
                lw_top: lw
                    .first()
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| "—".to_owned()),
                rw_top: rw
                    .first()
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| "—".to_owned()),
                d_top: d
                    .first()
                    .map(|(_, n)| n.clone())
                    .unwrap_or_else(|| "—".to_owned()),
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::model::Season;
    use crate::season_stats::SeasonType;
    use crate::stats_repository::StatsRepository;

    fn build_pool(seeds: &[(u32, &str, &str, Position, f64)]) -> StatsRepository {
        let mut r = StatsRepository::new();
        for &(id, name, team, pos, pace) in seeds {
            let normalized = crate::name::normalize_name(name);
            let identity = fixtures::identity(id).name(name, &normalized).build();
            let mut stats = fixtures::stats(id, 20242025, team).position(pos).build();
            if let Some(ref mut ps) = stats.totals.pace_score {
                ps.pace_82 = pace;
            }
            r.upsert_identity(identity).unwrap();
            r.upsert_stats(stats).unwrap();
        }
        r
    }

    fn metrics_for(seeds: &[(u32, &str, &str, Position, f64)]) -> Vec<CrossTeamMetrics> {
        let repo = build_pool(seeds);
        let views: Vec<_> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        compute_all_views(&views)
    }

    #[test]
    fn l0_cross_team_rank_1_on_own_team() {
        // Top player on SEA should be rank 1 on their own team
        let metrics = metrics_for(&[
            (1, "Elite", "SEA", Position::Center, 140.0),
            (2, "Mid",   "SEA", Position::Center,  70.0),
            (3, "Depth", "SEA", Position::Center,  40.0),
        ]);
        let elite = metrics.iter().find(|m| m.avg_other_line < 2.0).unwrap();
        assert_eq!(elite.own_line, 1, "top player must be rank 1 on own team");
    }

    #[test]
    fn l0_cross_team_buried_detection() {
        // "Buried" is 3rd C on EDM (behind two elite players) but would be
        // #1 C on all other teams which have only weak/no centers.
        let metrics = metrics_for(&[
            (1, "Star",   "EDM", Position::Center, 140.0),
            (2, "Good",   "EDM", Position::Center, 120.0),
            (3, "Buried", "EDM", Position::Center, 110.0),
            (4, "SEA-C1", "SEA", Position::Center,  40.0),
            (5, "NYR-C1", "NYR", Position::Center,  38.0),
            (6, "TOR-C1", "TOR", Position::Center,  35.0),
        ]);
        let buried = metrics.iter().find(|m| m.own_line == 3).unwrap();
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
