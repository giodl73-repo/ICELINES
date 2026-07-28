//! Official NHL shift-chart intervals and shared deployment aggregation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

pub const SHIFT_OVERLAP_SCHEMA: &str = "nhl_shift_overlap.v1";
pub const SHIFT_CHART_SOURCE: &str = "NHL stats REST /shiftcharts";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfficialShiftChartRow {
    pub game_id: u64,
    pub player_id: u32,
    #[serde(default)]
    pub first_name: String,
    #[serde(default)]
    pub last_name: String,
    #[serde(default)]
    pub team_abbrev: String,
    pub period: u8,
    pub start_time: String,
    pub end_time: String,
    /// NHL leaves duration null on a small number of boundary/event rows. The
    /// aggregator derives duration from start/end and never trusts this field.
    pub duration: Option<String>,
    pub shift_number: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OfficialShiftChartResponse {
    pub data: Vec<OfficialShiftChartRow>,
    pub total: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShiftOverlapPlayerRow {
    pub player_id: u32,
    pub display_name: String,
    pub games: u32,
    pub ice_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShiftOverlapPairRow {
    pub player_one_id: u32,
    pub player_two_id: u32,
    pub shared_games: u32,
    pub shared_seconds: u64,
    /// Shared seconds divided by the lower of the players' total ice seconds.
    pub lower_player_overlap_pct: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShiftOverlapTrioRow {
    pub player_ids: [u32; 3],
    pub shared_games: u32,
    pub shared_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShiftOverlapReport {
    pub schema: String,
    pub source: String,
    pub team: String,
    pub season: u32,
    pub games_requested: usize,
    pub games_loaded: usize,
    pub players: Vec<ShiftOverlapPlayerRow>,
    pub pairs: Vec<ShiftOverlapPairRow>,
    pub trios: Vec<ShiftOverlapTrioRow>,
    pub disclosures: Vec<String>,
}

pub fn build_shift_overlap_report(
    team: &str,
    season: u32,
    games_requested: usize,
    player_ids: &BTreeSet<u32>,
    games: &[(u64, Vec<OfficialShiftChartRow>)],
) -> Result<ShiftOverlapReport, String> {
    let team = team.trim().to_ascii_uppercase();
    if team.len() != 3 || player_ids.is_empty() {
        return Err("shift overlap requires a three-letter team and roster players".to_owned());
    }
    let mut names = BTreeMap::<u32, String>::new();
    let mut player_games = BTreeMap::<u32, u32>::new();
    let mut player_seconds = BTreeMap::<u32, u64>::new();
    let mut pair_games = BTreeMap::<(u32, u32), u32>::new();
    let mut pair_seconds = BTreeMap::<(u32, u32), u64>::new();
    let mut trio_games = BTreeMap::<[u32; 3], u32>::new();
    let mut trio_seconds = BTreeMap::<[u32; 3], u64>::new();

    for (_, rows) in games {
        let mut boundaries = BTreeMap::<(u8, u16), Vec<(u32, bool)>>::new();
        let mut appeared = BTreeSet::new();
        for row in rows.iter().filter(|row| {
            row.team_abbrev.eq_ignore_ascii_case(&team) && player_ids.contains(&row.player_id)
        }) {
            let start = parse_clock(&row.start_time)
                .ok_or_else(|| format!("invalid shift start time {}", row.start_time))?;
            let end = parse_clock(&row.end_time)
                .ok_or_else(|| format!("invalid shift end time {}", row.end_time))?;
            if end <= start || row.period == 0 {
                continue;
            }
            names.insert(
                row.player_id,
                format!("{} {}", row.first_name.trim(), row.last_name.trim()),
            );
            appeared.insert(row.player_id);
            boundaries
                .entry((row.period, start))
                .or_default()
                .push((row.player_id, true));
            boundaries
                .entry((row.period, end))
                .or_default()
                .push((row.player_id, false));
        }
        for player in appeared {
            *player_games.entry(player).or_default() += 1;
        }
        let mut game_pairs = BTreeSet::new();
        let mut game_trios = BTreeSet::new();
        let mut active = BTreeSet::<u32>::new();
        let mut prior: Option<(u8, u16)> = None;
        for (point, changes) in boundaries {
            if let Some((prior_period, prior_second)) = prior {
                if prior_period == point.0 && point.1 > prior_second {
                    let duration = u64::from(point.1 - prior_second);
                    let active = active.iter().copied().collect::<Vec<_>>();
                    for player in &active {
                        *player_seconds.entry(*player).or_default() += duration;
                    }
                    for first in 0..active.len() {
                        for second in first + 1..active.len() {
                            let pair = (active[first], active[second]);
                            *pair_seconds.entry(pair).or_default() += duration;
                            game_pairs.insert(pair);
                            for third in second + 1..active.len() {
                                let trio = [active[first], active[second], active[third]];
                                *trio_seconds.entry(trio).or_default() += duration;
                                game_trios.insert(trio);
                            }
                        }
                    }
                }
            }
            // End before start at an identical boundary avoids phantom overlap.
            for (player, _) in changes.iter().filter(|(_, start)| !*start) {
                active.remove(player);
            }
            for (player, _) in changes.iter().filter(|(_, start)| *start) {
                active.insert(*player);
            }
            prior = Some(point);
        }
        for pair in game_pairs {
            *pair_games.entry(pair).or_default() += 1;
        }
        for trio in game_trios {
            *trio_games.entry(trio).or_default() += 1;
        }
    }

    let mut players = player_seconds
        .iter()
        .map(|(player_id, ice_seconds)| ShiftOverlapPlayerRow {
            player_id: *player_id,
            display_name: names
                .get(player_id)
                .cloned()
                .unwrap_or_else(|| format!("Player {player_id}")),
            games: player_games.get(player_id).copied().unwrap_or(0),
            ice_seconds: *ice_seconds,
        })
        .collect::<Vec<_>>();
    players.sort_by(|a, b| {
        b.ice_seconds
            .cmp(&a.ice_seconds)
            .then(a.player_id.cmp(&b.player_id))
    });
    let mut pairs = pair_seconds
        .into_iter()
        .map(|((player_one_id, player_two_id), shared_seconds)| {
            let lower = player_seconds[&player_one_id].min(player_seconds[&player_two_id]);
            ShiftOverlapPairRow {
                player_one_id,
                player_two_id,
                shared_games: pair_games[&(player_one_id, player_two_id)],
                shared_seconds,
                lower_player_overlap_pct: if lower == 0 {
                    0.0
                } else {
                    shared_seconds as f64 / lower as f64
                },
            }
        })
        .collect::<Vec<_>>();
    pairs.sort_by_key(|row| std::cmp::Reverse(row.shared_seconds));
    let mut trios = trio_seconds
        .into_iter()
        .map(|(player_ids, shared_seconds)| ShiftOverlapTrioRow {
            player_ids,
            shared_games: trio_games[&player_ids],
            shared_seconds,
        })
        .collect::<Vec<_>>();
    trios.sort_by_key(|row| std::cmp::Reverse(row.shared_seconds));
    Ok(ShiftOverlapReport {
        schema: SHIFT_OVERLAP_SCHEMA.to_owned(),
        source: SHIFT_CHART_SOURCE.to_owned(),
        team,
        season,
        games_requested,
        games_loaded: games.len(),
        players,
        pairs,
        trios,
        disclosures: vec![
            "Intervals are official all-situations shift deployment; power play, penalty kill, and even strength are not yet separated.".to_owned(),
            "Shared ice establishes deployment affinity, not whether either player causally improved goals, shots, or expected goals.".to_owned(),
        ],
    })
}

fn parse_clock(value: &str) -> Option<u16> {
    let (minutes, seconds) = value.split_once(':')?;
    let minutes = minutes.parse::<u16>().ok()?;
    let seconds = seconds.parse::<u16>().ok()?;
    (seconds < 60).then_some(minutes * 60 + seconds)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(player_id: u32, start: &str, end: &str) -> OfficialShiftChartRow {
        OfficialShiftChartRow {
            game_id: 1,
            player_id,
            first_name: "Player".to_owned(),
            last_name: player_id.to_string(),
            team_abbrev: "NYR".to_owned(),
            period: 1,
            start_time: start.to_owned(),
            end_time: end.to_owned(),
            duration: Some("00:30".to_owned()),
            shift_number: 1,
        }
    }

    #[test]
    fn aggregates_exact_pair_and_trio_interval_overlap() {
        let report = build_shift_overlap_report(
            "NYR",
            20252026,
            1,
            &BTreeSet::from([1, 2, 3]),
            &[(
                (1),
                vec![
                    row(1, "00:00", "01:00"),
                    row(2, "00:20", "00:50"),
                    row(3, "00:30", "00:40"),
                ],
            )],
        )
        .unwrap();
        assert_eq!(report.pairs[0].shared_seconds, 30);
        assert_eq!(report.trios[0].shared_seconds, 10);
        assert_eq!(report.games_loaded, 1);
    }

    #[test]
    fn accepts_live_boundary_rows_with_null_duration() {
        let response: OfficialShiftChartResponse = serde_json::from_str(
            r#"{"data":[{"gameId":2025020002,"playerId":8471215,"firstName":"Evgeni","lastName":"Malkin","teamAbbrev":"PIT","period":1,"startTime":"20:00","endTime":"20:00","duration":null,"shiftNumber":20}],"total":1}"#,
        )
        .unwrap();
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].duration, None);
    }
}
