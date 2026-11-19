//! Explicit seasonal preview adapter. No career/current inference or live release.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{env, fs};
fn read(path: &str) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn season_label(s: u64) -> String {
    format!("{}–{:02}", s / 10000, s % 100)
}
fn bounded_history(entries: &[Value], season: u64) -> Vec<Value> {
    entries
        .iter()
        .filter(|v| v["season"].as_u64().is_some_and(|s| s <= season))
        .cloned()
        .collect()
}
fn project(mut profiles: Value, input: &Value) -> Result<Value, String> {
    if input["untouched_holdout"] != false {
        return Err("explicit retrospective source manifest required".into());
    }
    let windows = input["windows"]
        .as_array()
        .ok_or("missing explicit windows")?;
    let mut loaded = Vec::new();
    for w in windows {
        let source = read(w["reports"].as_str().ok_or("missing source path")?)?;
        let screen = read(w["screen"].as_str().ok_or("missing screen path")?)?;
        if source["season"] != w["season"]
            || screen["season"] != w["season"]
            || screen["season_type"] != "regular"
        {
            return Err("source/screen window mismatch".into());
        }
        loaded.push((
            w["season"].as_u64().ok_or("missing season")?,
            source,
            screen,
        ));
    }
    let names = [
        ("accuracy-ledger", "Shooting Conversion", "task_skills"),
        ("pp-efficiency", "PP Production Rate", "task_skills"),
        ("penalty-balance", "Whistle Balance", "playing_tendencies"),
    ];
    for profile in profiles
        .as_object_mut()
        .ok_or("profiles must be an object")?
        .values_mut()
    {
        let actor = profile["request"]["player_id"]
            .as_u64()
            .ok_or("missing actor")?;
        let season = profile["request"]["first_season"]
            .as_u64()
            .ok_or("exact selected season required")?;
        if profile["request"]["last_season"].as_u64() != Some(season)
            || profile["request"]["season_type"] != "regular"
            || profile["latest_available"]["season"].as_u64() != Some(season)
            || profile["numerical_evidence"]["season"].as_u64() != Some(season)
        {
            return Err("all primary records must match one selected season".into());
        }
        let current = profile["league_current_season"]
            .as_u64()
            .ok_or("missing league current season")?;
        let current_scope = input["current_season"].as_u64() == Some(season) && season == current;
        let as_of = if current_scope {
            let stamp = input["current_as_of"]
                .as_str()
                .ok_or("current scope requires a capture date")?;
            chrono::DateTime::parse_from_rfc3339(stamp).map_err(|e| e.to_string())?;
            Some(stamp)
        } else {
            None
        };
        let group = profile["numerical_evidence"]["group"]
            .as_str()
            .ok_or("missing group")?
            .to_owned();
        let (_, source, screen) = loaded
            .iter()
            .find(|(s, _, _)| *s == season)
            .ok_or("selected window unavailable")?;
        let kind = if group == "Goalies" {
            "goalies"
        } else {
            "skaters"
        };
        let rows: Vec<_> = source[kind]["data"]
            .as_array()
            .ok_or("missing actor rows")?
            .iter()
            .filter(|r| r["playerId"].as_u64() == Some(actor))
            .collect();
        if rows.len() != 1 || rows[0]["gamesPlayed"] != profile["latest_available"]["games"] {
            return Err("profile/source actor or GP mismatch".into());
        }
        profile["profile_window"] = json!({"player_id":actor,"kind":"season","season":season,"season_type":"regular","as_of":null,"label":format!("Season · {}",season_label(season)),"coverage":"selected_season","choices":[{"kind":"season","label":format!("Season · {}",season_label(season)),"available":true},{"kind":"career","label":"Career · complete coverage not loaded","available":false},{"kind":"current","label":format!("Current · {} not loaded",season_label(current)),"available":false}],"note":"One selected time window governs the whole profile. Source accounts retain their dates; biographical facts retain their historical meaning. Current requires dated season-to-date records. Career requires verified complete coverage and pooled opportunities, not averaged seasonal percentiles."});
        if current_scope {
            let historical = input["historical_season"]
                .as_u64()
                .ok_or("current preview requires its historical alternative")?;
            profile["profile_window"] = json!({"player_id":actor,"kind":"current","season":season,"season_type":"regular","as_of":as_of,"label":format!("Current · {}",season_label(season)),"coverage":"season_to_date","choices":[{"kind":"season","label":format!("Season · {}",season_label(historical)),"available":true},{"kind":"career","label":"Career · complete coverage not loaded","available":false},{"kind":"current","label":format!("Current · {}",season_label(season)),"available":true}],"note":"Current season-to-date through the declared capture time, not a live feed. All primary statistics and opportunities use this window. Historical class/method accounts retain their dates; current continuity is unverified. Career coverage is not established."});
        }
        let mut patterns = Vec::new();
        if group != "Goalies" {
            for (id, name, placement) in names {
                let result = screen["results"]
                    .as_array()
                    .ok_or("missing candidate results")?
                    .iter()
                    .find(|r| r["definition"]["id"] == id)
                    .ok_or("candidate result missing")?;
                let g = result["primary"]["groups"]
                    .as_array()
                    .ok_or("missing candidate groups")?
                    .iter()
                    .find(|g| g["group"] == group)
                    .ok_or("candidate group missing")?;
                let observation = g["players"]
                    .as_array()
                    .ok_or("missing candidate players")?
                    .iter()
                    .find(|p| p["player_id"].as_u64() == Some(actor));
                let state = if observation.is_none() {
                    "insufficient_exposure"
                } else if g["eligible"].as_u64().unwrap_or(0) < 20 {
                    "insufficient_reference_population"
                } else if observation.unwrap()["qualifies"] == true {
                    "earned"
                } else {
                    "not_earned"
                };
                let mut history = Vec::new();
                for (year, _, s) in &loaded {
                    if *year > season {
                        continue;
                    }
                    let point = s["results"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|r| r["definition"]["id"] == id)
                        .and_then(|r| r["primary"]["groups"].as_array())
                        .and_then(|groups| groups.iter().find(|g| g["group"] == group));
                    if let Some(h) = point {
                        let o = h["players"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .find(|p| p["player_id"].as_u64() == Some(actor));
                        history.push(json!({"season":year,"state":if o.is_none(){"insufficient_exposure"}else if o.unwrap()["qualifies"]==true{"earned"}else{"not_earned"},"eligible_peers":h["eligible"],"conditions":o.map(|p|p["conditions"].clone()).unwrap_or(json!([]))}));
                    }
                }
                history.sort_by_key(|h| h["season"].as_u64());
                let conditions = observation
                    .map(|p| p["conditions"].clone())
                    .unwrap_or(json!([]));
                let mut opportunities = match id {
                    "accuracy-ledger" => {
                        json!({"games":rows[0]["gamesPlayed"],"shots":rows[0]["shots"],"minimum_shots":100})
                    }
                    "pp-efficiency" => {
                        json!({"games":rows[0]["gamesPlayed"],"minimum_pp_minutes":120,"pp_points":rows[0]["ppPoints"],"pp_seconds":conditions.as_array().and_then(|c|c.get(1)).map(|c|c["value"].clone())})
                    }
                    _ => json!({"games":rows[0]["gamesPlayed"],"minimum_games":20}),
                };
                if id == "pp-efficiency"
                    && profile["deployment_evidence"]["season"].as_u64() == Some(season)
                    && profile["deployment_evidence"]["player_id"].as_u64() == Some(actor)
                {
                    if let Some(row) = profile["deployment_evidence"]["rows"]
                        .as_array()
                        .and_then(|rows| rows.iter().find(|r| r["key"] == "pp"))
                    {
                        opportunities["pp_seconds"] = row["seconds"].clone();
                    }
                }
                if id == "penalty-balance" {
                    if let Some(row) = source["penalties"]["data"].as_array().and_then(|rows| {
                        rows.iter().find(|r| r["playerId"].as_u64() == Some(actor))
                    }) {
                        opportunities["penalties_drawn"] = row["penaltiesDrawn"].clone();
                        opportunities["penalties_taken"] = row["penalties"].clone();
                    }
                }
                patterns.push(json!({"id":id,"name":name,"placement":placement,"state":state,"reference_mode":"same_season","reference_label":format!("{} eligible {} peers",season_label(season),group),"eligible_peers":g["eligible"],"threshold":60,"conditions":conditions,"opportunities":opportunities,"minimums":result["definition"]["minimums"],"history":bounded_history(&history,season),"note":result["definition"]["meaning"]}));
            }
        }
        let selected_window = windows
            .iter()
            .find(|w| w["season"].as_u64() == Some(season))
            .unwrap();
        let input_hash = |key: &str| -> Result<String, String> {
            Ok(format!(
                "{:x}",
                Sha256::digest(
                    fs::read(
                        selected_window[key]
                            .as_str()
                            .ok_or("missing evidence path")?
                    )
                    .map_err(|e| e.to_string())?
                )
            ))
        };
        let mut urls = source["source_urls"]
            .as_array()
            .ok_or("missing source URLs")?
            .clone();
        if profile["deployment_evidence"]["season"].as_u64() == Some(season) {
            if let Some(url) = profile["deployment_evidence"]["source_url"].as_str() {
                urls.push(json!(url));
            }
        }
        profile["experimental_patterns"] = json!({"schema":"rinkcraft.scoped-patterns.v1","player_id":actor,"season":season,"season_type":"regular","group":group,"status":"experimental_not_live_registry","patterns":patterns,"source_urls":urls,"source_sha256":input_hash("reports")?,"screen_sha256":input_hash("screen")?,"scope_note":"Season values and same-season eligible references. History ends at the selected season. Qualifying patterns are not 1–20 abilities or simulation bonuses."});
    }
    Ok(profiles)
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let run = || -> Result<(), String> {
        if a.len() != 4 {
            return Err(
                "Usage: BASE_PROFILE_FIXTURES LONGITUDINAL_INPUT_MANIFEST OUTPUT_PATCHES".into(),
            );
        }
        let p = project(read(&a[1])?, &read(&a[2])?)?;
        let patches: serde_json::Map<String, Value> = p.as_object().unwrap().iter().map(|(key, profile)|
            (key.clone(), json!({"profile_window":profile["profile_window"],"experimental_patterns":profile["experimental_patterns"]}))).collect();
        fs::write(
            &a[3],
            serde_json::to_vec(&patches).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Projected season-scoped preview with explicit career/current coverage gaps.");
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_never_crosses_selected_season() {
        let h = vec![
            json!({"season":20232024}),
            json!({"season":20242025}),
            json!({"season":20252026}),
        ];
        assert_eq!(bounded_history(&h, 20242025).len(), 2);
    }
    #[test]
    fn missing_window_is_not_a_career_profile() {
        assert!(project(
            json!({"p":{"request":{"player_id":1,"season_type":"regular","first_season":null}}}),
            &json!({"untouched_holdout":false,"windows":[]})
        )
        .is_err());
    }
}
