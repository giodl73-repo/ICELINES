//! Offline exact route study. All mechanics live in the library.
use icelines_core::view_model::rinkcraft_task_season::{self as task, routes};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mix {
    id: String,
    mixture: routes::Mixture,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    schema: String,
    kind: String,
    focus: String,
    opponent: String,
    mixtures: Vec<Mix>,
    cases: Vec<task::StudyCase>,
}
fn read<T: for<'a> Deserialize<'a>>(p: &str) -> Result<T, String> {
    serde_json::from_slice(&fs::read(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
fn write(p: &Path, v: &impl Serialize) -> Result<(), String> {
    fs::write(p, serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn valid(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 80
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("Usage: rinkcraft_route_lab BASE_TASK_REQUEST PLAN OUTPUT_DIR".into());
    }
    let base: task::Request = read(&args[0])?;
    let plan: Plan = read(&args[1])?;
    if base.schema != task::SCHEMA
        || base.kind != "authored_game_scenario_not_nhl_estimate"
        || plan.schema != "rinkcraft.route-study-plan.0.1"
        || plan.kind != base.kind
        || plan.focus == plan.opponent
        || plan.mixtures.is_empty()
        || plan.mixtures.len() > 20
        || plan.cases.is_empty()
        || plan.cases.len() > 100
    {
        return Err("Invalid base or route plan identity/size".into());
    }
    let mut ids = BTreeSet::new();
    for m in &plan.mixtures {
        if !valid(&m.id) || !ids.insert(&m.id) {
            return Err("Invalid/duplicate mixture ID".into());
        }
    }
    let mut ids = BTreeSet::new();
    for c in &plan.cases {
        if !valid(&c.id) || !ids.insert(&c.id) || c.team != plan.focus {
            return Err("Invalid/duplicate case or wrong focus team".into());
        }
    }
    let mut filenames = BTreeSet::new();
    for c in &plan.cases {
        for m in &plan.mixtures {
            if !filenames.insert(format!("{}--{}.json", c.id, m.id)) {
                return Err("Case/mixture IDs collide in output filenames".into());
            }
        }
    }
    let out = Path::new(&args[2]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for c in &plan.cases {
        let request = task::apply_study_case(&base, c)?;
        // Use the same canonical JSON round-trip boundary as the season study.
        let q: task::Request =
            serde_json::from_slice(&serde_json::to_vec(&request).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        for m in &plan.mixtures {
            let team = |ts: &[task::Team], name: &str| {
                ts.iter()
                    .find(|t| t.team == name)
                    .cloned()
                    .ok_or("Missing study team")
            };
            let a = routes::inspect(
                &q.character_rules,
                q.rules.score_rule,
                &team(&q.reference, &plan.focus)?,
                &team(&q.reference, &plan.opponent)?,
                &m.mixture,
            )?;
            let b = routes::inspect(
                &q.character_rules,
                q.rules.score_rule,
                &team(&q.proposed, &plan.focus)?,
                &team(&q.proposed, &plan.opponent)?,
                &m.mixture,
            )?;
            let ratio =
                b["goal_probability"].as_f64().unwrap() / a["goal_probability"].as_f64().unwrap();
            let against_a = routes::inspect(
                &q.character_rules,
                q.rules.score_rule,
                &team(&q.reference, &plan.opponent)?,
                &team(&q.reference, &plan.focus)?,
                &m.mixture,
            )?;
            let against_b = routes::inspect(
                &q.character_rules,
                q.rules.score_rule,
                &team(&q.proposed, &plan.opponent)?,
                &team(&q.proposed, &plan.focus)?,
                &m.mixture,
            )?;
            let against_ratio = against_b["goal_probability"].as_f64().unwrap()
                / against_a["goal_probability"].as_f64().unwrap();
            let filename = format!("{}--{}.json", c.id, m.id);
            write(
                &out.join(&filename),
                &serde_json::json!({"case":c,"mixture_id":m.id,"reference":a,"proposed":b,
                    "reference_against":against_a,"proposed_against":against_b}),
            )?;
            rows.push(serde_json::json!({"case":c.id,"mixture":m.id,"result":filename,
                "reference_goal_probability":a["goal_probability"],"proposed_goal_probability":b["goal_probability"],"goal_probability_ratio":ratio,
                "reference_against_goal_probability":against_a["goal_probability"],"proposed_against_goal_probability":against_b["goal_probability"],"against_goal_probability_ratio":against_ratio,
                "reference_input_sha256":a["input_sha256"],"proposed_input_sha256":b["input_sha256"]}));
        }
        println!("{}: {} route mixtures", c.id, plan.mixtures.len());
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.route-study-summary.0.1","kind":plan.kind,
        "focus":plan.focus,"opponent":plan.opponent,"seed":null,"trials":null,"rows":rows}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
