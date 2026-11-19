//! Explicit public NHL captures -> season/player card files. No network or abilities.
use icelines_core::{
    identity::PlayerId,
    model::Season,
    season_stats::SeasonType,
    view_model::{
        rinkcraft_achievements as feats,
        rinkcraft_character::CharacterCatalog,
        rinkcraft_deployment::{self, UsageInput},
    },
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
};
#[path = "support/rinkcraft_lower_tail.rs"]
mod lower_tail;
fn read(spec: &Value) -> Result<(Vec<u8>, String), String> {
    let bytes =
        fs::read(spec["path"].as_str().ok_or("missing input path")?).map_err(|e| e.to_string())?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    if spec["sha256"].as_str() != Some(hash.as_str()) {
        return Err("input checksum mismatch".into());
    }
    Ok((bytes, hash))
}
fn n(v: &Value, k: &str) -> Result<u32, String> {
    v[k].as_u64()
        .and_then(|n| u32::try_from(n).ok())
        .ok_or_else(|| format!("invalid {k}"))
}
fn metric(key: &str, label: &str, value: Value, unit: &str) -> Value {
    json!({"key":key,"label":label,"value":value,"unit":unit,"status":if value.is_null(){"unavailable"}else{"available"},"aggregation":"selected_season","observed_seasons":1,"missing_seasons":[],"report_source":"NHL official reports","ratio":null})
}
fn usage(
    row: &Value,
    toi: Option<&Value>,
    season: u32,
    goalie: bool,
    source_hash: &str,
) -> Result<Option<Value>, String> {
    let Some(t) = toi else { return Ok(None) };
    if n(t, "gamesPlayed")? != n(row, "gamesPlayed")? || n(t, "seasonId")? != season {
        return Err("usage scope/GP mismatch".into());
    }
    let mut u = rinkcraft_deployment::build(UsageInput {
        player_id: PlayerId(n(row, "playerId")?),
        season: Season(season),
        season_type: SeasonType::Regular,
        games: n(row, "gamesPlayed")?,
        total: n(t, "timeOnIce")?,
        skater_strengths: if goalie {
            None
        } else {
            Some([
                n(t, "evTimeOnIce")?,
                n(t, "ppTimeOnIce")?,
                n(t, "shTimeOnIce")?,
            ])
        },
        goalie_starts: if goalie {
            Some(n(t, "gamesStarted")?)
        } else {
            None
        },
    })?;
    u.source_sha256 = Some(source_hash.into());
    u.source_url=Some(format!("https://api.nhle.com/stats/rest/en/{}/{}?isAggregate=false&isGame=false&cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",if goalie{"goalie"}else{"skater"},if goalie{"summary"}else{"timeonice"}));
    Ok(Some(serde_json::to_value(u).map_err(|e| e.to_string())?))
}
fn card(
    row: &Value,
    a: &Value,
    usage: Option<Value>,
    chars: Option<&CharacterCatalog>,
    capture: &str,
    current: u32,
    analysis: &str,
) -> Result<Value, String> {
    let id = n(row, "playerId")?;
    let year = n(a, "season")?;
    let games = n(row, "gamesPlayed")?;
    let goalie = a["group"] == "Goalies";
    let name = a
        .get("name")
        .and_then(Value::as_str)
        .or_else(|| {
            row[if goalie {
                "goalieFullName"
            } else {
                "skaterFullName"
            }]
            .as_str()
        })
        .ok_or("name missing")?;
    let position = if goalie {
        "Goalie"
    } else {
        row["positionCode"].as_str().ok_or("position missing")?
    };
    let fields = if goalie {
        vec![
            ("wins", "Wins"),
            ("losses", "Losses"),
            ("saves", "Saves"),
            ("shotsAgainst", "Shots against"),
            ("shutouts", "Shutouts"),
        ]
    } else {
        vec![
            ("goals", "Goals"),
            ("assists", "Assists"),
            ("points", "Points"),
            ("shots", "Shots"),
            ("ppPoints", "PP points"),
            ("shPoints", "SH points"),
        ]
    };
    let metrics = fields
        .into_iter()
        .map(|(key, label)| metric(key, label, row[key].clone(), "Count"))
        .collect::<Vec<_>>();
    let record = json!({"season":year,"games":games,"position":position,"teams":row["teamAbbrevs"].as_str().unwrap_or("").split(',').map(str::trim).filter(|s|!s.is_empty()).collect::<Vec<_>>(),"traded":row["teamAbbrevs"].as_str().unwrap_or("").contains(','),"metrics":metrics});
    let sheet = if let Some(c) = chars {
        c.sheet(PlayerId(id), Season(year))?
    } else {
        None
    };
    let mut observations = a.clone();
    let positive = a["feats"]
        .as_array()
        .ok_or("feats absent")?
        .iter()
        .filter(|f| {
            f["state"] == "earned"
                && !f["placements"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("risk_flags"))
        })
        .count();
    let caution = a["earned"].as_u64().ok_or("earned absent")? as usize - positive;
    if !goalie && games >= 20 {
        let lows = lower_tail::low_callouts(a)?;
        observations["profile_observations"] = json!({"schema":"rinkcraft.profile-observations.v1","policy":"lower-tail-observations.v0.1-exploratory","player_id":id,"season":year,"season_type":"regular","group":a["group"],"games":games,"source_sha256":a["source_sha256"],"analysis_sha256":analysis,"positive_count":positive,"caution_count":caution,"low_callouts":lows,"coverage_gaps":["Entry denial","Controlled exits & retrievals","Gap & lane protection","Net-front coverage","Rotations & cover","Passing under pressure","PK prevention"],"note":"Registry-only counts; experimental task extensions may be available in richer dossiers. Entries overlap, not independent abilities. P20 is exploratory; no gameplay penalties."});
    }
    let mut conditions = BTreeMap::new();
    for f in a["feats"].as_array().unwrap() {
        for c in f["conditions"].as_array().unwrap() {
            conditions.insert(c["key"].as_str().unwrap(), c);
        }
    }
    let keys = if goalie {
        vec!["starts", "shutouts", "shots-faced", "save-fraction"]
    } else if a["group"] == "Defense" {
        vec![
            "assists",
            "shots",
            "hits",
            "blocks",
            "takeaways",
            "giveaways",
        ]
    } else {
        vec!["points", "goals", "assists", "shots", "hits", "blocks"]
    };
    let facets=keys.iter().filter_map(|key|conditions.get(key).map(|c|{let m=json!({"key":key,"label":c["label"],"unit":c["unit"],"value":c["value"],"percentile":c["percentile"],"measured_peers":c["peers"],"numerator":c["numerator"],"denominator":c["denominator"],"reference_sha256":c["reference_sha256"],"domain":"observed record","interpretation":"All-strength activity/outcome, affected by deployment; not a capability grade.","state":if c["value"].is_null(){"unavailable"}else if c["percentile"].is_null(){"unranked"}else{"ranked"}});json!({"label":c["label"],"measurement":m})})).collect::<Vec<_>>();
    let mins = usage
        .as_ref()
        .and_then(|u| u["rows"][0]["seconds"].as_f64())
        .map(|s| s / 60.);
    let kind = if year == current { "current" } else { "season" };
    let label = format!(
        "{} · {}–{:02}",
        if year == current { "Current" } else { "Season" },
        year / 10000,
        year % 100
    );
    let numerical=mins.map(|minutes|json!({"season":year,"player_id":id,"name":name,"group":a["group"],"situation":"all strengths","games":games,"minutes":minutes,"eligible_peers":conditions.values().map(|c|c["peers"].as_u64().unwrap_or(0)).max().unwrap_or(0),"facets":facets,"metrics":[],"leading":[],"source_credit":"NHL official reports; complete captured cohort","sources":[],"unknowns":[],"scope_label":label}));
    Ok(
        json!({"schema":"icelines.rinkcraft-preview-profile.v1","player_name":name,"request":{"player_id":id,"first_season":year,"last_season":year,"season_type":"regular"},"scope":"selected season","league_current_season":current,"current_season_available":year==current,"included_seasons":[year],"latest_available":record,"timeline":[record],"summary":metrics,"changes":[],"job_families":[],"unmeasured_dimensions":[],"limitations":["Statistical patterns are not capability grades. Coaching assignments are unassessed."],"character_sheet":sheet,"documentary_catalog_available":chars.is_some(),"attribute_seasons":[],"numerical_evidence":numerical,"achievements":observations,"deployment_evidence":usage,"profile_window":{"player_id":id,"kind":kind,"season":year,"season_type":"regular","as_of":if year==current{Some(capture)}else{None},"coverage":if year==current{"season_to_date"}else{"selected_season"},"label":label,"choices":[{"kind":kind,"label":label,"available":true}],"note":"Exact selected season. Captured public reports, not a live scoreboard. Missing assessments are not weaknesses."},"experimental_patterns":{"schema":"rinkcraft.scoped-patterns.v1","player_id":id,"season":year,"season_type":"regular","group":a["group"],"status":"registry_only","patterns":[],"scope_note":"Registry only; no simulation bonuses."}}),
    )
}
fn run(manifest: &Value, out: &Path) -> Result<(), String> {
    if manifest["schema"] != "rinkcraft.preview-catalog-input.v1" {
        return Err("unsupported input manifest".into());
    }
    if out.exists() {
        return Err("new output directory required".into());
    }
    let current = n(manifest, "current_season")?;
    let mut index = Vec::new();
    let mut seen = BTreeSet::new();
    let chars = if manifest["characters"].is_object() {
        let (b, _) = read(&manifest["characters"])?;
        let c: CharacterCatalog = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
        c.validate()?;
        Some(c)
    } else {
        None
    };
    for w in manifest["windows"].as_array().ok_or("windows absent")? {
        let year = n(w, "season")?;
        if !seen.insert(year) {
            return Err("duplicate season".into());
        }
        let capture = w["captured_through"]
            .as_str()
            .ok_or("capture timestamp absent")?;
        if chrono::DateTime::parse_from_rfc3339(capture).is_err()
            && chrono::NaiveDate::parse_from_str(capture, "%Y-%m-%d").is_err()
        {
            return Err("invalid capture timestamp/date".into());
        }
        let (bytes, hash) = read(&w["reports"])?;
        let input = icelines_sources::nhl::achievements::parse(&bytes)?;
        if input.season != year {
            return Err("source season mismatch".into());
        }
        let source: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let (toi_bytes, toi_hash) = read(&w["timeonice"])?;
        let toi: Value = serde_json::from_slice(&toi_bytes).map_err(|e| e.to_string())?;
        let mut times = BTreeMap::new();
        for r in toi["data"].as_array().ok_or("TOI rows absent")? {
            if n(r, "seasonId")? != year || times.insert(n(r, "playerId")?, r).is_some() {
                return Err("TOI mixed/duplicate actor".into());
            }
        }
        if toi["total"].as_u64() != Some(times.len() as u64) {
            return Err("incomplete TOI cohort".into());
        }
        let catalog = feats::evaluate(&input, &feats::default_rules())?;
        let mut profiles = BTreeMap::new();
        for a in catalog.profiles {
            profiles.insert(
                a.player_id,
                serde_json::to_value(a).map_err(|e| e.to_string())?,
            );
        }
        let directory = out.join("data").join(year.to_string());
        fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
        let mut players = Vec::new();
        for (key, goalie) in [("skaters", false), ("goalies", true)] {
            for row in source[key]["data"].as_array().ok_or("source rows absent")? {
                let id = n(row, "playerId")?;
                let a = profiles.get(&id).ok_or("feat actor absent")?;
                let u = usage(
                    row,
                    if goalie {
                        Some(row)
                    } else {
                        times.get(&id).copied()
                    },
                    year,
                    goalie,
                    if goalie { &hash } else { &toi_hash },
                )?;
                let p = card(row, a, u, chars.as_ref(), capture, current, &hash)?;
                let path = format!("data/{year}/{id}.json");
                fs::write(
                    directory.join(format!("{id}.json")),
                    serde_json::to_vec(&p).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                players.push(json!({"id":id,"name":p["player_name"],"position":p["latest_available"]["position"],"group":a["group"],"teams":p["latest_available"]["teams"],"games":a["games"],"url":path}));
            }
        }
        players.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
        index.push(json!({"season":year,"label":format!("{}–{:02}",year/10000,year%100),"captured_through":capture,"reports_sha256":hash,"players":players}));
    }
    if !seen.contains(&current) {
        return Err("current season is not supplied".into());
    }
    index.sort_by_key(|w| std::cmp::Reverse(w["season"].as_u64()));
    fs::write(out.join("catalog.json"),serde_json::to_vec(&json!({"schema":"rinkcraft.public-preview.v1","current_season":current,"windows":index,"coverage":"All players appearing in supplied regular-season NHL reports; not a verified current active roster. Classes are documentary, capabilities unrated."})).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
    Ok(())
}
fn main() {
    let args: Vec<_> = env::args().collect();
    let result = (|| {
        if args.len() != 3 {
            return Err("Usage: INPUT_MANIFEST NEW_OUTPUT_DIRECTORY".into());
        }
        let m: Value = serde_json::from_slice(&fs::read(&args[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        run(&m, Path::new(&args[2]))
    })();
    if let Err(e) = result {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_value_stays_missing() {
        let m = metric("hits", "Hits", Value::Null, "Count");
        assert!(m["value"].is_null());
        assert_eq!(m["status"], "unavailable");
    }
    #[test]
    fn malformed_counts_rejected() {
        assert!(n(&json!({"games":-1}), "games").is_err());
        assert!(n(&json!({"games":1.5}), "games").is_err());
    }
    #[test]
    fn usage_must_match_scope_and_partition() {
        let row = json!({"playerId":1,"gamesPlayed":20});
        let t = json!({"seasonId":20242025,"gamesPlayed":20,"timeOnIce":100,"evTimeOnIce":80,"ppTimeOnIce":20,"shTimeOnIce":1});
        assert!(usage(&row, Some(&t), 20242025, false, "h").is_err());
        assert!(usage(&row, None, 20242025, false, "h").unwrap().is_none());
    }
}
