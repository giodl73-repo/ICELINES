//! Public goalie and skater inputs for repeatable stat-estimate refreshes.
use icelines_core::view_model::rinkcraft_stat_scores::Player;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

#[derive(Debug)]
pub struct Observation {
    pub player: Player,
    pub notes: Vec<String>,
}
fn n(v: &Value, key: &str) -> Result<Option<f64>, String> {
    if v[key].is_null() {
        return Ok(None);
    }
    v[key]
        .as_f64()
        .filter(|n| n.is_finite() && *n >= 0. && n.fract() == 0.)
        .map(Some)
        .ok_or_else(|| format!("Invalid {key}"))
}
fn count(v: &Value, key: &str) -> Result<u32, String> {
    v[key]
        .as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("Missing/invalid {key}"))
}
fn ratio(metrics: &mut BTreeMap<String, f64>, key: &str, a: Option<f64>, b: Option<f64>) {
    if let Some((a, b)) = a.zip(b).filter(|(_, b)| *b > 0.) {
        metrics.insert(key.into(), a / b);
    }
}
fn rows_by_id(rows: &[Value], season: u32) -> Result<BTreeMap<u32, &Value>, String> {
    let mut index = BTreeMap::new();
    for r in rows {
        let id = count(r, "playerId")?;
        if id == 0 || count(r, "seasonId")? != season || index.insert(id, r).is_some() {
            return Err("Duplicate or wrong-season NHL row".into());
        }
    }
    Ok(index)
}
#[derive(Default, Debug)]
struct Shots {
    on: u32,
    goals: u32,
    low: u32,
    low_goals: u32,
    high: u32,
    high_goals: u32,
    rebound: u32,
    rebound_goals: u32,
    cross: u32,
    cross_goals: u32,
    generated: u32,
    expected: f64,
}
fn shot_totals(input: impl Read, season: u32) -> Result<BTreeMap<u32, Shots>, String> {
    let mut reader = csv::Reader::from_reader(input);
    let headers = reader.headers().map_err(|e| e.to_string())?.clone();
    let names = [
        "shotID",
        "game_id",
        "season",
        "isPlayoffGame",
        "period",
        "goalieIdForShot",
        "shotOnEmptyNet",
        "shotWasOnGoal",
        "goal",
        "shotRebound",
        "shotAngleReboundRoyalRoad",
        "shotGeneratedRebound",
        "xGoal",
        "xRebound",
    ];
    let indices = names
        .iter()
        .map(|k| {
            headers
                .iter()
                .position(|h| h == *k)
                .ok_or_else(|| format!("Missing shot column {k}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut seen = BTreeSet::new();
    let mut out = BTreeMap::<u32, Shots>::new();
    for (number, row) in reader.records().enumerate() {
        if number >= 500_000 {
            return Err("Shot file exceeds 500,000 rows".into());
        }
        let r = row.map_err(|e| e.to_string())?;
        let number = |i: usize| -> Result<f64, String> {
            r.get(indices[i])
                .and_then(|v| v.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v >= 0.)
                .ok_or_else(|| format!("Invalid shot {}", names[i]))
        };
        let integer = |i: usize| -> Result<u32, String> {
            let n = number(i)?;
            if n.fract() != 0. || n > u32::MAX as f64 {
                return Err(format!("Invalid integer {}", names[i]));
            }
            Ok(n as u32)
        };
        let flag = |i: usize| -> Result<bool, String> {
            match integer(i)? {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(format!("Invalid flag {}", names[i])),
            }
        };
        if integer(2)? != season / 10000 {
            return Err("Shot season mismatch".into());
        }
        if !seen.insert((integer(1)?, integer(0)?)) {
            return Err("Duplicate shot".into());
        }
        if flag(3)? || integer(4)? >= 5 || flag(6)? {
            continue;
        }
        if r.get(indices[5]).is_none_or(|v| v.is_empty() || v == "0") {
            continue;
        }
        let id = integer(5)?;
        if id == 0 {
            continue;
        }
        let on = flag(7)?;
        let goal = flag(8)?;
        let rebound = flag(9)?;
        let cross = flag(10)?;
        let generated = flag(11)?;
        let xg = number(12)?;
        let xr = number(13)?;
        if goal && !on || xg > 1. || xr > 1. {
            return Err("Inconsistent shot outcomes/probabilities".into());
        }
        let s = out.entry(id).or_default();
        s.generated += u32::from(generated);
        s.expected += xr;
        if on {
            s.on += 1;
            s.goals += u32::from(goal);
            if xg < 0.08 {
                s.low += 1;
                s.low_goals += u32::from(goal)
            }
            if xg >= 0.20 {
                s.high += 1;
                s.high_goals += u32::from(goal)
            }
            if rebound {
                s.rebound += 1;
                s.rebound_goals += u32::from(goal)
            }
            if cross && rebound {
                s.cross += 1;
                s.cross_goals += u32::from(goal)
            }
        }
    }
    Ok(out)
}
#[derive(serde::Deserialize)]
struct MoneyGoalie {
    #[serde(rename = "playerId")]
    id: u32,
    season: u32,
    situation: String,
    games_played: u32,
    ongoal: f64,
}
pub fn goalie_inputs(
    season: u32,
    nhl: &[Value],
    summary: &str,
    shots: impl Read,
) -> Result<Vec<Observation>, String> {
    let index = rows_by_id(nhl, season)?;
    let totals = shot_totals(shots, season)?;
    let mut mp = BTreeMap::new();
    for row in csv::Reader::from_reader(summary.as_bytes()).deserialize::<MoneyGoalie>() {
        let r = row.map_err(|e| e.to_string())?;
        if r.season != season / 10000 {
            return Err("MoneyPuck goalie season mismatch".into());
        }
        if r.situation != "all" {
            continue;
        }
        if r.id == 0
            || !r.ongoal.is_finite()
            || r.ongoal < 0.
            || r.ongoal.fract() != 0.
            || mp.contains_key(&r.id)
        {
            return Err("Invalid/duplicate MoneyPuck goalie".into());
        }
        mp.insert(r.id, r);
    }
    let mut out = Vec::new();
    for (id, r) in index {
        let games = count(r, "gamesPlayed")?;
        let minutes = n(r, "timeOnIce")?.ok_or("NHL goalie total TOI missing")? / 60.;
        let mut metrics = BTreeMap::new();
        let mut notes = vec![];
        let saves = n(r, "saves")?;
        let shots = n(r, "shotsAgainst")?;
        if saves.zip(shots).is_some_and(|(a, b)| a > b) {
            return Err("NHL saves exceed shots".into());
        }
        ratio(&mut metrics, "goalie-save-fraction", saves, shots);
        ratio(
            &mut metrics,
            "goalie-assists",
            n(r, "assists")?.map(|v| v * 60.),
            Some(minutes),
        );
        if let Some(s) = totals.get(&id) {
            ratio(
                &mut metrics,
                "goalie-low-save",
                Some(f64::from(s.low - s.low_goals)),
                Some(f64::from(s.low)),
            );
            ratio(
                &mut metrics,
                "goalie-high-save",
                Some(f64::from(s.high - s.high_goals)),
                Some(f64::from(s.high)),
            );
            ratio(
                &mut metrics,
                "goalie-cross-save",
                Some(f64::from(s.cross - s.cross_goals)),
                Some(f64::from(s.cross)),
            );
            ratio(
                &mut metrics,
                "goalie-rebound-save",
                Some(f64::from(s.rebound - s.rebound_goals)),
                Some(f64::from(s.rebound)),
            );
            ratio(
                &mut metrics,
                "goalie-rebound-ratio",
                Some(f64::from(s.generated)),
                Some(s.expected),
            );
            if let Some(m) = mp.get(&id) {
                if m.ongoal != f64::from(s.on) {
                    notes.push(format!("MoneyPuck summary reports {} on-goal events; its shot archive supplies {}. Archive-based rates use archive counts throughout.",m.ongoal,s.on));
                }
                if m.games_played != games {
                    notes.push(format!("Same-season provider coverage differs: NHL {games} GP; MoneyPuck {} GP. Each rate uses its own provider numerator and denominator.",m.games_played));
                }
            }
        } else {
            notes.push("No attributed regular-season MoneyPuck shots supplied; four shot-derived attributes remain unavailable.".into());
        }
        out.push(Observation {
            player: Player {
                player_id: id,
                group: "Goalies".into(),
                games,
                minutes,
                metrics,
            },
            notes,
        });
    }
    Ok(out)
}

pub fn skater_inputs(
    season: u32,
    summary: &[Value],
    realtime: &[Value],
    penalties: &[Value],
    toi: &[Value],
) -> Result<Vec<Player>, String> {
    let summary = rows_by_id(summary, season)?;
    let rt = rows_by_id(realtime, season)?;
    let pen = rows_by_id(penalties, season)?;
    let time = rows_by_id(toi, season)?;
    let mut out = Vec::new();
    for (id, r) in summary {
        let games = count(r, "gamesPlayed")?;
        let mut metrics = BTreeMap::new();
        let t = time.get(&id).copied();
        let minutes = n(t.ok_or("NHL skater total TOI row missing")?, "timeOnIce")?
            .ok_or("NHL skater total TOI missing")?
            / 60.;
        for other in [t, rt.get(&id).copied(), pen.get(&id).copied()]
            .into_iter()
            .flatten()
        {
            if count(other, "gamesPlayed")? != games {
                return Err(format!("NHL reports disagree on {id} GP; refresh again"));
            }
        }
        for (key, source, field) in [
            ("goals", Some(r), "goals"),
            ("shots", Some(r), "shots"),
            ("assists", Some(r), "assists"),
            ("giveaways", rt.get(&id).copied(), "giveaways"),
            ("takeaways", rt.get(&id).copied(), "takeaways"),
            ("blocks", rt.get(&id).copied(), "blockedShots"),
            ("drawn", pen.get(&id).copied(), "penaltiesDrawn"),
            ("taken", pen.get(&id).copied(), "penalties"),
        ] {
            if let Some(row) = source {
                ratio(
                    &mut metrics,
                    key,
                    n(row, field)?.map(|v| v * 60.),
                    Some(minutes),
                );
            }
        }
        let group = match r["positionCode"].as_str() {
            Some("D") => "Defense",
            Some("C" | "L" | "R" | "LW" | "RW") => "Forwards",
            _ => return Err("Unknown skater position".into()),
        };
        out.push(Player {
            player_id: id,
            group: group.into(),
            games,
            minutes,
            metrics,
        });
    }
    Ok(out)
}
