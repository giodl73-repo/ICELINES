//! Public five-on-five team results while a skater is on/off ice, not isolated impact.
use icelines_core::view_model::rinkcraft_stat_scores::Player;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};
/// Refresh only the selected window's defensive context; ability grades stay intact.
pub fn enrich(
    profiles: &mut [icelines_core::view_model::rinkcraft_stat_scores::Profile],
    season: u32,
    input: impl Read,
    source_note: &str,
) -> Result<usize, String> {
    let players = profiles
        .iter()
        .filter(|p| p.season == season && p.group != "Goalies")
        .map(|p| Player {
            player_id: p.player_id,
            group: p.group.clone(),
            games: p.games,
            minutes: p.minutes,
            metrics: BTreeMap::new(),
        })
        .collect::<Vec<_>>();
    let mut measured = parse(input, season, &players)?;
    let matched = measured.len();
    for p in profiles
        .iter_mut()
        .filter(|p| p.season == season && p.group != "Goalies")
    {
        if let Some(c) = &mut p.skater_context {
            c.values.retain(|key, _| !key.starts_with("def-5v5-"));
            if let Some(values) = measured.remove(&p.player_id) {
                c.values.extend(values);
            }
            c.notes
                .retain(|n| !n.starts_with("MoneyPuck.com five-on-five source:"));
            c.notes
                .push(format!("MoneyPuck.com five-on-five source: {source_note}"));
            if !c.source_credit.contains("MoneyPuck.com") {
                c.source_credit
                    .push_str("; MoneyPuck.com five-on-five results");
            }
            c.defending = None;
        }
    }
    icelines_core::view_model::rinkcraft_defending_record::attach(profiles, season);
    Ok(matched)
}

pub fn parse(
    input: impl Read,
    season: u32,
    players: &[Player],
) -> Result<BTreeMap<u32, BTreeMap<String, f64>>, String> {
    let mut csv = csv::Reader::from_reader(input);
    let headers = csv.headers().map_err(|e| e.to_string())?.clone();
    let names = [
        "playerId",
        "season",
        "situation",
        "games_played",
        "icetime",
        "timeOnBench",
        "OnIce_A_xGoals",
        "OffIce_A_xGoals",
        "OnIce_A_unblockedShotAttempts",
        "I_F_dZoneShiftStarts",
        "I_F_oZoneShiftStarts",
    ];
    let at = names
        .iter()
        .map(|key| {
            headers
                .iter()
                .position(|h| h == *key)
                .ok_or_else(|| format!("Missing defensive column {key}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let official: BTreeMap<_, _> = players.iter().map(|p| (p.player_id, p)).collect();
    let mut seen = BTreeSet::new();
    let mut out = BTreeMap::new();
    for record in csv.records() {
        let row = record.map_err(|e| e.to_string())?;
        let number = |i: usize| -> Result<f64, String> {
            row[at[i]]
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && *v >= 0.)
                .ok_or_else(|| format!("Invalid defensive {}", names[i]))
        };
        let count = |i: usize| -> Result<u32, String> {
            let v = number(i)?;
            if v.fract() != 0. || v > u32::MAX as f64 {
                return Err("Invalid defensive count".into());
            }
            Ok(v as u32)
        };
        if count(1)? != season / 10000 {
            return Err("Wrong defensive season".into());
        }
        if &row[at[2]] != "5on5" {
            continue;
        }
        let id = count(0)?;
        if id == 0 || !seen.insert(id) {
            return Err("Duplicate/invalid five-on-five actor".into());
        }
        let gp = count(3)?;
        let Some(p) = official.get(&id).filter(|p| p.games == gp) else {
            continue;
        };
        let seconds = number(4)?;
        let bench = number(5)?;
        let xga = number(6)?;
        let off_xga = number(7)?;
        let fa = count(8)?;
        let dz = count(9)?;
        let oz = count(10)?;
        if seconds > p.minutes * 60. + 1. {
            return Err("Five-on-five time exceeds NHL all-strength time".into());
        }
        if xga > fa as f64
            || (seconds == 0. && (xga > 0. || fa > 0))
            || (bench == 0. && off_xga > 0.)
        {
            return Err("Defensive events without valid exposure".into());
        }
        let mut values = BTreeMap::from([
            ("def-5v5-seconds".into(), seconds),
            ("def-5v5-bench-seconds".into(), bench),
            ("def-5v5-xga".into(), xga),
            ("def-5v5-off-xga".into(), off_xga),
            ("def-5v5-fa".into(), fa as f64),
        ]);
        if seconds > 0. {
            values.insert("def-5v5-xga60".into(), xga * 3600. / seconds);
            values.insert("def-5v5-fa60".into(), fa as f64 * 3600. / seconds);
        }
        if bench > 0. {
            values.insert("def-5v5-off-xga60".into(), off_xga * 3600. / bench);
        }
        let starts = dz.checked_add(oz).ok_or("Zone start count overflow")?;
        if starts > 0 {
            values.insert("def-5v5-dz-share".into(), dz as f64 / starts as f64);
        }
        out.insert(id, values);
    }
    if seen.is_empty() {
        return Err("No five-on-five records".into());
    }
    Ok(out)
}
