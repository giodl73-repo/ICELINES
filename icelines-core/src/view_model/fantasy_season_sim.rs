use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Position;
use crate::view_model::fantasy_assistant::{FantasyActiveSlotKind, FantasyAssistantRules};

pub const FANTASY_SEASON_SIM_SCHEMA: &str = "fantasy_season_sim.v1";
const PICKUP_RETENTION_HORIZON_GAMES: f64 = 3.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySeasonSimPlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub fantasy_points_per_game: f64,
    pub game_dates: Vec<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySeasonSimConfig {
    pub fantasy_teams: usize,
    pub playoff_teams: usize,
    pub trials: usize,
    pub seed: u64,
    pub daily_injury_rate: f64,
    pub min_injury_days: u16,
    pub max_injury_days: u16,
    pub weekly_trade_probability: f64,
    pub weekly_pickup_limit: u8,
    #[serde(default = "default_user_pickup_reserve")]
    pub user_proactive_pickup_reserve: u8,
    #[serde(default = "default_true")]
    pub user_exceptional_reserve_enabled: bool,
    #[serde(default = "default_exceptional_reserve_min_value")]
    pub user_exceptional_reserve_min_value: f64,
    #[serde(default = "default_exceptional_reserve_min_games")]
    pub user_exceptional_reserve_min_games: i8,
    pub opponent_pickup_accuracy: f64,
    pub user_roster_player_keys: Vec<String>,
}

impl Default for FantasySeasonSimConfig {
    fn default() -> Self {
        Self {
            fantasy_teams: 12,
            playoff_teams: 6,
            trials: 100,
            seed: 20_262_027,
            daily_injury_rate: 0.0015,
            min_injury_days: 2,
            max_injury_days: 28,
            weekly_trade_probability: 0.10,
            weekly_pickup_limit: 4,
            user_proactive_pickup_reserve: default_user_pickup_reserve(),
            user_exceptional_reserve_enabled: default_true(),
            user_exceptional_reserve_min_value: default_exceptional_reserve_min_value(),
            user_exceptional_reserve_min_games: default_exceptional_reserve_min_games(),
            opponent_pickup_accuracy: 1.0,
            user_roster_player_keys: Vec::new(),
        }
    }
}

const fn default_user_pickup_reserve() -> u8 {
    1
}

const fn default_true() -> bool {
    true
}

const fn default_exceptional_reserve_min_value() -> f64 {
    6.0
}

