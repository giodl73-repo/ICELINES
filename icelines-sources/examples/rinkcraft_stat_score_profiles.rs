//! Add simple Rust stat estimates to an existing player-preview catalog.
use icelines_core::view_model::rinkcraft_stat_scores::{build, observed_player};
use serde_json::{
    value::{to_raw_value, RawValue},
    Value,
};
use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path},
};
type RawMap = BTreeMap<String, Box<RawValue>>;
fn finish_profiles(
    files: &mut [(String, Vec<u8>)],
    catalog: &Value,
) -> Result<(), Box<dyn std::error::Error>> {
    // Same-snapshot roster GP gives a simple estimate of team games played.
    // Multi-team players keep the standard pace unless a limit is supplied.
    let current = catalog["current_season"].as_u64().unwrap_or(0);
    let nominal: u32 = if current >= 20262027 { 84 } else { 82 };
    let mut teams = BTreeMap::new();
    let mut team_games = BTreeMap::<String, u32>::new();
    for w in catalog["windows"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|w| w["season"].as_u64() == Some(current))
    {
        for p in w["players"].as_array().into_iter().flatten() {
            if let Some(clubs) = p["teams"].as_array().filter(|clubs| clubs.len() == 1) {
                if let Some(team) = clubs[0]
                    .as_str()
                    .filter(|s| s.len() == 3 && s.bytes().all(|c| c.is_ascii_uppercase()))
                {
                    let gp = p["games"].as_u64().unwrap_or(0) as u32;
                    team_games
                        .entry(team.into())
                        .and_modify(|n| *n = (*n).max(gp))
                        .or_insert(gp);
                    if let Some(id) = p["id"].as_u64() {
                        teams.insert(id, team.to_string());
                    }
                }
            }
        }
    }
    let mut achievements = Vec::new();
    for (_, bytes) in files.iter() {
        let p: Value = serde_json::from_slice(bytes)?;
        if !p["achievements"].is_null() {
            achievements.push(serde_json::from_value::<
                icelines_core::view_model::rinkcraft_achievements::AchievementProfile,
            >(p["achievements"].clone())?);
        }
    }
    icelines_core::view_model::rinkcraft_achievements::refresh_profiles(&mut achievements);
    let refreshed = achievements
        .into_iter()
        .map(|p| ((p.season, p.player_id), p))
        .collect::<BTreeMap<_, _>>();
    for (_, bytes) in files.iter_mut() {
        let p: Value = serde_json::from_slice(bytes)?;
        let mut enriched: RawMap = serde_json::from_slice(bytes)?;
        if p["numerical_evidence"].is_object() {
            let n = &p["numerical_evidence"];
            let measures = n["facets"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|f| {
                    let m = &f["measurement"];
                    Some(
                        icelines_core::view_model::rinkcraft_stat_scores::RecordMetric {
                            key: m["key"].as_str()?.into(),
                            label: f["label"].as_str()?.into(),
                            unit: m["unit"].as_str()?.into(),
                            value: m["value"].as_f64(),
                        },
                    )
                })
                .collect::<Vec<_>>();
            let supplied_max = p["max_possible_games"].as_u64().map(|v| v as u32);
            let inferred_max = if n["season"].as_u64() == Some(current) {
                n["player_id"]
                    .as_u64()
                    .and_then(|id| teams.get(&id))
                    .and_then(|team| team_games.get(team))
                    .map(|team_gp| {
                        n["games"].as_u64().unwrap_or(0) as u32 + nominal.saturating_sub(*team_gp)
                    })
            } else {
                None
            };
            let mut projection = icelines_core::view_model::rinkcraft_stat_scores::estimate_record(
                n["season"].as_u64().unwrap_or(0) as u32,
                n["games"].as_u64().unwrap_or(0) as u32,
                n["minutes"].as_f64().unwrap_or(0.),
                supplied_max.or(inferred_max),
                &measures,
            );
            if projection.target_games < nominal && n["season"].as_u64() == Some(current) {
                projection.basis = format!("{} GP ceiling · current pace", projection.target_games);
            }
            let mut evidence: RawMap = serde_json::from_str(enriched["numerical_evidence"].get())?;
            evidence.insert("estimated_record".into(), to_raw_value(&projection)?);
            enriched.insert("numerical_evidence".into(), to_raw_value(&evidence)?);
            *bytes = serde_json::to_vec(&enriched)?;
        }
        let a = &p["achievements"];
        if let (Some(season), Some(id)) = (a["season"].as_u64(), a["player_id"].as_u64()) {
            if let Some(updated) = refreshed.get(&(season as u32, id as u32)) {
                let mut root: RawMap = serde_json::from_slice(bytes)?;
                let mut record: RawMap = serde_json::from_str(root["achievements"].get())?;
                record.insert("feats".into(), to_raw_value(&updated.feats)?);
                record.insert("earned".into(), to_raw_value(&updated.earned)?);
                record.insert(
                    "minimum_games".into(),
                    to_raw_value(&updated.minimum_games)?,
                );
                record.insert("notes".into(), to_raw_value(&updated.notes)?);
                root.insert("achievements".into(), to_raw_value(&record)?);
                *bytes = serde_json::to_vec(&root)?;
            }
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&args.len()) {
        return Err(
            "Usage: rinkcraft_stat_score_profiles CATALOG_DIRECTORY NEW_OUTPUT_DIRECTORY [RETAINED_EDGE_DIRECTORY]".into(),
        );
    }
    let base = Path::new(&args[0]);
    let out = Path::new(&args[1]);
    let retained_edge = args.get(2).map(Path::new);
    let mut recovered = Vec::new();
    if out.exists() {
        return Err("Output directory already exists".into());
    }
    let catalog_bytes = fs::read(base.join("catalog.json"))?;
    let catalog: Value = serde_json::from_slice(&catalog_bytes)?;
    let mut rendered = Vec::new();
    let mut scored = 0;
    let mut complete = 0;
    for window in catalog["windows"]
        .as_array()
        .ok_or("Missing catalog windows")?
    {
        let season = window["season"].as_u64().ok_or("Missing season")? as u32;
        let mut originals = Vec::new();
        let mut inputs = Vec::new();
        for actor in window["players"]
            .as_array()
            .ok_or("Missing catalog players")?
        {
            let url = actor["url"].as_str().ok_or("Missing player URL")?;
            if Path::new(url)
                .components()
                .any(|p| !matches!(p, Component::Normal(_)))
            {
                return Err("Player path must be relative".into());
            }
            let mut bytes = fs::read(base.join(url))?;
            let p: Value = serde_json::from_slice(&bytes)?;
            let n = &p["numerical_evidence"];
            let group = n["group"].as_str().unwrap_or("");
            if ["Forwards", "Defense"].contains(&group) {
                let id = n["player_id"].as_u64().ok_or("Missing player ID")? as u32;
                if actor["id"].as_u64() != Some(id as u64)
                    || n["season"].as_u64() != Some(season as u64)
                {
                    return Err("Actor/window mismatch".into());
                }
                let record: icelines_core::view_model::rinkcraft_achievements::AchievementProfile =
                    serde_json::from_value(p["achievements"].clone())?;
                let usage = serde_json::from_value(p["deployment_evidence"].clone())?;
                let mut edge: Option<icelines_core::view_model::rinkcraft_edge::EdgeEvidence> =
                    if n["edge_evidence"].is_null() {
                        None
                    } else {
                        Some(serde_json::from_value(n["edge_evidence"].clone())?)
                    };
                if edge.is_none() {
                    if let Some(dir) = retained_edge {
                        match fs::read(dir.join(format!("{season}/{id}.json"))) {
                            Ok(raw) => {
                                let status = &n["edge_capture_status"];
                                let parsed = icelines_sources::nhl::edge::parse(
                                    &raw,
                                    id,
                                    season,
                                    group,
                                    record.games,
                                    status["source_url"]
                                        .as_str()
                                        .ok_or("Retained EDGE source URL missing")?,
                                    status["attempted_at"]
                                        .as_str()
                                        .ok_or("Retained EDGE capture date missing")?,
                                );
                                match parsed {
                                    Ok(observation) => {
                                        let mut root: RawMap = serde_json::from_slice(&bytes)?;
                                        let mut frame: RawMap =
                                            serde_json::from_str(root["numerical_evidence"].get())?;
                                        let mut capture_status = status.clone();
                                        capture_status["state"] = "available".into();
                                        capture_status["reason"] = Value::Null;
                                        frame.insert(
                                            "edge_capture_status".into(),
                                            to_raw_value(&capture_status)?,
                                        );
                                        frame.insert(
                                            "edge_evidence".into(),
                                            to_raw_value(&observation)?,
                                        );
                                        root.insert(
                                            "numerical_evidence".into(),
                                            to_raw_value(&frame)?,
                                        );
                                        bytes = serde_json::to_vec(&root)?;
                                        edge = Some(observation);
                                        recovered.push((season, id));
                                    }
                                    Err(reason) => eprintln!(
                                        "EDGE remains unavailable for {season}/{id}: {reason}"
                                    ),
                                }
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                            Err(e) => return Err(e.into()),
                        }
                    }
                }
                let input = observed_player(&record, &usage, edge.as_ref())
                    .map_err(|e| format!("{season}/{id}: {e}"))?;
                if input.player_id != id || input.group != group || record.season != season {
                    return Err("Attribute source actor/window mismatch".into());
                }
                inputs.push(input);
                originals.push((id, url.to_string(), bytes));
            } else {
                rendered.push((url.to_string(), bytes));
            }
        }
        let profiles = build(season, "all strengths · rates per60", &inputs)?;
        for (profile, (_, url, bytes)) in profiles.iter().zip(originals) {
            let mut root: RawMap = serde_json::from_slice(&bytes)?;
            let mut n: RawMap = serde_json::from_str(
                root.get("numerical_evidence")
                    .ok_or("Missing numerical evidence")?
                    .get(),
            )?;
            n.insert("stat_attributes".into(), to_raw_value(profile)?);
            root.insert("numerical_evidence".into(), to_raw_value(&n)?);
            scored += profile.scores.iter().filter(|s| s.score.is_some()).count();
            complete += usize::from(profile.scores.iter().all(|s| s.score.is_some()));
            rendered.push((url, serde_json::to_vec(&root)?));
        }
    }
    finish_profiles(&mut rendered, &catalog)?;
    fs::create_dir_all(out)?;
    fs::write(out.join("catalog.json"), catalog_bytes)?;
    if base.join("edge-coverage.json").exists() {
        let mut coverage: Value =
            serde_json::from_slice(&fs::read(base.join("edge-coverage.json"))?)?;
        if let Some(rows) = coverage["records"].as_array_mut() {
            for row in rows {
                if recovered
                    .iter()
                    .any(|&(season, id)| row["season"] == season && row["player_id"] == id)
                {
                    row["state"] = "available".into();
                    row["reason"] = Value::Null;
                }
            }
        }
        let rows = coverage["records"]
            .as_array()
            .ok_or("EDGE coverage records missing")?;
        let available = rows.iter().filter(|r| r["state"] == "available").count();
        let unavailable = rows.len() - available;
        coverage["available"] = available.into();
        coverage["unavailable"] = unavailable.into();
        fs::write(
            out.join("edge-coverage.json"),
            serde_json::to_vec(&coverage)?,
        )?;
    }
    println!("Recovered {} retained EDGE observations.", recovered.len());
    for (url, bytes) in rendered {
        let path = out.join(url);
        fs::create_dir_all(path.parent().ok_or("Invalid output path")?)?;
        fs::write(path, bytes)?;
    }
    println!(
        "Scored {scored} attributes; {complete} skater-season profiles have all six estimates."
    );
    Ok(())
}
