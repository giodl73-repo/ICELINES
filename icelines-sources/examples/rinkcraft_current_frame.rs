//! Current-season frame from an explicit complete captured NHL cohort.
use icelines_core::view_model::{rinkcraft_achievements as feats, rinkcraft_numbers as numbers};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, env, fs};
fn read(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| e.to_string())
}
fn decode(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn capture_sources(
    capture: &Value,
    season: u32,
) -> Result<(Vec<numbers::Source>, String, String), String> {
    let mut sources = Vec::new();
    let mut times = Vec::new();
    for page in capture["pages"].as_array().ok_or("missing capture pages")? {
        let url = page["url"].as_str().ok_or("missing page URL")?;
        let u = url::Url::parse(url).map_err(|e| e.to_string())?;
        let exp = u
            .query_pairs()
            .find(|(k, _)| k == "cayenneExp")
            .map(|(_, v)| v.into_owned())
            .ok_or("missing source window")?;
        let filters: Vec<_> = exp.split(" and ").collect();
        if u.scheme() != "https"
            || u.host_str() != Some("api.nhle.com")
            || !filters.contains(&format!("seasonId={season}").as_str())
            || !filters.contains(&"gameTypeId=2")
        {
            return Err("capture URL window/host mismatch".into());
        }
        let bytes = read(page["path"].as_str().ok_or("missing raw path")?)?;
        let digest = hash(&bytes);
        if page["sha256"].as_str() != Some(digest.as_str()) {
            return Err("raw capture hash mismatch".into());
        }
        let stamp = page["accessed_utc"]
            .as_str()
            .ok_or("missing capture timestamp")?;
        let t = chrono::DateTime::parse_from_rfc3339(stamp).map_err(|e| e.to_string())?;
        if t.year() < season as i32 / 10000 || t.year() > season as i32 % 10000 {
            return Err("capture date outside current season years".into());
        }
        times.push(t);
        sources.push(numbers::Source {
            url: url.into(),
            sha256: digest,
            bytes: bytes.len(),
        });
    }
    times.sort();
    if times.is_empty() {
        return Err("empty capture".into());
    }
    Ok((
        sources,
        times[0].to_rfc3339(),
        times.last().unwrap().to_rfc3339(),
    ))
}
use chrono::Datelike;
fn assert_capture_matches(kind: &str, report: &Value, capture: &Value) -> Result<(), String> {
    let route = match kind {
        "skaters" => "skater/summary",
        "realtime" => "skater/realtime",
        "penalties" => "skater/penalties",
        "goalies" => "goalie/summary",
        "timeonice" => "skater/timeonice",
        _ => return Err("unknown report assembly".into()),
    };
    let mut captured = BTreeMap::new();
    let mut total = None;
    for page in capture["pages"]
        .as_array()
        .ok_or("missing capture pages")?
        .iter()
        .filter(|p| p["report"] == kind)
    {
        let u = url::Url::parse(page["url"].as_str().ok_or("missing page URL")?)
            .map_err(|e| e.to_string())?;
        if u.path() != format!("/stats/rest/en/{route}") {
            return Err("raw route differs from report kind".into());
        }
        let raw = decode(&read(page["path"].as_str().ok_or("missing raw path")?)?)?;
        if total.is_some_and(|n| Some(n) != raw["total"].as_u64()) {
            return Err("raw page totals changed".into());
        }
        total = raw["total"].as_u64();
        for row in raw["data"].as_array().ok_or("missing raw rows")? {
            let id = row["playerId"].as_u64().ok_or("missing raw actor")?;
            if captured.insert(id, row.clone()).is_some() {
                return Err("duplicate raw actor".into());
            }
        }
    }
    let rows = report["data"].as_array().ok_or("missing assembled rows")?;
    if total != Some(captured.len() as u64)
        || total != report["total"].as_u64()
        || rows.len() != captured.len()
    {
        return Err("raw pages do not cover assembly".into());
    }
    for row in rows {
        let raw = captured
            .get(&row["playerId"].as_u64().ok_or("missing assembled actor")?)
            .ok_or("assembled actor absent from raw")?;
        let object = row.as_object().ok_or("invalid assembled row")?;
        let original = raw.as_object().ok_or("invalid raw row")?;
        if object.len() != original.len() {
            return Err("assembled fields differ from raw".into());
        }
        for (key, value) in object {
            let source = original.get(key).ok_or("assembled key absent from raw")?;
            let agrees = match (value.as_f64(), source.as_f64()) {
                (Some(a), Some(b)) => (a - b).abs() <= 1e-12 * a.abs().max(b.abs()).max(1.),
                _ => value == source,
            };
            if !agrees {
                return Err(format!("assembled {kind}.{key} differs from raw"));
            }
        }
    }
    Ok(())
}
fn frame(base: &Value, reports: &[u8], toi: &[u8], capture: &Value) -> Result<Value, String> {
    let input = icelines_sources::nhl::achievements::parse(reports)?;
    let (sources, start, end) = capture_sources(capture, input.season)?;
    let raw = decode(reports)?;
    let time = decode(toi)?;
    for kind in ["skaters", "realtime", "penalties", "goalies"] {
        assert_capture_matches(kind, &raw[kind], capture)?;
    }
    assert_capture_matches("timeonice", &time, capture)?;
    let catalog = feats::evaluate(&input, &feats::default_rules())?;
    let mut output = serde_json::Map::new();
    for (key, old) in base.as_object().ok_or("explicit base profiles required")? {
        let actor = old["request"]["player_id"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .ok_or("missing base actor")?;
        if old["league_current_season"].as_u64() != Some(input.season as u64) {
            return Err("captured season is not the declared current season".into());
        }
        let player = input
            .players
            .iter()
            .find(|p| p.player_id == actor)
            .ok_or("current actor missing; no historical fallback")?;
        let achievement = catalog
            .profiles
            .iter()
            .find(|p| p.player_id == actor)
            .unwrap();
        let goalie = player.group == "Goalies";
        let report = if goalie { &raw["goalies"] } else { &time };
        let bytes = serde_json::to_vec(report).map_err(|e| e.to_string())?;
        let route = if goalie { "goalies" } else { "timeonice" };
        let url = capture["pages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["report"] == route)
            .and_then(|p| p["url"].as_str())
            .ok_or("matching workload capture required")?;
        let mut deployment =
            icelines_sources::nhl::deployment::parse(&bytes, actor, input.season, 2, goalie, url)?;
        if deployment.games != player.games {
            return Err("current workload GP conflict".into());
        }
        deployment.source_label = "NHL current-season report assembly".into();
        deployment.notes.push(format!("Captured {start} through {end}. Input hash covers assembled rows; individual raw-page hashes are retained separately. Reports may lag games and are not an atomic live feed."));
        let keys: &[&str] = if goalie {
            &["save-fraction", "shots-faced", "starts", "shutouts"]
        } else {
            &["goals", "assists", "points", "shots", "hits", "blocks"]
        };
        let mut metrics = Vec::new();
        for key in keys {
            let c = achievement
                .feats
                .iter()
                .flat_map(|f| &f.conditions)
                .find(|c| c.key == *key)
                .ok_or("required measured metric unavailable")?;
            let state = if c.value.is_none() {
                "unavailable"
            } else if player.games < achievement.minimum_games {
                "insufficient_exposure"
            } else if c.peers < 20 {
                "insufficient_reference"
            } else {
                "ranked"
            };
            metrics.push(numbers::Measurement{key:(*key).into(),label:c.label.clone(),domain:if goalie{"Recorded goalie outcomes"}else{"Recorded skater activity"}.into(),interpretation:"NHL all-strength observed record. Raw percentile is not an ability, causal effect or stable talent estimate.".into(),unit:c.unit.clone(),value:c.value,percentile:c.percentile,numerator:c.numerator,denominator:c.denominator,scale:1.,measured_peers:c.peers,reference_sha256:c.reference_sha256.clone(),state:state.into(),formula:format!("{} / {}",c.numerator.map(|n|n.to_string()).unwrap_or("unknown".into()),c.denominator.map(|n|n.to_string()).unwrap_or("unknown".into()))});
        }
        let evidence=numbers::EvidenceCard{finishing_research:None,context_evidence:None,feature_context:Vec::new(),style_evidence:None,schema:numbers::SCHEMA.into(),season:input.season,player_id:actor,name:player.name.clone(),group:player.group.clone(),situation:"all strengths".into(),games:player.games,minutes:deployment.rows.iter().find(|r|r.key=="total").ok_or("missing total workload")?.seconds as f64/60.,eligible_peers:metrics.first().map(|m|m.measured_peers).unwrap_or(0),facets:metrics.iter().map(|m|numbers::EvidenceFacet{explanation:None,label:m.label.clone(),measurement:m.clone()}).collect(),leading:Vec::new(),metrics,source_credit:format!("NHL public seasonal reports. Captured {start} through {end}; all strengths. Opportunity-gated raw peer percentiles."),sources:sources.clone(),unknowns:vec!["No current 5v5, xG, tracking, task-success or current coaching-assignment evidence in these reports.".into(),"Class, methods and source accounts are historical dossier material; current continuity is unverified.".into()]};
        let mut evidence_value = serde_json::to_value(&evidence).map_err(|e| e.to_string())?;
        evidence_value["source_label"] = json!("NHL public reports");
        evidence_value["peer_rules"]=json!("Same-season all-strength F/D/G references; minimum 20 skater GP, 15 goalie GP and 20 measured peers. Raw midrank percentiles. These experimental gates are not reliability standards or skill grades.");
        let source_rows = raw[if goalie { "goalies" } else { "skaters" }]["data"]
            .as_array()
            .ok_or("missing source actor rows")?;
        let source_row = source_rows
            .iter()
            .find(|r| r["playerId"].as_u64() == Some(actor as u64))
            .unwrap();
        let mut totals: BTreeMap<String, f64> = player.counts.clone();
        totals.insert("games".into(), player.games as f64);
        let raw_metrics:Vec<_>=totals.iter().map(|(key,value)|json!({"key":key,"label":key.replace('_'," "),"unit":"Count","value":value,"status":"available","aggregation":"selected_season_count","observed_seasons":1,"missing_seasons":[],"report_source":"NHL captured current-season reports","ratio":null})).collect();
        let position = if goalie {
            "Goalie"
        } else if player.group == "Defense" {
            "Defense"
        } else {
            "Forward"
        };
        let record = json!({"season":input.season,"games":player.games,"position":position,"teams":source_row["teamAbbrevs"].as_str().map(|s|s.split(',').map(str::trim).collect::<Vec<_>>()).unwrap_or_default(),"traded":source_row["teamAbbrevs"].as_str().is_some_and(|s|s.contains(',')),"metrics":raw_metrics});
        output.insert(key.clone(),json!({"request":{"player_id":actor,"season_type":"regular","first_season":input.season,"last_season":input.season},"scope":"current_season_to_date","league_current_season":input.season,"current_season_available":true,"source_windows":[{"season":input.season,"season_type":"regular","missing_reports":["current_5v5_xg_tracking_and_task_opportunities"]}],"included_seasons":[input.season],"absent_seasons_in_requested_range":[],"latest_available":record,"summary":raw_metrics,"timeline":[record],"changes":[],"attribute_seasons":[],"numerical_evidence":evidence_value,"numerical_snapshot":{"selected_season":input.season,"newest_supplied_season":input.season,"requested_season":input.season,"state":"exact","note":format!("Current season-to-date snapshot captured through {end}. Historical dossier is not a current task assessment.")},"numerical_history":null,"numerical_comparison_choices":[],"deployment_evidence":deployment,"achievements":achievement,"unmeasured_dimensions":["Current task effectiveness","Current 5v5/xG/tracking","Current system duty"],"limitations":["Partial-season descriptive capture. Missing/insufficient opportunities are not low ability.","Historical documentary identity is dated; current continuity unverified."],"capture_period":{"from":start,"through":end}}));
    }
    Ok(Value::Object(output))
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let run = || -> Result<(), String> {
        if a.len() != 6 {
            return Err(
                "Usage: BASE_FIXTURES CURRENT_REPORTS CURRENT_TOI CAPTURE_MANIFEST OUTPUT_PATCHES"
                    .into(),
            );
        }
        let result = frame(
            &decode(&read(&a[1])?)?,
            &read(&a[2])?,
            &read(&a[3])?,
            &decode(&read(&a[4])?)?,
        )?;
        fs::write(
            &a[5],
            serde_json::to_vec(&result).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Built explicit current-season frames; no historical stat fallback.");
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
    fn empty_capture_does_not_get_a_fake_asof_date() {
        assert!(capture_sources(&json!({"pages":[]}), 20262027).is_err());
    }
    #[test]
    fn historical_source_url_cannot_be_current() {
        assert!(capture_sources(&json!({"pages":[{"url":"https://api.nhle.com/stats/rest/en/skater/summary?cayenneExp=seasonId%3D20242025%20and%20gameTypeId%3D2"}]}),20262027).is_err());
    }
}
