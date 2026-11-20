//! Exploratory chronological reference replay and explicit attribute coverage.
use icelines_core::view_model::{
    rinkcraft_capability_index::rules, rinkcraft_seasonal_records as sr,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRef {
    path: String,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema: String,
    base_directory: String,
    exposure: FileRef,
    observations: FileRef,
    training_seasons: Vec<u32>,
    validation_season: u32,
    monitor_season: u32,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct YearRef {
    season: u32,
    members: Vec<(u32, f64)>,
    source_reference_sha256: Option<String>,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct Reference {
    group: String,
    key: String,
    usable: bool,
    player_seasons: usize,
    unique_players: usize,
    years: Vec<YearRef>,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
struct Epoch {
    schema: String,
    training_seasons: Vec<u32>,
    training_input_sha256: String,
    weighting: String,
    references: Vec<Reference>,
    sha256: String,
}
const METRICS: [&str; 5] = [
    "skating_peak_mph",
    "bursts_per_60",
    "shot_peak_mph",
    "shots_per_60",
    "conversion_pct",
];
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(r: &FileRef) -> Result<Vec<u8>, String> {
    let b = fs::read(&r.path).map_err(|e| e.to_string())?;
    if digest(&b) != r.sha256 {
        return Err("input checksum mismatch".into());
    }
    Ok(b)
}
fn fit(input: &sr::Input, years: &[u32]) -> Result<Epoch, String> {
    let selected: BTreeSet<_> = years.iter().copied().collect();
    if selected.is_empty() || selected.len() != years.len() {
        return Err("unique nonempty training seasons required".into());
    }
    let mut subset = input.clone();
    subset.records.retain(|r| selected.contains(&r.season));
    subset.reference_label = "Frozen training records only".into();
    if selected
        .iter()
        .any(|s| !subset.records.iter().any(|r| r.season == *s))
    {
        return Err("training season missing".into());
    }
    let catalog = sr::evaluate(&subset)?;
    let mut references = Vec::new();
    for group in ["Forwards", "Defense"] {
        for key in METRICS {
            let mut refs = Vec::new();
            let mut ids = BTreeSet::new();
            let mut total = 0;
            for year in &selected {
                let mut members = Vec::new();
                let mut source = None;
                for p in catalog
                    .profiles
                    .iter()
                    .filter(|p| p.season == *year && p.group == group && p.eligible)
                {
                    let m = p.measurements.iter().find(|m| m.key == key).unwrap();
                    if let Some(v) = m.value {
                        members.push((p.player_id, v));
                        ids.insert(p.player_id);
                        source = m.reference_sha256.clone();
                    }
                }
                members.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
                total += members.len();
                refs.push(YearRef {
                    season: *year,
                    members,
                    source_reference_sha256: source,
                });
            }
            references.push(Reference {
                group: group.into(),
                key: key.into(),
                usable: refs.iter().all(|r| r.members.len() >= 20),
                player_seasons: total,
                unique_players: ids.len(),
                years: refs,
            });
        }
    }
    let mut epoch = Epoch {
        schema: "rinkcraft.frozen-record-epoch.v1".into(),
        training_seasons: selected.into_iter().collect(),
        training_input_sha256: catalog.input_sha256,
        weighting: "equal season; within-season empirical midrank CDF".into(),
        references,
        sha256: String::new(),
    };
    epoch.sha256 = digest(&serde_json::to_vec(&epoch).unwrap());
    Ok(epoch)
}
fn percentile(reference: &Reference, value: f64) -> Option<f64> {
    if !reference.usable || !value.is_finite() {
        return None;
    }
    let total: f64 = reference
        .years
        .iter()
        .map(|y| {
            let below = y.members.partition_point(|(_, v)| *v < value);
            let upper = y.members.partition_point(|(_, v)| *v <= value);
            100. * (below as f64 + 0.5 * (upper - below) as f64) / y.members.len() as f64
        })
        .sum();
    Some(total / reference.years.len() as f64)
}
fn evaluate_epoch(
    epoch: &Epoch,
    input: &sr::Input,
    season: u32,
    phase: &str,
) -> Result<Value, String> {
    if epoch.training_seasons.iter().any(|s| season <= *s) {
        return Err("target must follow every training season".into());
    }
    let mut subset = input.clone();
    subset.records.retain(|r| r.season == season);
    let catalog = sr::evaluate(&subset)?;
    let mut counts: BTreeMap<(String, String), (usize, usize, f64)> = BTreeMap::new();
    let mut rows = Vec::new();
    let mut candidates: BTreeMap<(String, String, String), usize> = BTreeMap::new();
    for p in catalog.profiles {
        let mut ranks = BTreeMap::new();
        let mut measurements = Vec::new();
        for key in METRICS {
            let m = p.measurements.iter().find(|m| m.key == key).unwrap();
            let r = epoch
                .references
                .iter()
                .find(|r| r.group == p.group && r.key == key)
                .unwrap();
            let frozen = if p.eligible {
                m.value.and_then(|v| percentile(r, v))
            } else {
                None
            };
            if let Some(q) = frozen {
                ranks.insert(key, (q, r.player_seasons as u64));
                let c = counts.entry((p.group.clone(), key.into())).or_default();
                c.0 += 1;
                c.1 += usize::from(q >= 90.);
                if let Some(now) = m.percentile {
                    c.2 += (q - now).abs();
                }
            }
            measurements.push(json!({"key":key,"value":m.value,"frozen_percentile":frozen,"same_season_percentile":m.percentile,"training_player_seasons":r.player_seasons,"training_unique_players":r.unique_players,"reference_usable":r.usable}));
        }
        let families = vec![
            sr::classify_pair(
                sr::Family::Skating,
                p.eligible,
                ranks.get("skating_peak_mph").copied(),
                ranks.get("bursts_per_60").copied(),
            )?,
            sr::classify_pair(
                sr::Family::Shooting,
                p.eligible,
                ranks.get("shot_peak_mph").copied(),
                ranks.get("shots_per_60").copied(),
            )?,
        ];
        for d in &families {
            *candidates
                .entry((
                    p.group.clone(),
                    d.family.clone(),
                    d.candidate.clone().unwrap_or(d.state.clone()),
                ))
                .or_default() += 1;
        }
        rows.push(json!({"player_id":p.player_id,"name":p.name,"group":p.group,"season":p.season,"eligible":p.eligible,"measurements":measurements,"frozen_families":families,"same_season_families":p.families}));
    }
    let summary:Vec<_>=counts.into_iter().map(|((group,key),(n,top,shift))|json!({"group":group,"key":key,"compared":n,"frozen_p90_count":top,"frozen_p90_share":top as f64/n as f64,"mean_absolute_percentile_shift":shift/n as f64})).collect();
    let census: Vec<_> = candidates
        .into_iter()
        .map(|((group, family, outcome), count)| json!({"group":group,"family":family,"outcome":outcome,"count":count}))
        .collect();
    Ok(
        json!({"phase":phase,"season":season,"epoch_sha256":epoch.sha256,"summary":summary,"census":census,"rows":rows,"interpretation":if phase=="partial_monitor"{"Partial exposure; season maxima are not exposure-equivalent to full-season references."}else{"Retrospective exploratory replay, not untouched prospective validation."}}),
    )
}
fn coverage(base: &Path, exposure: &Value, observations: &sr::Input) -> Result<Value, String> {
    let raw = fs::read(base.join("catalog.json")).map_err(|e| e.to_string())?;
    if exposure["catalog_sha256"] != digest(&raw) {
        return Err("catalog checksum mismatch".into());
    }
    let catalog: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
    let mut inventory = BTreeMap::new();
    for i in exposure["inputs"].as_array().ok_or("missing inventory")? {
        if inventory
            .insert(
                i["path"].as_str().ok_or("missing path")?.to_string(),
                i["sha256"].as_str().ok_or("missing hash")?.to_string(),
            )
            .is_some()
        {
            return Err("duplicate input path".into());
        }
    }
    let mut skaters: BTreeMap<_, _> = observations
        .records
        .iter()
        .map(|r| ((r.season as u64, r.player_id as u64), r))
        .collect();
    if skaters.len() != observations.records.len() {
        return Err("duplicate supplied observation".into());
    }
    let registry = rules();
    let mut rows = Vec::new();
    let (mut components, mut contextual, mut goalie_rows) = (0, 0, 0);
    for window in catalog["windows"].as_array().ok_or("missing windows")? {
        let season = window["season"].as_u64().ok_or("missing season")?;
        for actor in window["players"].as_array().ok_or("missing players")? {
            let id = actor["id"].as_u64().ok_or("missing actor")?;
            let path = format!("data/{season}/{id}.json");
            if actor["url"] != path {
                return Err("unexpected profile path".into());
            }
            let raw = fs::read(base.join(&path)).map_err(|e| e.to_string())?;
            let hash = digest(&raw);
            if inventory.remove(&path).as_deref() != Some(&hash) {
                return Err("profile checksum mismatch".into());
            }
            let p: Value = serde_json::from_slice(&raw).map_err(|e| e.to_string())?;
            let f = &p["numerical_evidence"];
            if f["player_id"] != id
                || f["season"] != season
                || f["group"] != actor["group"]
                || f["games"] != actor["games"]
            {
                return Err("profile identity mismatch".into());
            }
            let goalie = f["group"] == "Goalies";
            goalie_rows += usize::from(goalie);
            let mut context: BTreeMap<&str, Value> = BTreeMap::new();
            if !goalie {
                let r = skaters
                    .remove(&(season, id))
                    .ok_or("missing skater observation")?;
                if r.official_source_sha256 != hash
                    || f["games"] != r.games
                    || f["group"] != r.group
                {
                    return Err("observation/profile mismatch".into());
                }
                if let Some(e) = &r.edge {
                    for (key, facet) in [
                        ("max-skating-speed", "SKA-SPEED"),
                        ("speed-bursts-over-20", "SKA-SPEED"),
                        ("max-shot-speed", "SHO-VELOCITY"),
                    ] {
                        if let Some(m) = e.measures.iter().find(|m| m.key == key) {
                            context.entry(facet).or_insert_with(||json!([])).as_array_mut().unwrap().push(json!({"key":key,"value":m.value,"unit":m.unit,"source_sha256":e.source.sha256,"scope":"all strengths","evidence_kind":"recorded context, not a calibrated task estimate"}));
                        }
                    }
                }
                if let Some(v) = sr::conversion(r.goals, r.shots) {
                    context.insert("SHO-FINISH",json!([{"key":"raw_conversion_pct","value":v,"goals":r.goals,"shots":r.shots,"source_sha256":hash,"scope":"all strengths","evidence_kind":"unadjusted outcome context; no xG/history model loaded"}]));
                }
            }
            let attributes:Vec<_>=registry.attributes.iter().filter(|a|a.branch==if goalie{"goalie"}else{"skater"}).map(|a|{let facets:Vec<_>=a.components.iter().map(|c|{components+=1;let evidence=context.get(c.id.as_str()).cloned().unwrap_or(json!([]));contextual+=usize::from(!evidence.as_array().unwrap().is_empty());json!({"component_id":c.id,"measurement_state":"unmeasured_task","context":evidence,"release_state":"not_calibrated"})}).collect();json!({"attribute_id":a.id,"name":a.name,"headline_state":"unrated","facets":facets})}).collect();
            rows.push(json!({"player_id":id,"name":p["player_name"],"season":season,"group":f["group"],"source_profile_sha256":hash,"attributes":attributes,"goalie_stopping_context":if goalie{f["edge_evidence"]["locations"].clone()}else{Value::Null}}));
        }
    }
    if !inventory.is_empty() || !skaters.is_empty() {
        return Err("unconsumed inventory or observation".into());
    }
    let definitions:Vec<_>=registry.attributes.iter().map(|a|json!({"id":a.id,"name":a.name,"branch":a.branch,"construct":a.construct,"components":a.components})).collect();
    Ok(
        json!({"schema":"rinkcraft.attribute-coverage-ledger.v1","registry_sha256":registry.source_registry_sha256,"primary_reference_scope":registry.reference_scope,"definitions":definitions,"profiles":rows,"summary":{"player_seasons":rows.len(),"goalie_player_seasons":goalie_rows,"required_component_rows":components,"component_rows_with_context":contextual,"loaded_calibrated_task_estimates":0,"released_headline_scores":0},"notes":["Coverage describes this loaded cohort, not all research conducted elsewhere. The earlier conditional finishing pilot is retained separately.","Context is not measured task ability. More seasons do not turn maxima, volume or raw conversion into the other required facets.","Goalie stopping outcomes are broad context, not measurements of tracking, positioning, movement, rebounds, recovery or puck play."]}),
    )
}
fn run(manifest: &Path, out: &Path) -> Result<(), String> {
    if out.exists() {
        return Err("new output directory required".into());
    }
    let bytes = fs::read(manifest).map_err(|e| e.to_string())?;
    let m: Manifest = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if m.schema != "rinkcraft.calibration-replay-input.v1"
        || m.training_seasons.is_empty()
        || m.training_seasons.iter().any(|s| *s >= m.validation_season)
        || m.monitor_season <= m.validation_season
    {
        return Err("invalid chronological split".into());
    }
    let input: sr::Input =
        serde_json::from_slice(&read(&m.observations)?).map_err(|e| e.to_string())?;
    let exposure: Value = serde_json::from_slice(&read(&m.exposure)?).map_err(|e| e.to_string())?;
    let ledger = coverage(Path::new(&m.base_directory), &exposure, &input)?;
    let epoch = fit(&input, &m.training_seasons)?;
    let validation = evaluate_epoch(
        &epoch,
        &input,
        m.validation_season,
        "retrospective_validation",
    )?;
    let monitor = evaluate_epoch(&epoch, &input, m.monitor_season, "partial_monitor")?;
    let first = *m.training_seasons.iter().min().unwrap();
    let next = *m.training_seasons.iter().max().unwrap();
    let rolling = if first < next {
        let e = fit(&input, &[first])?;
        Some(
            json!({"epoch":e,"evaluation":evaluate_epoch(&e,&input,next,"retrospective_validation")?}),
        )
    } else {
        None
    };
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    for (name, value) in [
        ("coverage-ledger.json", ledger),
        ("epoch-model.json", serde_json::to_value(&epoch).unwrap()),
        (
            "epoch-evaluation.json",
            json!({"schema":"rinkcraft.epoch-evaluation.v1","manifest_sha256":digest(&bytes),"validation":validation,"monitor":monitor,"rolling_origin":rolling,"limits":["Exploratory chronological replay; later seasons were inspected in prior work and are not an untouched confirmatory holdout.","No repeated-player independence assumption, uncertainty interval or capability release. Epoch weights are authored, not optimized.","All-strength record references do not calibrate the registered five-on-five task battery. Current-season maxima have shorter exposure."]}),
        ),
    ] {
        fs::write(out.join(name), serde_json::to_vec(&value).unwrap())
            .map_err(|e| e.to_string())?;
    }
    println!("Attribute coverage and frozen multi-season reference replay written; no capability scores.");
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 3 {
        eprintln!("usage: rinkcraft_calibration_epoch MANIFEST_JSON NEW_DIRECTORY");
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&a[1]), Path::new(&a[2])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> sr::Input {
        sr::Input {
            schema: "icelines.seasonal-record-input.v1".into(),
            reference_label: "Synthetic".into(),
            season_type: "regular".into(),
            strength_scope: "all strengths".into(),
            settings: sr::Settings::default(),
            records: [20232024, 20242025, 20252026]
                .into_iter()
                .flat_map(|season| {
                    (1..=20).map(move |id| sr::Record {
                        player_id: id,
                        season,
                        name: String::new(),
                        group: "Forwards".into(),
                        games: 20,
                        minutes: 200.,
                        goals: Some(1),
                        shots: Some(id),
                        official_source_sha256: "a".repeat(64),
                        edge: None,
                    })
                })
                .collect(),
        }
    }
    #[test]
    fn future_values_cannot_change_training_epoch() {
        let i = input();
        let a = fit(&i, &[20232024, 20242025]).unwrap();
        let mut altered = i.clone();
        for r in altered.records.iter_mut().filter(|r| r.season == 20252026) {
            r.shots = Some(1000);
        }
        assert_eq!(a, fit(&altered, &[20232024, 20242025]).unwrap());
        altered.records[0].shots = Some(999);
        assert_ne!(
            a.sha256,
            fit(&altered, &[20232024, 20242025]).unwrap().sha256
        );
        assert!(evaluate_epoch(&a, &i, 20242025, "test").is_err());
    }
    #[test]
    fn season_weighting_is_not_player_season_pooling() {
        let r = Reference {
            group: "Forwards".into(),
            key: "test".into(),
            usable: true,
            player_seasons: 100,
            unique_players: 100,
            years: vec![
                YearRef {
                    season: 20232024,
                    members: (1..=20).map(|n| (n, n as f64)).collect(),
                    source_reference_sha256: None,
                },
                YearRef {
                    season: 20242025,
                    members: (101..=180).map(|n| (n, n as f64)).collect(),
                    source_reference_sha256: None,
                },
            ],
        };
        assert_eq!(percentile(&r, 20.), Some(48.75));
        let mut missing = r.clone();
        missing.usable = false;
        assert_eq!(percentile(&missing, 20.), None);
    }
    #[test]
    fn order_and_duplicate_seasons_are_checked() {
        let i = input();
        let a = fit(&i, &[20232024, 20242025]).unwrap();
        let mut reverse = i.clone();
        reverse.records.reverse();
        assert_eq!(a, fit(&reverse, &[20242025, 20232024]).unwrap());
        assert!(fit(&i, &[20232024, 20232024]).is_err());
        assert!(fit(&i, &[20212022]).is_err());
    }
}
