//! Regulation-only unit participation reconstructed from explicit shift intervals.
//! Personnel and exposure do not establish tactical stations or coaching duties.
use crate::deployment_audit::{id, number};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn clock(s: &str) -> Result<u32> {
    let (m, s) = s.split_once(':').ok_or("Invalid shift clock")?;
    let m: u32 = m.parse()?;
    let s: u32 = s.parse()?;
    if m > 20 || s >= 60 || m * 60 + s > 1200 {
        return Err("Out-of-period shift clock".into());
    }
    Ok(m * 60 + s)
}
#[derive(Clone)]
struct Shift {
    player: u64,
    team: u64,
    period: u32,
    start: u32,
    end: u32,
}

fn active(shifts: &[Shift], period: u32, start: u32, team: u64) -> BTreeSet<u64> {
    shifts
        .iter()
        .filter(|s| s.period == period && s.start <= start && start < s.end && s.team == team)
        .map(|s| s.player)
        .collect()
}
fn add(map: &mut BTreeMap<Vec<u64>, u32>, ids: impl Iterator<Item = u64>, duration: u32) {
    *map.entry(ids.collect()).or_default() += duration;
}
fn display(
    map: BTreeMap<Vec<u64>, u32>,
    positions: &BTreeMap<u64, (String, String)>,
    limit: usize,
) -> Value {
    let mut rows: Vec<_> = map.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    json!(rows.into_iter().take(limit).map(|(ids,seconds)|json!({"seconds":seconds,
        "members":ids.iter().map(|id|json!({"id":id,"name":positions[id].0,"position":positions[id].1,"tactical_duty":"undocumented"})).collect::<Vec<_>>()
    })).collect::<Vec<_>>())
}

/// Constant-participant half-open segments; no event-time goals or blame assigned.
pub fn reconstruct(boxscore: &Value, raw: &Value, team: u64) -> Result<Value> {
    reconstruct_inner(boxscore, raw, team, None, false, false)
}

/// Complete, untruncated personnel combinations while the selected player is on ice.
pub fn reconstruct_player(boxscore: &Value, raw: &Value, team: u64, player: u64) -> Result<Value> {
    reconstruct_inner(boxscore, raw, team, Some(player), false, false)
}

/// All regulation team combinations, with no display truncation.
pub fn reconstruct_complete(boxscore: &Value, raw: &Value, team: u64) -> Result<Value> {
    validate_complete(raw)?;
    reconstruct_inner(boxscore, raw, team, None, true, false)
}
fn validate_complete(raw: &Value) -> Result<()> {
    let rows = raw["data"].as_array().ok_or("Missing shifts")?;
    if !rows.iter().any(|r| r["typeCode"].as_u64() == Some(517)) {
        return Err("No shift exposure rows".into());
    }
    if let Some(total) = raw["total"].as_u64() {
        if total != rows.len() as u64 {
            return Err("Incomplete shift capture".into());
        }
    }
    Ok(())
}

/// Untruncated team participation segments, including explicitly invalid occupancy.
/// Conflicting intervals remain censored; no event-time responsibility is assigned.
pub fn reconstruct_segments(boxscore: &Value, raw: &Value, team: u64) -> Result<Value> {
    validate_complete(raw)?;
    reconstruct_inner(boxscore, raw, team, None, true, true)
}

