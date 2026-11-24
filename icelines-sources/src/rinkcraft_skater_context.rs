use icelines_core::view_model::{rinkcraft_skater_context::Context, rinkcraft_stat_scores::Player};
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};
#[derive(Default, Clone, Debug)]
pub struct Assists {
    pub primary: u32,
    pub secondary: u32,
}
/// Credited scoring involvement, not all passes or passing success.
pub fn goal_assists(bytes: &[u8], id: u64, season: u32) -> Result<BTreeMap<u32, Assists>, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if v["id"].as_u64() != Some(id)
        || v["season"].as_u64() != Some(season as u64)
        || v["gameType"] != 2
        || !matches!(v["gameState"].as_str(), Some("OFF" | "FINAL"))
    {
        return Err("Wrong finalized game scope".into());
    }
    let roster: BTreeMap<_, _> = v["rosterSpots"]
        .as_array()
        .ok_or("Missing roster")?
        .iter()
        .map(|r| {
            Ok((
                r["playerId"].as_u64().ok_or("Missing actor")?,
                r["teamId"].as_u64().ok_or("Missing team")?,
            ))
        })
        .collect::<Result<_, String>>()?;
    let mut seen = BTreeSet::new();
    let mut out = BTreeMap::<u32, Assists>::new();
    let plays = v["plays"]
        .as_array()
        .filter(|p| !p.is_empty())
        .ok_or("Missing game events")?;
    for p in plays {
        if !seen.insert(p["eventId"].as_u64().ok_or("Missing event identity")?) {
            return Err("Duplicate game event".into());
        }
        if p["typeDescKey"] != "goal" || p["periodDescriptor"]["periodType"] == "SO" {
            continue;
        }
        let d = &p["details"];
        if d["assist1PlayerId"].is_number() && d["assist1PlayerId"] == d["assist2PlayerId"] {
            return Err("Duplicate assist actor".into());
        }
        for (field, primary) in [("assist1PlayerId", true), ("assist2PlayerId", false)] {
            if let Some(actor) = d[field].as_u64() {
                let team = d["eventOwnerTeamId"].as_u64().ok_or("Missing goal team")?;
                if roster.get(&actor).copied() != Some(team) {
                    return Err("Assist actor/team absent from roster".into());
                }
                let a = out
                    .entry(u32::try_from(actor).map_err(|e| e.to_string())?)
                    .or_default();
                if primary {
                    a.primary += 1
                } else {
                    a.secondary += 1
                }
            }
        }
    }
    Ok(out)
}
#[derive(Default, Clone, Debug)]
pub struct Shooting {
    pub attempts: u32,
    pub goals: u32,
    pub xg: f64,
    pub distance: f64,
}
pub fn shooting(input: impl Read, season: u32) -> Result<BTreeMap<u32, Shooting>, String> {
    let mut csv = csv::Reader::from_reader(input);
    let h = csv.headers().map_err(|e| e.to_string())?.clone();
    let keys = [
        "shotID",
        "game_id",
        "season",
        "isPlayoffGame",
        "period",
        "shotOnEmptyNet",
        "shooterPlayerId",
        "goal",
        "xGoal",
        "shotDistance",
        "shotWasOnGoal",
    ];
    let at = keys
        .iter()
        .map(|k| {
            h.iter()
                .position(|v| v == *k)
                .ok_or_else(|| format!("Missing shot {k}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = BTreeMap::<u32, Shooting>::new();
    let mut seen = BTreeSet::new();
    for (i, row) in csv.records().enumerate() {
        if i >= 500000 {
            return Err("Shot archive exceeds row bound".into());
        }
        let row = row.map_err(|e| e.to_string())?;
        let n = |i: usize| {
            row.get(at[i])
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.)
                .ok_or_else(|| format!("Invalid {}", keys[i]))
        };
        let integer = |i: usize| -> Result<u32, String> {
            let v = n(i)?;
            if v.fract() != 0. || v > u32::MAX as f64 {
                return Err("Invalid shot integer".into());
            }
            Ok(v as u32)
        };
        if integer(2)? != season / 10000 || !seen.insert((integer(1)?, integer(0)?)) {
            return Err("Wrong season or duplicate shot".into());
        }
        if integer(3)? > 1 || integer(5)? > 1 {
            return Err("Invalid shot competition/net flag".into());
        }
        if integer(3)? == 1 || integer(4)? >= 5 || integer(5)? == 1 {
            continue;
        }
        let Some(id) = row.get(at[6]).filter(|v| !v.is_empty() && *v != "0") else {
            continue;
        };
        let id = id.parse::<u32>().map_err(|e| e.to_string())?;
        let goal = integer(7)?;
        let xg = n(8)?;
        let on = integer(10)?;
        if goal > 1 || on > 1 || (goal == 1 && on == 0) || xg > 1. {
            return Err("Invalid shot outcome/probability".into());
        }
        let s = out.entry(id).or_default();
        s.attempts += 1;
        s.goals += goal;
        s.xg += xg;
        s.distance += n(9)?;
    }
    Ok(out)
}
pub fn build(
    players: &[Player],
    summary: &[Value],
    rt: &[Value],
    toi: &[Value],
    assists: Option<&BTreeMap<u32, Assists>>,
    shots: Option<&BTreeMap<u32, Shooting>>,
) -> Result<BTreeMap<u32, Context>, String> {
    let index = |rows: &[Value]| {
        rows.iter()
            .filter_map(|r| r["playerId"].as_u64().map(|id| (id as u32, r.clone())))
            .collect::<BTreeMap<_, _>>()
    };
    let summary = index(summary);
    let rt = index(rt);
    let toi = index(toi);
    let mut out = BTreeMap::new();
    for p in players {
        for index in [&summary, &rt, &toi] {
            if let Some(row) = index.get(&p.player_id) {
                if row["gamesPlayed"].as_u64() != Some(p.games as u64) {
                    return Err("Skater detail GP differs from official profile".into());
                }
            }
        }
        let mut values = p.metrics.clone();
        let mut notes=vec!["Recorded events do not measure pass completion, puck touches, anticipation, or isolated defensive effectiveness. EDGE zone time is puck location while on ice, not the player's own location.".into()];
        if let Some(row) = summary.get(&p.player_id) {
            let number = |r: &Value, k: &str| r[k].as_f64().filter(|v| v.is_finite() && *v >= 0.);
            if let Some(a) = assists {
                let a = a.get(&p.player_id).cloned().unwrap_or_default();
                if number(row, "assists") == Some(f64::from(a.primary + a.secondary)) {
                    values.insert("primary".into(), a.primary as f64);
                    values.insert("secondary".into(), a.secondary as f64);
                    if p.minutes > 0. {
                        values.insert("primary-rate".into(), a.primary as f64 * 60. / p.minutes);
                    }
                } else {
                    notes.push("Assist-type detail unavailable: captured goal-log totals differ from the NHL summary snapshot.".into());
                }
            }
            if let Some(t) = toi.get(&p.player_id) {
                for (key, points, goals, time) in [
                    ("ev-assists", "evPoints", "evGoals", "evTimeOnIce"),
                    ("pp-assists", "ppPoints", "ppGoals", "ppTimeOnIce"),
                ] {
                    if let Some(((pts, g), seconds)) = number(row, points)
                        .zip(number(row, goals))
                        .zip(number(t, time))
                    {
                        if pts < g {
                            return Err("Goals exceed points".into());
                        }
                        if seconds > 0. {
                            values.insert(key.into(), (pts - g) * 3600. / seconds);
                        }
                    }
                }
                if let Some(seconds) = number(t, "shTimeOnIce").filter(|_| p.games > 0) {
                    values.insert("sh-seconds".into(), seconds / p.games as f64);
                }
            }
            if let Some(r) = rt.get(&p.player_id) {
                if let Some(hits) = number(r, "hits").filter(|_| p.minutes > 0.) {
                    values.insert("hits".into(), hits * 60. / p.minutes);
                }
            }
        }
        if let Some((g, (s, a))) = values.get("giveaways").copied().zip(
            values
                .get("shots")
                .copied()
                .zip(values.get("assists").copied()),
        ) {
            if g + s + a > 0. {
                values.insert("loss-share".into(), g / (g + s + a));
            }
        }
        if let Some((a, b)) = values
            .get("drawn")
            .copied()
            .zip(values.get("taken").copied())
        {
            values.insert("net-penalties".into(), a - b);
        }
        if let Some(s) = shots
            .and_then(|s| s.get(&p.player_id))
            .filter(|s| s.attempts > 0)
        {
            values.insert("ixg".into(), s.xg);
            values.insert("finishing".into(), s.goals as f64 - s.xg);
            values.insert("shot-quality".into(), s.xg / s.attempts as f64);
            values.insert("shot-distance".into(), s.distance / s.attempts as f64);
        }
        let credit = format!(
            "NHL season reports{}{}{}",
            if p.metrics.contains_key("peak-speed") {
                " / NHL EDGE"
            } else {
                ""
            },
            if assists.is_some() {
                " / captured NHL goal logs"
            } else {
                ""
            },
            if shots.is_some() {
                " / MoneyPuck.com"
            } else {
                ""
            }
        );
        out.insert(p.player_id, Context::new(values, credit, notes));
    }
    Ok(out)
}
