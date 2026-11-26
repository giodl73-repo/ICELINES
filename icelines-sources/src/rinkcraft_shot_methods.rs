//! Credited unblocked shot-event types from finalized public NHL game data.
use icelines_core::view_model::rinkcraft_shot_methods::{label, Counts};
use serde_json::Value;
use std::collections::BTreeMap;
pub fn parse_game(
    bytes: &[u8],
    id: u64,
    season: u32,
) -> Result<BTreeMap<u32, BTreeMap<String, Counts>>, String> {
    let admitted = crate::rinkcraft_pressure::parse_game(bytes, id, season)?;
    let v: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let roster = v["rosterSpots"]
        .as_array()
        .ok_or("Missing shot-method roster")?
        .iter()
        .map(|r| {
            (
                r["playerId"].as_u64().unwrap(),
                (r["teamId"].as_u64().unwrap(), r["positionCode"] == "G"),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut out = admitted
        .into_keys()
        .map(|id| (id, BTreeMap::<String, Counts>::new()))
        .collect::<BTreeMap<_, _>>();
    for e in v["plays"].as_array().unwrap() {
        if e["periodDescriptor"]["periodType"] == "SO" {
            continue;
        }
        let kind = e["typeDescKey"].as_str().unwrap_or("");
        if !["goal", "shot-on-goal", "missed-shot"].contains(&kind) {
            continue;
        }
        let period = e["periodDescriptor"]["number"]
            .as_u64()
            .ok_or("Missing shot-method period")?;
        let limit = match period {
            1..=3 if e["periodDescriptor"]["periodType"] == "REG" => 1200,
            4 if e["periodDescriptor"]["periodType"] == "OT" => 300,
            _ => return Err("Invalid regular-season shot-method period".into()),
        };
        let (m, s) = e["timeInPeriod"]
            .as_str()
            .ok_or("Missing shot-method clock")?
            .split_once(':')
            .ok_or("Invalid shot-method clock")?;
        let (m, s) = (
            m.parse::<u32>().map_err(|e| e.to_string())?,
            s.parse::<u32>().map_err(|e| e.to_string())?,
        );
        if m > 20 || s >= 60 || m * 60 + s > limit {
            return Err("Invalid shot-method clock".into());
        }
        let d = &e["details"];
        let actor = d[if kind == "goal" {
            "scoringPlayerId"
        } else {
            "shootingPlayerId"
        }]
        .as_u64()
        .ok_or("Missing shot-method actor")?;
        let (team, goalie) = roster.get(&actor).ok_or("Unknown shot-method actor")?;
        if d["eventOwnerTeamId"].as_u64() != Some(*team) {
            return Err("Shot-method actor/team mismatch".into());
        }
        if *goalie {
            continue;
        }
        let key = d["shotType"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        let key = if label(&key).is_some() {
            key
        } else {
            "unknown".into()
        };
        let c = out
            .get_mut(&(actor as u32))
            .ok_or("Shot-method skater absent")?
            .entry(key)
            .or_default();
        c.attempts += 1;
        c.on_goal += u32::from(kind != "missed-shot");
        c.goals += u32::from(kind == "goal");
    }
    Ok(out)
}