fn reconstruct_inner(
    boxscore: &Value,
    raw: &Value,
    team: u64,
    focus: Option<u64>,
    complete: bool,
    emit_segments: bool,
) -> Result<Value> {
    let game = id(boxscore, "id")?;
    let home = id(&boxscore["homeTeam"], "id")?;
    let away = id(&boxscore["awayTeam"], "id")?;
    let other = if team == home {
        away
    } else if team == away {
        home
    } else {
        return Err("Team absent from game".into());
    };
    let mut positions = BTreeMap::new();
    let mut roster_teams = BTreeMap::new();
    for (side, tid) in [("homeTeam", home), ("awayTeam", away)] {
        for group in ["forwards", "defense", "goalies"] {
            for p in boxscore["playerByGameStats"][side][group]
                .as_array()
                .ok_or("Missing game roster group")?
            {
                let pid = id(p, "playerId")?;
                let pos = if group == "goalies" {
                    "G"
                } else {
                    p["position"].as_str().ok_or("Missing game position")?
                };
                let name = p["name"]["default"]
                    .as_str()
                    .ok_or("Missing game name")?
                    .to_owned();
                if positions.insert(pid, (name, pos.to_owned())).is_some() {
                    return Err("Duplicate game roster player".into());
                }
                roster_teams.insert(pid, tid);
            }
        }
    }
    if let Some(pid) = focus {
        if roster_teams.get(&pid) != Some(&team) {
            return Err("Focused player absent from requested team roster".into());
        }
    }
    let mut focused_states = BTreeMap::<&str, u32>::new();
    let mut focused_segments = Vec::new();
    let mut shifts: Vec<Shift> = Vec::new();
    let mut duplicate_rows = 0;
    let mut conflicts = Vec::<(u32, u32, u32, u64)>::new();
    let mut zero = 0;
    let mut ot = 0;
    for row in raw["data"].as_array().ok_or("Missing shift array")? {
        if number(row, "typeCode")? != 517.0 {
            continue;
        }
        if id(row, "gameId")? != game {
            return Err("Wrong game shift".into());
        }
        let period = id(row, "period")? as u32;
        if period > 3 {
            ot += 1;
            continue;
        }
        if period == 0 {
            return Err("Invalid period".into());
        }
        let pid = id(row, "playerId")?;
        let tid = id(row, "teamId")?;
        if roster_teams.get(&pid) != Some(&tid) {
            return Err("Shift participant absent from game roster".into());
        }
        let start = clock(row["startTime"].as_str().ok_or("Missing start")?)?;
        let end = clock(row["endTime"].as_str().ok_or("Missing end")?)?;
        if end < start {
            return Err("Reversed shift interval".into());
        }
        if end == start {
            zero += 1;
            continue;
        }
        if shifts
            .iter()
            .any(|s| s.player == pid && s.period == period && s.start == start && s.end == end)
        {
            duplicate_rows += 1;
            continue;
        }
        for s in shifts
            .iter()
            .filter(|s| s.player == pid && s.period == period && start < s.end && end > s.start)
        {
            conflicts.push((period, start.min(s.start), end.max(s.end), pid));
        }
        shifts.push(Shift {
            player: pid,
            team: tid,
            period,
            start,
            end,
        });
    }
    let mut lines = BTreeMap::new();
    let mut pairs = BTreeMap::new();
    let mut pp = BTreeMap::new();
    let mut pk = BTreeMap::new();
    let mut totals = BTreeMap::<&str, u32>::new();
    let mut player_pp = BTreeMap::<u64, u32>::new();
    for period in 1..=3 {
        let edges: BTreeSet<_> = shifts
            .iter()
            .filter(|s| s.period == period)
            .flat_map(|s| [s.start, s.end])
            .chain([0, 1200])
            .collect();
        let edges: Vec<_> = edges.into_iter().collect();
        for edge in edges.windows(2) {
            let seconds = edge[1] - edge[0];
            if conflicts
                .iter()
                .any(|(p, start, end, _)| *p == period && *start <= edge[0] && edge[0] < *end)
            {
                *totals.entry("censored_conflicting_shift").or_default() += seconds;
                continue;
            }
            let own = active(&shifts, period, edge[0], team);
            let opp = active(&shifts, period, edge[0], other);
            let og = own.iter().filter(|id| positions[id].1 == "G").count();
            let ag = opp.iter().filter(|id| positions[id].1 == "G").count();
            let os = own.len() - og;
            let aps = opp.len() - ag;
            let state = if og > 1 || ag > 1 || own.len() > 6 || opp.len() > 6 || os < 3 || aps < 3 {
                "invalid_occupancy"
            } else if og == 1 && ag == 1 && os == 5 && aps == 5 {
                "5v5"
            } else if og == 1 && ag == 1 && os == 5 && aps == 4 {
                "5v4"
            } else if og == 1 && ag == 1 && os == 4 && aps == 5 {
                "4v5"
            } else {
                "other"
            };
            *totals.entry(state).or_default() += seconds;
            if emit_segments {
                focused_segments.push(json!({"period":period,"start_seconds":edge[0],"end_seconds":edge[1],"state":state,
                    "own_participants":own.iter().map(|id|json!({"id":id,"position":positions[id].1})).collect::<Vec<_>>()}));
            }
            if let Some(pid) = focus {
                if !own.contains(&pid) {
                    continue;
                }
                *focused_states.entry(state).or_default() += seconds;
                focused_segments.push(json!({"period":period,"start_seconds":edge[0],"end_seconds":edge[1],"state":state,
                    "own_participants":own.iter().map(|id|json!({"id":id,"name":positions[id].0,"position":positions[id].1})).collect::<Vec<_>>()}));
            }
            let skaters: Vec<_> = own
                .iter()
                .filter(|id| positions[id].1 != "G")
                .copied()
                .collect();
            if state == "5v5" {
                let d: Vec<_> = skaters
                    .iter()
                    .filter(|id| positions[id].1 == "D")
                    .copied()
                    .collect();
                let f: Vec<_> = skaters
                    .iter()
                    .filter(|id| positions[id].1 != "D")
                    .copied()
                    .collect();
                if d.len() == 2 && f.len() == 3 {
                    add(&mut lines, f.into_iter(), seconds);
                    add(&mut pairs, d.into_iter(), seconds);
                }
            } else if state == "5v4" {
                add(&mut pp, skaters.iter().copied(), seconds);
                for pid in skaters {
                    *player_pp.entry(pid).or_default() += seconds;
                }
            } else if state == "4v5" {
                add(&mut pk, skaters.into_iter(), seconds);
            }
        }
    }
    let limit = if focus.is_some() || complete {
        usize::MAX
    } else {
        12
    };
    let mut report = json!({"schema":"icelines.observed-units.0.1-experimental","game_id":game,"team_id":team,"game_date":boxscore["gameDate"],
        "scope":"latest-game regulation only; units ranked by observed overlap, not coach labels",
        "state_seconds":totals,"zero_duration_rows_ignored":zero,"overtime_rows_excluded":ot,
        "exact_duplicate_intervals_ignored":duplicate_rows,
        "conflicting_shift_spans":conflicts.iter().map(|(p,start,end,pid)|json!({"period":p,"start":start,"end":end,"player_id":pid})).collect::<Vec<_>>(),
        "five_on_five_trios":display(lines,&positions,limit),"five_on_five_pairs":display(pairs,&positions,if focus.is_some()||complete{limit}else{8}),
        "five_on_four_units":display(pp,&positions,limit),"four_on_five_units":display(pk,&positions,limit),
        "five_on_four_player_seconds":player_pp.iter().map(|(id,seconds)|json!({"id":id,"name":positions[id].0,"seconds":seconds})).collect::<Vec<_>>(),
        "limitations":["Shift timestamps are second-resolution and may contain change artifacts","Invalid occupancy excluded from unit statistics and reported",
            "Exact duplicate player intervals counted once; entire union of conflicting same-player intervals censored",
            "No inferred quarterback/flank/bumper/net-front duties","One game is not a season assignment","Overtime and other strengths excluded from displayed units"]});
    if let Some(pid) = focus {
        report["schema"] = json!("icelines.player-game-units.0.1-experimental");
        report["scope"]=json!("complete regulation combinations during focused player exposure; not coach-assigned line numbers");
        report["focused_player_id"] = json!(pid);
        report["focused_state_seconds"] = json!(focused_states);
        report["focused_segments"] = json!(focused_segments);
        let own = report.as_object_mut().unwrap();
        let overlap = own.remove("five_on_four_player_seconds").unwrap();
        own.insert("coincident_five_on_four_player_seconds".into(), overlap);
    }
    if complete {
        report["schema"] = json!("icelines.complete-team-units.0.1-experimental");
        report["scope"]=json!("complete regulation team combinations, no display truncation; no coach-assigned line numbers");
    }
    if emit_segments {
        report["participation_segments"] = json!(focused_segments);
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Value, Value) {
        let mut boxscore = json!({"id":100,"gameDate":"2000-01-01","homeTeam":{"id":1},"awayTeam":{"id":2},"playerByGameStats":{}});
        let mut shifts = Vec::new();
        for (side, team, base) in [("homeTeam", 1, 0), ("awayTeam", 2, 10)] {
            let forwards:Vec<_>=(1..=3).map(|i|json!({"playerId":base+i,"position":"C","name":{"default":format!("F{base}-{i}")}})).collect();
            let defense:Vec<_>=(4..=5).map(|i|json!({"playerId":base+i,"position":"D","name":{"default":format!("D{base}-{i}")}})).collect();
            boxscore["playerByGameStats"][side] = json!({"forwards":forwards,"defense":defense,"goalies":[{"playerId":base+6,"name":{"default":"Goalie"}}]});
            for period in 1..=3 {
                for player in 1..=6 {
                    shifts.push(json!({"gameId":100,"teamId":team,"playerId":base+player,"period":period,"typeCode":517,"startTime":"00:00","endTime":"20:00"}));
                }
            }
        }
        (boxscore, json!({"data":shifts}))
    }
    #[test]
    fn full_regulation_conserves_exposure_and_excludes_goal_marker_rows() {
        let (b, mut s) = fixture();
        s["data"]
            .as_array_mut()
            .unwrap()
            .push(json!({"typeCode":505}));
        let r = reconstruct(&b, &s, 1).unwrap();
        assert_eq!(r["state_seconds"]["5v5"], 3600);
        assert_eq!(r["five_on_five_trios"][0]["seconds"], 3600);
        assert_eq!(r["five_on_five_pairs"][0]["seconds"], 3600);
        assert_eq!(r["five_on_four_units"].as_array().unwrap().len(), 0);
    }
    #[test]
    fn absent_goalie_does_not_become_five_on_five() {
        let (b, mut s) = fixture();
        s["data"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["playerId"] != 6);
        let r = reconstruct(&b, &s, 1).unwrap();
        assert!(r["state_seconds"].get("5v5").is_none());
        assert_eq!(r["state_seconds"]["other"], 3600);
    }
    #[test]
    fn duplicate_intervals_do_not_inflate_exposure_and_wrong_game_is_rejected() {
        let (b, mut s) = fixture();
        let first = s["data"][0].clone();
        s["data"].as_array_mut().unwrap().push(first);
        let r = reconstruct(&b, &s, 1).unwrap();
        assert_eq!(r["exact_duplicate_intervals_ignored"], 1);
        assert_eq!(r["state_seconds"]["5v5"], 3600);
        let (b, mut s) = fixture();
        s["data"][0]["gameId"] = json!(101);
        assert!(reconstruct(&b, &s, 1).is_err());
    }
    #[test]
    fn conflicting_interval_union_is_censored_without_choosing_a_record() {
        let (b, mut s) = fixture();
        let mut conflict = s["data"][0].clone();
        conflict["endTime"] = json!("19:59");
        s["data"].as_array_mut().unwrap().push(conflict);
        let r = reconstruct(&b, &s, 1).unwrap();
        assert_eq!(r["state_seconds"]["censored_conflicting_shift"], 1200);
        assert_eq!(r["state_seconds"]["5v5"], 2400);
    }
    #[test]
    fn clock_bounds() {
        assert_eq!(clock("20:00").unwrap(), 1200);
        assert!(clock("20:01").is_err());
        assert!(clock("01:60").is_err());
    }
    #[test]
    fn focused_history_preserves_segments_and_rejects_absent_player() {
        let (b, s) = fixture();
        let report = reconstruct_player(&b, &s, 1, 1).unwrap();
        assert_eq!(report["focused_state_seconds"]["5v5"], 3600);
        assert_eq!(report["five_on_five_trios"][0]["seconds"], 3600);
        let seconds: u64 = report["focused_segments"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["end_seconds"].as_u64().unwrap() - r["start_seconds"].as_u64().unwrap())
            .sum();
        assert_eq!(seconds, 3600);
        assert!(reconstruct_player(&b, &s, 1, 99).is_err());
    }
    #[test]
    fn boundary_does_not_double_count_changing_player() {
        let shifts = vec![
            Shift {
                player: 1,
                team: 1,
                period: 1,
                start: 0,
                end: 30,
            },
            Shift {
                player: 2,
                team: 1,
                period: 1,
                start: 30,
                end: 60,
            },
        ];
        assert_eq!(active(&shifts, 1, 30, 1), BTreeSet::from([2]));
    }
    #[test]
    fn shared_pp_player_is_one_player_with_combined_exposure() {
        let mut pp = BTreeMap::new();
        add(&mut pp, [1, 2, 3, 4, 5].into_iter(), 60);
        add(&mut pp, [1, 6, 7, 8, 9].into_iter(), 40);
        let total: u32 = pp
            .iter()
            .filter(|(ids, _)| ids.contains(&1))
            .map(|(_, s)| *s)
            .sum();
        assert_eq!(total, 100);
        assert_eq!(pp.values().sum::<u32>(), 100);
    }
}
