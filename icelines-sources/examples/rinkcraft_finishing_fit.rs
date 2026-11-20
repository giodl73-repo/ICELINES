//! Hash-verified port of the earlier 5-on-5 conditional finishing pilot.
use icelines_core::view_model::{rinkcraft_finishing as f, rinkcraft_finishing_fit as fit};
use serde::Deserialize;
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
struct Job {
    schema: String,
    legacy_config: FileRef,
    data_root: String,
}
fn digest(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn read(r: &FileRef) -> Result<Vec<u8>, String> {
    let b = fs::read(&r.path).map_err(|e| e.to_string())?;
    if digest(&b) != r.sha256 {
        return Err("config checksum mismatch".into());
    }
    Ok(b)
}
fn deviance(goals: u32, expected: f64) -> Result<f64, String> {
    if !expected.is_finite() || expected <= 0. {
        return Err("positive forecast required".into());
    }
    let g = f64::from(goals);
    Ok(if goals == 0 {
        2. * expected
    } else {
        2. * (g * (g / expected).ln() - (g - expected))
    })
}
fn component(
    prior: &fit::Prior,
    goals: u64,
    xg: f64,
    probability: f64,
    min_xg: f64,
) -> Result<Value, String> {
    let p = f::update_multiplier(prior.shape, prior.rate, goals, xg)?;
    let ci = f::credible_interval(&p, probability)?;
    let sensitivity:Vec<_>=[0.5,2.].into_iter().map(|s|json!({"prior_strength_factor":s,"mean":(prior.shape*s+goals as f64)/(prior.rate*s+xg)})).collect();
    Ok(
        json!({"goals":goals,"provider_xg":xg,"posterior":p,"interval":ci,"status":if xg>=min_xg{"experimental_conditional_component"}else{"insufficient_xg_exposure"},"minimum_display_xg":min_xg,"prior_strength_sensitivity":sensitivity,"headline_score":Value::Null}),
    )
}
fn run(job: &Path, out: &Path) -> Result<(), String> {
    if out.exists() {
        return Err("new output directory required".into());
    }
    let job_bytes = fs::read(job).map_err(|e| e.to_string())?;
    let j: Job = serde_json::from_slice(&job_bytes).map_err(|e| e.to_string())?;
    if j.schema != "rinkcraft.finishing-fit-replay.v1" {
        return Err("unsupported replay schema".into());
    }
    let config_bytes = read(&j.legacy_config)?;
    let c: Value = serde_json::from_slice(&config_bytes).map_err(|e| e.to_string())?;
    if c["schema"] != "icelines.capability-pilot-config.v1" || c["situation"] != "5on5" {
        return Err("unexpected pilot scope".into());
    }
    let years: Vec<u32> =
        serde_json::from_value(c["train_seasons"].clone()).map_err(|e| e.to_string())?;
    let target = c["holdout_season"].as_u64().ok_or("missing target")? as u32;
    if years.iter().any(|y| *y >= target) {
        return Err("training/target overlap".into());
    }
    let minimum_games = c["minimum_reference_games"]
        .as_u64()
        .ok_or("missing GP floor")? as u32;
    let minimum_minutes = c["minimum_reference_minutes"]
        .as_f64()
        .ok_or("missing minutes floor")?;
    let min_xg = c["minimum_scored_xg"].as_f64().ok_or("missing xG floor")?;
    let probability = c["interval_probability"]
        .as_f64()
        .ok_or("missing interval")?;
    if !min_xg.is_finite()
        || min_xg < 0.
        || !probability.is_finite()
        || probability <= 0.
        || probability >= 1.
    {
        return Err("invalid component display settings".into());
    }
    let bounds: [f64; 2] =
        serde_json::from_value(c["prior_shape_bounds"].clone()).map_err(|e| e.to_string())?;
    let mut annual = BTreeMap::new();
    let mut sources = Vec::new();
    for s in c["sources"]
        .as_array()
        .ok_or("missing sources")?
        .iter()
        .filter(|s| s["kind"] == "skaters")
    {
        let season = s["season"].as_u64().ok_or("missing season")? as u32;
        if !years.contains(&season) && season != target {
            return Err("outside-epoch source".into());
        }
        let path = Path::new(&j.data_root).join(s["path"].as_str().ok_or("missing source path")?);
        let bytes = fs::read(&path).map_err(|e| e.to_string())?;
        if s["sha256"] != digest(&bytes) || s["bytes"] != bytes.len() {
            return Err("source checksum/length mismatch".into());
        }
        let rows = icelines_sources::moneypuck_finishing::parse(&bytes, season)?;
        sources.push(json!({"season":season,"path":path,"sha256":digest(&bytes),"bytes":bytes.len(),"url":s["url"],"five_on_five_rows":rows.len()}));
        if annual.insert(season, rows).is_some() {
            return Err("duplicate annual source".into());
        }
    }
    let expected: BTreeSet<_> = years.iter().copied().chain([target]).collect();
    if annual.keys().copied().collect::<BTreeSet<_>>() != expected {
        return Err("missing annual source".into());
    }
    let training_rows: Vec<_> = years
        .iter()
        .flat_map(|y| annual[y].iter().map(|r| r.row.clone()))
        .collect();
    let input = fit::Input {
        training_seasons: years.clone(),
        minimum_games,
        minimum_minutes,
        shape_bounds: bounds,
        rows: training_rows.clone(),
    };
    let prior = fit::fit(&input)?;
    let mut profiles = Vec::new();
    let mut totals: BTreeMap<(String, String), (usize, f64, f64, f64, f64)> = BTreeMap::new();
    let mut new_players = 0;
    for target_row in annual[&target].iter().filter(|r| {
        r.row.games >= minimum_games && r.row.minutes >= minimum_minutes && r.row.xg > 0.
    }) {
        let r = &target_row.row;
        let history: Vec<_> = training_rows
            .iter()
            .filter(|h| h.player_id == r.player_id)
            .collect();
        let historical_goals: u64 = history.iter().map(|h| u64::from(h.goals)).sum();
        let historical_xg: f64 = history.iter().map(|h| h.xg).sum();
        if history.is_empty() {
            new_players += 1;
        }
        let post = f::update_multiplier(prior.shape, prior.rate, historical_goals, historical_xg)?;
        let forecast = post.mean * r.xg;
        let league = prior.mean * r.xg;
        let weak = (historical_goals as f64 + 0.5) / (historical_xg + 0.5 / prior.mean) * r.xg;
        let group = if r.position == f::Position::D {
            "Defense"
        } else {
            "Forwards"
        };
        let mut policies = Vec::new();
        for (name, expected) in [
            ("league", league),
            ("weak_pooling", weak),
            ("empirical_bayes", forecast),
        ] {
            let d = deviance(r.goals, expected)?;
            let ae = (f64::from(r.goals) - expected).abs();
            for scope in ["All", group] {
                let t = totals.entry((scope.into(), name.into())).or_default();
                t.0 += 1;
                t.1 += d;
                t.2 += ae;
                t.3 += f64::from(r.goals);
                t.4 += expected;
            }
            policies.push(json!({"policy":name,"expected_goals":expected,"poisson_deviance":d,"absolute_goal_error":ae}));
        }
        profiles.push(json!({"player_id":r.player_id,"name":target_row.name,"position":r.position,"target_season":target,"target_games":r.games,"target_minutes":r.minutes,"target_goals":r.goals,"target_provider_xg":r.xg,"training_history":history,"forecast_from_training_only":{"goals":historical_goals,"provider_xg":historical_xg,"multiplier":post,"interval":f::credible_interval(&post,probability)?,"baseline_only":historical_xg==0.,"no_historical_rows":history.is_empty(),"historical_rows":history.len(),"policies":policies},"target_season_component":component(&prior,u64::from(r.goals),r.xg,probability,min_xg)?,"pooled_available_window_component":component(&prior,historical_goals+u64::from(r.goals),historical_xg+r.xg,probability,min_xg)?,"headline_scores_released":false}));
    }
    let metrics:Vec<_>=totals.into_iter().map(|((group,policy),(n,d,ae,g,p))|json!({"group":group,"policy":policy,"players":n,"mean_poisson_deviance":d/n as f64,"mean_absolute_goal_error":ae/n as f64,"observed_goals":g,"predicted_goals":p})).collect();
    let result = json!({"schema":"rinkcraft.finishing-fit-replay.v1","job_sha256":digest(&job_bytes),"legacy_config_sha256":digest(&config_bytes),"training_seasons":years,"target_season":target,"situation":"5on5","sources":sources,"prior":prior,"profiles":profiles,"evaluation":{"players":profiles.len(),"new_players":new_players,"metrics":metrics,"bootstrap_recomputed":false,"conditional_comparison_gate_reassessed":false},"limits":["Conditional Gamma-Poisson approximation; fixed provider xG is treated as known exposure. Prior-parameter, provider-model and context uncertainty are excluded from intervals.","Held-out goals enter evaluation and retrospective target-season assessment only, never prior fitting or training-only forecasts. Forecasts condition on observed target xG, not preseason opportunities.","Retrospective source/model vintage and F/D comparability remain unresolved. No prospective, causal, stable-talent or full Shooting claim.","No 1-20 score is emitted. Display xG thresholds are authored scope choices, not reliability validation. Pooled three-season component assumes a constant effect over that named window, not lifetime/current ability.","Original Python bootstrap and source-vintage findings remain separate; this port does not recompute their intervals or certify a promotion gate."]});
    if result["evaluation"]["players"] == 0 {
        return Err("empty held-out evaluation".into());
    }
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    fs::write(
        out.join("replay.json"),
        serde_json::to_vec(&result).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        out.join("training-input.json"),
        serde_json::to_vec(&input).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    println!(
        "Rust prior fit: {} players, shape {:.8}, rate {:.8}; {} later-season component readouts.",
        result["prior"]["peers"],
        result["prior"]["shape"].as_f64().unwrap(),
        result["prior"]["rate"].as_f64().unwrap(),
        result["evaluation"]["players"]
    );
    Ok(())
}
fn main() {
    let a: Vec<_> = env::args().collect();
    if a.len() != 3 {
        eprintln!("usage: rinkcraft_finishing_fit JOB_JSON NEW_DIRECTORY");
        std::process::exit(2);
    }
    if let Err(e) = run(Path::new(&a[1]), Path::new(&a[2])) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
