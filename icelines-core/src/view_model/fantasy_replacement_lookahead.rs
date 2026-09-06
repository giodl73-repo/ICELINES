use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

use crate::model::Position;

use super::fantasy_assistant::{
    build_fantasy_daily_lineup, FantasyAssistantRules, FantasyLineupPlayerInput,
    FantasyPlayerAvailabilityStatus,
};
use super::fantasy_schedule::FantasyScheduleGameInput;

pub const FANTASY_REPLACEMENT_LOOKAHEAD_SCHEMA: &str = "fantasy_replacement_lookahead.v1";

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyReplacementPlayerInput {
    pub player_key: String,
    pub player: String,
    pub nhl_team: String,
    pub positions: Vec<Position>,
    pub projected_value_per_game: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyReplacementLookaheadInput {
    pub fantasy_team: String,
    pub start: NaiveDate,
    pub weeks: usize,
    pub off_night_max_games: usize,
    pub acquisitions_remaining: usize,
    pub rules: FantasyAssistantRules,
    pub roster: Vec<FantasyReplacementPlayerInput>,
    pub drop_player_keys: BTreeSet<String>,
    pub injury_replacement_keys: BTreeSet<String>,
    pub candidates: Vec<FantasyReplacementPlayerInput>,
    pub games: Vec<FantasyScheduleGameInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyReplacementWeekRow {
    pub week: usize,
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub usable_starts_delta: i32,
    pub projected_points_delta: f64,
    pub candidate_usable_starts: usize,
    pub candidate_quiet_night_starts: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyReplacementOptionRow {
    pub rank: usize,
    pub add_player_key: String,
    pub add_player: String,
    pub add_nhl_team: String,
    pub add_positions: Vec<Position>,
    pub drop_player_key: String,
    pub drop_player: String,
    pub drop_nhl_team: String,
    pub injury_replacement: bool,
    pub usable_starts_delta: i32,
    pub projected_points_delta: f64,
    pub candidate_usable_starts: usize,
    pub candidate_quiet_night_starts: usize,
    pub weighted_score: f64,
    pub posture: String,
    pub weeks: Vec<FantasyReplacementWeekRow>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyReplacementLookaheadView {
    pub schema: String,
    pub fantasy_team: String,
    pub start: NaiveDate,
    pub end: NaiveDate,
    pub weeks: usize,
    pub acquisitions_remaining: usize,
    pub rows: Vec<FantasyReplacementOptionRow>,
    pub disclosures: Vec<String>,
}

#[derive(Default)]
struct WeekTotals {
    starts: usize,
    points: f64,
    candidate_starts: usize,
    candidate_quiet_starts: usize,
}

pub fn build_fantasy_replacement_lookahead(
    input: FantasyReplacementLookaheadInput,
) -> Result<FantasyReplacementLookaheadView, String> {
    input.rules.validate()?;
    if !(1..=8).contains(&input.weeks) {
        return Err("replacement lookahead requires between one and eight weeks".to_owned());
    }
    if input.off_night_max_games == 0 {
        return Err("off-night threshold must be at least one game".to_owned());
    }
    if input.roster.is_empty() {
        return Err("replacement lookahead requires a roster".to_owned());
    }
    if input.drop_player_keys.is_empty() {
        return Err("replacement lookahead requires at least one drop target".to_owned());
    }

    let roster_keys = input
        .roster
        .iter()
        .map(|player| player.player_key.as_str())
        .collect::<BTreeSet<_>>();
    for key in &input.drop_player_keys {
        if !roster_keys.contains(key.as_str()) {
            return Err(format!("drop target `{key}` is not on the roster"));
        }
    }

    let start =
        input.start - Duration::days(i64::from(input.start.weekday().num_days_from_monday()));
    let end = start + Duration::days((input.weeks * 7 - 1) as i64);
    let mut games_by_date = BTreeMap::<NaiveDate, Vec<&FantasyScheduleGameInput>>::new();
    for game in &input.games {
        if game.date >= start && game.date <= end {
            games_by_date.entry(game.date).or_default().push(game);
        }
    }

    let baseline = simulate_weeks(
        &input.rules,
        &input.roster,
        None,
        start,
        input.weeks,
        input.off_night_max_games,
        &games_by_date,
    )?;
    let roster_by_key = input
        .roster
        .iter()
        .map(|player| (player.player_key.as_str(), player))
        .collect::<BTreeMap<_, _>>();
    let mut rows = Vec::new();

    for drop_key in &input.drop_player_keys {
        let drop_player = roster_by_key[drop_key.as_str()];
        for candidate in &input.candidates {
            if roster_keys.contains(candidate.player_key.as_str()) {
                continue;
            }
            let replacement_roster = input
                .roster
                .iter()
                .filter(|player| player.player_key != *drop_key)
                .cloned()
                .chain(std::iter::once(candidate.clone()))
                .collect::<Vec<_>>();
            let replacement = simulate_weeks(
                &input.rules,
                &replacement_roster,
                Some(candidate.player_key.as_str()),
                start,
                input.weeks,
                input.off_night_max_games,
                &games_by_date,
            )?;

            let weeks = baseline
                .iter()
                .zip(&replacement)
                .enumerate()
                .map(|(index, (base, changed))| FantasyReplacementWeekRow {
                    week: index + 1,
                    week_start: start + Duration::days((index * 7) as i64),
                    week_end: start + Duration::days((index * 7 + 6) as i64),
                    usable_starts_delta: changed.starts as i32 - base.starts as i32,
                    projected_points_delta: changed.points - base.points,
                    candidate_usable_starts: changed.candidate_starts,
                    candidate_quiet_night_starts: changed.candidate_quiet_starts,
                })
                .collect::<Vec<_>>();
            let usable_starts_delta = weeks.iter().map(|week| week.usable_starts_delta).sum();
            let projected_points_delta = weeks.iter().map(|week| week.projected_points_delta).sum();
            let candidate_usable_starts =
                weeks.iter().map(|week| week.candidate_usable_starts).sum();
            let candidate_quiet_night_starts = weeks
                .iter()
                .map(|week| week.candidate_quiet_night_starts)
                .sum();
            let weights = [1.5, 1.0, 0.75];
            let weighted_score = weeks
                .iter()
                .enumerate()
                .map(|(index, week)| {
                    let weight = weights.get(index).copied().unwrap_or(0.5);
                    weight * week.projected_points_delta
                        + 0.25 * week.candidate_quiet_night_starts as f64
                })
                .sum::<f64>();
            let injury_replacement = input.injury_replacement_keys.contains(drop_key);
            let week_one = &weeks[0];
            let posture = if input.acquisitions_remaining == 0 {
                "blocked: no acquisitions remaining"
            } else if injury_replacement && projected_points_delta > 0.0 {
                "replace injury"
            } else if week_one.projected_points_delta >= 4.0
                || (projected_points_delta >= 8.0 && usable_starts_delta > 0)
            {
                "review now"
            } else {
                "hold acquisition"
            };

            rows.push(FantasyReplacementOptionRow {
                rank: 0,
                add_player_key: candidate.player_key.clone(),
                add_player: candidate.player.clone(),
                add_nhl_team: candidate.nhl_team.clone(),
                add_positions: candidate.positions.clone(),
                drop_player_key: drop_player.player_key.clone(),
                drop_player: drop_player.player.clone(),
                drop_nhl_team: drop_player.nhl_team.clone(),
                injury_replacement,
                usable_starts_delta,
                projected_points_delta,
                candidate_usable_starts,
                candidate_quiet_night_starts,
                weighted_score,
                posture: posture.to_owned(),
                weeks,
            });
        }
    }

    rows.sort_by(|a, b| {
        b.injury_replacement
            .cmp(&a.injury_replacement)
            .then_with(|| b.weighted_score.total_cmp(&a.weighted_score))
            .then_with(|| b.usable_starts_delta.cmp(&a.usable_starts_delta))
            .then_with(|| a.add_player_key.cmp(&b.add_player_key))
            .then_with(|| a.drop_player_key.cmp(&b.drop_player_key))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index + 1;
    }

    Ok(FantasyReplacementLookaheadView {
        schema: FANTASY_REPLACEMENT_LOOKAHEAD_SCHEMA.to_owned(),
        fantasy_team: input.fantasy_team,
        start,
        end,
        weeks: input.weeks,
        acquisitions_remaining: input.acquisitions_remaining,
        rows,
        disclosures: vec![
            "Projected value per game is a descriptive completed-season rate, not a guarantee of future production.".to_owned(),
            "Each option reruns the legal daily lineup after the same add/drop across every displayed week.".to_owned(),
            "Quiet-night starts occur on dates with no more than the configured number of NHL games.".to_owned(),
            "Verify current injury designations, waiver timing, lineup locks, and platform eligibility in Yahoo before acting.".to_owned(),
        ],
    })
}

fn simulate_weeks(
    rules: &FantasyAssistantRules,
    roster: &[FantasyReplacementPlayerInput],
    candidate_key: Option<&str>,
    start: NaiveDate,
    weeks: usize,
    off_night_max_games: usize,
    games_by_date: &BTreeMap<NaiveDate, Vec<&FantasyScheduleGameInput>>,
) -> Result<Vec<WeekTotals>, String> {
    let mut totals = (0..weeks)
        .map(|_| WeekTotals::default())
        .collect::<Vec<_>>();
    for day_index in 0..weeks * 7 {
        let date = start + Duration::days(day_index as i64);
        let slate = games_by_date.get(&date).cloned().unwrap_or_default();
        let teams_playing = slate
            .iter()
            .flat_map(|game| [&game.away_team, &game.home_team])
            .map(|team| team.trim().to_ascii_uppercase())
            .collect::<BTreeSet<_>>();
        let lineup = build_fantasy_daily_lineup(
            rules.clone(),
            roster
                .iter()
                .map(|player| FantasyLineupPlayerInput {
                    player_key: player.player_key.clone(),
                    display_name: player.player.clone(),
                    nhl_team: player.nhl_team.clone(),
                    platform_positions: player.positions.clone(),
                    projected_value: player.projected_value_per_game,
                    has_game: teams_playing.contains(&player.nhl_team.trim().to_ascii_uppercase()),
                    status: FantasyPlayerAvailabilityStatus::Healthy,
                    locked_slot: None,
                    locked: false,
                })
                .collect(),
        )?;
        let week = &mut totals[day_index / 7];
        for row in lineup.active.iter().filter(|row| row.has_game) {
            week.starts += 1;
            week.points += row.projected_value;
            if candidate_key.is_some_and(|key| row.player_key == key) {
                week.candidate_starts += 1;
                if slate.len() <= off_night_max_games {
                    week.candidate_quiet_starts += 1;
                }
            }
        }
    }
    Ok(totals)
}

#[cfg(test)]
mod tests {
    use super::super::fantasy_assistant::FantasyActiveSlotKind;
    use super::*;

    fn player(key: &str, team: &str, value: f64) -> FantasyReplacementPlayerInput {
        FantasyReplacementPlayerInput {
            player_key: key.to_owned(),
            player: key.to_owned(),
            nhl_team: team.to_owned(),
            positions: vec![Position::RightWing],
            projected_value_per_game: value,
        }
    }

    fn game(date: NaiveDate, team: &str) -> FantasyScheduleGameInput {
        FantasyScheduleGameInput {
            game_id: date.format("%Y%m%d").to_string().parse().unwrap(),
            date,
            away_team: team.to_owned(),
            home_team: "XXX".to_owned(),
        }
    }

    #[test]
    fn replacement_reports_weekly_legal_start_and_points_delta() {
        let start = NaiveDate::from_ymd_opt(2026, 10, 5).unwrap();
        let mut rules = FantasyAssistantRules::configured_2026();
        rules.active_slots = BTreeMap::from([(FantasyActiveSlotKind::RightWing, 1)]);
        rules.bench_slots = 1;
        rules.ir_slots = 0;
        rules.ir_plus_slots = 0;
        let view = build_fantasy_replacement_lookahead(FantasyReplacementLookaheadInput {
            fantasy_team: "Test".to_owned(),
            start,
            weeks: 1,
            off_night_max_games: 4,
            acquisitions_remaining: 1,
            rules,
            roster: vec![player("starter", "AAA", 8.0), player("drop", "BBB", 2.0)],
            drop_player_keys: BTreeSet::from(["drop".to_owned()]),
            injury_replacement_keys: BTreeSet::new(),
            candidates: vec![player("add", "CCC", 5.0)],
            games: vec![game(start, "AAA"), game(start + Duration::days(1), "CCC")],
        })
        .unwrap();

        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].usable_starts_delta, 1);
        assert_eq!(view.rows[0].projected_points_delta, 5.0);
        assert_eq!(view.rows[0].candidate_quiet_night_starts, 1);
        assert_eq!(view.rows[0].posture, "review now");
    }
}
