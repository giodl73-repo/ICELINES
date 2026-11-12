//! Dated, appearance-based linemate history. No line-number or assignment inference.
use crate::observed_units::reconstruct_player;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Appearance {
    pub game_id: u64,
    pub game_date: String,
    pub season: u64,
    pub game_type_id: u64,
    pub team_id: u64,
    pub team_abbrev: String,
    pub opponent: String,
    pub box_path: Option<String>,
    pub shifts_path: Option<String>,
}
fn read(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(Path::new(path))?)?)
}
fn combination_key(v: &Value) -> Vec<u64> {
    v["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_u64().unwrap())
        .collect()
}
fn run_key(day: &Value) -> Option<String> {
    if day["status"] != "ok"
        || !day["primary_trio"].is_object()
        || day["primary_trio_tied"].as_bool() == Some(true)
    {
        return None;
    }
    Some(format!(
        "{}:{}:{}:{:?}",
        day["appearance"]["season"],
        day["appearance"]["gameTypeId"],
        day["appearance"]["teamId"],
        combination_key(&day["primary_trio"])
    ))
}

fn runs(days: &[Value]) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut last_key = None;
    for day in days {
        let key = run_key(day);
        if key.is_none() {
            last_key = None;
            continue;
        }
        let date = day["appearance"]["gameDate"].clone();
        if key != last_key {
            out.push(json!({"start_date":date,"end_date":date,"appearances":1,"team":day["appearance"]["teamAbbrev"],"season":day["appearance"]["season"],"game_type":day["appearance"]["gameTypeId"],"trio":day["primary_trio"]["members"],"trio_seconds":day["primary_trio"]["seconds"],"focused_five_on_five_seconds":day["focused_five_on_five_seconds"]}));
        } else {
            let r = out.last_mut().unwrap();
            r["end_date"] = date;
            r["appearances"] = json!(r["appearances"].as_u64().unwrap() + 1);
            r["trio_seconds"] = json!(
                r["trio_seconds"].as_u64().unwrap()
                    + day["primary_trio"]["seconds"].as_u64().unwrap()
            );
            r["focused_five_on_five_seconds"] = json!(
                r["focused_five_on_five_seconds"].as_u64().unwrap()
                    + day["focused_five_on_five_seconds"].as_u64().unwrap()
            );
        }
        last_key = key;
    }
    out
}

