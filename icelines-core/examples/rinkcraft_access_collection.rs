//! Frozen authored opening-access comparison; all resolution lives in core Rust.
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
    modes: Vec<seq::access::Mode>,
    recovery: seq::recovery::Rules,
    support: Option<seq::support::Rules>,
    arrival: Option<seq::support::arrival::Rules>,
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
fn team(ts: &[task::Team], name: &str) -> Result<task::Team, String> {
    ts.iter()
        .find(|t| t.team == name)
        .cloned()
        .ok_or_else(|| format!("Missing team {name}"))
}
fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 4 {
        return Err(
            "Usage: rinkcraft_access_collection BASE_REQUEST SEQUENCE_RULES STUDY OUTPUT_DIR"
                .into(),
        );
    }
    let base: task::Request = read(&args[0])?;
    let rules: seq::Rules = read(&args[1])?;
    let study: Study = read(&args[2])?;
    if base.schema != task::SCHEMA
        || base.kind != "authored_game_scenario_not_nhl_estimate"
        || study.kind != base.kind
        || study.schema
            != if study.arrival.is_some() {
                "rinkcraft.support-arrival-study.0.1"
            } else if study.support.is_some() {
                "rinkcraft.opening-support-study.0.1"
            } else {
                "rinkcraft.opening-access-study.0.1"
            }
        || study.focus == study.opponent
        || (study.arrival.is_some() && study.support.is_none())
        || study.cases.is_empty()
        || study.cases.len() > 20
        || study.policies.is_empty()
        || study.policies.len() > 8
        || study.budgets.is_empty()
        || study.budgets.len() > 4
        || study.budgets.iter().any(|&x| x == 0 || x > 60)
        || study.budgets.iter().collect::<BTreeSet<_>>().len() != study.budgets.len()
        || study.modes.is_empty()
        || study.modes.len() > 3
        || study
            .modes
            .iter()
            .enumerate()
            .any(|(i, x)| study.modes[..i].contains(x))
        || study.cases.len() * study.policies.len() * study.budgets.len() * study.modes.len() > 512
    {
        return Err("Invalid opening-access grid identity or size".into());
    }
    let mut ids = BTreeSet::new();
    for c in &study.cases {
        if !valid(&c.id)
            || !ids.insert(&c.id)
            || (c.team != study.focus && c.team != study.opponent)
            || !c.unit_weights.is_empty()
            || !c.swap_creator_receiver.is_empty()
        {
            return Err("Opening study requires unique score-only cases".into());
        }
    }
    let mut ids = BTreeSet::new();
    for p in &study.policies {
        if !valid(&p.id) || !ids.insert(&p.id) {
            return Err("Invalid/duplicate policy".into());
        }
    }
    let out = Path::new(&args[3]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    let mut names = BTreeSet::new();
    for c in &study.cases {
        let q = task::apply_study_case(&base, c)?;
        let a = team(&q.reference, &study.focus)?;
        let b = team(&q.reference, &study.opponent)?;
        let x = team(&q.proposed, &study.focus)?;
        let y = team(&q.proposed, &study.opponent)?;
        for policy in &study.policies {
            for &budget in &study.budgets {
                for &mode in &study.modes {
                    let mode_name = serde_json::to_value(mode)
                        .map_err(|e| e.to_string())?
                        .as_str()
                        .unwrap()
                        .to_owned();
                    let filename = format!(
                        "{}--{}--{}--clock-{}.json",
                        c.id, policy.id, mode_name, budget
                    );
                    if !names.insert(filename.clone()) {
                        return Err("Output name collision".into());
                    }
                    let plan = seq::Plan {
                        opening: task::threats::Threat::DumpRetrieve,
                        completed_prefix: 3,
                        budget_seconds: budget,
                        first_policy: policy.definition.first.default,
                        second_policy: policy.definition.second.default,
                    };
                    let ar = seq::access::Rules {
                        schema: "rinkcraft.opening-access-rules.0.1".into(),
                        version: "0.1.0-playtest".into(),
                        kind: study.kind.clone(),
                        mode,
                    };
                    let inspect = |first: &task::Team, second: &task::Team| {
                        if let Some(arr) = &study.arrival {
                            seq::support::arrival::inspect(
                                &q.character_rules,
                                &rules,
                                first,
                                second,
                                &plan,
                                &policy.definition,
                                &study.recovery,
                                &ar,
                                study.support.as_ref().unwrap(),
                                arr,
                            )
                        } else if let Some(sr) = &study.support {
                            seq::support::inspect(
                                &q.character_rules,
                                &rules,
                                first,
                                second,
                                &plan,
                                &policy.definition,
                                &study.recovery,
                                &ar,
                                sr,
                            )
                        } else {
                            seq::access::inspect(
                                &q.character_rules,
                                &rules,
                                first,
                                second,
                                &plan,
                                &policy.definition,
                                &study.recovery,
                                &ar,
                            )
                        }
                    };
                    let reference = inspect(&a, &b)?;
                    let proposed = inspect(&x, &y)?;
                    write(
                        &out.join(&filename),
                        &serde_json::json!({"case":c,"policy":policy.id,"reference":reference,"proposed":proposed}),
                    )?;
                    rows.push(serde_json::json!({"case":c.id,"policy":policy.id,"mode":mode,"budget_seconds":budget,"result":filename,
                "reference_goals":reference["goal_probability_by_team"],"proposed_goals":proposed["goal_probability_by_team"],
                "proposed_control":proposed["terminal_control_probability_by_team"],"expected_seconds":proposed["expected_elapsed_seconds"]}));
                }
            }
        }
        println!(
            "{}: {} comparisons",
            c.id,
            study.policies.len() * study.budgets.len() * study.modes.len()
        );
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":if study.arrival.is_some(){"rinkcraft.support-arrival-summary.0.1"}else if study.support.is_some(){"rinkcraft.opening-support-summary.0.1"}else{"rinkcraft.opening-access-summary.0.1"},"kind":study.kind,"seed":null,"trials":null,"rows":rows}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
