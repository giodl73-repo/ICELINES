//! NHL credited draw outcomes with pre-event score context; not possession value.
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Debug, Default, Clone)]
pub struct Counts {
    pub late_states: BTreeMap<String, icelines_core::view_model::rinkcraft_under_pressure::Draws>,
    pub taken: u32,
    pub won: u32,
    pub late_taken: u32,
    pub late_won: u32,
}
pub fn parse_game(bytes: &[u8], id: u64, season: u32) -> Result<BTreeMap<u32, Counts>, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if v["id"].as_u64() != Some(id)
        || v["season"].as_u64() != Some(season as u64)
        || v["gameType"] != 2
        || !matches!(v["gameState"].as_str(), Some("OFF" | "FINAL"))
    {
        return Err("Wrong finalized faceoff game scope".into());
    }
    let (away, home) = (
        v["awayTeam"]["id"].as_u64().ok_or("Missing away team")?,
        v["homeTeam"]["id"].as_u64().ok_or("Missing home team")?,
    );
    if away == home {
        return Err("Duplicate game team".into());
    }
    let mut roster = BTreeMap::new();
    let mut out = BTreeMap::<u32, Counts>::new();
    for r in v["rosterSpots"]
        .as_array()
        .ok_or("Missing faceoff roster")?
    {
        let id = u32::try_from(r["playerId"].as_u64().ok_or("Missing roster actor")?)
            .map_err(|e| e.to_string())?;
        let team = r["teamId"].as_u64().ok_or("Missing roster team")?;
        if id == 0 || ![away, home].contains(&team) || roster.insert(id, team).is_some() {
            return Err("Invalid or duplicate roster actor".into());
        }
        if r["positionCode"] != "G" {
            out.insert(id, Counts::default());
        }
    }
    let mut ids = BTreeSet::new();
    let mut order = BTreeSet::new();
    let mut plays = v["plays"]
        .as_array()
        .filter(|p| !p.is_empty() && p.len() <= 20000)
        .ok_or("Missing/bounded game plays")?
        .iter()
        .collect::<Vec<_>>();
    for p in &plays {
        if !ids.insert(p["eventId"].as_u64().ok_or("Missing event identity")?)
            || !order.insert(p["sortOrder"].as_u64().ok_or("Missing event order")?)
        {
            return Err("Duplicate event identity/order".into());
        }
    }
    plays.sort_by_key(|p| p["sortOrder"].as_u64());
    let mut score = [0u64, 0u64];
    for p in plays {
        if p["periodDescriptor"]["periodType"] == "SO" {
            continue;
        }
        let d = &p["details"];
        if p["typeDescKey"] == "goal" {
            let team = d["eventOwnerTeamId"]
                .as_u64()
                .ok_or("Missing scoring team")?;
            if ![away, home].contains(&team) {
                return Err("Unknown scoring team".into());
            }
            let next = [
                d["awayScore"].as_u64().ok_or("Missing goal score")?,
                d["homeScore"].as_u64().ok_or("Missing goal score")?,
            ];
            let mut expected = score;
            expected[usize::from(team == home)] += 1;
            if next != expected {
                return Err("Incomplete/inconsistent pre-draw score sequence".into());
            }
            score = next;
        }
        if p["typeDescKey"] != "faceoff" {
            continue;
        }
        let actor = |key: &str| {
            u32::try_from(d[key].as_u64().ok_or("Missing faceoff actor")?)
                .map_err(|e| e.to_string())
        };
        let (won, lost) = (actor("winningPlayerId")?, actor("losingPlayerId")?);
        let team = d["eventOwnerTeamId"].as_u64().ok_or("Missing draw team")?;
        if won == lost
            || roster.get(&won) != Some(&team)
            || roster.get(&won) == roster.get(&lost)
            || !out.contains_key(&won)
            || !out.contains_key(&lost)
        {
            return Err("Invalid faceoff actor/team".into());
        }
        let clock = p["timeInPeriod"].as_str().ok_or("Missing draw clock")?;
        let (m, s) = clock.split_once(':').ok_or("Invalid draw clock")?;
        let (m, s) = (
            m.parse::<u32>().map_err(|e| e.to_string())?,
            s.parse::<u32>().map_err(|e| e.to_string())?,
        );
        if s >= 60 || m > 20 || (m == 20 && s != 0) {
            return Err("Invalid draw clock".into());
        }
        let period = p["periodDescriptor"]["number"]
            .as_u64()
            .filter(|n| *n > 0)
            .ok_or("Missing draw period")?;
        let late = period == 3
            && p["periodDescriptor"]["periodType"] == "REG"
            && m * 60 + s >= 900
            && score[0].abs_diff(score[1]) <= 1;
        for (id, win) in [(won, true), (lost, false)] {
            let c = out.get_mut(&id).ok_or("Missing draw actor")?;
            c.taken += 1;
            c.won += u32::from(win);
            if late {
                c.late_taken += 1;
                c.late_won += u32::from(win);
                let own = usize::from(roster[&id] == home);
                let state = match score[own].cmp(&score[1 - own]) {
                    std::cmp::Ordering::Equal => "tied",
                    std::cmp::Ordering::Greater => "leading",
                    std::cmp::Ordering::Less => "trailing",
                };
                let b = c.late_states.entry(state.into()).or_default();
                b.taken += 1;
                b.won += u32::from(win);
            }
        }
    }
    let final_score = [
        v["awayTeam"]["score"]
            .as_u64()
            .ok_or("Missing final score")?,
        v["homeTeam"]["score"]
            .as_u64()
            .ok_or("Missing final score")?,
    ];
    let shootout = v["gameOutcome"]["lastPeriodType"] == "SO"
        && score[0] == score[1]
        && ((final_score == [score[0] + 1, score[1]]) || (final_score == [score[0], score[1] + 1]));
    if final_score != score && !shootout {
        return Err("Final score does not reconcile with event sequence".into());
    }
    Ok(out)
}
