//! Offline fixed-model batch runner. Input preparation and independent oracle checks are external.
use icelines_core::view_model::rinkcraft_finishing::{self as f, HistoryRow, Model, Target};
use serde::Deserialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    model: Model,
    cases: Vec<Case>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    target: Target,
    history: Vec<HistoryRow>,
    observed_goals: u32,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: rinkcraft_finishing_replay REQUEST_JSON OUTPUT_JSON".into());
    }
    let input = std::fs::read(&args[0])?;
    let r: Request = serde_json::from_slice(&input)?;
    if r.cases.is_empty() {
        return Err("Empty replay batch".into());
    }
    let mut targets = BTreeSet::new();
    for c in &r.cases {
        if !targets.insert((c.target.player_id, c.target.season)) {
            return Err("Duplicate target player-season".into());
        }
    }
    let rows=r.cases.iter().map(|c|{let p=f::predict(&r.model,&c.target,&c.history)?;
  let dev=json!({"global_baseline":f::poisson_deviance(c.observed_goals,p.policies.global_baseline)?,"global_player":f::poisson_deviance(c.observed_goals,p.policies.global_player)?,"position_baseline":f::poisson_deviance(c.observed_goals,p.policies.position_baseline)?,"position_player":f::poisson_deviance(c.observed_goals,p.policies.position_player)?});
  Ok::<_,String>(json!({"player_id":c.target.player_id,"season":c.target.season,"prediction":p,"deviance":dev}))}).collect::<Result<Vec<_>,_>>()?;
    let output = json!({"schema":"icelines.fixed-finishing-replay.v0.1","request_sha256":format!("{:x}",Sha256::digest(&input)),"model":r.model,"rows":rows,"prior_refitted":false,"bootstrap_or_quantile_intervals_computed":false,"headline_scores_released":false,"null_deviance_means":"Infinite deviance for positive observed goals at zero expectation, never zero error."});
    std::fs::write(&args[1], serde_json::to_vec_pretty(&output)?)?;
    Ok(())
}
