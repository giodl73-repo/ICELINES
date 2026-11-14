//! Thin fixed-profile/opponent grid adapter; all transformations live in Rust core.
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
    profiles: Vec<task::StudyCase>,
    opponents: Vec<task::StudyCase>,
    policies: Vec<Policy>,
    budgets: Vec<u16>,
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
            "Usage: rinkcraft_policy_matchups BASE_REQUEST SEQUENCE_RULES STUDY OUTPUT_DIR".into(),
        );
    }
    let base: task::Request = read(&args[0])?;
    let rules: seq::Rules = read(&args[1])?;
    let study: Study = read(&args[2])?;
    if study.schema != "rinkcraft.policy-matchup-study.0.1"
        || study.kind != "authored_game_scenario_not_nhl_estimate"
        || study.focus == study.opponent
        || study.profiles.is_empty()
        || study.profiles.len() > 20
        || study.opponents.is_empty()
        || study.opponents.len() > 40
        || study.policies.is_empty()
        || study.policies.len() > 8
        || study.budgets.is_empty()
        || study.budgets.len() > 12
        || study.budgets.iter().any(|&b| b == 0 || b > 60)
        || study.budgets.iter().collect::<BTreeSet<_>>().len() != study.budgets.len()
        || study.profiles.len() * study.opponents.len() * study.policies.len() * study.budgets.len()
            > 1000
    {
        return Err("Invalid matchup study identity or size".into());
    }
    for (cases, team) in [
        (&study.profiles, &study.focus),
        (&study.opponents, &study.opponent),
    ] {
        let mut ids = BTreeSet::new();
        for c in cases {
            if !valid(&c.id) || !ids.insert(&c.id) || &c.team != team {
                return Err("Invalid/duplicate matchup case or team".into());
            }
        }
    }
    let mut ids = BTreeSet::new();
    for p in &study.policies {
        if !valid(&p.id) || !ids.insert(&p.id) {
            return Err("Invalid/duplicate matchup policy".into());
        }
    }
    let mut filenames = BTreeSet::new();
    for a in &study.profiles {
        for b in &study.opponents {
            for p in &study.policies {
                for t in &study.budgets {
                    if !filenames.insert(format!("{}--{}--{}--{}.json", a.id, b.id, p.id, t)) {
                        return Err("Output filename collision".into());
                    }
                }
            }
        }
    }
    let out = Path::new(&args[3]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for a in &study.profiles {
        for b in &study.opponents {
            for p in &study.policies {
                for &budget in &study.budgets {
                    let plan = seq::Plan {
                        opening: task::threats::Threat::DumpRetrieve,
                        completed_prefix: 3,
                        budget_seconds: budget,
                        first_policy: p.definition.first.default,
                        second_policy: p.definition.second.default,
                    };
                    let v = seq::matchups::compare(&base, a, b, &rules, &plan, &p.definition)?;
                    let filename = format!("{}--{}--{}--{}.json", a.id, b.id, p.id, budget);
                    write(&out.join(&filename), &v)?;
                    rows.push(serde_json::json!({"profile":a.id,"opponent":b.id,"policy":p.id,"budget_seconds":budget,"result":filename,
            "reference_goal_for":v["reference_goal_for"],"proposed_goal_for":v["proposed_goal_for"],
            "reference_goal_against":v["reference_goal_against"],"proposed_goal_against":v["proposed_goal_against"],
            "goal_for_delta":v["goal_for_delta"],"goal_against_delta":v["goal_against_delta"],
            "proposed_bounded_goal_balance":v["proposed_bounded_goal_balance"],"input_sha256":v["input_sha256"]}));
                }
            }
            println!("{} / {}: {} policies", a.id, b.id, study.policies.len());
        }
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":"rinkcraft.policy-matchup-summary.0.1","kind":study.kind,"seed":null,"trials":null,"rows":rows}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
