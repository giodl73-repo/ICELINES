//! Literal coverage/timebase audit of the public BDC-2026 tracking CSV.
//! No possession detector, velocity estimator, NHL identity mapping or grades.
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: String,
    pub source_sha256: String,
    pub rows: u64,
    pub frames: u64,
    pub periods: Vec<u32>,
    pub clock_first: String,
    pub clock_last: String,
    pub adjacent_frames_same_game_clock: u64,
    pub adjacent_frames_decreasing_game_clock: u64,
    pub adjacent_frames_increasing_game_clock: u64,
    pub game_clock_values: usize,
    pub frames_per_clock_histogram: BTreeMap<u64, u64>,
    pub numeric_image_suffix_gaps: BTreeMap<u64, u64>,
    pub non_increasing_image_suffix_pairs: u64,
    pub frames_without_puck: u64,
    pub frames_with_multiple_pucks: u64,
    pub frames_without_puck_position: u64,
    pub player_rows: u64,
    pub puck_rows: u64,
    pub unidentified_jersey_rows: u64,
    pub frames_with_unidentified_players: u64,
    pub duplicate_player_id_frames: u64,
    pub player_count_histogram: BTreeMap<u64, u64>,
    pub positioned_player_count_histogram: BTreeMap<u64, u64>,
    pub tracked_actor_keys: usize,
    pub team_player_ids_with_multiple_jersey_labels: Vec<String>,
    pub player_rows_outside_rink_rectangle: u64,
    pub puck_rows_outside_rink_rectangle: u64,
    pub rows_with_missing_coordinates: u64,
    pub rows_with_invalid_coordinates: u64,
    pub player_rows_missing_xy: u64,
    pub puck_rows_missing_xy: u64,
    pub player_z_values: BTreeMap<String, u64>,
    pub puck_z_values: BTreeMap<String, u64>,
    pub limits: Vec<String>,
}
#[derive(Default)]
struct Frame {
    id: String,
    period: u32,
    clock: String,
    seconds: u32,
    players: u64,
    pucks: u64,
    positioned_players: u64,
    positioned_pucks: u64,
    unidentified: bool,
    duplicate: bool,
    actors: BTreeSet<(String, String)>,
}
fn clock(text: &str) -> Result<u32, String> {
    let (m, s) = text
        .split_once(':')
        .ok_or("Malformed whole-second game clock")?;
    let m: u32 = m.parse().map_err(|_| "Invalid clock minutes")?;
    let s: u32 = s.parse().map_err(|_| "Invalid clock seconds")?;
    if s >= 60 || m > 60 {
        return Err("Clock outside supported range".into());
    }
    Ok(m * 60 + s)
}
fn finish(frame: &Frame, report: &mut Report, clocks: &mut BTreeMap<(u32, String), u64>) {
    report.frames += 1;
    report.frames_without_puck += u64::from(frame.pucks == 0);
    report.frames_with_multiple_pucks += u64::from(frame.pucks > 1);
    report.frames_without_puck_position += u64::from(frame.positioned_pucks == 0);
    report.frames_with_unidentified_players += u64::from(frame.unidentified);
    report.duplicate_player_id_frames += u64::from(frame.duplicate);
    *report
        .player_count_histogram
        .entry(frame.players)
        .or_default() += 1;
    *report
        .positioned_player_count_histogram
        .entry(frame.positioned_players)
        .or_default() += 1;
    *clocks
        .entry((frame.period, frame.clock.clone()))
        .or_default() += 1;
}
pub fn audit(raw: &[u8]) -> Result<Report, String> {
    let mut reader = csv::Reader::from_reader(raw);
    let headers = reader.headers().map_err(|e| e.to_string())?.clone();
    let expected = [
        "Image Id",
        "Period",
        "Game Clock",
        "Player or Puck",
        "Team",
        "Player Id",
        "Player Jersey Number",
        "Rink Location X (Feet)",
        "Rink Location Y (Feet)",
        "Rink Location Z (Feet)",
        "Goal Score",
    ];
    if headers.iter().collect::<Vec<_>>() != expected {
        return Err("Expected exact BDC-2026 tracking headers; no silent schema aliases".into());
    }
    let mut report = Report {
        schema: "rinkcraft.public-tracking-feasibility.v1".into(), source_sha256: format!("{:x}", Sha256::digest(raw)),
        rows: 0, frames: 0, periods: vec![], clock_first: String::new(), clock_last: String::new(),
        adjacent_frames_same_game_clock: 0, adjacent_frames_decreasing_game_clock: 0, adjacent_frames_increasing_game_clock: 0,
        game_clock_values: 0, frames_per_clock_histogram: BTreeMap::new(), numeric_image_suffix_gaps: BTreeMap::new(), non_increasing_image_suffix_pairs: 0,
        frames_without_puck: 0, frames_with_multiple_pucks: 0, player_rows: 0, puck_rows: 0, unidentified_jersey_rows: 0,
        frames_with_unidentified_players: 0, duplicate_player_id_frames: 0, player_count_histogram: BTreeMap::new(), tracked_actor_keys: 0,
        team_player_ids_with_multiple_jersey_labels: vec![], player_rows_outside_rink_rectangle: 0, puck_rows_outside_rink_rectangle: 0,
        rows_with_missing_coordinates: 0, rows_with_invalid_coordinates: 0,
        frames_without_puck_position: 0, positioned_player_count_histogram: BTreeMap::new(),
        player_rows_missing_xy: 0, puck_rows_missing_xy: 0,
        player_z_values: BTreeMap::new(), puck_z_values: BTreeMap::new(),
        limits: vec![
            "Anonymized competition and real identities unverified; never map these IDs onto NHL players.".into(),
            "Whole-second game clocks and image suffixes do not authenticate capture FPS, elapsed video time, stoppages or subsecond alignment.".into(),
            "Coordinates are broadcast-derived estimates; calibration, identity continuity, goalie roles and complete visibility remain unverified.".into(),
            "No inferred carrier, check/contact, pass reception, intended target, feasible option set, defensive duty or capability score.".into(),
            "Out-of-rectangle rows are diagnostics, not automatically errors; players may be outside the playing surface.".into(),
        ],
    };
    let mut frame: Option<Frame> = None;
    let mut seen_frames = BTreeSet::new();
    let mut clocks = BTreeMap::new();
    let mut periods = BTreeSet::new();
    let mut identities: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for result in reader.records() {
        let row = result.map_err(|e| e.to_string())?;
        report.rows += 1;
        let id = &row[0];
        if id.trim().is_empty() {
            return Err("Missing frame ID".into());
        }
        let period: u32 = row[1].parse().map_err(|_| "Invalid period")?;
        if period == 0 {
            return Err("Invalid period zero".into());
        }
        let seconds = clock(&row[2])?;
        if frame.as_ref().is_none_or(|f| f.id != id) {
            if !seen_frames.insert(id.to_string()) {
                return Err(
                    "Noncontiguous repeated frame; explicitly reconcile ordering before audit"
                        .into(),
                );
            }
            if let Some(old) = frame.take() {
                finish(&old, &mut report, &mut clocks);
                if old.period == period {
                    if old.seconds == seconds {
                        report.adjacent_frames_same_game_clock += 1;
                    } else if old.seconds > seconds {
                        report.adjacent_frames_decreasing_game_clock += 1;
                    } else {
                        report.adjacent_frames_increasing_game_clock += 1;
                    }
                    let suffix = |s: &str| s.rsplit('_').next().and_then(|v| v.parse::<u64>().ok());
                    if let (Some(a), Some(b)) = (suffix(&old.id), suffix(id)) {
                        if b > a {
                            *report.numeric_image_suffix_gaps.entry(b - a).or_default() += 1;
                        } else {
                            report.non_increasing_image_suffix_pairs += 1;
                        }
                    }
                }
            } else {
                report.clock_first = row[2].into();
            }
            frame = Some(Frame {
                id: id.into(),
                period,
                clock: row[2].into(),
                seconds,
                ..Default::default()
            });
        }
        let frame = frame.as_mut().unwrap();
        if frame.period != period || frame.clock != row[2] {
            return Err("Inconsistent period/clock within frame".into());
        }
        report.clock_last = row[2].into();
        periods.insert(period);
        let coordinates = [7, 8, 9].map(|i| row[i].parse::<f64>().ok().filter(|v| v.is_finite()));
        report.rows_with_missing_coordinates +=
            u64::from([7, 8, 9].iter().any(|i| row[*i].trim().is_empty()));
        report.rows_with_invalid_coordinates += u64::from(
            [7, 8, 9]
                .iter()
                .enumerate()
                .any(|(axis, i)| !row[*i].trim().is_empty() && coordinates[axis].is_none()),
        );
        let outside = coordinates[0]
            .zip(coordinates[1])
            .is_some_and(|(x, y)| x.abs() > 100. || y.abs() > 42.5);
        let positioned = coordinates[0].is_some() && coordinates[1].is_some();
        let missing_xy = row[7].trim().is_empty() || row[8].trim().is_empty();
        match &row[3] {
            "Player" => {
                if !["Home", "Away"].contains(&&row[4]) || row[5].trim().is_empty() {
                    return Err("Invalid tracked player team/ID".into());
                }
                report.player_rows += 1;
                frame.players += 1;
                frame.positioned_players += u64::from(positioned);
                report.player_rows_missing_xy += u64::from(missing_xy);
                *report.player_z_values.entry(row[9].into()).or_default() += 1;
                report.player_rows_outside_rink_rectangle += u64::from(outside);
                if !frame.actors.insert((row[4].into(), row[5].into())) {
                    frame.duplicate = true;
                }
                let labels = identities
                    .entry(format!("{}:{}", &row[4], &row[5]))
                    .or_default();
                if row[6].trim().is_empty() {
                    report.unidentified_jersey_rows += 1;
                    frame.unidentified = true;
                } else {
                    labels.insert(row[6].into());
                }
            }
            "Puck" => {
                report.puck_rows += 1;
                frame.pucks += 1;
                frame.positioned_pucks += u64::from(positioned);
                report.puck_rows_missing_xy += u64::from(missing_xy);
                *report.puck_z_values.entry(row[9].into()).or_default() += 1;
                report.puck_rows_outside_rink_rectangle += u64::from(outside);
            }
            _ => return Err("Unknown tracking entity type".into()),
        }
    }
    if let Some(frame) = frame {
        finish(&frame, &mut report, &mut clocks);
    } else {
        return Err("No tracking rows".into());
    }
    report.game_clock_values = clocks.len();
    for n in clocks.values() {
        *report.frames_per_clock_histogram.entry(*n).or_default() += 1;
    }
    report.periods = periods.into_iter().collect();
    report.tracked_actor_keys = identities.len();
    report.team_player_ids_with_multiple_jersey_labels = identities
        .into_iter()
        .filter(|(_, labels)| labels.len() > 1)
        .map(|(key, _)| key)
        .collect();
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    const HEADER: &str = "Image Id,Period,Game Clock,Player or Puck,Team,Player Id,Player Jersey Number,Rink Location X (Feet),Rink Location Y (Feet),Rink Location Z (Feet),Goal Score\n";
    #[test]
    fn clocks_missing_pucks_and_identity_changes_stay_visible() {
        let csv = format!("{HEADER}game_1,1,20:00,Player,Home,1,10,0,0,1,\ngame_1,1,20:00,Puck,n/a,,,0,0,0.02,\ngame_2,1,20:00,Player,Home,1,,110,0,1,\ngame_3,1,19:59,Player,Home,1,11,0,0,1,\n");
        let r = audit(csv.as_bytes()).unwrap();
        assert_eq!((r.rows, r.frames), (4, 3));
        assert_eq!(r.adjacent_frames_same_game_clock, 1);
        assert_eq!(r.adjacent_frames_decreasing_game_clock, 1);
        assert_eq!(r.frames_without_puck, 2);
        assert_eq!(r.unidentified_jersey_rows, 1);
        assert_eq!(r.player_rows_outside_rink_rectangle, 1);
        assert_eq!(r.team_player_ids_with_multiple_jersey_labels, ["Home:1"]);
        assert_eq!(r.numeric_image_suffix_gaps[&1], 2);
    }
    #[test]
    fn ambiguous_order_and_schema_are_rejected() {
        let row = "game_1,1,20:00,Player,Home,1,10,0,0,1,\n";
        let csv = format!("{HEADER}{row}game_2,1,19:59,Puck,n/a,,,0,0,0.02,\n{row}");
        assert!(audit(csv.as_bytes()).is_err());
        assert!(audit(HEADER.replace("Image Id", "ImageID").as_bytes()).is_err());
        let csv = format!(
            "{HEADER}game_1,1,20:00,Player,Home,1,10,0,0,1,\ngame_1,1,19:59,Puck,n/a,,,0,0,0.02,\n"
        );
        assert!(audit(csv.as_bytes()).is_err());
    }
    #[test]
    fn missing_and_invalid_coordinates_remain_in_the_census() {
        let csv = format!("{HEADER}game_1,1,20:00,Player,Home,1,10,,0,1,\ngame_1,1,20:00,Puck,n/a,,,NaN,0,0.02,\n");
        let r = audit(csv.as_bytes()).unwrap();
        assert_eq!(r.rows, 2);
        assert_eq!(r.rows_with_missing_coordinates, 1);
        assert_eq!(r.rows_with_invalid_coordinates, 1);
        assert_eq!(r.frames_without_puck, 0);
        assert_eq!(r.frames_without_puck_position, 1);
        assert_eq!(r.player_rows_missing_xy, 1);
        assert_eq!(r.positioned_player_count_histogram[&0], 1);
    }
}
