//! Memoized endpoint grid; all numerical blending lives in core Rust.
use icelines_core::view_model::rinkcraft_task_season::{self as task, threats::sequence as seq};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    id: String,
    definition: seq::HolderPolicies,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    schema: String,
    kind: String,
    focus: String,
    opponent: String,
    cases: Vec<task::StudyCase>,
    policies: Vec<Policy>,
    budgets: Vec<u16>,
    costs: Vec<u16>,
    shares: Vec<f64>,
    recovery_template: seq::recovery::Rules,
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
    if args.len() != 4 {
        return Err(
            "Usage: rinkcraft_recovery_sensitivity BASE_REQUEST SEQUENCE_RULES STUDY OUTPUT_DIR"
                .into(),
        );
    }
    let base: task::Request = read(&args[0])?;
    let rules: seq::Rules = read(&args[1])?;
    let study: Study = read(&args[2])?;
    if base.schema != task::SCHEMA
        || base.kind != "authored_game_scenario_not_nhl_estimate"
        || study.kind != base.kind
        || study.schema != "rinkcraft.recovery-sensitivity-study.0.1"
        || study.focus == study.opponent
        || study.cases.is_empty()
        || study.cases.len() > 20
        || study.policies.is_empty()
        || study.policies.len() > 8
        || study.budgets.is_empty()
        || study.budgets.len() > 4
        || study.budgets.iter().any(|&x| x == 0 || x > 60)
        || study.budgets.iter().collect::<BTreeSet<_>>().len() != study.budgets.len()
        || study.costs.is_empty()
        || study.costs.len() > 8
        || study.costs.iter().any(|&x| x == 0 || x > 30)
        || study.costs.iter().collect::<BTreeSet<_>>().len() != study.costs.len()
        || study.shares.is_empty()
        || study.shares.len() > 8
        || study.shares.iter().enumerate().any(|(i, x)| {
            !x.is_finite() || !(0.0..=1.0).contains(x) || study.shares[..i].contains(x)
        })
        || study.cases.len()
            * study.policies.len()
            * study.budgets.len()
            * study.costs.len()
            * study.shares.len()
            > 512
    {
        return Err("Invalid recovery sensitivity grid identity or size".into());
    }
    let mut ids = BTreeSet::new();
    for c in &study.cases {
        if !valid(&c.id)
            || !ids.insert(&c.id)
            || c.team != study.focus
            || !c.unit_weights.is_empty()
            || !c.swap_creator_receiver.is_empty()
        {
            return Err("Sensitivity needs unique own score-only cases".into());
        }
    }
    let mut ids = BTreeSet::new();
    for p in &study.policies {
        if !valid(&p.id) || !ids.insert(&p.id) {
            return Err("Invalid/duplicate sensitivity policy".into());
        }
    }
    let mut names = BTreeSet::new();
    for c in &study.cases {
        for p in &study.policies {
            for cost in &study.costs {
                for b in &study.budgets {
                    if !names.insert(format!("{}--{}--cost-{}--clock-{}", c.id, p.id, cost, b)) {
                        return Err("Sensitivity filename collision".into());
                    }
                }
            }
        }
    }
    let out = Path::new(&args[3]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    let mut contexts = Vec::new();
    for c in &study.cases {
        let q = task::apply_study_case(&base, c)?;
        let team = |ts: &[task::Team], name: &str| {
            ts.iter()
                .find(|t| t.team == name)
                .cloned()
                .ok_or("Missing sensitivity team")
        };
        let a = team(&q.reference, &study.focus)?;
        let b = team(&q.reference, &study.opponent)?;
        let x = team(&q.proposed, &study.focus)?;
        let y = team(&q.proposed, &study.opponent)?;
        for policy in &study.policies {
            for &cost in &study.costs {
                for &budget in &study.budgets {
                    let plan = seq::Plan {
                        opening: task::threats::Threat::DumpRetrieve,
                        completed_prefix: 3,
                        budget_seconds: budget,
                        first_policy: policy.definition.first.default,
                        second_policy: policy.definition.second.default,
                    };
                    let mut rr = study.recovery_template.clone();
                    rr.cost_seconds = cost;
                    let ra = seq::recovery::sensitivity::prepare(
                        &q.character_rules,
                        &rules,
                        &a,
                        &b,
                        &plan,
                        &policy.definition,
                        &rr,
                    )?;
                    let pa = seq::recovery::sensitivity::prepare(
                        &q.character_rules,
                        &rules,
                        &x,
                        &y,
                        &plan,
                        &policy.definition,
                        &rr,
                    )?;
                    let context =
                        format!("{}--{}--cost-{}--clock-{}", c.id, policy.id, cost, budget);
                    let endpoint_file = format!("{context}--endpoints.json");
                    write(
                        &out.join(&endpoint_file),
                        &serde_json::json!({"case":c,"policy":policy.id,"reference_disabled":ra.disabled(),"reference_enabled":ra.enabled(),"proposed_disabled":pa.disabled(),"proposed_enabled":pa.enabled()}),
                    )?;
                    contexts.push(serde_json::json!({"case":c.id,"policy":policy.id,"cost_seconds":cost,"budget_seconds":budget,"endpoints":endpoint_file}));
                    for (index, &share) in study.shares.iter().enumerate() {
                        let r = ra.project(share)?;
                        let p = pa.project(share)?;
                        let filename = format!("{context}--share-{index}.json");
                        write(
                            &out.join(&filename),
                            &serde_json::json!({"case":c.id,"policy":policy.id,"endpoints":endpoint_file,"reference":r,"proposed":p}),
                        )?;
                        rows.push(serde_json::json!({"case":c.id,"policy":policy.id,"cost_seconds":cost,"budget_seconds":budget,"admission_share":share,"result":filename,"endpoints":endpoint_file,
                    "reference_goals":r["goal_probability_by_team"],"proposed_goals":p["goal_probability_by_team"],"proposed_control":p["terminal_control_probability_by_team"],
                    "expected_seconds":p["expected_elapsed_seconds"],"expected_recovery_control_changes":p["expected_recovery_control_changes"]}));
                    }
                }
            }
        }
        println!(
            "{}: {} endpoint contexts / {} shares",
            c.id,
            study.policies.len() * study.costs.len() * study.budgets.len(),
            study.shares.len()
        );
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.recovery-sensitivity-summary.0.1","kind":study.kind,"seed":null,"trials":null,"contexts":contexts,"rows":rows}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
