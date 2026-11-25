//! Matched regulation exposure, not block/takeaway opportunities or a clutch rating.
use crate::rinkcraft_pressure;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerExposure {
    pub late_lead_seconds: u32,
    pub unresolved_team_seconds: u32,
    pub late_lead_blocks: u32,
    pub late_lead_takeaways: u32,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct GameExposure {
    pub players: BTreeMap<u32, PlayerExposure>,
    pub excluded_players: BTreeMap<u32, String>,
}
fn n(v: &Value, key: &str) -> Result<u64, String> {
    v[key]
        .as_u64()
        .ok_or_else(|| format!("Missing exposure {key}"))
}
fn clock(s: &str) -> Result<u32, String> {
    let (m, s) = s.split_once(':').ok_or("Invalid exposure clock")?;
    let (m, s) = (
        m.parse::<u32>().map_err(|e| e.to_string())?,
        s.parse::<u32>().map_err(|e| e.to_string())?,
    );
    if m > 65 || s >= 60 {
        return Err("Out-of-bounds exposure clock".into());
    }
    Ok(m * 60 + s)
}
fn time(v: &Value, key: &str) -> Result<u32, String> {
    clock(
        v[key]
            .as_str()
            .ok_or_else(|| format!("Missing exposure {key}"))?,
    )
}
/// Same-second goals define a boundary with no duration between events. Their
/// post-goal score applies to the next half-open interval; actions retain their
/// own event-order pre-event score in the existing parser.
pub fn analyze(
    pbp: &[u8],
    boxscore: &[u8],
    shifts: &[u8],
    game: u64,
    season: u32,
) -> Result<GameExposure, String> {
    let counts = rinkcraft_pressure::parse_game(pbp, game, season)?;
    let p: Value = serde_json::from_slice(pbp).map_err(|e| e.to_string())?;
    let b: Value = serde_json::from_slice(boxscore).map_err(|e| e.to_string())?;
    let s: Value = serde_json::from_slice(shifts).map_err(|e| e.to_string())?;
    if n(&b, "id")? != game
        || n(&b, "season")? != u64::from(season)
        || b["gameType"] != 2
        || !matches!(b["gameState"].as_str(), Some("OFF" | "FINAL"))
    {
        return Err("Wrong finalized exposure box scope".into());
    }
    let teams = [n(&p["awayTeam"], "id")?, n(&p["homeTeam"], "id")?];
    let mut roster = BTreeMap::<u32, (u64, bool, u32)>::new();
    let pbp_roster = p["rosterSpots"]
        .as_array()
        .ok_or("Missing exposure roster")?;
    for (side, team) in [("awayTeam", teams[0]), ("homeTeam", teams[1])] {
        if n(&b[side], "id")? != team || b[side]["score"] != p[side]["score"] {
            return Err("Box and event game teams/scores differ".into());
        }
        for group in ["forwards", "defense", "goalies"] {
            for row in b["playerByGameStats"][side][group]
                .as_array()
                .ok_or("Missing box roster group")?
            {
                let id = u32::try_from(n(row, "playerId")?).map_err(|e| e.to_string())?;
                let goalie = group == "goalies";
                let r = pbp_roster
                    .iter()
                    .find(|r| r["playerId"].as_u64() == Some(u64::from(id)))
                    .ok_or("Box actor absent from events")?;
                if n(r, "teamId")? != team
                    || (r["positionCode"] == "G") != goalie
                    || roster
                        .insert(id, (team, goalie, time(row, "toi")?))
                        .is_some()
                {
                    return Err("Inconsistent exposure roster".into());
                }
            }
        }
    }
    let rows = s["data"]
        .as_array()
        .filter(|r| !r.is_empty() && r.len() <= 20000)
        .ok_or("Missing/bounded native shift rows")?;
    if !s["adapter"].is_null() || n(&s, "total")? != rows.len() as u64 {
        return Err("Exposure requires complete native shift response".into());
    }
    let mut intervals = BTreeMap::<(u32, u64), BTreeSet<(u32, u32)>>::new();
    for row in rows.iter().filter(|r| r["typeCode"] == 517) {
        let id = u32::try_from(n(row, "playerId")?).map_err(|e| e.to_string())?;
        let team = n(row, "teamId")?;
        if n(row, "gameId")? != game || roster.get(&id).map(|r| r.0) != Some(team) {
            return Err("Wrong exposure shift actor/game/team".into());
        }
        let period = n(row, "period")?;
        let (start, end) = (time(row, "startTime")?, time(row, "endTime")?);
        let limit = match period {
            1..=3 => 1200,
            4 => 300,
            _ => return Err("Invalid regular-season shift period".into()),
        };
        if start > end || end > limit {
            return Err("Invalid exposure shift interval".into());
        }
        if start < end {
            intervals
                .entry((id, period))
                .or_default()
                .insert((start, end));
        }
    }
    let mut eligible = BTreeSet::new();
    let mut excluded_players = BTreeMap::new();
    for (&id, &(_, goalie, toi)) in &roster {
        if goalie {
            continue;
        }
        let mut seconds = 0u32;
        let mut overlap = false;
        for ((pid, _), spans) in &intervals {
            if *pid != id {
                continue;
            }
            let mut prior_end = 0;
            for &(start, end) in spans {
                overlap |= start < prior_end;
                prior_end = end;
                seconds += end - start;
            }
        }
        if overlap || seconds.abs_diff(toi) > 1 {
            excluded_players.insert(
                id,
                format!(
                    "Shift/box TOI not reconciled: {seconds}s vs {toi}s; overlapping={overlap}"
                ),
            );
        } else {
            eligible.insert(id);
        }
    }
    let mut goals = Vec::new();
    let mut plays = p["plays"].as_array().unwrap().iter().collect::<Vec<_>>();
    plays.sort_by_key(|r| r["sortOrder"].as_u64());
    let mut prior = 0;
    for e in plays
        .iter()
        .filter(|e| e["typeDescKey"] == "goal" && e["periodDescriptor"]["periodType"] != "SO")
    {
        let period = n(&e["periodDescriptor"], "number")?;
        let t = time(e, "timeInPeriod")?;
        let limit = match period {
            1..=3 => 1200,
            4 => 300,
            _ => return Err("Invalid exposure goal period".into()),
        };
        if t > limit
            || e["periodDescriptor"]["periodType"] != if period <= 3 { "REG" } else { "OT" }
        {
            return Err("Invalid exposure goal clock/type".into());
        }
        let absolute = (period - 1) * 1200 + u64::from(t);
        if absolute < prior {
            return Err("Nonchronological exposure goal sequence".into());
        }
        prior = absolute;
        if period <= 3 {
            goals.push((
                absolute,
                [
                    n(&e["details"], "awayScore")?,
                    n(&e["details"], "homeScore")?,
                ],
            ));
        }
    }
    let edges = goals
        .iter()
        .map(|(t, _)| *t)
        .filter(|t| *t > 3300 && *t < 3600)
        .chain([3300, 3600])
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut players = BTreeMap::new();
    for (own, team) in teams.into_iter().enumerate() {
        let mut windows = Vec::new();
        for w in edges.windows(2) {
            let score = goals
                .iter()
                .rev()
                .find(|(t, _)| *t <= w[0])
                .map(|(_, s)| *s)
                .unwrap_or([0, 0]);
            if score[own] == score[1 - own] + 1 {
                windows.push(((w[0] - 2400) as u32, (w[1] - 2400) as u32));
            }
        }
        let report = icelines_core::observed_units::reconstruct_segments(&b, &s, team)
            .map_err(|e| e.to_string())?;
        let mut known = 0;
        let mut seconds = BTreeMap::<u32, u32>::new();
        for seg in report["participation_segments"]
            .as_array()
            .ok_or("Missing participation segments")?
            .iter()
            .filter(|r| r["period"] == 3 && r["state"] != "invalid_occupancy")
        {
            let (start, end) = (
                n(seg, "start_seconds")? as u32,
                n(seg, "end_seconds")? as u32,
            );
            let duration = windows
                .iter()
                .map(|&(a, b)| end.min(b).saturating_sub(start.max(a)))
                .sum::<u32>();
            known += duration;
            for actor in seg["own_participants"].as_array().unwrap() {
                let id = n(actor, "id")? as u32;
                if eligible.contains(&id) {
                    *seconds.entry(id).or_default() += duration;
                }
            }
        }
        let unresolved = windows.iter().map(|(a, b)| b - a).sum::<u32>() - known;
        for &id in eligible.iter().filter(|id| roster[id].0 == team) {
            let c = counts
                .get(&id)
                .ok_or("Eligible skater absent from event roster")?;
            players.insert(
                id,
                PlayerExposure {
                    late_lead_seconds: seconds.get(&id).copied().unwrap_or(0),
                    unresolved_team_seconds: unresolved,
                    late_lead_blocks: c.actions.late_lead_blocks,
                    late_lead_takeaways: c.actions.late_lead_takeaways,
                },
            );
        }
    }
    Ok(GameExposure {
        players,
        excluded_players,
    })
}
