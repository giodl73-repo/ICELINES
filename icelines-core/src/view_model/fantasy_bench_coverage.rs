use std::collections::{BTreeMap, BTreeSet};

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::fantasy_assistant::{
    build_fantasy_daily_lineup, FantasyActiveSlotKind, FantasyAssistantRules,
    FantasyLineupPlayerInput, FantasyPlayerAvailabilityStatus,
};
use super::fantasy_schedule::FantasyScheduleGameInput;

pub const FANTASY_BENCH_COVERAGE_SCHEMA: &str = "fantasy_bench_coverage.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyBenchCoveragePlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub projected_value_per_game: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyBenchCoverageInput {
    pub fantasy_team: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub off_night_max_games: usize,
    pub rules: FantasyAssistantRules,
    pub players: Vec<FantasyBenchCoveragePlayerInput>,
    pub games: Vec<FantasyScheduleGameInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyBenchCoveragePairRow {
    pub starter_key: String,
    pub starter: String,
    pub slot_kind: FantasyActiveSlotKind,
    pub dates: Vec<NaiveDate>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyBenchCoveragePlayerRow {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub baseline_bench_slot: String,
    pub scheduled_games: usize,
    pub usable_substitute_starts: usize,
    pub quiet_night_starts: usize,
    pub bench_collisions: usize,
    pub projected_substitute_value: f64,
    pub covers: Vec<FantasyBenchCoveragePairRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyBenchCoverageView {
    pub schema: String,
    pub fantasy_team: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub off_night_max_games: usize,
    pub baseline_starters: usize,
    pub rows: Vec<FantasyBenchCoveragePlayerRow>,
    pub uncovered_starter_dates: BTreeMap<String, Vec<NaiveDate>>,
    pub disclosures: Vec<String>,
}

#[derive(Default)]
struct CoverageTotals {
    scheduled: usize,
    usable: usize,
    quiet: usize,
    collisions: usize,
    value: f64,
    covers: BTreeMap<(String, FantasyActiveSlotKind), BTreeSet<NaiveDate>>,
}

pub fn build_fantasy_bench_coverage(
    input: FantasyBenchCoverageInput,
) -> Result<FantasyBenchCoverageView, String> {
    input.rules.validate()?;
    if input.end < input.start {
        return Err("bench coverage requires end on or after start".to_owned());
    }
    if input.off_night_max_games == 0 {
        return Err("off-night threshold must be at least one game".to_owned());
    }
    if input.players.is_empty() {
        return Err("bench coverage requires at least one rostered player".to_owned());
    }

    let baseline = build_fantasy_daily_lineup(
        input.rules.clone(),
        lineup_inputs(&input.players, &BTreeSet::new(), true),
    )?;
    let starters_by_slot = baseline
        .active
        .iter()
        .map(|row| {
            (
                row.slot_id.clone(),
                (row.player_key.clone(), row.player.clone(), row.slot_kind),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let bench_by_key = baseline
        .bench_assignments
        .iter()
        .map(|row| (row.player_key.clone(), row.bench_slot.clone()))
        .collect::<BTreeMap<_, _>>();
    let player_by_key = input
        .players
        .iter()
        .map(|player| (player.player_key.as_str(), player))
        .collect::<BTreeMap<_, _>>();
    let mut totals = bench_by_key
        .keys()
        .map(|key| (key.clone(), CoverageTotals::default()))
        .collect::<BTreeMap<_, _>>();
    let mut uncovered = BTreeMap::<String, Vec<NaiveDate>>::new();
    let mut games_by_date = BTreeMap::<NaiveDate, Vec<&FantasyScheduleGameInput>>::new();
    for game in &input.games {
        if game.date >= input.start && game.date <= input.end {
            games_by_date.entry(game.date).or_default().push(game);
        }
    }

    let mut date = input.start;
    while date <= input.end {
        let slate = games_by_date.get(&date).cloned().unwrap_or_default();
        let teams_playing = slate
            .iter()
            .flat_map(|game| [&game.away_team, &game.home_team])
            .map(|team| team.trim().to_ascii_uppercase())
            .collect::<BTreeSet<_>>();
        let lineup = build_fantasy_daily_lineup(
            input.rules.clone(),
            lineup_inputs(&input.players, &teams_playing, false),
        )?;
        let active_by_slot = lineup
            .active
            .iter()
            .filter(|row| row.has_game)
            .map(|row| (row.slot_id.as_str(), row.player_key.as_str()))
            .collect::<BTreeMap<_, _>>();

        for (bench_key, total) in &mut totals {
            let player = player_by_key[bench_key.as_str()];
            if !teams_playing.contains(&player.nhl_team.trim().to_ascii_uppercase()) {
                continue;
            }
            total.scheduled += 1;
            let active_slot = active_by_slot
                .iter()
                .find_map(|(slot, key)| (*key == bench_key).then_some(*slot));
            let Some(active_slot) = active_slot else {
                total.collisions += 1;
                continue;
            };
            total.usable += 1;
            total.value += player.projected_value_per_game;
            if slate.len() <= input.off_night_max_games {
                total.quiet += 1;
            }
            if let Some((starter_key, _, kind)) = starters_by_slot.get(active_slot) {
                let starter = player_by_key[starter_key.as_str()];
                if !teams_playing.contains(&starter.nhl_team.trim().to_ascii_uppercase()) {
                    total
                        .covers
                        .entry((starter_key.clone(), *kind))
                        .or_default()
                        .insert(date);
                }
            }
        }

        for (slot, (starter_key, starter_name, _)) in &starters_by_slot {
            let starter = player_by_key[starter_key.as_str()];
            if teams_playing.contains(&starter.nhl_team.trim().to_ascii_uppercase()) {
                continue;
            }
            let covered = active_by_slot
                .get(slot.as_str())
                .is_some_and(|key| bench_by_key.contains_key(*key));
            if !covered {
                uncovered
                    .entry(starter_name.clone())
                    .or_default()
                    .push(date);
            }
        }
        date += Duration::days(1);
    }

    let mut rows = totals
        .into_iter()
        .map(|(key, totals)| {
            let player = player_by_key[key.as_str()];
            let covers = totals
                .covers
                .into_iter()
                .map(
                    |((starter_key, slot_kind), dates)| FantasyBenchCoveragePairRow {
                        starter: player_by_key[starter_key.as_str()].player.clone(),
                        starter_key,
                        slot_kind,
                        dates: dates.into_iter().collect(),
                    },
                )
                .collect();
            FantasyBenchCoveragePlayerRow {
                player_key: key.clone(),
                player: player.player.clone(),
                nhl_team: player.nhl_team.clone(),
                positions: player.positions.clone(),
                baseline_bench_slot: bench_by_key[&key].clone(),
                scheduled_games: totals.scheduled,
                usable_substitute_starts: totals.usable,
                quiet_night_starts: totals.quiet,
                bench_collisions: totals.collisions,
                projected_substitute_value: totals.value,
                covers,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| {
        b.usable_substitute_starts
            .cmp(&a.usable_substitute_starts)
            .then_with(|| b.quiet_night_starts.cmp(&a.quiet_night_starts))
            .then_with(|| {
                b.projected_substitute_value
                    .total_cmp(&a.projected_substitute_value)
            })
            .then_with(|| a.player_key.cmp(&b.player_key))
    });

    Ok(FantasyBenchCoverageView {
        schema: FANTASY_BENCH_COVERAGE_SCHEMA.to_owned(),
        fantasy_team: input.fantasy_team,
        start: input.start,
        end: input.end,
        off_night_max_games: input.off_night_max_games,
        baseline_starters: starters_by_slot.len(),
        rows,
        uncovered_starter_dates: uncovered,
        disclosures: vec![
            "Baseline starters are the highest-value legal full-roster assignment; saved roster membership does not encode Yahoo's current BN labels.".to_owned(),
            "A covered start requires the baseline bench player to enter the same active slot while its baseline starter has no game.".to_owned(),
            "Goalie schedule opportunities are not confirmed starts.".to_owned(),
        ],
    })
}

fn lineup_inputs(
    players: &[FantasyBenchCoveragePlayerInput],
    teams_playing: &BTreeSet<String>,
    all_have_games: bool,
) -> Vec<FantasyLineupPlayerInput> {
    players
        .iter()
        .map(|player| FantasyLineupPlayerInput {
            player_key: player.player_key.clone(),
            display_name: player.player.clone(),
            nhl_team: player.nhl_team.clone(),
            platform_positions: player.positions.clone(),
            projected_value: player.projected_value_per_game,
            has_game: all_have_games
                || teams_playing.contains(&player.nhl_team.trim().to_ascii_uppercase()),
            status: FantasyPlayerAvailabilityStatus::Healthy,
            locked_slot: None,
            locked: false,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game(date: NaiveDate, team: &str) -> FantasyScheduleGameInput {
        FantasyScheduleGameInput {
            game_id: date.format("%Y%m%d").to_string().parse().unwrap(),
            date,
            away_team: team.to_owned(),
            home_team: "XXX".to_owned(),
        }
    }

    #[test]
    fn bench_defenseman_reports_the_starter_and_dates_covered() {
        let start = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::Defense, 1)]);
        rules.bench_slots = 1;
        rules.ir_slots = 0;
        rules.ir_plus_slots = 0;
        let view = build_fantasy_bench_coverage(FantasyBenchCoverageInput {
            fantasy_team: "Test".to_owned(),
            start,
            end: start + Duration::days(2),
            off_night_max_games: 4,
            rules,
            players: vec![
                FantasyBenchCoveragePlayerInput {
                    player_key: "starter".to_owned(),
                    player: "Starter D".to_owned(),
                    nhl_team: "AAA".to_owned(),
                    positions: vec![Position::Defense],
                    projected_value_per_game: 8.0,
                },
                FantasyBenchCoveragePlayerInput {
                    player_key: "bench".to_owned(),
                    player: "Bench D".to_owned(),
                    nhl_team: "BBB".to_owned(),
                    positions: vec![Position::Defense],
                    projected_value_per_game: 5.0,
                },
            ],
            games: vec![game(start, "AAA"), game(start + Duration::days(1), "BBB")],
        })
        .unwrap();

        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].player, "Bench D");
        assert_eq!(view.rows[0].usable_substitute_starts, 1);
        assert_eq!(view.rows[0].covers[0].starter, "Starter D");
        assert_eq!(view.rows[0].covers[0].dates, [start + Duration::days(1)]);
        assert_eq!(
            view.uncovered_starter_dates["Starter D"],
            [start + Duration::days(2)]
        );
    }
}
