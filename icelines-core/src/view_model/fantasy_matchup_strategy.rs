use std::collections::BTreeSet;

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::{
    build_fantasy_daily_lineup, FantasyAssistantRules, FantasyLineupPlayerInput,
    FantasyPlayerAvailabilityStatus,
};

pub const FANTASY_MATCHUP_STRATEGY_SCHEMA: &str = "fantasy_matchup_strategy.v1";
const SKATER_VOLATILITY: f64 = 0.35;
const GOALIE_VOLATILITY: f64 = 0.55;
const EIGHTY_PERCENT_Z: f64 = 1.281_551_565_545;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyMatchupStrategy {
    Floor,
    Balanced,
    Upside,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupStrategyPlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub projected_value_per_game: f64,
    pub game_dates: BTreeSet<NaiveDate>,
    pub status: FantasyPlayerAvailabilityStatus,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupStrategyTeamInput {
    pub team: String,
    pub players: Vec<FantasyMatchupStrategyPlayerInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupSwingInput {
    pub add_player_key: String,
    pub add_player: String,
    pub drop_player_key: String,
    pub drop_player: String,
    pub incremental_usable_starts: f64,
    pub projected_value_delta: f64,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupPointsSnapshotInput {
    /// Last matchup date already represented in both point totals.
    pub through_date: NaiveDate,
    pub user_points: f64,
    pub opponent_points: f64,
    pub source: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupStrategyInput {
    pub league: String,
    pub scoring_scheme: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub strategy: FantasyMatchupStrategy,
    pub rules: FantasyAssistantRules,
    pub user: FantasyMatchupStrategyTeamInput,
    pub opponent: FantasyMatchupStrategyTeamInput,
    pub current_points: Option<FantasyMatchupPointsSnapshotInput>,
    pub largest_legal_swing: Option<FantasyMatchupSwingInput>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupDailyProjectionRow {
    pub date: NaiveDate,
    pub projected_points: f64,
    pub usable_starts: usize,
    pub scheduled_player_games: usize,
    pub benched_player_games: usize,
    pub bench_collision_value: f64,
    pub benched_players: Vec<String>,
    #[serde(default)]
    pub starting_players: Vec<FantasyMatchupDailyStartRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupDailyStartRow {
    pub slot_id: String,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub projected_points: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupTeamProjection {
    pub team: String,
    pub current_points: f64,
    pub remaining_projected_points: f64,
    pub projected_points: f64,
    pub floor_points: f64,
    pub upside_points: f64,
    pub usable_starts: usize,
    pub scheduled_player_games: usize,
    pub benched_player_games: usize,
    pub bench_collision_value: f64,
    pub daily: Vec<FantasyMatchupDailyProjectionRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyMatchupStrategyView {
    pub schema: String,
    pub competition_mode: String,
    pub league: String,
    pub scoring_scheme: String,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub matchup_state: String,
    pub current_through_date: Option<NaiveDate>,
    pub current_totals_source: Option<String>,
    pub strategy: FantasyMatchupStrategy,
    pub user: FantasyMatchupTeamProjection,
    pub opponent: FantasyMatchupTeamProjection,
    pub expected_margin: f64,
    pub downside_margin: f64,
    pub upside_margin: f64,
    pub modeled_win_probability: f64,
    pub largest_legal_swing: Option<FantasyMatchupSwingInput>,
    pub recommendation: String,
    pub model_notes: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn build_fantasy_matchup_strategy(
    input: FantasyMatchupStrategyInput,
) -> Result<FantasyMatchupStrategyView, String> {
    input.rules.validate()?;
    if input.week_end < input.week_start {
        return Err("matchup week end cannot precede its start".to_owned());
    }
    if input.user.team == input.opponent.team {
        return Err("matchup strategy requires two different teams".to_owned());
    }

    let (projection_start, user_current, opponent_current, matchup_state, through_date, source) =
        match &input.current_points {
            Some(snapshot) => {
                if snapshot.through_date < input.week_start
                    || snapshot.through_date > input.week_end
                {
                    return Err(
                        "current matchup totals must be through a date inside the selected week"
                            .to_owned(),
                    );
                }
                if !snapshot.user_points.is_finite()
                    || !snapshot.opponent_points.is_finite()
                    || snapshot.user_points < 0.0
                    || snapshot.opponent_points < 0.0
                {
                    return Err(
                        "current matchup point totals must be finite and non-negative".to_owned(),
                    );
                }
                if snapshot.source.trim().is_empty() {
                    return Err("current matchup totals require a source label".to_owned());
                }
                (
                    snapshot.through_date + Duration::days(1),
                    snapshot.user_points,
                    snapshot.opponent_points,
                    if snapshot.through_date == input.week_end {
                        "final"
                    } else {
                        "in_week"
                    },
                    Some(snapshot.through_date),
                    Some(snapshot.source.clone()),
                )
            }
            None => (input.week_start, 0.0, 0.0, "pre_week", None, None),
        };

    let (user, user_variance) = project_team(
        &input.user,
        projection_start,
        input.week_end,
        user_current,
        &input.rules,
    )?;
    let (opponent, opponent_variance) = project_team(
        &input.opponent,
        projection_start,
        input.week_end,
        opponent_current,
        &input.rules,
    )?;
    let expected_margin = user.projected_points - opponent.projected_points;
    let downside_margin = user.floor_points - opponent.upside_points;
    let upside_margin = user.upside_points - opponent.floor_points;
    let margin_deviation = (user_variance + opponent_variance).sqrt();
    let modeled_win_probability = if margin_deviation <= f64::EPSILON {
        if expected_margin > 0.0 {
            1.0
        } else if expected_margin < 0.0 {
            0.0
        } else {
            0.5
        }
    } else {
        logistic_normal_cdf(expected_margin / margin_deviation)
    };
    let recommendation = recommendation(
        input.strategy,
        expected_margin,
        downside_margin,
        upside_margin,
        input.largest_legal_swing.as_ref(),
    );

    Ok(FantasyMatchupStrategyView {
        schema: FANTASY_MATCHUP_STRATEGY_SCHEMA.to_owned(),
        competition_mode: "points".to_owned(),
        league: input.league,
        scoring_scheme: input.scoring_scheme,
        week_start: input.week_start,
        week_end: input.week_end,
        matchup_state: matchup_state.to_owned(),
        current_through_date: through_date,
        current_totals_source: source,
        strategy: input.strategy,
        user,
        opponent,
        expected_margin,
        downside_margin,
        upside_margin,
        modeled_win_probability,
        largest_legal_swing: input.largest_legal_swing,
        recommendation,
        model_notes: vec![
            match &input.current_points {
                Some(snapshot) => format!(
                    "current point totals through {} are fixed; only later game dates are projected",
                    snapshot.through_date
                ),
                None => "pre-week points-mode projection; no current matchup totals supplied"
                    .to_owned(),
            },
            "80% team bands use independent per-start volatility proxies: 35% skaters and 55% goalies"
                .to_owned(),
            "win probability is a deterministic logistic approximation from the modeled margin distribution, not betting odds"
                .to_owned(),
        ],
        warnings: input.warnings,
    })
}

fn project_team(
    input: &FantasyMatchupStrategyTeamInput,
    projection_start: NaiveDate,
    week_end: NaiveDate,
    current_points: f64,
    rules: &FantasyAssistantRules,
) -> Result<(FantasyMatchupTeamProjection, f64), String> {
    let mut daily = Vec::new();
    let mut variance = 0.0;
    let mut date = projection_start;
    while date <= week_end {
        let lineup_inputs = input
            .players
            .iter()
            .map(|player| FantasyLineupPlayerInput {
                player_key: player.player_key.clone(),
                display_name: player.player.clone(),
                nhl_team: player.nhl_team.clone(),
                platform_positions: player.positions.clone(),
                projected_value: player.projected_value_per_game,
                has_game: player.game_dates.contains(&date),
                status: player.status,
                locked_slot: None,
                locked: false,
            })
            .collect::<Vec<_>>();
        let lineup = build_fantasy_daily_lineup(rules.clone(), lineup_inputs)?;
        let active_keys = lineup
            .active
            .iter()
            .filter(|row| row.has_game && row.status.expected_available())
            .map(|row| row.player_key.as_str())
            .collect::<BTreeSet<_>>();
        let scheduled = input
            .players
            .iter()
            .filter(|player| {
                player.game_dates.contains(&date) && player.status.expected_available()
            })
            .collect::<Vec<_>>();
        let benched = scheduled
            .iter()
            .filter(|player| !active_keys.contains(player.player_key.as_str()))
            .copied()
            .collect::<Vec<_>>();
        let bench_collision_value = canonical_zero(
            benched
                .iter()
                .map(|player| player.projected_value_per_game)
                .sum(),
        );
        for row in lineup
            .active
            .iter()
            .filter(|row| row.has_game && row.status.expected_available())
        {
            let volatility = if row.platform_positions.contains(&Position::Goalie) {
                GOALIE_VOLATILITY
            } else {
                SKATER_VOLATILITY
            };
            variance += (row.projected_value * volatility).powi(2);
        }
        daily.push(FantasyMatchupDailyProjectionRow {
            date,
            projected_points: canonical_zero(lineup.projected_active_value),
            usable_starts: lineup.usable_starts,
            scheduled_player_games: scheduled.len(),
            benched_player_games: benched.len(),
            bench_collision_value,
            benched_players: benched.iter().map(|player| player.player.clone()).collect(),
            starting_players: lineup
                .active
                .iter()
                .filter(|row| row.has_game && row.status.expected_available())
                .map(|row| FantasyMatchupDailyStartRow {
                    slot_id: row.slot_id.clone(),
                    player_key: row.player_key.clone(),
                    player: row.player.clone(),
                    nhl_team: row.nhl_team.clone(),
                    positions: row.platform_positions.clone(),
                    projected_points: canonical_zero(row.projected_value),
                })
                .collect(),
        });
        date += Duration::days(1);
    }

    let remaining_projected_points =
        canonical_zero(daily.iter().map(|row| row.projected_points).sum::<f64>());
    let projected_points = canonical_zero(current_points + remaining_projected_points);
    let deviation = variance.sqrt();
    Ok((
        FantasyMatchupTeamProjection {
            team: input.team.clone(),
            current_points,
            remaining_projected_points,
            projected_points,
            floor_points: canonical_zero(projected_points - EIGHTY_PERCENT_Z * deviation),
            upside_points: projected_points + EIGHTY_PERCENT_Z * deviation,
            usable_starts: daily.iter().map(|row| row.usable_starts).sum(),
            scheduled_player_games: daily.iter().map(|row| row.scheduled_player_games).sum(),
            benched_player_games: daily.iter().map(|row| row.benched_player_games).sum(),
            bench_collision_value: daily.iter().map(|row| row.bench_collision_value).sum(),
            daily,
        },
        variance,
    ))
}

fn logistic_normal_cdf(z: f64) -> f64 {
    1.0 / (1.0 + (-1.702 * z).exp())
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 {
        0.0
    } else {
        value
    }
}

fn recommendation(
    strategy: FantasyMatchupStrategy,
    expected_margin: f64,
    downside_margin: f64,
    upside_margin: f64,
    swing: Option<&FantasyMatchupSwingInput>,
) -> String {
    let posture = match strategy {
        FantasyMatchupStrategy::Floor if downside_margin >= 0.0 => {
            "Protect the modeled floor; the downside band still leads"
        }
        FantasyMatchupStrategy::Floor => {
            "The downside band trails; reduce avoidable bench collisions and volatility"
        }
        FantasyMatchupStrategy::Balanced if expected_margin >= 0.0 => {
            "Hold the expected-value edge while preserving the injury pickup reserve"
        }
        FantasyMatchupStrategy::Balanced => {
            "The baseline trails; pursue a positive legal swing without sacrificing roster structure"
        }
        FantasyMatchupStrategy::Upside if upside_margin > 0.0 => {
            "An upset path exists in the upside band; prioritize incremental usable starts"
        }
        FantasyMatchupStrategy::Upside => {
            "Even the upside band trails; a material roster move or opponent miss is required"
        }
    };
    match swing {
        Some(swing) if swing.projected_value_delta > 0.0 => format!(
            "{posture}. Best current one-move swing: add {} for {} ({:+.1} modeled value).",
            swing.add_player, swing.drop_player, swing.projected_value_delta
        ),
        _ => format!("{posture}. No positive legal one-move swing is currently available."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::FantasyActiveSlotKind;
    use std::collections::BTreeMap;

    fn rules() -> FantasyAssistantRules {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::Center, 1)]);
        rules.bench_slots = 1;
        rules
    }

    fn player(key: &str, value: f64, dates: &[NaiveDate]) -> FantasyMatchupStrategyPlayerInput {
        FantasyMatchupStrategyPlayerInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: "NYR".to_owned(),
            positions: vec![Position::Center],
            projected_value_per_game: value,
            game_dates: dates.iter().copied().collect(),
            status: FantasyPlayerAvailabilityStatus::Healthy,
        }
    }

    #[test]
    fn points_strategy_uses_legal_daily_assignment_and_counts_collisions() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let view = build_fantasy_matchup_strategy(FantasyMatchupStrategyInput {
            league: "League".to_owned(),
            scoring_scheme: "custom".to_owned(),
            week_start: monday,
            week_end: monday,
            strategy: FantasyMatchupStrategy::Balanced,
            rules: rules(),
            user: FantasyMatchupStrategyTeamInput {
                team: "Dawgs".to_owned(),
                players: vec![
                    player("star", 10.0, &[monday]),
                    player("bench", 6.0, &[monday]),
                ],
            },
            opponent: FantasyMatchupStrategyTeamInput {
                team: "Rival".to_owned(),
                players: vec![player("rival", 8.0, &[monday])],
            },
            current_points: None,
            largest_legal_swing: None,
            warnings: Vec::new(),
        })
        .unwrap();

        assert_eq!(view.schema, FANTASY_MATCHUP_STRATEGY_SCHEMA);
        assert_eq!(view.user.projected_points, 10.0);
        assert_eq!(view.user.current_points, 0.0);
        assert_eq!(view.user.remaining_projected_points, 10.0);
        assert_eq!(view.user.usable_starts, 1);
        assert_eq!(view.user.scheduled_player_games, 2);
        assert_eq!(view.user.benched_player_games, 1);
        assert_eq!(view.user.bench_collision_value, 6.0);
        assert_eq!(view.user.daily[0].starting_players.len(), 1);
        assert_eq!(view.user.daily[0].starting_players[0].player, "star");
        assert_eq!(view.user.daily[0].starting_players[0].slot_id, "C1");
        assert_eq!(
            view.user.daily[0].starting_players[0].projected_points,
            10.0
        );
        assert_eq!(view.expected_margin, 2.0);
        assert!(view.modeled_win_probability > 0.5);
    }

    #[test]
    fn points_strategy_is_deterministic_and_rejects_same_team() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let input = FantasyMatchupStrategyInput {
            league: "League".to_owned(),
            scoring_scheme: "custom".to_owned(),
            week_start: monday,
            week_end: monday + Duration::days(6),
            strategy: FantasyMatchupStrategy::Floor,
            rules: rules(),
            user: FantasyMatchupStrategyTeamInput {
                team: "Same".to_owned(),
                players: vec![],
            },
            opponent: FantasyMatchupStrategyTeamInput {
                team: "Same".to_owned(),
                players: vec![],
            },
            current_points: None,
            largest_legal_swing: None,
            warnings: Vec::new(),
        };
        assert!(build_fantasy_matchup_strategy(input).is_err());
    }

    #[test]
    fn current_totals_are_fixed_and_elapsed_dates_are_not_projected_twice() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let tuesday = monday + Duration::days(1);
        let view = build_fantasy_matchup_strategy(FantasyMatchupStrategyInput {
            league: "League".to_owned(),
            scoring_scheme: "custom".to_owned(),
            week_start: monday,
            week_end: tuesday,
            strategy: FantasyMatchupStrategy::Balanced,
            rules: rules(),
            user: FantasyMatchupStrategyTeamInput {
                team: "Dawgs".to_owned(),
                players: vec![player("star", 10.0, &[monday, tuesday])],
            },
            opponent: FantasyMatchupStrategyTeamInput {
                team: "Rival".to_owned(),
                players: vec![player("rival", 8.0, &[monday, tuesday])],
            },
            current_points: Some(FantasyMatchupPointsSnapshotInput {
                through_date: monday,
                user_points: 12.0,
                opponent_points: 9.0,
                source: "manual Yahoo paste".to_owned(),
            }),
            largest_legal_swing: None,
            warnings: Vec::new(),
        })
        .unwrap();

        assert_eq!(view.matchup_state, "in_week");
        assert_eq!(view.current_through_date, Some(monday));
        assert_eq!(view.user.current_points, 12.0);
        assert_eq!(view.user.remaining_projected_points, 10.0);
        assert_eq!(view.user.projected_points, 22.0);
        assert_eq!(view.user.daily.len(), 1);
        assert_eq!(view.user.daily[0].date, tuesday);
        assert_eq!(view.expected_margin, 5.0);
    }

    #[test]
    fn non_available_status_cannot_contribute_a_projected_start() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let mut unavailable = player("injured star", 20.0, &[monday]);
        unavailable.status = FantasyPlayerAvailabilityStatus::Out;
        let view = build_fantasy_matchup_strategy(FantasyMatchupStrategyInput {
            league: "League".to_owned(),
            scoring_scheme: "custom".to_owned(),
            week_start: monday,
            week_end: monday,
            strategy: FantasyMatchupStrategy::Floor,
            rules: rules(),
            user: FantasyMatchupStrategyTeamInput {
                team: "Dawgs".to_owned(),
                players: vec![unavailable, player("healthy", 5.0, &[monday])],
            },
            opponent: FantasyMatchupStrategyTeamInput {
                team: "Rival".to_owned(),
                players: Vec::new(),
            },
            current_points: None,
            largest_legal_swing: None,
            warnings: Vec::new(),
        })
        .unwrap();

        assert_eq!(view.user.projected_points, 5.0);
        assert_eq!(view.user.usable_starts, 1);
        assert_eq!(view.user.scheduled_player_games, 1);
    }
}
