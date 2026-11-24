//! Descriptive candidate comparison. No production grades or simulator values change.
use super::rinkcraft_stat_scores::{self as scores, Catalog, Profile};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Row {
    pub player_id: u32,
    pub name: String,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub reference_eligible: bool,
    pub baseline: Option<u8>,
    pub candidate: Option<u8>,
    pub delta: Option<i16>,
    pub component_value: Option<f64>,
    pub component_percentile: Option<f64>,
    pub reference_players: usize,
    pub pp_share: Option<f64>,
    pub input_detail: String,
    /// NHL all-strength assists/60 or goals/60; a common future outcome for all
    /// candidates of this attribute, not a recipe-specific target.
    #[serde(default)]
    pub production_rate: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub group: String,
    pub reference_players: usize,
    pub paired_players: usize,
    pub unavailable_players: usize,
    pub up: usize,
    pub down: usize,
    pub unchanged: usize,
    pub mean_absolute_change: Option<f64>,
    pub largest_change: Option<i16>,
    pub baseline_pp_correlation: Option<f64>,
    pub candidate_pp_correlation: Option<f64>,
    pub pp_pairs: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: String,
    pub label: String,
    pub attribute: String,
    pub formula: String,
    pub prior_attempts: f64,
    pub component_unit: String,
    pub groups: Vec<Summary>,
    pub rows: Vec<Row>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub schema: String,
    pub season: u32,
    pub candidates: Vec<Candidate>,
    pub notes: Vec<String>,
}
fn eligible(p: &Profile) -> bool {
    p.games >= 10 && p.minutes >= 100.
}
fn value(p: &Profile, key: &str) -> Option<f64> {
    p.skater_context.as_ref()?.values.get(key).copied()
}
fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-7 * (1. + a.abs().max(b.abs()))
}
fn validate_inputs(p: &Profile) -> Result<(), String> {
    if let Some(share) = value(p, "pp-share") {
        if !(0. ..=1.).contains(&share) {
            return Err("PP share must be a fraction".into());
        }
    }
    if let Some(primary) = value(p, "primary") {
        if primary.fract() != 0. || value(p, "secondary").is_some_and(|v| v.fract() != 0.) {
            return Err("Assist credits must be whole counts".into());
        }
        if let Some(rate) = value(p, "primary-rate") {
            if !close(rate * p.minutes / 60., primary) {
                return Err("Primary assist rate has a different time window".into());
            }
        }
        if let Some((secondary, rate)) = value(p, "secondary").zip(value(p, "assists")) {
            if !close((primary + secondary) * 60. / p.minutes, rate) {
                return Err("Assist credits do not reconcile with all-strength production".into());
            }
        }
    }
    if let Some(n) = value(p, "quality-attempts") {
        let g = value(p, "quality-goals").ok_or("Shot count missing paired goal count")?;
        let x = value(p, "ixg").ok_or("Shot count missing paired xG")?;
        if n.fract() != 0. || n < 1. || g.fract() != 0. || g > n || x > n {
            return Err("Invalid shot population".into());
        }
        if value(p, "finishing").is_some_and(|v| !close(v, g - x)) {
            return Err("Finishing residual differs from same-archive goals/xG".into());
        }
    }
    Ok(())
}
fn metric(p: &Profile, kind: usize, prior: f64) -> Option<f64> {
    if p.games == 0 || p.minutes <= 0. {
        return None;
    }
    match kind {
        0 => value(p, "primary-rate"),
        1 => Some((value(p, "primary")? + 0.5 * value(p, "secondary")?) * 60. / p.minutes),
        _ => {
            let n = value(p, "quality-attempts")?;
            if n <= 0. {
                return None;
            }
            Some(100. * (value(p, "quality-goals")? - value(p, "ixg")?) / (n + prior))
        }
    }
}
/// Quantized midranks match the current attribute system, including signed measurements.
fn percentile(v: f64, reference: &[f64]) -> Option<f64> {
    if reference.len() < 20 {
        return None;
    }
    let v = scores::rank_value(v);
    Some(
        100. * (reference.iter().filter(|x| **x < v).count() as f64
            + 0.5 * reference.iter().filter(|x| **x == v).count() as f64)
            / reference.len() as f64,
    )
}
fn correlation(pairs: &[(f64, f64)]) -> Option<f64> {
    if pairs.len() < 2 {
        return None;
    }
    let n = pairs.len() as f64;
    let mx = pairs.iter().map(|p| p.0).sum::<f64>() / n;
    let my = pairs.iter().map(|p| p.1).sum::<f64>() / n;
    let xx = pairs.iter().map(|p| (p.0 - mx).powi(2)).sum::<f64>();
    let yy = pairs.iter().map(|p| (p.1 - my).powi(2)).sum::<f64>();
    if xx <= 0. || yy <= 0. {
        return None;
    }
    Some(
        (pairs.iter().map(|p| (p.0 - mx) * (p.1 - my)).sum::<f64>() / (xx * yy).sqrt())
            .clamp(-1., 1.),
    )
}
pub fn compare(
    catalog: &Catalog,
    season: u32,
    names: &BTreeMap<u32, String>,
) -> Result<Report, String> {
    catalog.validate()?;
    let profiles = catalog
        .profiles
        .iter()
        .filter(|p| p.season == season && p.group != "Goalies")
        .collect::<Vec<_>>();
    if profiles.is_empty() {
        return Err("No skaters in selected season".into());
    }
    for p in &profiles {
        validate_inputs(p)?;
    }
    let mut candidates = vec![];
    for (id,label,attribute,kind,prior,formula) in [
  ("passing-primary","Primary assists only","Passing",0,0.,"Rank primary assists per60; 100% of candidate Passing."),
  ("passing-credits","Primary + half secondary","Passing",1,0.,"Rank (primary assists + 0.5 × secondary assists) per60. The credit weight is a hypothesis."),
  ("shooting-adjusted","Adjusted finishing · no prior","Shooting",2,0.,"50% existing goals/60 percentile + 30% existing shots/60 percentile + 20% rank of 100 × (archive goals − xG) / attempts."),
  ("shooting-prior25","Adjusted finishing · 25-attempt prior","Shooting",2,25.,"Same 50/30/20 recipe; residual = 100 × (archive goals − xG) / (attempts + 25). Prior assumes zero excess goals."),
  ("shooting-prior100","Adjusted finishing · 100-attempt prior","Shooting",2,100.,"Same 50/30/20 recipe; residual = 100 × (archive goals − xG) / (attempts + 100). Prior assumes zero excess goals."),
 ] {
  let references:[Vec<f64>;2]=["Forwards","Defense"].map(|group|profiles.iter().filter(|p|p.group==group&&eligible(p)).filter_map(|p|metric(p,kind,prior)).map(scores::rank_value).collect());
  let mut rows=vec![];
  for p in &profiles {
   let reference=&references[usize::from(p.group=="Defense")];let v=metric(p,kind,prior);let rank=v.and_then(|v|percentile(v,reference));
   let baseline=p.scores.iter().find(|s|s.label==attribute).ok_or("Missing attribute")?;
   let points=if attribute=="Passing" {
    if baseline.parts.len()!=1 || baseline.parts[0].key!="assists" || !close(baseline.parts[0].weight,1.) {return Err("Incompatible baseline Passing recipe".into());}
    rank
   }else {
    let part=|key:&str,w:f64|->Result<Option<f64>,String>{let t=baseline.parts.iter().find(|t|t.key==key).ok_or("Incompatible Shooting recipe")?;if !close(t.weight,w){return Err("Incompatible baseline Shooting weights".into());}Ok(t.favorable_percentile)};
    match (part("goals",0.5)?,part("shots",0.3)?,part("conversion",0.2)?,rank){(Some(g),Some(s),Some(_),Some(r))=>Some(0.5*g+0.3*s+0.2*r),_=>None}
   };
   let candidate=points.map(scores::level);let delta=baseline.score.zip(candidate).map(|(a,b)|b as i16-a as i16);
   let display=|k:&str|value(p,k).map(|v|format!("{v:.2}")).unwrap_or_else(||"—".into());
   let detail=if kind<2 {format!("{} primary · {} secondary · {} primary/60",display("primary"),display("secondary"),display("primary-rate"))}
    else{format!("{} archive goals − {} xG · {} attempts · prior {prior:.0}",display("quality-goals"),display("ixg"),display("quality-attempts"))};
   rows.push(Row{player_id:p.player_id,name:names.get(&p.player_id).cloned().unwrap_or_else(||format!("Player {}",p.player_id)),group:p.group.clone(),games:p.games,minutes:p.minutes,reference_eligible:eligible(p),baseline:baseline.score,candidate,delta,component_value:v,component_percentile:rank,reference_players:reference.len(),pp_share:value(p,"pp-share"),input_detail:detail,production_rate:value(p,if attribute=="Passing"{"assists"}else{"goals"})});
  }
  let groups=["Forwards","Defense"].into_iter().enumerate().map(|(i,group)|{
   let eligible=rows.iter().filter(|r|r.group==group&&r.reference_eligible).collect::<Vec<_>>();
   let paired=eligible.iter().filter(|r|r.delta.is_some()).collect::<Vec<_>>();
   let pp=paired.iter().filter_map(|r|Some((r.baseline? as f64,r.candidate? as f64,r.pp_share?))).collect::<Vec<_>>();
   Summary{group:group.into(),reference_players:references[i].len(),paired_players:paired.len(),unavailable_players:eligible.len()-paired.len(),up:paired.iter().filter(|r|r.delta.unwrap()>0).count(),down:paired.iter().filter(|r|r.delta.unwrap()<0).count(),unchanged:paired.iter().filter(|r|r.delta==Some(0)).count(),mean_absolute_change:(!paired.is_empty()).then(||paired.iter().map(|r|r.delta.unwrap().unsigned_abs() as f64).sum::<f64>()/paired.len() as f64),largest_change:paired.iter().map(|r|r.delta.unwrap().abs()).max(),baseline_pp_correlation:correlation(&pp.iter().map(|p|(p.0,p.2)).collect::<Vec<_>>()),candidate_pp_correlation:correlation(&pp.iter().map(|p|(p.1,p.2)).collect::<Vec<_>>()),pp_pairs:pp.len()}
  }).collect();
  candidates.push(Candidate{id:id.into(),label:label.into(),attribute:attribute.into(),formula:formula.into(),prior_attempts:prior,component_unit:if kind<2{"credits/60"}else{"excess goals /100 adjusted attempts"}.into(),groups,rows});
 }
    Ok(Report{schema:"rinkcraft.attribute-calibration.v1".into(),season,candidates,notes:vec![
  "Diagnostic comparison only. No candidate is promoted and no card grades or simulator bonuses are changed.".into(),
  "Rank references: same-season Forwards/Defense, at least 10 GP and 100 minutes; at least 20 measured peers. Smaller recipients stay labelled by their GP/minutes.".into(),
  "Shooting keeps existing NHL production percentiles fixed. Its adjusted finishing component uses only MoneyPuck.com goals/xG/attempts with a goalie in net. Component populations are reported separately.".into(),
  "Weights and prior sizes are hypotheses. Larger ranking changes or weaker PP correlation do not establish better skill measurement. Held-out seasons and external validation remain necessary.".into(),
  "PP correlations are descriptive Pearson correlations of 1–20 estimates and PP time share on exactly the same paired reference players; they are not causal effects.".into(),
 ]})
}
