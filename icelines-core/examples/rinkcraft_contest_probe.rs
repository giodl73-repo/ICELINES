//! Read authored arithmetic cases; print exact counts without fetching NHL data.
use icelines_core::view_model::rinkcraft_contest::Contest;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    schema: String,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    id: String,
    contest: Contest,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 1 {
        return Err("Usage: rinkcraft_contest_probe INPUT.json".into());
    }
    let raw = std::fs::read(&args[0])?;
    if raw.len() > 65536 {
        return Err("Probe input exceeds 64 KiB".into());
    }
    let probe: Probe = serde_json::from_slice(&raw)?;
    if probe.schema != "rinkcraft.contest-probe.v1"
        || probe.cases.is_empty()
        || probe.cases.len() > 256
    {
        return Err("Invalid arithmetic probe schema or case count".into());
    }
    let mut ids = std::collections::BTreeSet::new();
    let mut results = Vec::new();
    for case in probe.cases {
        if case.id.is_empty() || case.id.len() > 80 || !ids.insert(case.id.clone()) {
            return Err("Invalid/duplicate case ID".into());
        }
        results.push(serde_json::json!({"id":case.id,"contest":case.contest,"terms":case.contest.terms()?,"counts":case.contest.exact_counts()?}));
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"schema":"icelines.contest-probe-result.v1","status":"authored_game_arithmetic_not_nhl_probability","enumeration":"all 400 ordered pairs of fair d20 results","results":results})
        )?
    );
    Ok(())
}