/// Includes failed games as missing rows; no silent drop or inferred nonappearance.
pub fn build(mut appearances: Vec<Appearance>, player: u64) -> Result<Value> {
    appearances.sort_by(|a, b| {
        a.game_date
            .cmp(&b.game_date)
            .then_with(|| a.game_id.cmp(&b.game_id))
    });
    let mut ids = BTreeSet::new();
    let mut days = Vec::new();
    for a in appearances {
        if !ids.insert(a.game_id) {
            return Err("Duplicate appearance game ID".into());
        }
        let result = (|| -> Result<Value> {
            let b = read(a.box_path.as_deref().ok_or("Missing box capture")?)?;
            if b["id"].as_u64() != Some(a.game_id)
                || b["gameDate"].as_str() != Some(a.game_date.as_str())
            {
                return Err("Appearance/source game or date mismatch".into());
            }
            let shifts = read(a.shifts_path.as_deref().ok_or("Missing shift capture")?)?;
            let data = shifts["data"].as_array().ok_or("Missing shift data")?;
            if data.is_empty() {
                return Err("No shift exposure rows returned".into());
            }
            if let Some(total) = shifts["total"].as_u64() {
                if total != data.len() as u64 {
                    return Err("Incomplete paginated shift capture".into());
                }
            }
            reconstruct_player(&b, &shifts, a.team_id, player)
        })();
        match result {
            Ok(units)=>{
                let trios=units["five_on_five_trios"].as_array().unwrap();let primary=trios.first().cloned().unwrap_or(Value::Null);
                let tied=trios.len()>1 && trios[0]["seconds"]==trios[1]["seconds"];
                let exposure=units["focused_state_seconds"]["5v5"].as_u64().unwrap_or(0);
                let assigned:u64=trios.iter().map(|r|r["seconds"].as_u64().unwrap()).sum();
                if assigned>exposure{return Err("Trio exposure exceeds player exposure".into());}
                let share=if exposure>0 {primary["seconds"].as_f64().map(|s|s/exposure as f64)}else{None};
                days.push(json!({"appearance":a,"status":"ok","focused_five_on_five_seconds":exposure,
                    "unassigned_nonstandard_five_on_five_seconds":exposure-assigned,
                    "primary_trio":primary,"primary_trio_share":share,"primary_trio_tied":tied,
                    "display_pattern":if share.is_none(){"no_valid_trio"}else if tied{"tied_most_used"}else if share.unwrap()>=0.5{"at_least_half"}else{"mixed"},"units":units}));
            }
            Err(e)=>days.push(json!({"appearance":a,"status":"missing_or_invalid_evidence","error":e.to_string()})),
        }
    }
    let mut summaries = BTreeMap::<(u64, u64, String), Value>::new();
    let mut partners = BTreeMap::<(u64, u64, String, u64), (String, u64)>::new();
    for d in &days {
        let a = &d["appearance"];
        let season = a["season"].as_u64().unwrap();
        let phase = a["gameTypeId"].as_u64().unwrap();
        let team = a["teamAbbrev"].as_str().unwrap().to_owned();
        let summary=summaries.entry((season,phase,team.clone())).or_insert_with(||json!({"appearances":0,"resolved":0,"focused_five_on_five_seconds":0,"unassigned_nonstandard_seconds":0}));
        summary["appearances"] = json!(summary["appearances"].as_u64().unwrap() + 1);
        if d["status"] != "ok" {
            continue;
        }
        summary["resolved"] = json!(summary["resolved"].as_u64().unwrap() + 1);
        summary["focused_five_on_five_seconds"] = json!(
            summary["focused_five_on_five_seconds"].as_u64().unwrap()
                + d["focused_five_on_five_seconds"].as_u64().unwrap()
        );
        summary["unassigned_nonstandard_seconds"] = json!(
            summary["unassigned_nonstandard_seconds"].as_u64().unwrap()
                + d["unassigned_nonstandard_five_on_five_seconds"]
                    .as_u64()
                    .unwrap()
        );
        for trio in d["units"]["five_on_five_trios"].as_array().unwrap() {
            for member in trio["members"].as_array().unwrap() {
                let pid = member["id"].as_u64().unwrap();
                if pid == player {
                    continue;
                }
                let entry = partners
                    .entry((season, phase, team.clone(), pid))
                    .or_insert_with(|| (member["name"].as_str().unwrap().to_owned(), 0));
                entry.1 += trio["seconds"].as_u64().unwrap();
            }
        }
    }
    let mut seasons = Vec::new();
    for ((season, phase, team), mut summary) in summaries {
        let mut rows: Vec<_> = partners
            .iter()
            .filter(|((s, p, t, _), _)| *s == season && *p == phase && *t == team)
            .map(
                |((_, _, _, pid), (name, seconds))| json!({"id":pid,"name":name,"seconds":seconds}),
            )
            .collect();
        rows.sort_by(|a, b| {
            b["seconds"]
                .as_u64()
                .cmp(&a["seconds"].as_u64())
                .then_with(|| a["id"].as_u64().cmp(&b["id"].as_u64()))
        });
        summary["season"] = json!(season);
        summary["game_type"] = json!(phase);
        summary["team"] = json!(team);
        summary["forward_linemates"] = json!(rows);
        seasons.push(summary);
    }
    let runs = runs(&days);
    let successful = days.iter().filter(|d| d["status"] == "ok").count();
    Ok(
        json!({"schema":"icelines.player-line-history.0.1-experimental","player_id":player,"appearances":days.len(),"resolved_games":successful,
        "scope":"NHL appearance dates; regulation only; all focused combinations retained","primary_share_display_threshold":0.5,
        "limits":["No inferred coach line number or tactical assignment","No appearance row means no supported assignment, not a scratch diagnosis",
            "Primary runs mean consecutive appearances with the same most-used trio, not continuous calendar assignment","Separate regular season/playoffs and team/season boundaries",
            "Overlap share is not causal chemistry; forward linemate shared seconds count two teammates per standard trio","Conflict censoring and nonstandard personnel remain explicit"],
        "season_team_summaries":seasons,"primary_trio_runs":runs,"days":days}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tied_primary_does_not_create_an_arbitrary_run() {
        let mut d = day("2000-01-01", 1, 1, 2, &[1, 2, 3]);
        d["primary_trio_tied"] = json!(true);
        assert!(runs(&[d]).is_empty());
    }
    fn day(date: &str, team: u64, season: u64, phase: u64, ids: &[u64]) -> Value {
        json!({"status":"ok","appearance":{"gameDate":date,"teamId":team,"teamAbbrev":"X","season":season,"gameTypeId":phase},"primary_trio":{"members":ids.iter().map(|id|json!({"id":id})).collect::<Vec<_>>(),"seconds":100},"focused_five_on_five_seconds":200})
    }
    #[test]
    fn runs_break_on_team_season_phase_and_missing_evidence() {
        let a = day("2000-01-01", 1, 1, 2, &[1, 2, 3]);
        let b = day("2000-01-02", 1, 1, 2, &[1, 2, 3]);
        let c = day("2000-01-03", 2, 1, 2, &[1, 2, 3]);
        assert_eq!(runs(&[a.clone(), b, c]).len(), 2);
        assert_eq!(
            runs(&[a.clone(), day("2000-01-04", 1, 2, 2, &[1, 2, 3])]).len(),
            2
        );
        assert_eq!(
            runs(&[a.clone(), day("2000-01-04", 1, 1, 3, &[1, 2, 3])]).len(),
            2
        );
        assert_eq!(runs(&[a.clone(), json!({"status":"missing"}), a]).len(), 2);
    }
    #[test]
    fn absent_capture_is_preserved_as_missing_game() {
        let a = Appearance {
            game_id: 1,
            game_date: "2000-01-01".into(),
            season: 1,
            game_type_id: 2,
            team_id: 1,
            team_abbrev: "X".into(),
            opponent: "Y".into(),
            box_path: None,
            shifts_path: None,
        };
        let r = build(vec![a.clone()], 1).unwrap();
        assert_eq!(r["appearances"], 1);
        assert_eq!(r["resolved_games"], 0);
        assert!(build(vec![a.clone(), a], 1).is_err());
    }
}
