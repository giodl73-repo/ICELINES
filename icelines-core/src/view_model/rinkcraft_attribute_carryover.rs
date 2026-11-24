//! Forward-only paired-season diagnostics; no fitted weights or grade promotion.
use super::rinkcraft_attribute_calibration::{Report, Row};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comparison {
    pub from: u32,
    pub to: u32,
    pub recipe: String,
    pub label: String,
    pub attribute: String,
    pub group: String,
    pub sample: String,
    pub min_games: u32,
    pub min_minutes: f64,
    pub eligible_before: usize,
    pub eligible_after: usize,
    pub matched_players: usize,
    pub unavailable_pairs: usize,
    pub no_eligible_same_group_next: usize,
    pub baseline_repeatability: Option<f64>,
    pub candidate_repeatability: Option<f64>,
    pub baseline_next_production: Option<f64>,
    pub candidate_next_production: Option<f64>,
    pub component_repeatability: Option<f64>,
    pub baseline_mean_absolute_change: Option<f64>,
    pub candidate_mean_absolute_change: Option<f64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct History {
    pub schema: String,
    pub seasons: Vec<u32>,
    pub comparisons: Vec<Comparison>,
    pub notes: Vec<String>,
}
fn correlation(p: &[(f64, f64)]) -> Option<f64> {
    if p.len() < 3 {
        return None;
    }
    let n = p.len() as f64;
    let x = p.iter().map(|p| p.0).sum::<f64>() / n;
    let y = p.iter().map(|p| p.1).sum::<f64>() / n;
    let xx = p.iter().map(|p| (p.0 - x).powi(2)).sum::<f64>();
    let yy = p.iter().map(|p| (p.1 - y).powi(2)).sum::<f64>();
    if xx <= 0. || yy <= 0. {
        return None;
    }
    Some((p.iter().map(|p| (p.0 - x) * (p.1 - y)).sum::<f64>() / (xx * yy).sqrt()).clamp(-1., 1.))
}
fn validate(report: &Report) -> Result<(), String> {
    let year = report.season / 10000;
    if report.schema != "rinkcraft.attribute-calibration.v1"
        || !(2008..=2200).contains(&year)
        || report.season % 10000 != year + 1
    {
        return Err("Invalid calibration report season/schema".into());
    }
    let mut ids = BTreeSet::new();
    for c in &report.candidates {
        if !ids.insert(&c.id) {
            return Err("Duplicate candidate".into());
        }
        let mut players = BTreeSet::new();
        for r in &c.rows {
            if !players.insert(r.player_id)
                || !["Forwards", "Defense"].contains(&r.group.as_str())
                || !r.minutes.is_finite()
                || r.minutes < 0.
                || [r.baseline, r.candidate]
                    .into_iter()
                    .flatten()
                    .any(|v| !(1..=20).contains(&v))
                || r.production_rate.is_some_and(|v| !v.is_finite() || v < 0.)
                || r.component_value.is_some_and(|v| !v.is_finite())
            {
                return Err("Invalid/duplicate calibration player".into());
            }
        }
    }
    if report.candidates.is_empty() {
        return Err("Empty calibration report".into());
    }
    Ok(())
}
pub fn compare(reports: &[Report]) -> Result<History, String> {
    if reports.len() < 2 {
        return Err("Provide at least two consecutive season reports".into());
    }
    let mut reports = reports.iter().collect::<Vec<_>>();
    reports.sort_by_key(|r| r.season);
    for r in &reports {
        validate(r)?;
    }
    let mut comparisons = vec![];
    for pair in reports.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        if a.season.checked_add(10001) != Some(b.season) {
            return Err("Seasons must be unique and consecutive".into());
        }
        if a.candidates.len() != b.candidates.len() {
            return Err("Recipe set changed between seasons".into());
        }
        for ca in &a.candidates {
            let cb = b
                .candidates
                .iter()
                .find(|c| c.id == ca.id)
                .ok_or("Recipe missing in following season")?;
            if ca.attribute != cb.attribute
                || ca.formula != cb.formula
                || ca.prior_attempts != cb.prior_attempts
                || ca.component_unit != cb.component_unit
            {
                return Err("Recipe changed between seasons".into());
            }
            let next: BTreeMap<_, _> = cb.rows.iter().map(|r| (r.player_id, r)).collect();
            for group in ["Forwards", "Defense"] {
                for (sample, gp, minutes) in
                    [("10 GP / 100 min", 10, 100.), ("40 GP / 500 min", 40, 500.)]
                {
                    let eligible =
                        |r: &&Row| r.group == group && r.games >= gp && r.minutes >= minutes;
                    let before = ca.rows.iter().filter(eligible).collect::<Vec<_>>();
                    let eligible_after = cb.rows.iter().filter(eligible).count();
                    let mut matched = vec![];
                    let mut unavailable = 0;
                    let mut no_next = 0;
                    for ra in &before {
                        let Some(rb) = next.get(&ra.player_id).filter(|rb| {
                            rb.group == group && rb.games >= gp && rb.minutes >= minutes
                        }) else {
                            no_next += 1;
                            continue;
                        };
                        match (
                            ra.baseline,
                            ra.candidate,
                            rb.baseline,
                            rb.candidate,
                            rb.production_rate,
                            ra.component_value,
                            rb.component_value,
                        ) {
                            (
                                Some(ba),
                                Some(ca),
                                Some(bb),
                                Some(cb),
                                Some(outcome),
                                Some(va),
                                Some(vb),
                            ) => matched.push((
                                ba as f64, ca as f64, bb as f64, cb as f64, outcome, va, vb,
                            )),
                            _ => unavailable += 1,
                        }
                    }
                    let cor = |f: fn(&(f64, f64, f64, f64, f64, f64, f64)) -> (f64, f64)| {
                        correlation(&matched.iter().map(f).collect::<Vec<_>>())
                    };
                    let mean = |f: fn(&(f64, f64, f64, f64, f64, f64, f64)) -> f64| {
                        (!matched.is_empty())
                            .then(|| matched.iter().map(f).sum::<f64>() / matched.len() as f64)
                    };
                    comparisons.push(Comparison {
                        from: a.season,
                        to: b.season,
                        recipe: ca.id.clone(),
                        label: ca.label.clone(),
                        attribute: ca.attribute.clone(),
                        group: group.into(),
                        sample: sample.into(),
                        min_games: gp,
                        min_minutes: minutes,
                        eligible_before: before.len(),
                        eligible_after,
                        matched_players: matched.len(),
                        unavailable_pairs: unavailable,
                        no_eligible_same_group_next: no_next,
                        baseline_repeatability: cor(|p| (p.0, p.2)),
                        candidate_repeatability: cor(|p| (p.1, p.3)),
                        baseline_next_production: cor(|p| (p.0, p.4)),
                        candidate_next_production: cor(|p| (p.1, p.4)),
                        component_repeatability: cor(|p| (p.5, p.6)),
                        baseline_mean_absolute_change: mean(|p| (p.0 - p.2).abs()),
                        candidate_mean_absolute_change: mean(|p| (p.1 - p.3).abs()),
                    });
                }
            }
        }
    }
    Ok(History {schema:"rinkcraft.attribute-carryover.v1".into(),seasons:reports.iter().map(|r|r.season).collect(),comparisons,notes:vec![
        "Forward-only season pairs; recipes are unchanged and no weights are fitted. Production card grades remain unchanged.".into(),
        "Each row uses the same players for baseline and candidate, with the sample threshold met in both seasons and unchanged F/D group. Missing outcomes are excluded from both; attrition is reported.".into(),
        "Repeatability correlates grades with the next season's corresponding grades. Production correlates each earlier grade with a common next-season NHL outcome: assists/60 for Passing, goals/60 for Shooting, all strengths.".into(),
        "Component repeatability correlates the raw recipe component across seasons. All correlations are Pearson; constant values or fewer than three pairs are unavailable.".into(),
        "Survivor selection, deployment, teammates, aging and injuries remain confounders. No confidence intervals or causal skill claims; small differences are not proof of a better recipe. The larger-sample filter does not rebuild the original reference ranks.".into(),
    ]})
}
