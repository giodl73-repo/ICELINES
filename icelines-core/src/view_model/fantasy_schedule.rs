use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, Duration, NaiveDate};
use serde::{Deserialize, Serialize};

pub const FANTASY_SCHEDULE_SCHEMA: &str = "fantasy_schedule.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasyScheduleGameInput {
    pub game_id: u64,
    pub date: NaiveDate,
    pub away_team: String,
    pub home_team: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyDailySlateRow {
    pub date: NaiveDate,
    pub games: usize,
    pub quiet: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyWeeklyTeamRow {
    pub team: String,
    pub games: usize,
    pub quiet_slate_games: usize,
    pub scarcity_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScheduleWeekRow {
    pub week_start: NaiveDate,
    pub week_end: NaiveDate,
    pub league_games: usize,
    pub quiet_dates: Vec<NaiveDate>,
    pub teams: Vec<FantasyWeeklyTeamRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScheduleTeamRow {
    pub team: String,
    pub games: usize,
    pub quiet_slate_games: usize,
    pub scarcity_score: f64,
    pub equivalence_class: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScheduleClassRow {
    pub class_id: usize,
    pub teams: Vec<String>,
    pub average_within_overlap_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScheduleOverlapRow {
    pub team_a: String,
    pub team_b: String,
    pub shared_game_dates: usize,
    pub overlap_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScheduleComplementRow {
    pub team: String,
    pub average_roster_overlap_pct: f64,
    pub quiet_slate_games: usize,
    pub equivalence_class: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyRosterScheduleView {
    pub teams: Vec<String>,
    pub team_player_counts: BTreeMap<String, usize>,
    pub roster_player_slots: usize,
    pub collision_dates: usize,
    pub total_team_games: usize,
    pub distinct_active_dates: usize,
    pub utilization_pct: f64,
    pub highest_overlap_pairs: Vec<FantasyScheduleOverlapRow>,
    pub best_complements: Vec<FantasyScheduleComplementRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyScheduleView {
    pub schema: String,
    pub season: u32,
    pub game_count: usize,
    pub season_start: NaiveDate,
    pub season_end: NaiveDate,
    pub off_night_max_games: usize,
    pub daily_slates: Vec<FantasyDailySlateRow>,
    pub weeks: Vec<FantasyScheduleWeekRow>,
    pub teams: Vec<FantasyScheduleTeamRow>,
    pub equivalence_classes: Vec<FantasyScheduleClassRow>,
    pub roster: Option<FantasyRosterScheduleView>,
    pub disclosures: Vec<String>,
}

pub fn build_fantasy_schedule_view(
    games: Vec<FantasyScheduleGameInput>,
    season: u32,
    off_night_max_games: usize,
    class_count: usize,
    roster_teams: Vec<String>,
) -> Result<FantasyScheduleView, String> {
    if games.is_empty() {
        return Err("fantasy schedule input has no regular-season games".to_owned());
    }
    if off_night_max_games == 0 {
        return Err("off-night threshold must be at least one game".to_owned());
    }

    let mut deduped = BTreeMap::new();
    for mut game in games {
        game.away_team = game.away_team.trim().to_ascii_uppercase();
        game.home_team = game.home_team.trim().to_ascii_uppercase();
        deduped.entry(game.game_id).or_insert(game);
    }
    let games: Vec<_> = deduped.into_values().collect();
    let season_start = games.iter().map(|game| game.date).min().unwrap();
    let season_end = games.iter().map(|game| game.date).max().unwrap();

    let mut games_by_date: BTreeMap<NaiveDate, Vec<&FantasyScheduleGameInput>> = BTreeMap::new();
    let mut team_dates: BTreeMap<String, BTreeSet<NaiveDate>> = BTreeMap::new();
    for game in &games {
        games_by_date.entry(game.date).or_default().push(game);
        team_dates
            .entry(game.away_team.clone())
            .or_default()
            .insert(game.date);
        team_dates
            .entry(game.home_team.clone())
            .or_default()
            .insert(game.date);
    }

    let daily_slates = games_by_date
        .iter()
        .map(|(date, games)| FantasyDailySlateRow {
            date: *date,
            games: games.len(),
            quiet: games.len() <= off_night_max_games,
        })
        .collect::<Vec<_>>();

    let teams = team_dates.keys().cloned().collect::<Vec<_>>();
    let overlaps = pairwise_overlaps(&team_dates);
    let classes = build_classes(&teams, &overlaps, class_count.clamp(1, teams.len()));
    let class_by_team = classes
        .iter()
        .flat_map(|class| {
            class
                .teams
                .iter()
                .cloned()
                .map(move |team| (team, class.class_id))
        })
        .collect::<BTreeMap<_, _>>();

    let mut team_rows = teams
        .iter()
        .map(|team| {
            let dates = &team_dates[team];
            let quiet_slate_games = dates
                .iter()
                .filter(|date| games_by_date[*date].len() <= off_night_max_games)
                .count();
            FantasyScheduleTeamRow {
                team: team.clone(),
                games: dates.len(),
                quiet_slate_games,
                scarcity_score: scarcity_score(dates, &games_by_date),
                equivalence_class: class_by_team[team],
            }
        })
        .collect::<Vec<_>>();
    team_rows.sort_by(|a, b| {
        b.quiet_slate_games
            .cmp(&a.quiet_slate_games)
            .then_with(|| b.scarcity_score.total_cmp(&a.scarcity_score))
            .then_with(|| a.team.cmp(&b.team))
    });

    let mut by_week: BTreeMap<NaiveDate, Vec<&FantasyScheduleGameInput>> = BTreeMap::new();
    for game in &games {
        by_week.entry(monday_of(game.date)).or_default().push(game);
    }
    let weeks = by_week
        .into_iter()
        .map(|(week_start, week_games)| {
            let mut counts: BTreeMap<String, (usize, usize, f64)> = BTreeMap::new();
            for game in &week_games {
                let slate_games = games_by_date[&game.date].len();
                for team in [&game.away_team, &game.home_team] {
                    let entry = counts.entry(team.clone()).or_default();
                    entry.0 += 1;
                    entry.1 += usize::from(slate_games <= off_night_max_games);
                    entry.2 += 1.0 / slate_games as f64;
                }
            }
            let mut rows = counts
                .into_iter()
                .map(
                    |(team, (games, quiet_slate_games, scarcity_score))| FantasyWeeklyTeamRow {
                        team,
                        games,
                        quiet_slate_games,
                        scarcity_score,
                    },
                )
                .collect::<Vec<_>>();
            rows.sort_by(|a, b| {
                b.games
                    .cmp(&a.games)
                    .then_with(|| b.quiet_slate_games.cmp(&a.quiet_slate_games))
                    .then_with(|| b.scarcity_score.total_cmp(&a.scarcity_score))
                    .then_with(|| a.team.cmp(&b.team))
            });
            let quiet_dates = (0..7)
                .map(|day| week_start + Duration::days(day))
                .filter(|date| {
                    games_by_date
                        .get(date)
                        .is_some_and(|games| games.len() <= off_night_max_games)
                })
                .collect();
            FantasyScheduleWeekRow {
                week_start,
                week_end: week_start + Duration::days(6),
                league_games: week_games.len(),
                quiet_dates,
                teams: rows,
            }
        })
        .collect();

    let roster = build_roster_view(
        roster_teams,
        &team_dates,
        &team_rows,
        &overlaps,
        &class_by_team,
    );

    Ok(FantasyScheduleView {
        schema: FANTASY_SCHEDULE_SCHEMA.to_owned(),
        season,
        game_count: games.len(),
        season_start,
        season_end,
        off_night_max_games,
        daily_slates,
        weeks,
        teams: team_rows,
        equivalence_classes: classes,
        roster,
        disclosures: vec![
            format!("A quiet-slate game is one played on a date with {off_night_max_games} or fewer NHL games."),
            "Scarcity score sums 1 / league games on each date; higher values mean more usable off-night volume.".to_owned(),
            "Schedule equivalence classes group teams with similar exact game dates; draft across classes to reduce same-night collisions.".to_owned(),
            "Classes describe calendar fit only and do not score player quality, injuries, or lineup limits.".to_owned(),
        ],
    })
}

fn monday_of(date: NaiveDate) -> NaiveDate {
    date - Duration::days(date.weekday().num_days_from_monday() as i64)
}

fn scarcity_score(
    dates: &BTreeSet<NaiveDate>,
    games_by_date: &BTreeMap<NaiveDate, Vec<&FantasyScheduleGameInput>>,
) -> f64 {
    dates
        .iter()
        .map(|date| 1.0 / games_by_date[date].len() as f64)
        .sum()
}

fn pairwise_overlaps(
    team_dates: &BTreeMap<String, BTreeSet<NaiveDate>>,
) -> BTreeMap<(String, String), FantasyScheduleOverlapRow> {
    let teams = team_dates.keys().collect::<Vec<_>>();
    let mut out = BTreeMap::new();
    for (index, team_a) in teams.iter().enumerate() {
        for team_b in teams.iter().skip(index + 1) {
            let shared = team_dates[*team_a]
                .intersection(&team_dates[*team_b])
                .count();
            let denominator = team_dates[*team_a].len().min(team_dates[*team_b].len());
            let overlap_pct = if denominator == 0 {
                0.0
            } else {
                shared as f64 / denominator as f64 * 100.0
            };
            out.insert(
                ((*team_a).clone(), (*team_b).clone()),
                FantasyScheduleOverlapRow {
                    team_a: (*team_a).clone(),
                    team_b: (*team_b).clone(),
                    shared_game_dates: shared,
                    overlap_pct,
                },
            );
        }
    }
    out
}

fn overlap_pct(
    a: &str,
    b: &str,
    overlaps: &BTreeMap<(String, String), FantasyScheduleOverlapRow>,
) -> f64 {
    if a == b {
        return 100.0;
    }
    let key = if a < b {
        (a.to_owned(), b.to_owned())
    } else {
        (b.to_owned(), a.to_owned())
    };
    overlaps.get(&key).map_or(0.0, |row| row.overlap_pct)
}

fn build_classes(
    teams: &[String],
    overlaps: &BTreeMap<(String, String), FantasyScheduleOverlapRow>,
    class_count: usize,
) -> Vec<FantasyScheduleClassRow> {
    let capacity = teams.len().div_ceil(class_count);
    let mut seeds = Vec::new();
    if let Some(first) = teams.iter().max_by(|a, b| {
        let a_total: f64 = teams
            .iter()
            .map(|team| overlap_pct(a, team, overlaps))
            .sum();
        let b_total: f64 = teams
            .iter()
            .map(|team| overlap_pct(b, team, overlaps))
            .sum();
        a_total.total_cmp(&b_total).then_with(|| b.cmp(a))
    }) {
        seeds.push(first.clone());
    }
    while seeds.len() < class_count {
        let next = teams
            .iter()
            .filter(|team| !seeds.contains(team))
            .min_by(|a, b| {
                let a_nearest = seeds
                    .iter()
                    .map(|seed| overlap_pct(a, seed, overlaps))
                    .fold(0.0, f64::max);
                let b_nearest = seeds
                    .iter()
                    .map(|seed| overlap_pct(b, seed, overlaps))
                    .fold(0.0, f64::max);
                a_nearest.total_cmp(&b_nearest).then_with(|| a.cmp(b))
            })
            .cloned();
        if let Some(next) = next {
            seeds.push(next);
        } else {
            break;
        }
    }

    let mut groups = seeds.into_iter().map(|seed| vec![seed]).collect::<Vec<_>>();
    let mut remaining = teams
        .iter()
        .filter(|team| !groups.iter().flatten().any(|member| member == *team))
        .cloned()
        .collect::<BTreeSet<_>>();
    while !remaining.is_empty() {
        let mut best: Option<(String, usize, f64)> = None;
        for team in &remaining {
            for (index, group) in groups.iter().enumerate() {
                if group.len() >= capacity {
                    continue;
                }
                let similarity = group
                    .iter()
                    .map(|member| overlap_pct(team, member, overlaps))
                    .sum::<f64>()
                    / group.len() as f64;
                if best.as_ref().is_none_or(|current| {
                    similarity > current.2
                        || (similarity == current.2
                            && (team.as_str(), index) < (current.0.as_str(), current.1))
                }) {
                    best = Some((team.clone(), index, similarity));
                }
            }
        }
        let Some((team, index, _)) = best else { break };
        remaining.remove(&team);
        groups[index].push(team);
    }

    groups
        .into_iter()
        .enumerate()
        .map(|(index, mut teams)| {
            teams.sort();
            let mut sum = 0.0;
            let mut pairs = 0usize;
            for a in 0..teams.len() {
                for b in (a + 1)..teams.len() {
                    sum += overlap_pct(&teams[a], &teams[b], overlaps);
                    pairs += 1;
                }
            }
            FantasyScheduleClassRow {
                class_id: index + 1,
                teams,
                average_within_overlap_pct: if pairs == 0 { 0.0 } else { sum / pairs as f64 },
            }
        })
        .collect()
}

fn build_roster_view(
    roster_teams: Vec<String>,
    team_dates: &BTreeMap<String, BTreeSet<NaiveDate>>,
    team_rows: &[FantasyScheduleTeamRow],
    overlaps: &BTreeMap<(String, String), FantasyScheduleOverlapRow>,
    class_by_team: &BTreeMap<String, usize>,
) -> Option<FantasyRosterScheduleView> {
    let roster_teams = roster_teams
        .into_iter()
        .map(|team| team.trim().to_ascii_uppercase())
        .filter(|team| team_dates.contains_key(team))
        .collect::<Vec<_>>();
    if roster_teams.is_empty() {
        return None;
    }
    let mut team_player_counts = BTreeMap::<String, usize>::new();
    for team in &roster_teams {
        *team_player_counts.entry(team.clone()).or_default() += 1;
    }
    let teams = team_player_counts.keys().cloned().collect::<Vec<_>>();
    let mut date_counts = BTreeMap::<NaiveDate, usize>::new();
    for team in &teams {
        for date in &team_dates[team] {
            *date_counts.entry(*date).or_default() += team_player_counts[team];
        }
    }
    let total_team_games = teams
        .iter()
        .map(|team| team_dates[team].len() * team_player_counts[team])
        .sum();
    let distinct_active_dates = date_counts.len();
    let collision_dates = date_counts.values().filter(|count| **count > 1).count();
    let utilization_pct = if total_team_games == 0 {
        0.0
    } else {
        distinct_active_dates as f64 / total_team_games as f64 * 100.0
    };

    let mut pair_rows = Vec::new();
    for a in 0..teams.len() {
        for b in (a + 1)..teams.len() {
            let key = (teams[a].clone(), teams[b].clone());
            if let Some(row) = overlaps.get(&key) {
                pair_rows.push(row.clone());
            }
        }
    }
    pair_rows.sort_by(|a, b| {
        b.overlap_pct
            .total_cmp(&a.overlap_pct)
            .then_with(|| a.team_a.cmp(&b.team_a))
            .then_with(|| a.team_b.cmp(&b.team_b))
    });
    pair_rows.truncate(10);

    let roster_set = teams.iter().cloned().collect::<BTreeSet<_>>();
    let mut complements = team_rows
        .iter()
        .filter(|row| !roster_set.contains(&row.team))
        .map(|row| FantasyScheduleComplementRow {
            team: row.team.clone(),
            average_roster_overlap_pct: roster_teams
                .iter()
                .map(|team| overlap_pct(team, &row.team, overlaps))
                .sum::<f64>()
                / roster_teams.len() as f64,
            quiet_slate_games: row.quiet_slate_games,
            equivalence_class: class_by_team[&row.team],
        })
        .collect::<Vec<_>>();
    complements.sort_by(|a, b| {
        a.average_roster_overlap_pct
            .total_cmp(&b.average_roster_overlap_pct)
            .then_with(|| b.quiet_slate_games.cmp(&a.quiet_slate_games))
            .then_with(|| a.team.cmp(&b.team))
    });
    complements.truncate(10);

    Some(FantasyRosterScheduleView {
        teams,
        team_player_counts,
        roster_player_slots: roster_teams.len(),
        collision_dates,
        total_team_games,
        distinct_active_dates,
        utilization_pct,
        highest_overlap_pairs: pair_rows,
        best_complements: complements,
    })
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

    #[test]
    fn weeks_are_monday_through_sunday_and_quiet_games_are_counted() {
        let view = build_fantasy_schedule_view(
            vec![
                game(1, "2026-10-05", "NYR", "BOS"),
                game(2, "2026-10-06", "NYR", "MTL"),
                game(3, "2026-10-06", "BOS", "MTL"),
            ],
            20262027,
            1,
            2,
            vec!["NYR".to_owned(), "BOS".to_owned()],
        )
        .unwrap();
        assert_eq!(view.game_count, 3);
        assert_eq!(view.weeks[0].week_start.to_string(), "2026-10-05");
        assert_eq!(view.weeks[0].week_end.to_string(), "2026-10-11");
        let nyr = view.teams.iter().find(|row| row.team == "NYR").unwrap();
        assert_eq!(nyr.games, 2);
        assert_eq!(nyr.quiet_slate_games, 1);
        assert!(view.roster.is_some());
    }

    #[test]
    fn duplicate_game_ids_are_not_double_counted() {
        let input = game(1, "2026-10-05", "NYR", "BOS");
        let view =
            build_fantasy_schedule_view(vec![input.clone(), input], 20262027, 4, 2, Vec::new())
                .unwrap();
        assert_eq!(view.game_count, 1);
    }

    #[test]
    fn repeated_roster_team_preserves_player_collision_weight() {
        let view = build_fantasy_schedule_view(
            vec![game(1, "2026-10-05", "NYR", "BOS")],
            20262027,
            4,
            2,
            vec!["NYR".to_owned(), "NYR".to_owned(), "BOS".to_owned()],
        )
        .unwrap();
        let roster = view.roster.unwrap();
        assert_eq!(roster.roster_player_slots, 3);
        assert_eq!(roster.team_player_counts["NYR"], 2);
        assert_eq!(roster.total_team_games, 3);
        assert_eq!(roster.collision_dates, 1);
    }
}