const fn default_exceptional_reserve_min_games() -> i8 {
    3
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySeasonSimTeamRow {
    pub rank: usize,
    pub team: String,
    pub average_points: f64,
    pub average_finish: f64,
    pub first_place_probability: f64,
    pub playoff_probability: f64,
    pub championship_probability: f64,
    pub first_round_exit_probability: f64,
    pub semifinal_exit_probability: f64,
    pub final_exit_probability: f64,
    pub average_wins: f64,
    pub average_losses: f64,
    pub average_ties: f64,
    pub average_adds: f64,
    pub average_trades: f64,
    pub average_injuries: f64,
    pub average_injury_replacements_blocked: f64,
    pub average_injury_starts_lost: f64,
    pub average_roster_churn: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasySeasonEventKind {
    Injury,
    Recovery,
    AddDrop,
    InjuryReplacement,
    Trade,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySeasonEventRow {
    pub date: NaiveDate,
    pub week: usize,
    pub team: String,
    pub kind: FantasySeasonEventKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasySeasonSimView {
    pub schema: String,
    pub season: String,
    pub scoring_scheme: String,
    pub start_date: NaiveDate,
    pub end_date: NaiveDate,
    pub regular_season_weeks: usize,
    pub playoff_rounds: usize,
    pub locked_user_roster: Vec<String>,
    pub config: FantasySeasonSimConfig,
    pub teams: Vec<FantasySeasonSimTeamRow>,
    pub sample_events: Vec<FantasySeasonEventRow>,
    pub assumptions: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Clone)]
struct SimPlayer {
    input: FantasySeasonSimPlayerInput,
    dates: BTreeSet<NaiveDate>,
}

#[derive(Default, Clone)]
struct TrialTeam {
    roster: Vec<usize>,
    weekly_points: Vec<f64>,
    adds: u32,
    trades: u32,
    injuries: u32,
    injury_replacements_blocked: u32,
    injury_starts_lost: u32,
    churned: BTreeSet<usize>,
}

#[derive(Default)]
struct AggregateTeam {
    points: f64,
    finish_sum: u64,
    first_places: u32,
    playoffs: u32,
    titles: u32,
    first_round_exits: u32,
    semifinal_exits: u32,
    final_exits: u32,
    wins: u32,
    losses: u32,
    ties: u32,
    adds: u32,
    trades: u32,
    injuries: u32,
    injury_replacements_blocked: u32,
    injury_starts_lost: u32,
    churn: u32,
}

struct PlayoffOutcome {
    champion: usize,
    eliminated: Vec<(usize, usize)>,
}

pub fn simulate_fantasy_season(
    season: impl Into<String>,
    scoring_scheme: impl Into<String>,
    rules: FantasyAssistantRules,
    players: Vec<FantasySeasonSimPlayerInput>,
    mut config: FantasySeasonSimConfig,
) -> Result<FantasySeasonSimView, String> {
    rules.validate()?;
    config.fantasy_teams = config.fantasy_teams.max(2);
    config.playoff_teams = config.playoff_teams.clamp(1, config.fantasy_teams);
    config.trials = config.trials.max(1);
    config.weekly_pickup_limit = config
        .weekly_pickup_limit
        .min(rules.weekly_acquisition_limit);
    config.user_proactive_pickup_reserve = config
        .user_proactive_pickup_reserve
        .min(config.weekly_pickup_limit);
    if !config.user_exceptional_reserve_min_value.is_finite()
        || config.user_exceptional_reserve_min_value < 0.0
        || config.user_exceptional_reserve_min_games < 0
    {
        return Err("exceptional reserve thresholds must be finite and non-negative".to_owned());
    }
    config.daily_injury_rate = config.daily_injury_rate.clamp(0.0, 1.0);
    config.weekly_trade_probability = config.weekly_trade_probability.clamp(0.0, 1.0);
    config.opponent_pickup_accuracy = config.opponent_pickup_accuracy.clamp(0.0, 1.0);
    if config.min_injury_days == 0 || config.max_injury_days < config.min_injury_days {
        return Err("injury duration bounds are invalid".to_owned());
    }

    let mut players = players
        .into_iter()
        .filter(|player| {
            player.fantasy_points_per_game.is_finite()
                && player.fantasy_points_per_game >= 0.0
                && !player.positions.is_empty()
                && !player.game_dates.is_empty()
        })
        .map(|input| SimPlayer {
            dates: input.game_dates.iter().copied().collect(),
            input,
        })
        .collect::<Vec<_>>();
    players.sort_by(|a, b| {
        b.input
            .fantasy_points_per_game
            .total_cmp(&a.input.fantasy_points_per_game)
            .then_with(|| a.input.player_key.cmp(&b.input.player_key))
    });
    let required = config.fantasy_teams * rules.standard_roster_capacity();
    if players.len() < required {
        return Err(format!(
            "season simulation needs at least {required} eligible players; found {}",
            players.len()
        ));
    }
    let start_date = players
        .iter()
        .flat_map(|player| player.dates.iter().copied())
        .min()
        .ok_or_else(|| "season schedule is empty".to_owned())?;
    let end_date = players
        .iter()
        .flat_map(|player| player.dates.iter().copied())
        .max()
        .ok_or_else(|| "season schedule is empty".to_owned())?;
    let season_monday =
        start_date - Duration::days(i64::from(start_date.weekday().num_days_from_monday()));
    let total_weeks = ((end_date - season_monday).num_days() as usize / 7) + 1;
    let playoff_rounds = playoff_round_count(config.playoff_teams);
    if playoff_rounds >= total_weeks {
        return Err(format!(
            "season has {total_weeks} fantasy weeks but needs {playoff_rounds} playoff rounds"
        ));
    }
    let regular_season_weeks = total_weeks - playoff_rounds;
    let active_slots = rules
        .expanded_active_slots()
        .into_iter()
        .map(|slot| slot.kind)
        .collect::<Vec<_>>();
    let locked_keys = config
        .user_roster_player_keys
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    if locked_keys.len() > rules.standard_roster_capacity() {
        return Err(format!(
            "user roster has {} locked players but capacity is {}",
            locked_keys.len(),
            rules.standard_roster_capacity()
        ));
    }
    let player_by_key = players
        .iter()
        .enumerate()
        .map(|(index, player)| (player.input.player_key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let missing_locks = locked_keys
        .iter()
        .filter(|key| !player_by_key.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_locks.is_empty() {
        return Err(format!(
            "{} locked user-roster player(s) are absent from the simulation pool: {}",
            missing_locks.len(),
            missing_locks.join(", ")
        ));
    }
    let locked_indices = locked_keys
        .iter()
        .map(|key| player_by_key[key])
        .collect::<BTreeSet<_>>();
    let full_user_roster = if locked_indices.len() == rules.standard_roster_capacity() {
        let user_roster = locked_indices.iter().copied().collect::<Vec<_>>();
        if !roster_covers_active_slots(&user_roster, &active_slots, &players) {
            return Err(
                "the complete locked user roster cannot fill every configured active slot"
                    .to_owned(),
            );
        }
        Some((
            user_roster,
            synthetic_draft_excluding(&players, &rules, config.fantasy_teams - 1, &locked_indices)?,
        ))
    } else {
        None
    };
    let initial_rosters = if full_user_roster.is_none() {
        Some(synthetic_draft(&players, &rules, config.fantasy_teams)?)
    } else {
        None
    };
    let mut aggregates = (0..config.fantasy_teams)
        .map(|_| AggregateTeam::default())
        .collect::<Vec<_>>();
    let mut sample_events = Vec::new();

    for trial in 0..config.trials {
        let trial_seed = config.seed ^ ((trial as u64 + 1) * 0x9E37_79B9);
        let mut performance_rng = SimRng::new(trial_seed ^ 0x5C0E_0004);
        let trial_rosters = if let Some((user_roster, opponent_rosters)) = &full_user_roster {
            let mut rosters = vec![user_roster.clone()];
            rosters.extend((0..opponent_rosters.len()).map(|index| {
                opponent_rosters[(index + trial % opponent_rosters.len()) % opponent_rosters.len()]
                    .clone()
            }));
            rosters
        } else {
            let initial_rosters = initial_rosters
                .as_ref()
                .expect("partial-roster trials require a synthetic draft");
            let mut rosters = (0..config.fantasy_teams)
                .map(|team_index| {
                    initial_rosters
                        [(team_index + trial % config.fantasy_teams) % config.fantasy_teams]
                        .clone()
                })
                .collect::<Vec<_>>();
            apply_user_roster_locks(&mut rosters, &locked_indices, &active_slots, &players)?;
            rosters
        };
        let drafted = trial_rosters
            .iter()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut free_agents = (0..players.len())
            .filter(|index| !drafted.contains(index))
            .collect::<BTreeSet<_>>();
        let mut teams = trial_rosters
            .into_iter()
            .map(|roster| TrialTeam {
                roster,
                weekly_points: vec![0.0; total_weeks],
                ..TrialTeam::default()
            })
            .collect::<Vec<_>>();
        let mut injured_until = BTreeMap::<usize, NaiveDate>::new();
        let mut injury_replacements = BTreeMap::<usize, usize>::new();
        let mut waiver_until = BTreeMap::<usize, NaiveDate>::new();
        let mut weekly_adds = vec![0u8; config.fantasy_teams];
        let mut date = start_date;
        let mut previous_week = usize::MAX;
        while date <= end_date {
            let week = ((date - season_monday).num_days().max(0) / 7) as usize + 1;
            let seven_day_values = players
                .iter()
                .map(|player| seven_day_value(player, date))
                .collect::<Vec<_>>();
            clear_expired_waivers(date, &mut free_agents, &mut waiver_until);
            let recovered = injured_until
                .iter()
                .filter(|(_, until)| **until <= date)
                .map(|(player, _)| *player)
                .collect::<Vec<_>>();
            for player_index in recovered {
                injured_until.remove(&player_index);
                if let Some((team_index, _)) = teams
                    .iter()
                    .enumerate()
                    .find(|(_, team)| team.roster.contains(&player_index))
                {
                    if let Some(replacement) = release_injury_replacement(
                        player_index,
                        &mut teams[team_index].roster,
                        &mut free_agents,
                        &mut waiver_until,
                        &mut injury_replacements,
                        date,
                        rules.waiver_days,
                    ) {
                        if trial == 0 {
                            sample_events.push(event(
                                date,
                                week,
                                team_index,
                                FantasySeasonEventKind::Recovery,
                                format!(
                                    "{} returned; released replacement {}",
                                    players[player_index].input.player,
                                    players[replacement].input.player
                                ),
                            ));
                        }
                    }
                }
            }
            let is_new_week = week != previous_week;
            if is_new_week {
                weekly_adds.fill(0);
                previous_week = week;
            }
            let daily_priority =
                (trial + (date - start_date).num_days().max(0) as usize) % config.fantasy_teams;
            for offset in 0..config.fantasy_teams {
                let team_index = (daily_priority + offset) % config.fantasy_teams;
                let mut pickup_rng = SimRng::new(keyed_event_seed(
                    trial_seed,
                    0xA11C_E001 ^ date.num_days_from_ce() as u64,
                    week as u64,
                    team_index as u64,
                ));
                let selection_rank = if team_index == 0
                    || config.opponent_pickup_accuracy >= 1.0
                    || pickup_rng.chance(config.opponent_pickup_accuracy)
                {
                    0
                } else {
                    1 + pickup_rng.range(2) as usize
                };
                make_weekly_pickup_with_values(
                    team_index,
                    date,
                    week,
                    &players,
                    &seven_day_values,
                    &mut teams,
                    &mut free_agents,
                    &mut waiver_until,
                    rules.waiver_days,
                    &injured_until,
                    &mut injury_replacements,
                    &mut weekly_adds,
                    config.weekly_pickup_limit,
                    config.user_proactive_pickup_reserve,
                    config.user_exceptional_reserve_enabled,
                    config.user_exceptional_reserve_min_value,
                    config.user_exceptional_reserve_min_games,
                    &active_slots,
                    selection_rank,
                    trial == 0,
                    &mut sample_events,
                );
            }
            if is_new_week {
                let priority_start = (trial + week - 1) % config.fantasy_teams;
                for offset in 0..config.fantasy_teams {
                    let team_index = (priority_start + offset) % config.fantasy_teams;
                    let mut trade_rng = SimRng::new(keyed_event_seed(
                        trial_seed,
                        0x7ADE_0002,
                        week as u64,
                        team_index as u64,
                    ));
                    if trade_rng.chance(config.weekly_trade_probability) {
                        make_trade(
                            team_index,
                            date,
                            week,
                            &players,
                            &active_slots,
                            &mut teams,
                            &injured_until,
                            &mut injury_replacements,
                            &mut trade_rng,
                            trial == 0,
                            &mut sample_events,
                        );
                    }
                }
            }

            for offset in 0..config.fantasy_teams {
                let team_index = (daily_priority + offset) % config.fantasy_teams;
                let roster_snapshot = teams[team_index].roster.clone();
                for player_index in roster_snapshot {
                    let mut injury_rng = SimRng::new(keyed_event_seed(
                        trial_seed,
                        0x1A17_0003 ^ date.num_days_from_ce() as u64,
                        team_index as u64,
                        player_index as u64,
                    ));
                    if injured_until.contains_key(&player_index)
                        || !players[player_index].dates.contains(&date)
                        || !injury_rng.chance(config.daily_injury_rate)
                    {
                        continue;
                    }
                    let span = u32::from(config.max_injury_days - config.min_injury_days + 1);
                    let days = u32::from(config.min_injury_days) + injury_rng.range(span);
                    let until = date + Duration::days(i64::from(days));
                    injured_until.insert(player_index, until);
                    teams[team_index].injuries += 1;
                    if trial == 0 {
                        sample_events.push(event(
                            date,
                            week,
                            team_index,
                            FantasySeasonEventKind::Injury,
                            format!(
                                "{} injured for {} days",
                                players[player_index].input.player, days
                            ),
                        ));
                    }
                    let has_ir_room = injury_replacements
                        .keys()
                        .filter(|injured| teams[team_index].roster.contains(injured))
                        .count()
                        < usize::from(rules.ir_slots + rules.ir_plus_slots);
                    if days >= 7 && has_ir_room {
                        if weekly_adds[team_index] >= config.weekly_pickup_limit {
                            teams[team_index].injury_replacements_blocked += 1;
                        } else if let Some(replacement) = best_replacement(
                            &players[player_index],
                            &players,
                            &seven_day_values,
                            &free_agents,
                        ) {
                            teams[team_index].roster.push(replacement);
                            teams[team_index].adds += 1;
                            teams[team_index].churned.insert(replacement);
                            weekly_adds[team_index] += 1;
                            free_agents.remove(&replacement);
                            injury_replacements.insert(player_index, replacement);
                            if trial == 0 {
                                sample_events.push(event(
                                    date,
                                    week,
                                    team_index,
                                    FantasySeasonEventKind::InjuryReplacement,
                                    format!(
                                        "added {} for injured {}",
                                        players[replacement].input.player,
                                        players[player_index].input.player
                                    ),
                                ));
                            }
                        }
                    }
                }

                teams[team_index].injury_starts_lost += teams[team_index]
                    .roster
                    .iter()
                    .filter(|index| {
                        injured_until.contains_key(index) && players[**index].dates.contains(&date)
                    })
                    .count() as u32;
                let variance = 0.75 + performance_rng.unit() * 0.50;
                let daily_points = optimized_daily_value(
                    &teams[team_index].roster,
                    date,
                    &players,
                    &injured_until,
                    &active_slots,
                ) * variance;
                teams[team_index].weekly_points[week - 1] += daily_points;
            }
            date += Duration::days(1);
        }

        let mut records = vec![(0u32, 0u32, 0u32); config.fantasy_teams];
        for week in 0..regular_season_weeks {
            for (a, b) in fantasy_week_matchups(config.fantasy_teams, week) {
                let a_points = teams[a].weekly_points[week];
                let b_points = teams[b].weekly_points[week];
                if a_points > b_points {
                    records[a].0 += 1;
                    records[b].1 += 1;
                } else if b_points > a_points {
                    records[b].0 += 1;
                    records[a].1 += 1;
                } else {
                    records[a].2 += 1;
                    records[b].2 += 1;
                }
            }
        }
        let mut standings = (0..config.fantasy_teams).collect::<Vec<_>>();
        standings.sort_by(|a, b| {
            records[*b]
                .0
                .cmp(&records[*a].0)
                .then_with(|| records[*b].2.cmp(&records[*a].2))
                .then_with(|| {
                    regular_season_points(&teams[*b], regular_season_weeks)
                        .total_cmp(&regular_season_points(&teams[*a], regular_season_weeks))
                })
                .then_with(|| a.cmp(b))
        });
        let playoff = simulate_playoff_bracket(
            &standings[..config.playoff_teams],
            &teams,
            regular_season_weeks,
        );
        for (place, team_index) in standings.into_iter().enumerate() {
            let team = &teams[team_index];
            let aggregate = &mut aggregates[team_index];
            aggregate.points += regular_season_points(team, regular_season_weeks);
            aggregate.finish_sum += (place + 1) as u64;
            if place == 0 {
                aggregate.first_places += 1;
            }
            aggregate.adds += team.adds;
            aggregate.trades += team.trades;
            aggregate.injuries += team.injuries;
            aggregate.injury_replacements_blocked += team.injury_replacements_blocked;
            aggregate.injury_starts_lost += team.injury_starts_lost;
            aggregate.churn += team.churned.len() as u32;
            if place < config.playoff_teams {
                aggregate.playoffs += 1;
            }
            if team_index == playoff.champion {
                aggregate.titles += 1;
            }
            if let Some((round, _)) = playoff
                .eliminated
                .iter()
                .find(|(_, eliminated_team)| *eliminated_team == team_index)
            {
                match *round {
                    0 => aggregate.first_round_exits += 1,
                    1 => aggregate.semifinal_exits += 1,
                    _ => aggregate.final_exits += 1,
                }
            }
            aggregate.wins += records[team_index].0;
            aggregate.losses += records[team_index].1;
            aggregate.ties += records[team_index].2;
        }
    }

    sample_events.sort_by(|a, b| a.date.cmp(&b.date).then_with(|| a.team.cmp(&b.team)));
    sample_events.truncate(1_000);
    let trials = config.trials as f64;
    let mut rows = aggregates
        .into_iter()
        .enumerate()
        .map(|(index, aggregate)| FantasySeasonSimTeamRow {
            rank: 0,
            team: team_name(index),
            average_points: aggregate.points / trials,
            average_finish: aggregate.finish_sum as f64 / trials,
            first_place_probability: f64::from(aggregate.first_places) / trials,
            playoff_probability: f64::from(aggregate.playoffs) / trials,
            championship_probability: f64::from(aggregate.titles) / trials,
            first_round_exit_probability: f64::from(aggregate.first_round_exits) / trials,
            semifinal_exit_probability: f64::from(aggregate.semifinal_exits) / trials,
            final_exit_probability: f64::from(aggregate.final_exits) / trials,
            average_wins: f64::from(aggregate.wins) / trials,
            average_losses: f64::from(aggregate.losses) / trials,
            average_ties: f64::from(aggregate.ties) / trials,
            average_adds: f64::from(aggregate.adds) / trials,
            average_trades: f64::from(aggregate.trades) / trials,
            average_injuries: f64::from(aggregate.injuries) / trials,
            average_injury_replacements_blocked: f64::from(aggregate.injury_replacements_blocked)
                / trials,
            average_injury_starts_lost: f64::from(aggregate.injury_starts_lost) / trials,
            average_roster_churn: f64::from(aggregate.churn) / trials,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        a.average_finish
            .total_cmp(&b.average_finish)
            .then_with(|| b.average_wins.total_cmp(&a.average_wins))
            .then_with(|| b.average_points.total_cmp(&a.average_points))
            .then_with(|| a.team.cmp(&b.team))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    Ok(FantasySeasonSimView {
        schema: FANTASY_SEASON_SIM_SCHEMA.to_owned(),
        season: season.into(),
        scoring_scheme: scoring_scheme.into(),
        start_date,
        end_date,
        regular_season_weeks,
        playoff_rounds,
        locked_user_roster: locked_indices
            .iter()
            .map(|index| players[*index].input.player.clone())
            .collect(),
        config: config.clone(),
        teams: rows,
        sample_events,
        assumptions: vec![
            "synthetic snake draft uses the configured 16-player standard roster shape"
                .to_owned(),
            format!(
                "{} imported user-roster player(s) are locked to team one before remaining legal slots are filled",
                locked_indices.len()
            ),
            "synthetic team identities rotate through every draft seat across trials to remove fixed draft-order bias"
                .to_owned(),
            "weekly transaction and daily injury-replacement priority rotates across teams to remove fixed processing-order bias"
                .to_owned(),
            "pickup decisions, trades, injuries, and scoring variance use independent seeded random streams so scenario deltas do not rewrite unrelated luck"
                .to_owned(),
            "daily lineups use an exact slot-assignment optimizer over 2026-27 NHL game dates and platform position eligibility"
                .to_owned(),
            "injuries last 2-28 days by default; long injuries may consume IR/IR+ capacity and a weekly add"
                .to_owned(),
            "injured IR players are protected from synthetic drops/trades; replacement ownership follows later pickups or trades so recovery releases the current substitute"
                .to_owned(),
            "daily recoveries and replacement releases are processed before that date's weekly pickup and trade window"
                .to_owned(),
            format!(
                "dropped players and released injury substitutes remain unavailable for the configured {}-day waiver period",
                rules.waiver_days
            ),
            "weekly pickups rank complete legal add/drop pairs by seven-day schedule value; trades require near-fair value and leave both teams able to fill every active slot"
                .to_owned(),
            format!(
                "pickup value charges a {:.0}-game retention cost when the drop has a higher league-scored per-game rate, protecting elite quiet-week players",
                PICKUP_RETENTION_HORIZON_GAMES
            ),
            format!(
                "team one reserves {} of {} weekly acquisition(s) from proactive streaming through Friday for injury replacements, then releases the reserve Saturday; opponents do not reserve moves",
                config.user_proactive_pickup_reserve, config.weekly_pickup_limit
            ),
            format!(
                "exceptional reserve override is {} at a minimum {:.1} move score and +{} seven-day games; a currently injured roster disables it",
                if config.user_exceptional_reserve_enabled { "enabled" } else { "disabled" },
                config.user_exceptional_reserve_min_value,
                config.user_exceptional_reserve_min_games
            ),
            format!(
                "team one always selects the best projected weekly add; opponent pickup accuracy is {:.0}% and misses choose the second- or third-ranked add",
                config.opponent_pickup_accuracy * 100.0
            ),
            "game output varies by ±25% around completed-season league-scored points per game"
                .to_owned(),
            "regular-season seeds come from rotating weekly head-to-head matchups, with fantasy points as the standings tiebreaker"
                .to_owned(),
            format!(
                "the top {} teams enter a {}-round seeded head-to-head bracket using the final {} fantasy weeks",
                config.playoff_teams, playoff_rounds, playoff_rounds
            ),
        ],
        warnings: vec![
            "simulation is a stress model, not a calibrated player, injury, or championship forecast"
                .to_owned(),
            "trade consent, waiver priority, goalie start confirmation, and real platform locks are simplified"
                .to_owned(),
        ],
    })
}

fn playoff_round_count(playoff_teams: usize) -> usize {
    if playoff_teams <= 1 {
        0
    } else {
        usize::BITS as usize - (playoff_teams - 1).leading_zeros() as usize
    }
}

fn fantasy_week_matchups(team_count: usize, week: usize) -> Vec<(usize, usize)> {
    let bracket_count = if team_count.is_multiple_of(2) {
        team_count
    } else {
        team_count + 1
    };
    let bye = team_count;
    let mut rotation = (0..bracket_count).collect::<Vec<_>>();
    for _ in 0..(week % (bracket_count - 1)) {
        rotation[1..].rotate_right(1);
    }
    (0..bracket_count / 2)
        .filter_map(|index| {
            let a = rotation[index];
            let b = rotation[bracket_count - 1 - index];
            (a != bye && b != bye).then_some((a, b))
        })
        .collect()
}

fn regular_season_points(team: &TrialTeam, regular_season_weeks: usize) -> f64 {
    team.weekly_points[..regular_season_weeks].iter().sum()
}

fn simulate_playoff_bracket(
    seeds: &[usize],
    teams: &[TrialTeam],
    first_playoff_week: usize,
) -> PlayoffOutcome {
    if seeds.len() == 1 {
        return PlayoffOutcome {
            champion: seeds[0],
            eliminated: Vec::new(),
        };
    }
    let seed_rank = seeds
        .iter()
        .copied()
        .enumerate()
        .map(|(rank, team)| (team, rank))
        .collect::<BTreeMap<_, _>>();
    let bracket_size = seeds.len().next_power_of_two();
    let byes = bracket_size - seeds.len();
    let mut current = seeds[..byes].to_vec();
    let (first_round_winners, first_round_losers) =
        playoff_round_results(&seeds[byes..], teams, first_playoff_week, &seed_rank);
    current.extend(first_round_winners);
    current.sort_by_key(|team| seed_rank[team]);
    let mut eliminated = first_round_losers
        .into_iter()
        .map(|team| (0, team))
        .collect::<Vec<_>>();

    let mut week = first_playoff_week + 1;
    let mut round = 1;
    while current.len() > 1 {
        let (winners, losers) = playoff_round_results(&current, teams, week, &seed_rank);
        eliminated.extend(losers.into_iter().map(|team| (round, team)));
        current = winners;
        current.sort_by_key(|team| seed_rank[team]);
        week += 1;
        round += 1;
    }
    PlayoffOutcome {
        champion: current[0],
        eliminated,
    }
}

fn playoff_round_results(
    seeded_teams: &[usize],
    teams: &[TrialTeam],
    week: usize,
    seed_rank: &BTreeMap<usize, usize>,
) -> (Vec<usize>, Vec<usize>) {
    let mut winners = Vec::with_capacity(seeded_teams.len() / 2);
    let mut losers = Vec::with_capacity(seeded_teams.len() / 2);
    for index in 0..seeded_teams.len() / 2 {
        let higher_seed = seeded_teams[index];
        let lower_seed = seeded_teams[seeded_teams.len() - 1 - index];
        let higher_score = teams[higher_seed].weekly_points[week];
        let lower_score = teams[lower_seed].weekly_points[week];
        let (winner, loser) = if higher_score > lower_score
            || (higher_score == lower_score && seed_rank[&higher_seed] < seed_rank[&lower_seed])
        {
            (higher_seed, lower_seed)
        } else {
            (lower_seed, higher_seed)
        };
        winners.push(winner);
        losers.push(loser);
    }
    (winners, losers)
}

fn synthetic_draft(
    players: &[SimPlayer],
    rules: &FantasyAssistantRules,
    teams: usize,
) -> Result<Vec<Vec<usize>>, String> {
    synthetic_draft_excluding(players, rules, teams, &BTreeSet::new())
}

fn synthetic_draft_excluding(
    players: &[SimPlayer],
    rules: &FantasyAssistantRules,
    teams: usize,
    excluded: &BTreeSet<usize>,
) -> Result<Vec<Vec<usize>>, String> {
    let active_slots = rules
        .expanded_active_slots()
        .into_iter()
        .map(|slot| slot.kind)
        .collect::<Vec<_>>();
    let mut targets = Vec::new();
    for slot in active_slots.iter().copied() {
        targets.push(Some(slot));
    }
    targets.extend((0..rules.bench_slots).map(|_| None));
    let mut rosters = vec![Vec::new(); teams];
    let mut available = (0..players.len())
        .filter(|index| !excluded.contains(index))
        .collect::<BTreeSet<_>>();
    for (round, target) in targets.into_iter().enumerate() {
        let order = if round % 2 == 0 {
            (0..teams).collect::<Vec<_>>()
        } else {
            (0..teams).rev().collect::<Vec<_>>()
        };
        for team in order {
            let selected = available
                .iter()
                .copied()
                .find(|index| {
                    target.is_none_or(|slot| slot_accepts(slot, &players[*index].input.positions))
                })
                .or_else(|| available.iter().next().copied())
                .ok_or_else(|| "draft pool exhausted".to_owned())?;
            rosters[team].push(selected);
            available.remove(&selected);
        }
    }
    if rosters
        .iter()
        .any(|roster| !roster_covers_active_slots(roster, &active_slots, players))
    {
        return Err(
            "synthetic draft pool cannot produce a legal active roster for every team".to_owned(),
        );
    }
    Ok(rosters)
}

fn apply_user_roster_locks(
    rosters: &mut [Vec<usize>],
    locked: &BTreeSet<usize>,
    active_slots: &[FantasyActiveSlotKind],
    players: &[SimPlayer],
) -> Result<(), String> {
    for locked_player in locked.iter().copied() {
        if rosters[0].contains(&locked_player) {
            continue;
        }
        let source_team = rosters
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, roster)| roster.contains(&locked_player))
            .map(|(team, _)| team);
        let mut candidates = rosters[0]
            .iter()
            .copied()
            .filter(|candidate| !locked.contains(candidate))
            .collect::<Vec<_>>();
        candidates.sort_by(|a, b| {
            players[*a]
                .input
                .fantasy_points_per_game
                .total_cmp(&players[*b].input.fantasy_points_per_game)
                .then_with(|| a.cmp(b))
        });
        let replacement = candidates.into_iter().find(|candidate| {
            let mut user_roster = rosters[0].clone();
            replace_player(&mut user_roster, *candidate, locked_player);
            if !roster_covers_active_slots(&user_roster, active_slots, players) {
                return false;
            }
            source_team.is_none_or(|team| {
                let mut opponent_roster = rosters[team].clone();
                replace_player(&mut opponent_roster, locked_player, *candidate);
                roster_covers_active_slots(&opponent_roster, active_slots, players)
            })
        });
        let Some(replacement) = replacement else {
            return Err(format!(
                "could not preserve a legal roster while locking {}",
                players[locked_player].input.player
            ));
        };
        replace_player(&mut rosters[0], replacement, locked_player);
        if let Some(team) = source_team {
            replace_player(&mut rosters[team], locked_player, replacement);
        }
    }
    Ok(())
}

fn replace_player(roster: &mut [usize], outgoing: usize, incoming: usize) {
    if let Some(slot) = roster.iter_mut().find(|player| **player == outgoing) {
        *slot = incoming;
    }
}

fn roster_covers_active_slots(
    roster: &[usize],
    active_slots: &[FantasyActiveSlotKind],
    players: &[SimPlayer],
) -> bool {
    let mut player_slot = vec![None; roster.len()];
    for slot_index in 0..active_slots.len() {
        let mut seen = vec![false; roster.len()];
        if !assign_slot_to_player(
            slot_index,
            roster,
            active_slots,
            players,
            &mut seen,
            &mut player_slot,
        ) {
            return false;
        }
    }
    true
}

fn assign_slot_to_player(
    slot_index: usize,
    roster: &[usize],
    active_slots: &[FantasyActiveSlotKind],
    players: &[SimPlayer],
    seen: &mut [bool],
    player_slot: &mut [Option<usize>],
) -> bool {
    for (roster_index, player_index) in roster.iter().copied().enumerate() {
        if seen[roster_index]
            || !slot_accepts(
                active_slots[slot_index],
                &players[player_index].input.positions,
            )
        {
            continue;
        }
        seen[roster_index] = true;
        if player_slot[roster_index].is_none_or(|assigned_slot| {
            assign_slot_to_player(
                assigned_slot,
                roster,
                active_slots,
                players,
                seen,
                player_slot,
            )
        }) {
            player_slot[roster_index] = Some(slot_index);
            return true;
        }
    }
    false
}

fn slot_accepts(slot: FantasyActiveSlotKind, positions: &[Position]) -> bool {
    match slot {
        FantasyActiveSlotKind::Center => positions.contains(&Position::Center),
        FantasyActiveSlotKind::LeftWing => positions.contains(&Position::LeftWing),
        FantasyActiveSlotKind::RightWing => positions.contains(&Position::RightWing),
        FantasyActiveSlotKind::Defense => positions.contains(&Position::Defense),
        FantasyActiveSlotKind::Goalie => positions.contains(&Position::Goalie),
        FantasyActiveSlotKind::Utility => positions
            .iter()
            .any(|position| *position != Position::Goalie),
    }
}

#[allow(clippy::too_many_arguments)]
fn make_weekly_pickup_with_values(
    team_index: usize,
    date: NaiveDate,
    week: usize,
    players: &[SimPlayer],
    seven_day_values: &[f64],
    teams: &mut [TrialTeam],
    free_agents: &mut BTreeSet<usize>,
    waiver_until: &mut BTreeMap<usize, NaiveDate>,
    waiver_days: u8,
    injured_until: &BTreeMap<usize, NaiveDate>,
    injury_replacements: &mut BTreeMap<usize, usize>,
    weekly_adds: &mut [u8],
    limit: u8,
    user_proactive_reserve: u8,
    exceptional_reserve_enabled: bool,
    exceptional_reserve_min_value: f64,
    exceptional_reserve_min_games: i8,
    active_slots: &[FantasyActiveSlotKind],
    selection_rank: usize,
    record: bool,
    events: &mut Vec<FantasySeasonEventRow>,
) {
    let proactive_limit = proactive_pickup_limit(team_index, date, limit, user_proactive_reserve);
    if weekly_adds[team_index] >= limit {
        return;
    }
    let reserve_only = weekly_adds[team_index] >= proactive_limit;
    if reserve_only
        && (!exceptional_reserve_enabled
            || teams[team_index]
                .roster
                .iter()
                .any(|player| injured_until.contains_key(player)))
    {
        return;
    }
    let mut add_candidates = free_agents.iter().copied().collect::<Vec<_>>();
    add_candidates.sort_by(|a, b| {
        seven_day_values[*b]
            .total_cmp(&seven_day_values[*a])
            .then_with(|| a.cmp(b))
    });
    add_candidates.truncate(12);
    let mut moves = add_candidates
        .into_iter()
        .filter_map(|add| {
            teams[team_index]
                .roster
                .iter()
                .copied()
                .filter(|drop| !injured_until.contains_key(drop))
                .filter(|drop| {
                    let mut roster = teams[team_index].roster.clone();
                    replace_player(&mut roster, *drop, add);
                    let healthy_roster = roster
                        .into_iter()
                        .filter(|player| !injured_until.contains_key(player))
                        .collect::<Vec<_>>();
                    roster_covers_active_slots(&healthy_roster, active_slots, players)
                })
                .filter_map(|drop| {
                    pickup_move_score(add, drop, players, seven_day_values)
                        .map(|score| (drop, score))
                })
                .max_by(|a, b| a.1.total_cmp(&b.1).then_with(|| b.0.cmp(&a.0)))
                .map(|(drop, score)| (add, drop, score))
        })
        .collect::<Vec<_>>();
    moves.sort_by(|a, b| {
        b.2.total_cmp(&a.2)
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });
    if reserve_only {
        moves.retain(|(add, drop, score)| {
            exceptional_sim_reserve_move(
                *add,
                *drop,
                *score,
                date,
                players,
                exceptional_reserve_min_value,
                exceptional_reserve_min_games,
            )
        });
    }
    let Some((add, drop, _)) = moves
        .get(selection_rank.min(moves.len().saturating_sub(1)))
        .copied()
    else {
        return;
    };
    teams[team_index].roster.retain(|index| *index != drop);
    teams[team_index].roster.push(add);
    teams[team_index].adds += 1;
    teams[team_index].churned.insert(add);
    weekly_adds[team_index] += 1;
    free_agents.remove(&add);
    place_on_waivers(drop, date, waiver_days, free_agents, waiver_until);
    for replacement in injury_replacements.values_mut() {
        if *replacement == drop {
            *replacement = add;
        }
    }
    if record {
        events.push(event(
            date,
            week,
            team_index,
            FantasySeasonEventKind::AddDrop,
            format!(
                "added {} and dropped {}",
                players[add].input.player, players[drop].input.player
            ),
        ));
    }
}

fn exceptional_sim_reserve_move(
    add: usize,
    drop: usize,
    score: f64,
    date: NaiveDate,
    players: &[SimPlayer],
    min_value: f64,
    min_games: i8,
) -> bool {
    let end = date + Duration::days(6);
    let add_games = players[add].dates.range(date..=end).count() as i8;
    let drop_games = players[drop].dates.range(date..=end).count() as i8;
    score >= min_value && add_games - drop_games >= min_games
}

fn proactive_pickup_limit(
    team_index: usize,
    date: NaiveDate,
    limit: u8,
    user_proactive_reserve: u8,
) -> u8 {
    let reserve_is_active = date.weekday().num_days_from_monday() < 5;
    if team_index == 0 && reserve_is_active {
        limit.saturating_sub(user_proactive_reserve.min(limit))
    } else {
        limit
    }
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn make_weekly_pickup(
    team_index: usize,
    date: NaiveDate,
    week: usize,
    players: &[SimPlayer],
    teams: &mut [TrialTeam],
    free_agents: &mut BTreeSet<usize>,
    waiver_until: &mut BTreeMap<usize, NaiveDate>,
    waiver_days: u8,
    injured_until: &BTreeMap<usize, NaiveDate>,
    injury_replacements: &mut BTreeMap<usize, usize>,
    weekly_adds: &mut [u8],
    limit: u8,
    active_slots: &[FantasyActiveSlotKind],
    selection_rank: usize,
    record: bool,
    events: &mut Vec<FantasySeasonEventRow>,
) {
    let seven_day_values = players
        .iter()
        .map(|player| seven_day_value(player, date))
        .collect::<Vec<_>>();
    make_weekly_pickup_with_values(
        team_index,
        date,
        week,
        players,
        &seven_day_values,
        teams,
        free_agents,
        waiver_until,
        waiver_days,
        injured_until,
        injury_replacements,
        weekly_adds,
        limit,
        0,
        false,
        default_exceptional_reserve_min_value(),
        default_exceptional_reserve_min_games(),
        active_slots,
        selection_rank,
        record,
        events,
    );
}

fn pickup_move_score(
    add: usize,
    drop: usize,
    players: &[SimPlayer],
    seven_day_values: &[f64],
) -> Option<f64> {
    let add_week = seven_day_values[add];
    let drop_week = seven_day_values[drop];
    let retention_cost = (players[drop].input.fantasy_points_per_game
        - players[add].input.fantasy_points_per_game)
        .max(0.0)
        * PICKUP_RETENTION_HORIZON_GAMES;
    let required_gain = drop_week * 0.10 + 0.25 + retention_cost;
    let weekly_gain = add_week - drop_week;
    (weekly_gain > required_gain).then_some(weekly_gain - retention_cost)
}

fn best_replacement(
    injured: &SimPlayer,
    players: &[SimPlayer],
    seven_day_values: &[f64],
    free_agents: &BTreeSet<usize>,
) -> Option<usize> {
    free_agents
        .iter()
        .copied()
        .filter(|index| {
            players[*index]
                .input
                .positions
                .iter()
                .any(|position| injured.input.positions.contains(position))
        })
        .max_by(|a, b| {
            seven_day_values[*a]
                .total_cmp(&seven_day_values[*b])
                .then_with(|| b.cmp(a))
        })
}

fn release_injury_replacement(
    injured: usize,
    roster: &mut Vec<usize>,
    free_agents: &mut BTreeSet<usize>,
    waiver_until: &mut BTreeMap<usize, NaiveDate>,
    injury_replacements: &mut BTreeMap<usize, usize>,
    date: NaiveDate,
    waiver_days: u8,
) -> Option<usize> {
    let replacement = injury_replacements.remove(&injured)?;
    if !roster.contains(&replacement) {
        return None;
    }
    roster.retain(|player| *player != replacement);
    place_on_waivers(replacement, date, waiver_days, free_agents, waiver_until);
    Some(replacement)
}

fn place_on_waivers(
    player: usize,
    date: NaiveDate,
    waiver_days: u8,
    free_agents: &mut BTreeSet<usize>,
    waiver_until: &mut BTreeMap<usize, NaiveDate>,
) {
    free_agents.remove(&player);
    if waiver_days == 0 {
        free_agents.insert(player);
    } else {
        waiver_until.insert(player, date + Duration::days(i64::from(waiver_days)));
    }
}

fn clear_expired_waivers(
    date: NaiveDate,
    free_agents: &mut BTreeSet<usize>,
    waiver_until: &mut BTreeMap<usize, NaiveDate>,
) {
    let cleared = waiver_until
        .iter()
        .filter(|(_, clears)| **clears <= date)
        .map(|(player, _)| *player)
        .collect::<Vec<_>>();
    for player in cleared {
        waiver_until.remove(&player);
        free_agents.insert(player);
    }
}

#[allow(clippy::too_many_arguments)]
fn make_trade(
    a: usize,
    date: NaiveDate,
    week: usize,
    players: &[SimPlayer],
    active_slots: &[FantasyActiveSlotKind],
    teams: &mut [TrialTeam],
    injured_until: &BTreeMap<usize, NaiveDate>,
    injury_replacements: &mut BTreeMap<usize, usize>,
    rng: &mut SimRng,
    record: bool,
    events: &mut Vec<FantasySeasonEventRow>,
) {
    let mut b = rng.range(teams.len() as u32) as usize;
    if a == b {
        b = (b + 1) % teams.len();
    }
    let before =
        roster_balance(&teams[a].roster, players) + roster_balance(&teams[b].roster, players);
    for _ in 0..40 {
        let pa = teams[a].roster[rng.range(teams[a].roster.len() as u32) as usize];
        let pb = teams[b].roster[rng.range(teams[b].roster.len() as u32) as usize];
        if injured_until.contains_key(&pa) || injured_until.contains_key(&pb) {
            continue;
        }
        let va = players[pa].input.fantasy_points_per_game;
        let vb = players[pb].input.fantasy_points_per_game;
        let fair = (va - vb).abs() <= va.max(vb).max(0.1) * 0.18;
        if !fair || players[pa].input.positions == players[pb].input.positions {
            continue;
        }
        let mut ra = teams[a].roster.clone();
        let mut rb = teams[b].roster.clone();
        ra.retain(|index| *index != pa);
        rb.retain(|index| *index != pb);
        ra.push(pb);
        rb.push(pa);
        if !roster_covers_active_slots(&ra, active_slots, players)
            || !roster_covers_active_slots(&rb, active_slots, players)
        {
            continue;
        }
        let after = roster_balance(&ra, players) + roster_balance(&rb, players);
        if after > before + 0.01 {
            continue;
        }
        teams[a].roster = ra;
        teams[b].roster = rb;
        teams[a].trades += 1;
        teams[b].trades += 1;
        teams[a].churned.insert(pb);
        teams[b].churned.insert(pa);
        swap_injury_replacement_ownership(injury_replacements, pa, pb);
        if record {
            events.push(event(
                date,
                week,
                a,
                FantasySeasonEventKind::Trade,
                format!(
                    "traded {} to {} for {}",
                    players[pa].input.player,
                    team_name(b),
                    players[pb].input.player
                ),
            ));
            events.push(event(
                date,
                week,
                b,
                FantasySeasonEventKind::Trade,
                format!(
                    "traded {} to {} for {}",
                    players[pb].input.player,
                    team_name(a),
                    players[pa].input.player
                ),
            ));
        }
        break;
    }
}

fn optimized_daily_value(
    roster: &[usize],
    date: NaiveDate,
    players: &[SimPlayer],
    injured_until: &BTreeMap<usize, NaiveDate>,
    slots: &[FantasyActiveSlotKind],
) -> f64 {
    let mut candidates = roster
        .iter()
        .copied()
        .filter(|index| !injured_until.contains_key(index) && players[*index].dates.contains(&date))
        .collect::<Vec<_>>();
    candidates.sort_by(|a, b| {
        players[*b]
            .input
            .fantasy_points_per_game
            .total_cmp(&players[*a].input.fantasy_points_per_game)
            .then_with(|| a.cmp(b))
    });
    let mut selected = Vec::with_capacity(slots.len());
    for candidate in candidates {
        selected.push(candidate);
        if !can_assign_all(&selected, slots, players) {
            selected.pop();
        }
    }
    selected
        .into_iter()
        .map(|index| players[index].input.fantasy_points_per_game)
        .sum()
}

// Matchable player sets form a transversal matroid, so descending-value greedy
// plus an augmenting-path feasibility check yields the exact maximum-weight lineup.
fn can_assign_all(
    selected: &[usize],
    slots: &[FantasyActiveSlotKind],
    players: &[SimPlayer],
) -> bool {
    if selected.len() > slots.len() {
        return false;
    }
    let mut slot_owner = vec![None; slots.len()];
    for player_index in selected.iter().copied() {
        let mut seen = vec![false; slots.len()];
        if !assign_player(player_index, slots, players, &mut seen, &mut slot_owner) {
            return false;
        }
    }
    true
}

fn assign_player(
    player_index: usize,
    slots: &[FantasyActiveSlotKind],
    players: &[SimPlayer],
    seen: &mut [bool],
    slot_owner: &mut [Option<usize>],
) -> bool {
    for (slot_index, slot) in slots.iter().copied().enumerate() {
        if seen[slot_index] || !slot_accepts(slot, &players[player_index].input.positions) {
            continue;
        }
        seen[slot_index] = true;
        if slot_owner[slot_index]
            .is_none_or(|owner| assign_player(owner, slots, players, seen, slot_owner))
        {
            slot_owner[slot_index] = Some(player_index);
            return true;
        }
    }
    false
}

fn swap_injury_replacement_ownership(
    injury_replacements: &mut BTreeMap<usize, usize>,
    first: usize,
    second: usize,
) {
    for replacement in injury_replacements.values_mut() {
        if *replacement == first {
            *replacement = second;
        } else if *replacement == second {
            *replacement = first;
        }
    }
}

fn roster_balance(roster: &[usize], players: &[SimPlayer]) -> f64 {
    let mut counts = BTreeMap::<&'static str, usize>::new();
    for index in roster {
        for position in &players[*index].input.positions {
            *counts.entry(position.abbreviation()).or_default() += 1;
        }
    }
    [("C", 2usize), ("LW", 2), ("RW", 2), ("D", 3), ("G", 2)]
        .into_iter()
        .map(|(position, target)| target.saturating_sub(*counts.get(position).unwrap_or(&0)) as f64)
        .sum()
}

fn seven_day_value(player: &SimPlayer, date: NaiveDate) -> f64 {
    let end = date + Duration::days(6);
    let games = player.dates.range(date..=end).count() as f64;
    player.input.fantasy_points_per_game * games
}

fn event(
    date: NaiveDate,
    week: usize,
    team: usize,
    kind: FantasySeasonEventKind,
    message: String,
) -> FantasySeasonEventRow {
    FantasySeasonEventRow {
        date,
        week,
        team: team_name(team),
        kind,
        message,
    }
}

fn team_name(index: usize) -> String {
    if index == 0 {
        "Gio Simulation".to_owned()
    } else {
        format!("Synthetic Team {:02}", index + 1)
    }
}

fn keyed_event_seed(trial_seed: u64, domain: u64, first: u64, second: u64) -> u64 {
    trial_seed
        ^ domain.rotate_left(11)
        ^ first.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ second.wrapping_mul(0xBF58_476D_1CE4_E5B9)
}

struct SimRng(u64);

impl SimRng {
    fn new(seed: u64) -> Self {
        Self(seed.max(1))
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn unit(&mut self) -> f64 {
        self.next() as f64 / u64::MAX as f64
    }

    fn chance(&mut self, probability: f64) -> bool {
        self.unit() < probability
    }

    fn range(&mut self, upper: u32) -> u32 {
        if upper == 0 {
            0
        } else {
            (self.next() % u64::from(upper)) as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn season_sim_is_deterministic_and_includes_roster_change_events() {
        let rules = FantasyAssistantRules::configured_2026();
        let dates = (0..28)
            .map(|offset| NaiveDate::from_ymd_opt(2026, 10, 1).unwrap() + Duration::days(offset))
            .collect::<Vec<_>>();
        let positions = [
            Position::Center,
            Position::LeftWing,
            Position::RightWing,
            Position::Defense,
            Position::Goalie,
        ];
        let players = (0..80)
            .map(|index| FantasySeasonSimPlayerInput {
                player_key: format!("p{index}"),
                player: format!("Player {index}"),
                nhl_team: "NYR".to_owned(),
                positions: vec![positions[index % positions.len()]],
                fantasy_points_per_game: 1.0 + index as f64 / 50.0,
                game_dates: dates
                    .iter()
                    .copied()
                    .filter(|date| (date.ordinal() as usize + index).is_multiple_of(2))
                    .collect(),
            })
            .collect::<Vec<_>>();
        let config = FantasySeasonSimConfig {
            fantasy_teams: 4,
            playoff_teams: 2,
            trials: 3,
            seed: 7,
            daily_injury_rate: 1.0,
            weekly_trade_probability: 1.0,
            user_roster_player_keys: vec!["p79".to_owned()],
            ..FantasySeasonSimConfig::default()
        };
        let first = simulate_fantasy_season(
            "20262027",
            "test",
            rules.clone(),
            players.clone(),
            config.clone(),
        )
        .unwrap();
        let second = simulate_fantasy_season(
            "20262027",
            "test",
            rules.clone(),
            players.clone(),
            config.clone(),
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.teams.len(), 4);
        assert_eq!(first.locked_user_roster, vec!["Player 79"]);
        assert!(
            (first
                .teams
                .iter()
                .map(|team| team.first_place_probability)
                .sum::<f64>()
                - 1.0)
                .abs()
                < f64::EPSILON
        );
        for team in &first.teams {
            assert!((1.0..=4.0).contains(&team.average_finish));
            let postseason_paths = team.championship_probability
                + team.first_round_exit_probability
                + team.semifinal_exit_probability
                + team.final_exit_probability;
            assert!((team.playoff_probability - postseason_paths).abs() < f64::EPSILON);
        }
        assert!(first.teams.iter().any(|team| team.average_injuries > 0.0));
        assert!(first
            .teams
            .iter()
            .any(|team| team.average_injury_replacements_blocked > 0.0));
        assert!(first
            .sample_events
            .iter()
            .any(|event| event.kind == FantasySeasonEventKind::Injury));
        assert!(first
            .sample_events
            .iter()
            .any(|event| event.kind == FantasySeasonEventKind::Trade));

        let mut perfect_opponents = config.clone();
        perfect_opponents.weekly_pickup_limit = 0;
        perfect_opponents.opponent_pickup_accuracy = 1.0;
        let mut inaccurate_opponents = perfect_opponents.clone();
        inaccurate_opponents.opponent_pickup_accuracy = 0.0;
        let perfect = simulate_fantasy_season(
            "20262027",
            "test",
            rules.clone(),
            players.clone(),
            perfect_opponents,
        )
        .unwrap();
        let inaccurate =
            simulate_fantasy_season("20262027", "test", rules, players, inaccurate_opponents)
                .unwrap();
        assert_eq!(perfect.teams, inaccurate.teams);
        assert_eq!(perfect.sample_events, inaccurate.sample_events);
    }

    #[test]
    fn daily_optimizer_preserves_high_value_flexible_player() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let make = |key: &str, value: f64, positions: Vec<Position>| SimPlayer {
            input: FantasySeasonSimPlayerInput {
                player_key: key.to_owned(),
                player: key.to_owned(),
                nhl_team: "NYR".to_owned(),
                positions,
                fantasy_points_per_game: value,
                game_dates: vec![date],
            },
            dates: [date].into_iter().collect(),
        };
        let players = vec![
            make("flex", 10.0, vec![Position::Center, Position::RightWing]),
            make("center", 9.0, vec![Position::Center]),
            make("wing", 8.0, vec![Position::RightWing]),
        ];
        let value = optimized_daily_value(
            &[0, 1, 2],
            date,
            &players,
            &BTreeMap::new(),
            &[
                FantasyActiveSlotKind::Center,
                FantasyActiveSlotKind::RightWing,
            ],
        );
        assert_eq!(value, 19.0);
    }

    #[test]
    fn pickup_selection_rank_models_opponent_misses_without_changing_drop_logic() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let players = [1.0, 2.0, 5.0, 4.0, 3.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: format!("p{index}"),
                    player: format!("Player {index}"),
                    nhl_team: "NYR".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: value,
                    game_dates: vec![date],
                },
                dates: [date].into_iter().collect(),
            })
            .collect::<Vec<_>>();
        let mut teams = vec![TrialTeam {
            roster: vec![0, 1],
            ..TrialTeam::default()
        }];
        let mut free_agents = [2, 3, 4].into_iter().collect::<BTreeSet<_>>();
        let mut weekly_adds = vec![0];
        let mut waiver_until = BTreeMap::new();
        let injured_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::new();

        make_weekly_pickup(
            0,
            date,
            1,
            &players,
            &mut teams,
            &mut free_agents,
            &mut waiver_until,
            2,
            &injured_until,
            &mut injury_replacements,
            &mut weekly_adds,
            4,
            &[FantasyActiveSlotKind::Center],
            1,
            false,
            &mut Vec::new(),
        );

        assert_eq!(teams[0].roster, vec![1, 3]);
        assert_eq!(waiver_until.get(&0), Some(&(date + Duration::days(2))));
        assert!(free_agents.contains(&2));
    }

    #[test]
    fn weekly_pickup_preserves_required_position_coverage() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let make = |key: &str, value: f64, position: Position| SimPlayer {
            input: FantasySeasonSimPlayerInput {
                player_key: key.to_owned(),
                player: key.to_owned(),
                nhl_team: "NYR".to_owned(),
                positions: vec![position],
                fantasy_points_per_game: value,
                game_dates: vec![date],
            },
            dates: [date].into_iter().collect(),
        };
        let players = vec![
            make("goalie", 1.0, Position::Goalie),
            make("center", 2.0, Position::Center),
            make("upgrade", 5.0, Position::Center),
        ];
        let mut teams = vec![TrialTeam {
            roster: vec![0, 1],
            ..TrialTeam::default()
        }];
        let mut free_agents = [2].into_iter().collect::<BTreeSet<_>>();
        let mut weekly_adds = vec![0];
        let mut waiver_until = BTreeMap::new();
        let injured_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::new();

        make_weekly_pickup(
            0,
            date,
            1,
            &players,
            &mut teams,
            &mut free_agents,
            &mut waiver_until,
            2,
            &injured_until,
            &mut injury_replacements,
            &mut weekly_adds,
            4,
            &[FantasyActiveSlotKind::Center, FantasyActiveSlotKind::Goalie],
            0,
            false,
            &mut Vec::new(),
        );

        assert_eq!(teams[0].roster, vec![0, 2]);
        assert!(roster_covers_active_slots(
            &teams[0].roster,
            &[FantasyActiveSlotKind::Center, FantasyActiveSlotKind::Goalie,],
            &players
        ));
    }

    #[test]
    fn quiet_week_does_not_make_elite_player_droppable_for_streamer() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let star_date = date + Duration::days(10);
        let streamer_dates = (0..4)
            .map(|offset| date + Duration::days(offset))
            .collect::<Vec<_>>();
        let players = vec![
            SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: "star".to_owned(),
                    player: "Star".to_owned(),
                    nhl_team: "NYR".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: 5.0,
                    game_dates: vec![star_date],
                },
                dates: [star_date].into_iter().collect(),
            },
            SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: "streamer".to_owned(),
                    player: "Streamer".to_owned(),
                    nhl_team: "BOS".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: 1.0,
                    game_dates: streamer_dates.clone(),
                },
                dates: streamer_dates.into_iter().collect(),
            },
        ];
        let mut teams = vec![TrialTeam {
            roster: vec![0],
            ..TrialTeam::default()
        }];
        let mut free_agents = BTreeSet::from([1]);
        let mut waiver_until = BTreeMap::new();
        let injured_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::new();
        let mut weekly_adds = vec![0];

        make_weekly_pickup(
            0,
            date,
            1,
            &players,
            &mut teams,
            &mut free_agents,
            &mut waiver_until,
            2,
            &injured_until,
            &mut injury_replacements,
            &mut weekly_adds,
            4,
            &[FantasyActiveSlotKind::Center],
            0,
            false,
            &mut Vec::new(),
        );

        assert_eq!(teams[0].roster, vec![0]);
        assert_eq!(weekly_adds[0], 0);
        assert!(free_agents.contains(&1));
    }

    #[test]
    fn comparable_player_with_more_games_remains_a_valid_stream() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let add_dates = (0..4)
            .map(|offset| date + Duration::days(offset))
            .collect::<Vec<_>>();
        let players = vec![
            SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: "drop".to_owned(),
                    player: "Drop".to_owned(),
                    nhl_team: "NYR".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: 1.0,
                    game_dates: vec![date],
                },
                dates: [date].into_iter().collect(),
            },
            SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: "add".to_owned(),
                    player: "Add".to_owned(),
                    nhl_team: "BOS".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: 1.0,
                    game_dates: add_dates.clone(),
                },
                dates: add_dates.into_iter().collect(),
            },
        ];
        let mut teams = vec![TrialTeam {
            roster: vec![0],
            ..TrialTeam::default()
        }];
        let mut free_agents = BTreeSet::from([1]);
        let mut waiver_until = BTreeMap::new();
        let injured_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::new();
        let mut weekly_adds = vec![0];

        make_weekly_pickup(
            0,
            date,
            1,
            &players,
            &mut teams,
            &mut free_agents,
            &mut waiver_until,
            2,
            &injured_until,
            &mut injury_replacements,
            &mut weekly_adds,
            4,
            &[FantasyActiveSlotKind::Center],
            0,
            false,
            &mut Vec::new(),
        );

        assert_eq!(teams[0].roster, vec![1]);
        assert_eq!(weekly_adds[0], 1);
        assert_eq!(waiver_until.get(&0), Some(&(date + Duration::days(2))));
    }

    #[test]
    fn user_reserve_blocks_fourth_proactive_add_but_not_the_shared_hard_limit() {
        let monday = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let saturday = monday + Duration::days(5);
        assert_eq!(proactive_pickup_limit(0, monday, 4, 1), 3);
        assert_eq!(proactive_pickup_limit(1, monday, 4, 1), 4);
        assert_eq!(proactive_pickup_limit(0, monday, 4, 0), 4);
        assert_eq!(proactive_pickup_limit(0, monday, 4, 9), 0);
        assert_eq!(proactive_pickup_limit(0, saturday, 4, 1), 4);

        let weekly_adds = 3;
        assert!(weekly_adds >= proactive_pickup_limit(0, monday, 4, 1));
        assert!(weekly_adds < 4, "the fourth add remains available to IR");
    }

    #[test]
    fn simulated_exceptional_override_requires_value_and_configured_game_gain() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let make = |key: &str, offsets: &[i64]| {
            let game_dates = offsets
                .iter()
                .map(|offset| date + Duration::days(*offset))
                .collect::<Vec<_>>();
            SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: key.to_owned(),
                    player: key.to_owned(),
                    nhl_team: "NYR".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: 1.0,
                    game_dates: game_dates.clone(),
                },
                dates: game_dates.into_iter().collect(),
            }
        };
        let players = vec![make("drop", &[0]), make("add", &[0, 2, 4])];

        assert!(exceptional_sim_reserve_move(
            1, 0, 6.0, date, &players, 6.0, 2
        ));
        assert!(!exceptional_sim_reserve_move(
            1, 0, 5.99, date, &players, 6.0, 2
        ));
        assert!(!exceptional_sim_reserve_move(
            1, 0, 6.0, date, &players, 6.0, 3
        ));
    }

    #[test]
    fn weekly_pickups_can_use_multiple_moves_but_never_exceed_limit() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let players = [1.0, 2.0, 5.0, 4.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| {
                let game_dates = (0..3)
                    .map(|offset| date + Duration::days(offset))
                    .collect::<Vec<_>>();
                SimPlayer {
                    input: FantasySeasonSimPlayerInput {
                        player_key: format!("p{index}"),
                        player: format!("Player {index}"),
                        nhl_team: "NYR".to_owned(),
                        positions: vec![Position::Center],
                        fantasy_points_per_game: value,
                        game_dates: game_dates.clone(),
                    },
                    dates: game_dates.into_iter().collect(),
                }
            })
            .collect::<Vec<_>>();
        let mut teams = vec![TrialTeam {
            roster: vec![0, 1],
            ..TrialTeam::default()
        }];
        let mut free_agents = [2, 3].into_iter().collect::<BTreeSet<_>>();
        let injured_until = BTreeMap::new();
        let mut waiver_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::new();
        let mut weekly_adds = vec![0];

        for day in 0..3 {
            make_weekly_pickup(
                0,
                date + Duration::days(day),
                1,
                &players,
                &mut teams,
                &mut free_agents,
                &mut waiver_until,
                2,
                &injured_until,
                &mut injury_replacements,
                &mut weekly_adds,
                2,
                &[FantasyActiveSlotKind::Center],
                0,
                false,
                &mut Vec::new(),
            );
        }

        assert_eq!(weekly_adds[0], 2);
        assert_eq!(teams[0].adds, 2);
        assert!(teams[0].roster.contains(&2));
        assert!(teams[0].roster.contains(&3));
    }

    #[test]
    fn pickup_protects_ir_player_and_transfers_replacement_ownership() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let players = [0.1, 1.0, 2.0, 5.0]
            .into_iter()
            .enumerate()
            .map(|(index, value)| SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: format!("p{index}"),
                    player: format!("Player {index}"),
                    nhl_team: "NYR".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: value,
                    game_dates: vec![date],
                },
                dates: [date].into_iter().collect(),
            })
            .collect::<Vec<_>>();
        let mut teams = vec![TrialTeam {
            roster: vec![0, 1, 2],
            ..TrialTeam::default()
        }];
        let mut free_agents = [3].into_iter().collect::<BTreeSet<_>>();
        let injured_until = BTreeMap::from([(0, date + Duration::days(10))]);
        let mut waiver_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::from([(0, 1)]);
        let mut weekly_adds = vec![0];

        make_weekly_pickup(
            0,
            date,
            1,
            &players,
            &mut teams,
            &mut free_agents,
            &mut waiver_until,
            2,
            &injured_until,
            &mut injury_replacements,
            &mut weekly_adds,
            4,
            &[FantasyActiveSlotKind::Center],
            0,
            false,
            &mut Vec::new(),
        );

        assert!(teams[0].roster.contains(&0));
        assert!(!teams[0].roster.contains(&1));
        assert_eq!(injury_replacements.get(&0), Some(&3));
    }

    #[test]
    fn recovery_releases_current_replacement_exactly_once() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let mut roster = vec![0, 2, 3];
        let mut free_agents = [1].into_iter().collect::<BTreeSet<_>>();
        let mut waiver_until = BTreeMap::new();
        let mut injury_replacements = BTreeMap::from([(0, 3)]);

        assert_eq!(
            release_injury_replacement(
                0,
                &mut roster,
                &mut free_agents,
                &mut waiver_until,
                &mut injury_replacements,
                date,
                2
            ),
            Some(3)
        );
        assert_eq!(roster, vec![0, 2]);
        assert!(!free_agents.contains(&3));
        assert_eq!(waiver_until.get(&3), Some(&(date + Duration::days(2))));
        assert!(injury_replacements.is_empty());

        let roster_after_first_release = roster.clone();
        let free_agents_after_first_release = free_agents.clone();
        let waivers_after_first_release = waiver_until.clone();
        assert_eq!(
            release_injury_replacement(
                0,
                &mut roster,
                &mut free_agents,
                &mut waiver_until,
                &mut injury_replacements,
                date,
                2
            ),
            None
        );
        assert_eq!(roster, roster_after_first_release);
        assert_eq!(free_agents, free_agents_after_first_release);
        assert_eq!(waiver_until, waivers_after_first_release);

        clear_expired_waivers(
            date + Duration::days(1),
            &mut free_agents,
            &mut waiver_until,
        );
        assert!(!free_agents.contains(&3));
        clear_expired_waivers(
            date + Duration::days(2),
            &mut free_agents,
            &mut waiver_until,
        );
        assert!(free_agents.contains(&3));
        assert!(!waiver_until.contains_key(&3));
    }

    #[test]
    fn dropped_player_clears_on_exact_waiver_date_and_zero_day_is_immediate() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let mut free_agents = BTreeSet::from([4, 5]);
        let mut waiver_until = BTreeMap::new();

        place_on_waivers(4, date, 2, &mut free_agents, &mut waiver_until);
        assert!(!free_agents.contains(&4));
        assert_eq!(waiver_until.get(&4), Some(&(date + Duration::days(2))));

        clear_expired_waivers(
            date + Duration::days(1),
            &mut free_agents,
            &mut waiver_until,
        );
        assert!(!free_agents.contains(&4));
        clear_expired_waivers(
            date + Duration::days(2),
            &mut free_agents,
            &mut waiver_until,
        );
        assert!(free_agents.contains(&4));
        assert!(!waiver_until.contains_key(&4));

        place_on_waivers(5, date, 0, &mut free_agents, &mut waiver_until);
        assert!(free_agents.contains(&5));
        assert!(!waiver_until.contains_key(&5));
    }

    #[test]
    fn trade_swaps_replacement_ownership_for_both_teams() {
        let mut injury_replacements = BTreeMap::from([(0, 3), (4, 7), (8, 9)]);

        swap_injury_replacement_ownership(&mut injury_replacements, 3, 7);

        assert_eq!(injury_replacements.get(&0), Some(&7));
        assert_eq!(injury_replacements.get(&4), Some(&3));
        assert_eq!(injury_replacements.get(&8), Some(&9));
    }

    #[test]
    fn complete_locked_roster_must_cover_every_active_slot() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let players = (0..32)
            .map(|index| FantasySeasonSimPlayerInput {
                player_key: format!("p{index}"),
                player: format!("Player {index}"),
                nhl_team: "NYR".to_owned(),
                positions: vec![Position::Center],
                fantasy_points_per_game: 1.0,
                game_dates: vec![date],
            })
            .collect::<Vec<_>>();
        let config = FantasySeasonSimConfig {
            fantasy_teams: 2,
            playoff_teams: 1,
            trials: 1,
            user_roster_player_keys: (0..16).map(|index| format!("p{index}")).collect(),
            ..FantasySeasonSimConfig::default()
        };

        let error = simulate_fantasy_season(
            "20262027",
            "test",
            FantasyAssistantRules::configured_2026(),
            players,
            config,
        )
        .unwrap_err();

        assert!(error.contains("complete locked user roster"));
    }

    #[test]
    fn synthetic_draft_rejects_positionally_illegal_pool() {
        let date = NaiveDate::from_ymd_opt(2026, 10, 1).unwrap();
        let players = (0..32)
            .map(|index| SimPlayer {
                input: FantasySeasonSimPlayerInput {
                    player_key: format!("p{index}"),
                    player: format!("Player {index}"),
                    nhl_team: "NYR".to_owned(),
                    positions: vec![Position::Center],
                    fantasy_points_per_game: 1.0,
                    game_dates: vec![date],
                },
                dates: [date].into_iter().collect(),
            })
            .collect::<Vec<_>>();

        let error =
            synthetic_draft(&players, &FantasyAssistantRules::configured_2026(), 2).unwrap_err();

        assert!(error.contains("cannot produce a legal active roster"));
    }

    #[test]
    fn even_team_round_robin_has_every_pair_once_and_no_weekly_duplicates() {
        let teams = 16;
        let mut pairs = BTreeSet::new();
        for week in 0..(teams - 1) {
            let matchups = fantasy_week_matchups(teams, week);
            assert_eq!(matchups.len(), teams / 2);
            let mut weekly_teams = BTreeSet::new();
            for (a, b) in matchups {
                assert!(weekly_teams.insert(a));
                assert!(weekly_teams.insert(b));
                assert!(pairs.insert((a.min(b), a.max(b))));
            }
            assert_eq!(weekly_teams.len(), teams);
        }
        assert_eq!(pairs.len(), teams * (teams - 1) / 2);
    }

    #[test]
    fn odd_team_round_robin_rotates_one_bye_and_every_pair_once() {
        let teams = 5;
        let mut pairs = BTreeSet::new();
        let mut games_by_team = vec![0; teams];
        for week in 0..teams {
            let matchups = fantasy_week_matchups(teams, week);
            assert_eq!(matchups.len(), teams / 2);
            let mut weekly_teams = BTreeSet::new();
            for (a, b) in matchups {
                assert!(weekly_teams.insert(a));
                assert!(weekly_teams.insert(b));
                games_by_team[a] += 1;
                games_by_team[b] += 1;
                assert!(pairs.insert((a.min(b), a.max(b))));
            }
            assert_eq!(weekly_teams.len(), teams - 1);
        }
        assert_eq!(pairs.len(), teams * (teams - 1) / 2);
        assert_eq!(games_by_team, vec![teams - 1; teams]);
    }

    #[test]
    fn six_team_playoff_bracket_honors_byes_and_weekly_upsets() {
        assert_eq!(playoff_round_count(6), 3);
        let mut teams = (0..6)
            .map(|_| TrialTeam {
                weekly_points: vec![0.0; 4],
                ..TrialTeam::default()
            })
            .collect::<Vec<_>>();
        teams[5].weekly_points[1] = 20.0;
        teams[2].weekly_points[1] = 10.0;
        teams[3].weekly_points[1] = 15.0;
        teams[4].weekly_points[1] = 10.0;
        teams[5].weekly_points[2] = 20.0;
        teams[0].weekly_points[2] = 10.0;
        teams[3].weekly_points[2] = 20.0;
        teams[1].weekly_points[2] = 10.0;
        teams[5].weekly_points[3] = 20.0;
        teams[3].weekly_points[3] = 10.0;

        let outcome = simulate_playoff_bracket(&[0, 1, 2, 3, 4, 5], &teams, 1);
        assert_eq!(outcome.champion, 5);
        assert_eq!(
            outcome.eliminated,
            vec![(0, 2), (0, 4), (1, 0), (1, 1), (2, 3)]
        );
    }
}
