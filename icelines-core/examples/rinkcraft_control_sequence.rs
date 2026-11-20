//! Thin offline adapter for exact holder-preserving continuations.
use icelines_core::view_model::rinkcraft_task_season::{self as task, threats::sequence as seq};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, fs, path::Path};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    id: String,
    first: seq::Choice,
    second: seq::Choice,
    holder_policies: Option<seq::HolderPolicies>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Study {
    schema: String,
    kind: String,
    focus: String,
    opponent: String,
    budgets: Vec<u16>,
    policies: Vec<Policy>,
    cases: Vec<task::StudyCase>,
    recovery_rules: Option<seq::recovery::Rules>,
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
            "Usage: rinkcraft_control_sequence BASE_REQUEST SEQUENCE_RULES STUDY OUTPUT_DIR".into(),
        );
    }
    let base: task::Request = read(&args[0])?;
    let rules: seq::Rules = read(&args[1])?;
    let study: Study = read(&args[2])?;
    if base.schema != task::SCHEMA
        || base.kind != "authored_game_scenario_not_nhl_estimate"
        || study.kind != base.kind
        || !matches!(
            study.schema.as_str(),
            "rinkcraft.control-sequence-study.0.1"
                | "rinkcraft.holder-policy-study.0.1"
                | "rinkcraft.loose-recovery-study.0.1"
        )
        || study.focus == study.opponent
        || study.budgets.is_empty()
        || study.budgets.len() > 12
        || study.budgets.iter().any(|&x| x == 0 || x > 60)
        || study.budgets.iter().collect::<BTreeSet<_>>().len() != study.budgets.len()
        || study.policies.is_empty()
        || study.policies.len() > 8
        || study.cases.is_empty()
        || study.cases.len() > 100
    {
        return Err("Invalid sequence study identity or size".into());
    }
    let recovery_mode = study.schema == "rinkcraft.loose-recovery-study.0.1";
    let holder_mode = study.schema != "rinkcraft.control-sequence-study.0.1";
    if study.recovery_rules.is_some() != recovery_mode {
        return Err("Recovery schema/payload mismatch".into());
    }
    if study
        .policies
        .iter()
        .any(|x| x.holder_policies.is_some() != holder_mode)
    {
        return Err("Policy schema/payload mismatch".into());
    }
    let mut seen = BTreeSet::new();
    for x in &study.policies {
        if !valid(&x.id) || !seen.insert(&x.id) {
            return Err("Invalid/duplicate policy".into());
        }
    }
    let mut seen = BTreeSet::new();
    for x in &study.cases {
        if !valid(&x.id)
            || !seen.insert(&x.id)
            || (x.team != study.focus && x.team != study.opponent)
        {
            return Err("Invalid/duplicate case or focus".into());
        }
    }
    let mut names = BTreeSet::new();
    for c in &study.cases {
        for p in &study.policies {
            for b in &study.budgets {
                if !names.insert(format!("{}--{}--{}.json", c.id, p.id, b)) {
                    return Err("Output filename collision".into());
                }
            }
        }
    }
    let out = Path::new(&args[3]);
    fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut rows = Vec::new();
    for c in &study.cases {
        let q = task::apply_study_case(&base, c)?;
        let q: task::Request =
            serde_json::from_slice(&serde_json::to_vec(&q).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        let team = |ts: &[task::Team], n: &str| {
            ts.iter()
                .find(|x| x.team == n)
                .cloned()
                .ok_or("Missing sequence team")
        };
        for policy in &study.policies {
            for &budget in &study.budgets {
                let plan = seq::Plan {
                    opening: task::threats::Threat::DumpRetrieve,
                    completed_prefix: 3,
                    budget_seconds: budget,
                    first_policy: policy.first,
                    second_policy: policy.second,
                };
                let inspect =
                    |first: &task::Team, second: &task::Team| match &policy.holder_policies {
                        Some(ps) if recovery_mode => seq::recovery::inspect(
                            &q.character_rules,
                            &rules,
                            first,
                            second,
                            &plan,
                            ps,
                            study.recovery_rules.as_ref().unwrap(),
                        ),
                        Some(ps) => seq::inspect_with_policies(
                            &q.character_rules,
                            &rules,
                            first,
                            second,
                            &plan,
                            ps,
                        ),
                        None => seq::inspect(&q.character_rules, &rules, first, second, &plan),
                    };
                let a = inspect(
                    &team(&q.reference, &study.focus)?,
                    &team(&q.reference, &study.opponent)?,
                )?;
                let b = inspect(
                    &team(&q.proposed, &study.focus)?,
                    &team(&q.proposed, &study.opponent)?,
                )?;
                let filename = format!("{}--{}--{}.json", c.id, policy.id, budget);
                write(
                    &out.join(&filename),
                    &serde_json::json!({"case":c,"policy_id":policy.id,"reference":a,"proposed":b}),
                )?;
                let mut row = serde_json::json!({"case":c.id,"policy":policy.id,"budget_seconds":budget,"result":filename,
                "reference_goals":a["goal_probability_by_team"],"proposed_goals":b["goal_probability_by_team"],
                "reference_control":a["terminal_control_probability_by_team"],"proposed_control":b["terminal_control_probability_by_team"],
                "proposed_switch_probability":b["control_switch_probability"],"expected_seconds":b["expected_elapsed_seconds"]});
                if recovery_mode {
                    row.as_object_mut()
                        .unwrap()
                        .remove("proposed_switch_probability");
                    row["opening_switch_probability"] =
                        b["opening_control_switch_probability"].clone();
                    row["expected_recovery_control_changes"] =
                        b["expected_recovery_control_changes"].clone();
                    row["expected_total_control_changes"] =
                        b["expected_total_control_changes"].clone();
                }
                rows.push(row);
            }
        }
        println!(
            "{}: {} policies / {} budgets",
            c.id,
            study.policies.len(),
            study.budgets.len()
        );
    }
    write(
        &out.join("summary.json"),
        &serde_json::json!({"schema":if recovery_mode{"rinkcraft.loose-recovery-summary.0.1"}else if holder_mode{"rinkcraft.holder-policy-summary.0.1"}else{"rinkcraft.control-sequence-summary.0.1"},"kind":study.kind,"focus":study.focus,"opponent":study.opponent,"seed":null,"trials":null,"rows":rows}),
    )
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
