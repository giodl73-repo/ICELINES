use crate::model::Position;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TEAM_CEILING_SCHEMA: &str = "team_ceiling.v1";
pub const TEAM_CEILING_METHOD: &str =
    "current-roster depth ensemble over prior-season NHL production";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TeamCeilingLens {
    PointsPace,
    GoalScoring,
    Fantasy,
    Upside,
}

impl TeamCeilingLens {
    pub const ALL: [Self; 4] = [
        Self::PointsPace,
        Self::GoalScoring,
        Self::Fantasy,
        Self::Upside,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::PointsPace => "Points pace",
            Self::GoalScoring => "Goal scoring",
            Self::Fantasy => "Fantasy/peripherals",
            Self::Upside => "Age-adjusted upside",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCeilingPlayerInput {
    pub player_id: u32,
    pub player: String,
    pub team: String,
    pub prior_team: Option<String>,
    pub position: Position,
    pub age: u8,
    pub games_played: u32,
    pub points_per_82: Option<f64>,
    pub goals_per_82: Option<f64>,
    pub shots_per_82: Option<f64>,
    pub fantasy_per_82: Option<f64>,
    /// Goalie quality is expressed on a roughly 0-100 scale by the caller.
    pub goalie_quality: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCeilingLensScore {
    pub lens: TeamCeilingLens,
    pub label: String,
    pub score: f64,
    pub previous_score: f64,
    pub delta: f64,
    pub rank: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCeilingPlayerRow {
    pub player_id: u32,
    pub player: String,
    pub position: Position,
    pub age: u8,
    pub games_played: u32,
    pub prior_team: Option<String>,
    pub newcomer: bool,
    pub has_nhl_sample: bool,
    pub lens_scores: BTreeMap<TeamCeilingLens, Option<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCeilingRow {
    pub team: String,
    pub rank: u8,
    pub roster_players: u32,
    pub rated_players: u32,
    pub coverage_pct: f64,
    pub ensemble_score: f64,
    pub previous_ensemble_score: f64,
    pub delta: f64,
    pub ceiling_score: f64,
    pub playoff_chance_low_pct: f64,
    pub playoff_chance_high_pct: f64,
    pub lenses: Vec<TeamCeilingLensScore>,
    pub newcomers: Vec<String>,
    pub departures: Vec<String>,
    pub players: Vec<TeamCeilingPlayerRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamCeilingView {
    pub schema: String,
    pub method: String,
    pub roster_season: u32,
    pub stats_season: u32,
    pub teams: Vec<TeamCeilingRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum TeamCeilingError {
    #[error("team-ceiling input has no current-roster players")]
    EmptyCurrentRoster,
    #[error("team-ceiling input has no previous-roster players")]
    EmptyPreviousRoster,
}

#[derive(Default)]
struct TeamRaw {
    current: BTreeMap<TeamCeilingLens, f64>,
    previous: BTreeMap<TeamCeilingLens, f64>,
}

pub fn build_team_ceiling(
    current: Vec<TeamCeilingPlayerInput>,
    previous: Vec<TeamCeilingPlayerInput>,
    roster_season: u32,
    stats_season: u32,
) -> Result<TeamCeilingView, TeamCeilingError> {
    if current.is_empty() {
        return Err(TeamCeilingError::EmptyCurrentRoster);
    }
    if previous.is_empty() {
        return Err(TeamCeilingError::EmptyPreviousRoster);
    }

    let current_by_team = group_by_team(current);
    let previous_by_team = group_by_team(previous);
    let teams: BTreeSet<String> = current_by_team
        .keys()
        .chain(previous_by_team.keys())
        .cloned()
        .collect();
    let mut raw_by_team = BTreeMap::new();
    for team in &teams {
        let empty = Vec::new();
        let current_players = current_by_team.get(team).unwrap_or(&empty);
        let previous_players = previous_by_team.get(team).unwrap_or(&empty);
        let mut raw = TeamRaw::default();
        for lens in TeamCeilingLens::ALL {
            raw.current
                .insert(lens, aggregate_depth(current_players, lens));
            raw.previous
                .insert(lens, aggregate_depth(previous_players, lens));
        }
        raw_by_team.insert(team.clone(), raw);
    }

    let bounds: BTreeMap<TeamCeilingLens, (f64, f64)> = TeamCeilingLens::ALL
        .into_iter()
        .map(|lens| {
            let values: Vec<f64> = raw_by_team
                .values()
                .flat_map(|raw| [raw.current[&lens], raw.previous[&lens]])
                .collect();
            let min = values.iter().copied().fold(f64::INFINITY, f64::min);
            let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            (lens, (min, max))
        })
        .collect();

    let mut rows = Vec::new();
    for team in teams {
        let empty = Vec::new();
        let current_players = current_by_team.get(&team).unwrap_or(&empty);
        let previous_players = previous_by_team.get(&team).unwrap_or(&empty);
        let raw = &raw_by_team[&team];
        let mut lens_rows = Vec::new();
        for lens in TeamCeilingLens::ALL {
            let (min, max) = bounds[&lens];
            let score = normalize(raw.current[&lens], min, max);
            let previous_score = normalize(raw.previous[&lens], min, max);
            lens_rows.push(TeamCeilingLensScore {
                lens,
                label: lens.label().to_owned(),
                score,
                previous_score,
                delta: score - previous_score,
                rank: 0,
            });
        }
        let ensemble_score = mean(lens_rows.iter().map(|row| row.score));
        let previous_ensemble_score = mean(lens_rows.iter().map(|row| row.previous_score));
        let rated_players = current_players
            .iter()
            .filter(|player| has_sample(player))
            .count() as u32;
        let roster_players = current_players.len() as u32;
        let coverage_pct = percentage(rated_players, roster_players);
        let uncertainty = 10.0 + (100.0 - coverage_pct) * 0.25;
        let chance_mid = logistic_chance(ensemble_score);

        let previous_ids: BTreeSet<u32> = previous_players
            .iter()
            .map(|player| player.player_id)
            .collect();
        let current_ids: BTreeSet<u32> = current_players
            .iter()
            .map(|player| player.player_id)
            .collect();
        let mut newcomers: Vec<String> = current_players
            .iter()
            .filter(|player| !previous_ids.contains(&player.player_id))
            .map(|player| player.player.clone())
            .collect();
        let mut departures: Vec<String> = previous_players
            .iter()
            .filter(|player| !current_ids.contains(&player.player_id))
            .map(|player| player.player.clone())
            .collect();
        newcomers.sort();
        departures.sort();

        let mut players: Vec<TeamCeilingPlayerRow> = current_players
            .iter()
            .map(|player| player_row(player, &previous_ids))
            .collect();
        players.sort_by(|a, b| {
            let a_score = a.lens_scores[&TeamCeilingLens::Upside].unwrap_or(-1.0);
            let b_score = b.lens_scores[&TeamCeilingLens::Upside].unwrap_or(-1.0);
            b_score
                .partial_cmp(&a_score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.player.cmp(&b.player))
        });

        rows.push(TeamCeilingRow {
            team,
            rank: 0,
            roster_players,
            rated_players,
            coverage_pct,
            ensemble_score,
            previous_ensemble_score,
            delta: ensemble_score - previous_ensemble_score,
            ceiling_score: lens_rows
                .iter()
                .find(|row| row.lens == TeamCeilingLens::Upside)
                .map_or(0.0, |row| row.score),
            playoff_chance_low_pct: (chance_mid - uncertainty).clamp(1.0, 99.0),
            playoff_chance_high_pct: (chance_mid + uncertainty).clamp(1.0, 99.0),
            lenses: lens_rows,
            newcomers,
            departures,
            players,
        });
    }

    rows.sort_by(|a, b| {
        b.ensemble_score
            .partial_cmp(&a.ensemble_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.team.cmp(&b.team))
    });
    for (idx, row) in rows.iter_mut().enumerate() {
        row.rank = (idx + 1) as u8;
    }
    for lens in TeamCeilingLens::ALL {
        let mut order: Vec<(usize, f64)> = rows
            .iter()
            .enumerate()
            .map(|(idx, row)| {
                let score = row
                    .lenses
                    .iter()
                    .find(|value| value.lens == lens)
                    .map_or(0.0, |value| value.score);
                (idx, score)
            })
            .collect();
        order.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        for (rank, (idx, _)) in order.into_iter().enumerate() {
            if let Some(value) = rows[idx].lenses.iter_mut().find(|value| value.lens == lens) {
                value.rank = (rank + 1) as u8;
            }
        }
    }

    Ok(TeamCeilingView {
        schema: TEAM_CEILING_SCHEMA.to_owned(),
        method: TEAM_CEILING_METHOD.to_owned(),
        roster_season,
        stats_season,
        teams: rows,
        disclosures: vec![
            "Scores use current roster membership and prior-season NHL production; they are roster scenarios, not forecasts from a trained model.".to_owned(),
            "Depth aggregation retains the best 12 forwards, 6 defensemen, and 2 goalies under each lens.".to_owned(),
            "The previous-season comparison uses each player's final listed team and a 23-player depth selection (14 forwards, 7 defensemen, 2 goalies).".to_owned(),
            "All lens scores are normalized 0-100 across current and previous team totals, so deltas are comparable within this report.".to_owned(),
            "Playoff chance ranges are transparent logistic scenarios widened for missing player samples; they are not calibrated probabilities or betting odds.".to_owned(),
            "Players without a prior-season NHL sample remain on the roster and reduce coverage; they are not silently scored as zero.".to_owned(),
        ],
    })
}

fn group_by_team(
    players: Vec<TeamCeilingPlayerInput>,
) -> BTreeMap<String, Vec<TeamCeilingPlayerInput>> {
    let mut out: BTreeMap<String, Vec<TeamCeilingPlayerInput>> = BTreeMap::new();
    for mut player in players {
        player.team = player.team.trim().to_ascii_uppercase();
        out.entry(player.team.clone()).or_default().push(player);
    }
    out
}

fn aggregate_depth(players: &[TeamCeilingPlayerInput], lens: TeamCeilingLens) -> f64 {
    let mut forwards = scores_for(players, lens, |position| position.is_forward());
    let mut defense = scores_for(players, lens, |position| position.is_defense());
    let mut goalies = scores_for(players, lens, |position| position == Position::Goalie);
    forwards.sort_by(descending);
    defense.sort_by(descending);
    goalies.sort_by(descending);
    forwards.into_iter().take(12).sum::<f64>()
        + defense.into_iter().take(6).sum::<f64>()
        + goalies.into_iter().take(2).sum::<f64>()
}

fn scores_for(
    players: &[TeamCeilingPlayerInput],
    lens: TeamCeilingLens,
    predicate: impl Fn(Position) -> bool,
) -> Vec<f64> {
    players
        .iter()
        .filter(|player| predicate(player.position))
        .filter_map(|player| team_ceiling_player_lens_score(player, lens))
        .collect()
}

pub fn team_ceiling_player_lens_score(
    player: &TeamCeilingPlayerInput,
    lens: TeamCeilingLens,
) -> Option<f64> {
    if player.position == Position::Goalie {
        return player.goalie_quality;
    }
    match lens {
        TeamCeilingLens::PointsPace => player.points_per_82,
        TeamCeilingLens::GoalScoring => player.goals_per_82,
        TeamCeilingLens::Fantasy => player.fantasy_per_82,
        TeamCeilingLens::Upside => {
            let pace = player.points_per_82?;
            let goals = player.goals_per_82.unwrap_or(0.0);
            let shots = player.shots_per_82.unwrap_or(0.0);
            Some(
                pace * 0.55
                    + goals * 0.50
                    + shots * 0.04
                    + match player.age {
                        0..=22 => 8.0,
                        23..=25 => 5.0,
                        26..=29 => 2.0,
                        30..=32 => 0.0,
                        _ => -3.0,
                    },
            )
        }
    }
}

fn player_row(
    player: &TeamCeilingPlayerInput,
    previous_ids: &BTreeSet<u32>,
) -> TeamCeilingPlayerRow {
    TeamCeilingPlayerRow {
        player_id: player.player_id,
        player: player.player.clone(),
        position: player.position,
        age: player.age,
        games_played: player.games_played,
        prior_team: player.prior_team.clone(),
        newcomer: !previous_ids.contains(&player.player_id),
        has_nhl_sample: has_sample(player),
        lens_scores: TeamCeilingLens::ALL
            .into_iter()
            .map(|lens| (lens, team_ceiling_player_lens_score(player, lens)))
            .collect(),
    }
}

fn has_sample(player: &TeamCeilingPlayerInput) -> bool {
    player.games_played > 0
        && if player.position == Position::Goalie {
            player.goalie_quality.is_some()
        } else {
            player.points_per_82.is_some()
        }
}

fn descending(a: &f64, b: &f64) -> std::cmp::Ordering {
    b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal)
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    if !min.is_finite() || !max.is_finite() || (max - min).abs() < f64::EPSILON {
        50.0
    } else {
        ((value - min) / (max - min) * 100.0).clamp(0.0, 100.0)
    }
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let values: Vec<f64> = values.collect();
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn percentage(value: u32, total: u32) -> f64 {
    if total == 0 {
        0.0
    } else {
        value as f64 / total as f64 * 100.0
    }
}

fn logistic_chance(score: f64) -> f64 {
    15.0 + 75.0 / (1.0 + (-(score - 50.0) / 18.0).exp())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn skater(id: u32, team: &str, pace: Option<f64>, age: u8) -> TeamCeilingPlayerInput {
        TeamCeilingPlayerInput {
            player_id: id,
            player: format!("Player {id}"),
            team: team.to_owned(),
            prior_team: Some(team.to_owned()),
            position: Position::Center,
            age,
            games_played: pace.map_or(0, |_| 82),
            points_per_82: pace,
            goals_per_82: pace.map(|value| value * 0.4),
            shots_per_82: pace.map(|value| value * 2.5),
            fantasy_per_82: pace.map(|value| value * 3.0),
            goalie_quality: None,
        }
    }

    #[test]
    fn l0_team_ceiling_ranks_stronger_roster_and_reports_delta() {
        let current = vec![
            skater(1, "NYR", Some(100.0), 23),
            skater(2, "BOS", Some(50.0), 30),
        ];
        let previous = vec![
            skater(1, "NYR", Some(70.0), 22),
            skater(2, "BOS", Some(60.0), 29),
        ];
        let view = build_team_ceiling(current, previous, 20262027, 20252026).unwrap();
        assert_eq!(view.teams[0].team, "NYR");
        assert!(view.teams[0].delta > 0.0);
        assert!(view.teams[1].delta < 0.0);
    }

    #[test]
    fn l0_team_ceiling_missing_sample_reduces_coverage_without_zero_score() {
        let current = vec![skater(1, "NYR", Some(80.0), 25), skater(3, "NYR", None, 19)];
        let previous = vec![skater(1, "NYR", Some(80.0), 24)];
        let view = build_team_ceiling(current, previous, 20262027, 20252026).unwrap();
        assert_eq!(view.teams[0].coverage_pct, 50.0);
        let prospect = view.teams[0]
            .players
            .iter()
            .find(|row| row.player_id == 3)
            .unwrap();
        assert!(!prospect.has_nhl_sample);
        assert_eq!(prospect.lens_scores[&TeamCeilingLens::PointsPace], None);
    }
}
