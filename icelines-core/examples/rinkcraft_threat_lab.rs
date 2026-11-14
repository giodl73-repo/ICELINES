//! Batch adapter; exact mechanics and validation live in the Rust library.
use icelines_core::view_model::rinkcraft_task_season::{self as task, threats};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    kind: String,
    focus: String,
    opponent: String,
    budgets: Vec<u16>,
    cases: Vec<task::StudyCase>,
}
fn read<T: for<'a> Deserialize<'a>>(p: &str) -> Result<T, String> {
    serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write(p: &Path, v: &impl Serialize) -> Result<(), String> {
    fs::write(p, serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "Usage: rinkcraft_threat_lab BASE_TASK_REQUEST THREAT_RULES PLAN OUTPUT_DIR".into(),
        );
    }
    let base: task::Request = read(&args[0])?;
    let rules: threats::Rules = read(&args[1])?;
    let plan: Plan = read(&args[2])?;
    if base.schema != task::SCHEMA
        || base.kind != "authored_game_scenario_not_nhl_estimate"
        || plan.kind != base.kind
        || plan.schema != "rinkcraft.threat-study-plan.0.1"
        || plan.focus == plan.opponent
        || plan.cases.is_empty()
        || plan.cases.len() > 100
        || plan.budgets.is_empty()
        || plan.budgets.len() > 12
        || plan.budgets.iter().any(|&b| b == 0 || b > 120)
        || plan.budgets.iter().collect::<BTreeSet<_>>().len() != plan.budgets.len()
    {
        return Err("Invalid threat study identity or size".into());
    }
    let mut ids = BTreeSet::new();
    for c in &plan.cases {
        if c.team != plan.focus || !ids.insert(&c.id) {
            return Err("Duplicate case/wrong focus".into());
        }
    }
    let out = Path::new(&args[3]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for c in &plan.cases {
        let q = task::apply_study_case(&base, c)?;
        let q: task::Request =
            serde_json::from_slice(&serde_json::to_vec(&q).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let team = |ts: &[task::Team], name: &str| {
            ts.iter()
                .find(|t| t.team == name)
                .cloned()
                .ok_or("Missing study team")
        };
        let ra = team(&q.reference, &plan.focus)?;
        let rd = team(&q.reference, &plan.opponent)?;
        let pa = team(&q.proposed, &plan.focus)?;
        let pd = team(&q.proposed, &plan.opponent)?;
        for threat in [
            threats::Threat::Rush,
            threats::Threat::SlotPass,
            threats::Threat::LateralPass,
            threats::Threat::DumpRetrieve,
        ] {
            let name = serde_json::to_value(threat)
                .map_err(|e| e.to_string())?
                .as_str()
                .unwrap()
                .to_owned();
            for &budget in &plan.budgets {
                let a = threats::inspect(&q.character_rules, &rules, &ra, &rd, threat, budget)?;
                let b = threats::inspect(&q.character_rules, &rules, &pa, &pd, threat, budget)?;
                let aa = threats::inspect(&q.character_rules, &rules, &rd, &ra, threat, budget)?;
                let ab = threats::inspect(&q.character_rules, &rules, &pd, &pa, threat, budget)?;
                let filename = format!("{}--{}--{}.json", c.id, name, budget);
                write(
                    &out.join(&filename),
                    &serde_json::json!({"case":c,"reference":a,"proposed":b,"reference_against":aa,"proposed_against":ab}),
                )?;
                let ratio = |x: &serde_json::Value, y: &serde_json::Value| {
                    let n = x["goal_probability"].as_f64().unwrap();
                    if n > 0.0 {
                        Some(y["goal_probability"].as_f64().unwrap() / n)
                    } else {
                        None
                    }
                };
                rows.push(serde_json::json!({"case":c.id,"threat":threat,"budget_seconds":budget,"result":filename,
                    "reference_goal_probability":a["goal_probability"],"proposed_goal_probability":b["goal_probability"],
                    "goal_ratio":ratio(&a,&b),"against_goal_ratio":ratio(&aa,&ab),
                    "reference_expected_seconds":a["expected_elapsed_seconds"],"proposed_expected_seconds":b["expected_elapsed_seconds"],
                    "proposed_expiry_probability":b["time_expired_probability"]}));
            }
        }
        println!("{}: four threats / {} clocks", c.id, plan.budgets.len());
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.threat-study-summary.0.1","kind":plan.kind,"seed":null,"trials":null,"rows":rows}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
