//! Read an authored study protocol; emit one JSON record per case using Rust builds/experiments.
use icelines_core::view_model::rinkcraft_game::{build, BuildRequest, GameRules, ScoreMethod};
use icelines_core::view_model::rinkcraft_plan_experiment::{
    run, ExperimentRequest, MAX_INPUT_BYTES, MAX_TRIALS,
};
use icelines_core::view_model::rinkcraft_possession::Rules;
use serde::Deserialize;
use std::{collections::BTreeSet, path::Path};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    schema: String,
    character_rules_path: String,
    possession_rules_path: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    template_id: String,
    position: String,
    master_seed: u32,
    trials: u16,
    support_bonus: u8,
}
fn read(path: &Path, max: usize) -> Result<String, Box<dyn std::error::Error>> {
    if std::fs::metadata(path)?.len() > max as u64 {
        return Err("Study input exceeds file bound".into());
    }
    let raw = std::fs::read_to_string(path)?;
    if raw.len() > max {
        return Err("Study input exceeds file bound".into());
    }
    Ok(raw)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: rinkcraft_plan_study STUDY.json (rule paths relative to study)".into());
    }
    let path = Path::new(&args[0]);
    let base = path.parent().unwrap_or(Path::new("."));
    let study: Study = serde_json::from_str(&read(path, MAX_INPUT_BYTES)?)?;
    if study.schema != "rinkcraft.possession-study.v1"
        || study.cases.is_empty()
        || study.cases.len() > 16
    {
        return Err("Use study v1 with 1–16 cases".into());
    }
    let cr: GameRules =
        serde_json::from_str(&read(&base.join(study.character_rules_path), 65536)?)?;
    cr.validate()?;
    let rules: Rules = serde_json::from_str(&read(&base.join(study.possession_rules_path), 8192)?)?;
    rules.validate()?;
    let mut ids = BTreeSet::new();
    let mut total = 0u32;
    let mut prepared = Vec::new();
    for case in study.cases {
        total += u32::from(case.trials);
        if case.id.is_empty()
            || case.id.len() > 64
            || !case
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
            || !ids.insert(case.id.clone())
            || !(2..=MAX_TRIALS).contains(&case.trials)
            || total > 8192
        {
            return Err(
                "Case IDs must be unique; use 2–4096 trials per case and at most 8192 total".into(),
            );
        }
        let mut rules = rules.clone();
        rules.support_bonus = case.support_bonus;
        rules.validate()?;
        let hero = build(
            &cr,
            BuildRequest {
                template_id: case.template_id,
                name: case.id.clone(),
                position: case.position,
                scores: ScoreMethod::StandardArray,
                inspiration: None,
            },
        )?;
        prepared.push((
            case.id,
            ExperimentRequest {
                schema: "rinkcraft.possession-experiment-request.v1".into(),
                character_rules: cr.clone(),
                rules,
                hero,
                master_seed: case.master_seed,
                trials: case.trials,
            },
        ));
    }
    // All authored inputs are checked before any case is run or emitted.
    for (id, request) in prepared {
        eprintln!("Running {id}: {} paired seeds", request.trials);
        println!(
            "{}",
            serde_json::json!({"schema":"icelines.possession-study-case.v1","case_id":id,"experiment":run(request)?})
        );
    }
    Ok(())
}
