use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::fantasy_assistant::{
    build_fantasy_daily_lineup, FantasyAssistantRules, FantasyLineupPlayerInput,
    FantasyPlayerAvailabilityStatus,
};
use super::fantasy_schedule::FantasyScheduleGameInput;

pub const FANTASY_PLAYOFF_PORTFOLIO_SCHEMA: &str = "fantasy_playoff_portfolio.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyPlayoffRoundInput {
    pub label: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyPlayoffPlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub projected_value_per_game: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyPlayoffPortfolioInput {
    pub season: u32,
    pub fantasy_team: String,
    pub rules: FantasyAssistantRules,
    pub off_night_max_games: usize,
    pub rounds: Vec<FantasyPlayoffRoundInput>,
    pub players: Vec<FantasyPlayoffPlayerInput>,
    pub games: Vec<FantasyScheduleGameInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlayoffPlayerRoundRow {
    pub round: String,
    pub scheduled_games: usize,
    pub usable_starts: usize,
    pub quiet_slate_starts: usize,
    pub bench_collisions: usize,
    pub projected_usable_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlayoffPlayerRow {
    pub playoff_rank: usize,
    pub regular_value_rank: usize,
    /// Positive values identify schedule/lineup risers for the playoff window.
    pub rank_delta: i32,
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub projected_value_per_game: f64,
    pub scheduled_games: usize,
    pub usable_starts: usize,
    pub quiet_slate_starts: usize,
    pub bench_collisions: usize,
    pub projected_usable_value: f64,
    pub portfolio_score: f64,
    pub rounds: Vec<FantasyPlayoffPlayerRoundRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlayoffTeamRow {
    pub nhl_team: String,
    pub rostered_players: usize,
    pub scheduled_games: usize,
    pub usable_starts: usize,
    pub quiet_slate_starts: usize,
    pub bench_collisions: usize,
    pub projected_usable_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlayoffRoundRow {
    pub label: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub league_games: usize,
    pub roster_scheduled_games: usize,
    pub usable_starts: usize,
    pub quiet_slate_starts: usize,
    pub bench_collisions: usize,
    pub projected_usable_value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlayoffPortfolioView {
    pub schema: String,
    pub season: u32,
    pub fantasy_team: String,
    pub off_night_max_games: usize,
    pub rounds: Vec<FantasyPlayoffRoundRow>,
    pub players: Vec<FantasyPlayoffPlayerRow>,
    pub teams: Vec<FantasyPlayoffTeamRow>,
    #[serde(default)]
    pub candidate_fits: Vec<FantasyPlayoffCandidateFitRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyPlayoffCandidateFitRow {
    pub rank: usize,
    pub add_player_key: String,
    pub add_player: String,
    pub add_nhl_team: String,
    pub drop_player_key: String,
    pub drop_player: String,
    pub scheduled_games: usize,
    pub usable_starts: usize,
    pub quiet_slate_starts: usize,
    pub bench_collisions: usize,
    pub usable_starts_delta: i32,
    pub quiet_slate_starts_delta: i32,
    pub bench_collisions_delta: i32,
    pub projected_usable_value_delta: f64,
    pub portfolio_score_delta: f64,
}

#[derive(Default)]
struct Totals {
    scheduled: usize,
    usable: usize,
    quiet: usize,
    collisions: usize,
    value: f64,
}

pub fn build_fantasy_playoff_portfolio(
    input: FantasyPlayoffPortfolioInput,
) -> Result<FantasyPlayoffPortfolioView, String> {
    input.rules.validate()?;
    if input.off_night_max_games == 0 {
        return Err("off-night threshold must be at least one game".to_owned());
    }
    if input.rounds.is_empty() {
        return Err("playoff portfolio requires at least one round".to_owned());
    }
    if input.players.is_empty() {
        return Err("playoff portfolio requires at least one rostered player".to_owned());
    }
    for round in &input.rounds {
        if round.label.trim().is_empty() || round.end < round.start {
            return Err("every playoff round requires a label and a valid date range".to_owned());
        }
    }

    let mut games_by_date: BTreeMap<NaiveDate, Vec<&FantasyScheduleGameInput>> = BTreeMap::new();
    for game in &input.games {
        games_by_date.entry(game.date).or_default().push(game);
    }
    let mut per_player: BTreeMap<String, (Totals, Vec<FantasyPlayoffPlayerRoundRow>)> = input
        .players
        .iter()
        .map(|player| (player.player_key.clone(), (Totals::default(), Vec::new())))
        .collect();
    let mut round_rows = Vec::new();

    for round in &input.rounds {
        let mut round_totals: BTreeMap<String, Totals> = input
            .players
            .iter()
            .map(|player| (player.player_key.clone(), Totals::default()))
            .collect();
        let mut league_games = 0;
        let mut date = round.start;
        while date <= round.end {
            let slate = games_by_date.get(&date).cloned().unwrap_or_default();
            league_games += slate.len();
            let teams_playing = slate
                .iter()
                .flat_map(|game| [&game.away_team, &game.home_team])
                .map(|team| team.trim().to_ascii_uppercase())
                .collect::<BTreeSet<_>>();
            let lineup = build_fantasy_daily_lineup(
                input.rules.clone(),
                input
                    .players
                    .iter()
                    .map(|player| FantasyLineupPlayerInput {
                        player_key: player.player_key.clone(),
                        display_name: player.player.clone(),
                        nhl_team: player.nhl_team.clone(),
                        platform_positions: player.positions.clone(),
                        projected_value: player.projected_value_per_game,
                        has_game: teams_playing
                            .contains(&player.nhl_team.trim().to_ascii_uppercase()),
                        status: FantasyPlayerAvailabilityStatus::Healthy,
                        locked_slot: None,
                        locked: false,
                    })
                    .collect(),
            )?;
            let active = lineup
                .active
                .iter()
                .filter(|row| row.has_game)
                .map(|row| row.player_key.as_str())
                .collect::<BTreeSet<_>>();
            for player in &input.players {
                if !teams_playing.contains(&player.nhl_team.trim().to_ascii_uppercase()) {
                    continue;
                }
                let totals = round_totals.get_mut(&player.player_key).unwrap();
                totals.scheduled += 1;
                if active.contains(player.player_key.as_str()) {
                    totals.usable += 1;
                    totals.value += player.projected_value_per_game;
                    if slate.len() <= input.off_night_max_games {
                        totals.quiet += 1;
                    }
                } else {
                    totals.collisions += 1;
                }
            }
            date += Duration::days(1);
        }

        let mut aggregate = Totals::default();
        for player in &input.players {
            let totals = round_totals.remove(&player.player_key).unwrap();
            aggregate.scheduled += totals.scheduled;
            aggregate.usable += totals.usable;
            aggregate.quiet += totals.quiet;
            aggregate.collisions += totals.collisions;
            aggregate.value += totals.value;
            let (all, rows) = per_player.get_mut(&player.player_key).unwrap();
            all.scheduled += totals.scheduled;
            all.usable += totals.usable;
            all.quiet += totals.quiet;
            all.collisions += totals.collisions;
            all.value += totals.value;
            rows.push(FantasyPlayoffPlayerRoundRow {
                round: round.label.clone(),
                scheduled_games: totals.scheduled,
                usable_starts: totals.usable,
                quiet_slate_starts: totals.quiet,
                bench_collisions: totals.collisions,
                projected_usable_value: totals.value,
            });
        }
        round_rows.push(FantasyPlayoffRoundRow {
            label: round.label.clone(),
            start: round.start,
            end: round.end,
            league_games,
            roster_scheduled_games: aggregate.scheduled,
            usable_starts: aggregate.usable,
            quiet_slate_starts: aggregate.quiet,
            bench_collisions: aggregate.collisions,
            projected_usable_value: aggregate.value,
        });
    }

    let mut regular_order = input.players.iter().collect::<Vec<_>>();
    regular_order.sort_by(|a, b| {
        b.projected_value_per_game
            .total_cmp(&a.projected_value_per_game)
            .then_with(|| a.player_key.cmp(&b.player_key))
    });
    let regular_rank = regular_order
        .iter()
        .enumerate()
        .map(|(index, player)| (player.player_key.as_str(), index + 1))
        .collect::<BTreeMap<_, _>>();
    let mut players = input
        .players
        .iter()
        .map(|player| {
            let (totals, rounds) = per_player.remove(&player.player_key).unwrap();
            FantasyPlayoffPlayerRow {
                playoff_rank: 0,
                regular_value_rank: regular_rank[player.player_key.as_str()],
                rank_delta: 0,
                player_key: player.player_key.clone(),
                player: player.player.clone(),
                nhl_team: player.nhl_team.clone(),
                positions: player.positions.clone(),
                projected_value_per_game: player.projected_value_per_game,
                scheduled_games: totals.scheduled,
                usable_starts: totals.usable,
                quiet_slate_starts: totals.quiet,
                bench_collisions: totals.collisions,
                projected_usable_value: totals.value,
                portfolio_score: totals.value + totals.quiet as f64 * 0.25
                    - totals.collisions as f64 * 0.5,
                rounds,
            }
        })
        .collect::<Vec<_>>();
    players.sort_by(|a, b| {
        b.portfolio_score
            .total_cmp(&a.portfolio_score)
            .then_with(|| b.usable_starts.cmp(&a.usable_starts))
            .then_with(|| a.player_key.cmp(&b.player_key))
    });
    for (index, player) in players.iter_mut().enumerate() {
        player.playoff_rank = index + 1;
        player.rank_delta = player.regular_value_rank as i32 - player.playoff_rank as i32;
    }

    let mut teams: BTreeMap<String, FantasyPlayoffTeamRow> = BTreeMap::new();
    for player in &players {
        let row = teams
            .entry(player.nhl_team.clone())
            .or_insert_with(|| FantasyPlayoffTeamRow {
                nhl_team: player.nhl_team.clone(),
                rostered_players: 0,
                scheduled_games: 0,
                usable_starts: 0,
                quiet_slate_starts: 0,
                bench_collisions: 0,
                projected_usable_value: 0.0,
            });
        row.rostered_players += 1;
        row.scheduled_games += player.scheduled_games;
        row.usable_starts += player.usable_starts;
        row.quiet_slate_starts += player.quiet_slate_starts;
        row.bench_collisions += player.bench_collisions;
        row.projected_usable_value += player.projected_usable_value;
    }
    let mut teams = teams.into_values().collect::<Vec<_>>();
    teams.sort_by(|a, b| {
        b.projected_usable_value
            .total_cmp(&a.projected_usable_value)
            .then_with(|| b.usable_starts.cmp(&a.usable_starts))
            .then_with(|| a.nhl_team.cmp(&b.nhl_team))
    });

    Ok(FantasyPlayoffPortfolioView {
        schema: FANTASY_PLAYOFF_PORTFOLIO_SCHEMA.to_owned(),
        season: input.season,
        fantasy_team: input.fantasy_team,
        off_night_max_games: input.off_night_max_games,
        rounds: round_rows,
        players,
        teams,
        candidate_fits: Vec::new(),
        disclosures: vec![
            "Usable starts are produced by the saved league's legal daily lineup assignment; scheduled games alone do not imply fantasy value.".to_owned(),
            "Portfolio score is projected usable value plus 0.25 per quiet-slate start and minus 0.50 per bench collision; components remain separately visible.".to_owned(),
            "Positive rank delta means the player rises versus the roster's per-game value order because of playoff schedule and lineup fit.".to_owned(),
            "This schedule portfolio does not predict injuries, starting goalies, or future role changes.".to_owned(),
        ],
    })
}

/// Evaluate each candidate against every one-for-one drop while preserving the
/// exact playoff dates and the saved league's legal daily assignment rules.
pub fn rank_fantasy_playoff_candidate_fits(
    base: &FantasyPlayoffPortfolioInput,
    candidates: Vec<FantasyPlayoffPlayerInput>,
    top: usize,
) -> Result<Vec<FantasyPlayoffCandidateFitRow>, String> {
    if top == 0 {
        return Ok(Vec::new());
    }
    let baseline = build_fantasy_playoff_portfolio(base.clone())?;
    let baseline_usable: usize = baseline
        .rounds
        .iter()
        .map(|round| round.usable_starts)
        .sum();
    let baseline_quiet: usize = baseline
        .rounds
        .iter()
        .map(|round| round.quiet_slate_starts)
        .sum();
    let baseline_collisions: usize = baseline
        .rounds
        .iter()
        .map(|round| round.bench_collisions)
        .sum();
    let baseline_value: f64 = baseline
        .rounds
        .iter()
        .map(|round| round.projected_usable_value)
        .sum();
    let baseline_score: f64 = baseline.players.iter().map(|row| row.portfolio_score).sum();
    let roster_keys = base
        .players
        .iter()
        .map(|player| player.player_key.as_str())
        .collect::<BTreeSet<_>>();
    let mut rows = Vec::new();

    for candidate in candidates {
        if roster_keys.contains(candidate.player_key.as_str()) {
            continue;
        }
        let mut best: Option<FantasyPlayoffCandidateFitRow> = None;
        for (drop_index, drop) in base.players.iter().enumerate() {
            let mut hypothetical = base.clone();
            hypothetical.players[drop_index] = candidate.clone();
            let view = build_fantasy_playoff_portfolio(hypothetical)?;
            let usable: usize = view.rounds.iter().map(|round| round.usable_starts).sum();
            let quiet: usize = view
                .rounds
                .iter()
                .map(|round| round.quiet_slate_starts)
                .sum();
            let collisions: usize = view.rounds.iter().map(|round| round.bench_collisions).sum();
            let value: f64 = view
                .rounds
                .iter()
                .map(|round| round.projected_usable_value)
                .sum();
            let score: f64 = view.players.iter().map(|row| row.portfolio_score).sum();
            let candidate_row = view
                .players
                .iter()
                .find(|row| row.player_key == candidate.player_key)
                .expect("hypothetical candidate must remain in the portfolio");
            let row = FantasyPlayoffCandidateFitRow {
                rank: 0,
                add_player_key: candidate.player_key.clone(),
                add_player: candidate.player.clone(),
                add_nhl_team: candidate.nhl_team.clone(),
                drop_player_key: drop.player_key.clone(),
                drop_player: drop.player.clone(),
                scheduled_games: candidate_row.scheduled_games,
                usable_starts: candidate_row.usable_starts,
                quiet_slate_starts: candidate_row.quiet_slate_starts,
                bench_collisions: candidate_row.bench_collisions,
                usable_starts_delta: usable as i32 - baseline_usable as i32,
                quiet_slate_starts_delta: quiet as i32 - baseline_quiet as i32,
                bench_collisions_delta: collisions as i32 - baseline_collisions as i32,
                projected_usable_value_delta: value - baseline_value,
                portfolio_score_delta: score - baseline_score,
            };
            let replace = best.as_ref().is_none_or(|current| {
                row.portfolio_score_delta
                    .total_cmp(&current.portfolio_score_delta)
                    .then_with(|| {
                        row.projected_usable_value_delta
                            .total_cmp(&current.projected_usable_value_delta)
                    })
                    .then_with(|| current.drop_player_key.cmp(&row.drop_player_key))
                    .is_gt()
            });
            if replace {
                best = Some(row);
            }
        }
        if let Some(row) = best {
            rows.push(row);
        }
    }
    rows.sort_by(|a, b| {
        b.portfolio_score_delta
            .total_cmp(&a.portfolio_score_delta)
            .then_with(|| {
                b.projected_usable_value_delta
                    .total_cmp(&a.projected_usable_value_delta)
            })
            .then_with(|| a.add_player_key.cmp(&b.add_player_key))
    });
    rows.truncate(top);
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }
    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(id: u64, date: &str, away: &str, home: &str) -> FantasyScheduleGameInput {
        FantasyScheduleGameInput {
            game_id: id,
            date: NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap(),
            away_team: away.to_owned(),
            home_team: home.to_owned(),
        }
    }

    fn player(key: &str, team: &str, value: f64) -> FantasyPlayoffPlayerInput {
        FantasyPlayoffPlayerInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: team.to_owned(),
            positions: vec![Position::Center],
            projected_value_per_game: value,
        }
    }

    #[test]
    fn four_games_with_collisions_rank_below_three_usable_starts() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(
            super::super::fantasy_assistant::FantasyActiveSlotKind::Center,
            1,
        )]);
        rules.bench_slots = 2;
        let view = build_fantasy_playoff_portfolio(FantasyPlayoffPortfolioInput {
            season: 20262027,
            fantasy_team: "Sample Multicategory".to_owned(),
            rules,
            off_night_max_games: 1,
            rounds: vec![FantasyPlayoffRoundInput {
                label: "First round".to_owned(),
                start: NaiveDate::from_ymd_opt(2027, 3, 22).unwrap(),
                end: NaiveDate::from_ymd_opt(2027, 3, 28).unwrap(),
            }],
            players: vec![
                player("four-game", "NYR", 4.0),
                player("three-usable", "SEA", 3.9),
            ],
            games: vec![
                game(1, "2027-03-22", "NYR", "BOS"),
                game(2, "2027-03-23", "NYR", "BOS"),
                game(3, "2027-03-24", "NYR", "BOS"),
                game(4, "2027-03-25", "NYR", "BOS"),
                game(5, "2027-03-22", "SEA", "VAN"),
                game(6, "2027-03-23", "SEA", "VAN"),
                game(7, "2027-03-24", "SEA", "VAN"),
            ],
        })
        .unwrap();

        let four = view
            .players
            .iter()
            .find(|row| row.player_key == "four-game")
            .unwrap();
        let three = view
            .players
            .iter()
            .find(|row| row.player_key == "three-usable")
            .unwrap();
        assert_eq!(four.scheduled_games, 4);
        assert_eq!(four.usable_starts, 4);
        assert_eq!(three.usable_starts, 0);
        assert!(four.playoff_rank < three.playoff_rank);
    }

    #[test]
    fn exact_date_collision_can_make_four_games_worse_than_three() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(
            super::super::fantasy_assistant::FantasyActiveSlotKind::Center,
            1,
        )]);
        rules.bench_slots = 3;
        let mut crowded = player("crowded", "NYR", 3.0);
        let blocker = player("blocker", "COL", 10.0);
        let clear = player("clear", "SEA", 3.0);
        crowded.positions = vec![Position::Center];
        let view = build_fantasy_playoff_portfolio(FantasyPlayoffPortfolioInput {
            season: 20262027,
            fantasy_team: "Dawgs".to_owned(),
            rules,
            off_night_max_games: 1,
            rounds: vec![FantasyPlayoffRoundInput {
                label: "Final".to_owned(),
                start: NaiveDate::from_ymd_opt(2027, 3, 22).unwrap(),
                end: NaiveDate::from_ymd_opt(2027, 3, 28).unwrap(),
            }],
            players: vec![crowded, blocker, clear],
            games: vec![
                game(1, "2027-03-22", "NYR", "BOS"),
                game(2, "2027-03-23", "NYR", "BOS"),
                game(3, "2027-03-24", "NYR", "BOS"),
                game(4, "2027-03-25", "NYR", "BOS"),
                game(5, "2027-03-22", "COL", "DAL"),
                game(6, "2027-03-23", "COL", "DAL"),
                game(7, "2027-03-26", "SEA", "VAN"),
                game(8, "2027-03-27", "SEA", "VAN"),
                game(9, "2027-03-28", "SEA", "VAN"),
            ],
        })
        .unwrap();
        let crowded = view
            .players
            .iter()
            .find(|row| row.player_key == "crowded")
            .unwrap();
        let clear = view
            .players
            .iter()
            .find(|row| row.player_key == "clear")
            .unwrap();
        assert_eq!(
            (
                crowded.scheduled_games,
                crowded.usable_starts,
                crowded.bench_collisions
            ),
            (4, 2, 2)
        );
        assert_eq!((clear.scheduled_games, clear.usable_starts), (3, 3));
        assert!(clear.playoff_rank < crowded.playoff_rank);
    }

    #[test]
    fn candidate_fit_selects_drop_using_whole_roster_playoff_delta() {
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(
            super::super::fantasy_assistant::FantasyActiveSlotKind::Center,
            1,
        )]);
        rules.bench_slots = 1;
        let base = FantasyPlayoffPortfolioInput {
            season: 20262027,
            fantasy_team: "Dawgs".to_owned(),
            rules,
            off_night_max_games: 1,
            rounds: vec![FantasyPlayoffRoundInput {
                label: "Championship".to_owned(),
                start: NaiveDate::from_ymd_opt(2027, 3, 22).unwrap(),
                end: NaiveDate::from_ymd_opt(2027, 3, 28).unwrap(),
            }],
            players: vec![player("anchor", "COL", 8.0), player("bench", "NYR", 2.0)],
            games: vec![
                game(1, "2027-03-22", "COL", "DAL"),
                game(2, "2027-03-23", "COL", "DAL"),
                game(3, "2027-03-22", "NYR", "BOS"),
                game(4, "2027-03-23", "NYR", "BOS"),
                game(5, "2027-03-24", "SEA", "VAN"),
                game(6, "2027-03-25", "SEA", "VAN"),
                game(7, "2027-03-26", "SEA", "VAN"),
            ],
        };
        let rows =
            rank_fantasy_playoff_candidate_fits(&base, vec![player("candidate", "SEA", 3.0)], 10)
                .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].drop_player_key, "bench");
        assert_eq!(rows[0].usable_starts_delta, 3);
        assert!(rows[0].portfolio_score_delta > 0.0);
    }
}
