//! Diagnostic alternatives to recorded defensive activity. No production changes.
use super::{
    rinkcraft_attribute_calibration::{
        correlation, percentile, Candidate, Outcome, Report, Row, Summary,
    },
    rinkcraft_stat_scores::{self as scores, Catalog, Profile},
};
use std::collections::BTreeMap;
fn value(p: &Profile, key: &str) -> Option<f64> {
    p.skater_context.as_ref()?.values.get(key).copied()
}
fn minutes(p: &Profile) -> f64 {
    value(p, "def-5v5-seconds").unwrap_or(0.) / 60.
}
fn eligible(p: &Profile) -> bool {
    p.games >= 10 && minutes(p) >= 100.
}
fn metric(p: &Profile, kind: usize) -> Option<f64> {
    if p.games == 0 || minutes(p) <= 0. {
        return None;
    }
    match kind {
        0 => Some(-value(p, "def-5v5-xga60")?),
        1 => Some(-value(p, "def-5v5-fa60")?),
        _ => Some(value(p, "def-5v5-off-xga60")? - value(p, "def-5v5-xga60")?),
    }
}
fn activity(p: &Profile) -> Result<Option<f64>, String> {
    let s = p
        .scores
        .iter()
        .find(|s| s.label == "Defending")
        .ok_or("Missing Defending attribute")?;
    if s.parts.len() != 3 {
        return Err("Incompatible Defending recipe".into());
    }
    let mut points = 0.;
    let mut complete = true;
    for (key, weight) in [("blocks", 0.5), ("takeaways", 0.3), ("taken", 0.2)] {
        let part = s
            .parts
            .iter()
            .find(|p| p.key == key)
            .ok_or("Incompatible Defending recipe")?;
        if (part.weight - weight).abs() > 1e-9 {
            return Err("Incompatible Defending weight".into());
        }
        if let Some(v) = part.favorable_percentile {
            points += weight * v;
        } else {
            complete = false;
        }
    }
    Ok(complete.then_some(points))
}
pub fn compare(
    catalog: &Catalog,
    season: u32,
    names: &BTreeMap<u32, String>,
) -> Result<Report, String> {
    catalog.validate()?;
    let players = catalog
        .profiles
        .iter()
        .filter(|p| p.season == season && p.group != "Goalies")
        .collect::<Vec<_>>();
    if players.is_empty() {
        return Err("No skaters in selected season".into());
    }
    for p in &players {
        activity(p)?;
        for key in ["def-pk-share", "def-5v5-dz-share", "pp-share"] {
            if value(p, key).is_some_and(|v| !(0. ..=1.).contains(&v)) {
                return Err("Deployment share must be a fraction".into());
            }
        }
        if minutes(p) > p.minutes + 1. / 60. {
            return Err("Five-on-five exposure exceeds NHL workload".into());
        }
    }
    let recipes=[
        ("defending-xga","Expected-goal suppression",0,"Rank lower five-on-five on-ice xGA/60. Team results while the player is on ice, not isolated player impact."),
        ("defending-fa","Attempt suppression",1,"Rank lower five-on-five unblocked attempts against/60 (Fenwick against; shots on goal plus misses)."),
        ("defending-relative","On/off suppression",2,"Rank off-ice xGA/60 minus on-ice xGA/60, at five-on-five. Positive means fewer expected goals against on ice; sign is reversed from conventional on-minus-off relative rates."),
        ("defending-blend","Activity + suppression",3,"50% existing all-strength defensive-activity percentile + 25% lower five-on-five xGA/60 percentile + 25% on/off suppression percentile. Weights are hypotheses."),
    ];
    let mut candidates = vec![];
    for (id, label, kind, formula) in recipes {
        let references: [Vec<f64>; 2] = ["Forwards", "Defense"].map(|g| {
            players
                .iter()
                .filter(|p| p.group == g && eligible(p))
                .filter_map(|p| metric(p, kind))
                .map(scores::rank_value)
                .collect()
        });
        let on_references: [Vec<f64>; 2] = ["Forwards", "Defense"].map(|g| {
            players
                .iter()
                .filter(|p| p.group == g && eligible(p) && metric(p, kind).is_some())
                .filter_map(|p| metric(p, 0))
                .map(scores::rank_value)
                .collect()
        });
        let mut rows = vec![];
        for p in &players {
            let i = usize::from(p.group == "Defense");
            let raw = metric(p, kind);
            let rank = raw.and_then(|v| percentile(v, &references[i]));
            let points = if kind == 3 {
                match (
                    activity(p)?,
                    metric(p, 0).and_then(|v| percentile(v, &on_references[i])),
                    rank,
                ) {
                    (Some(b), Some(on), Some(rel)) => Some(0.5 * b + 0.25 * on + 0.25 * rel),
                    _ => None,
                }
            } else {
                rank
            };
            let baseline = p
                .scores
                .iter()
                .find(|s| s.label == "Defending")
                .unwrap()
                .score;
            let candidate = points.map(scores::level);
            let fmt = |key: &str| {
                value(p, key)
                    .map(|v| format!("{v:.2}"))
                    .unwrap_or_else(|| "—".into())
            };
            let pct = |key: &str| {
                value(p, key)
                    .map(|v| format!("{:.1}%", v * 100.))
                    .unwrap_or_else(|| "—".into())
            };
            let detail = format!(
                "5v5 xGA/60 {} on · {} off · FA/60 {} · PK time {} · DZ/(DZ+OZ) starts {}",
                fmt("def-5v5-xga60"),
                fmt("def-5v5-off-xga60"),
                fmt("def-5v5-fa60"),
                pct("def-pk-share"),
                pct("def-5v5-dz-share")
            );
            rows.push(Row {
                player_id: p.player_id,
                name: names
                    .get(&p.player_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Player {}", p.player_id)),
                group: p.group.clone(),
                games: p.games,
                minutes: minutes(p),
                exposure_missing: value(p, "def-5v5-seconds").is_none(),
                reference_eligible: eligible(p),
                baseline,
                candidate,
                delta: baseline.zip(candidate).map(|(b, c)| c as i16 - b as i16),
                component_value: raw,
                component_percentile: rank,
                reference_players: references[i].len(),
                pp_share: value(p, "pp-share"),
                input_detail: detail,
                production_rate: value(p, "def-5v5-xga60"),
            });
        }
        let groups = ["Forwards", "Defense"]
            .into_iter()
            .enumerate()
            .map(|(i, g)| {
                let all = rows
                    .iter()
                    .filter(|r| r.group == g && r.reference_eligible)
                    .collect::<Vec<_>>();
                let paired = all.iter().filter(|r| r.delta.is_some()).collect::<Vec<_>>();
                let pp = paired
                    .iter()
                    .filter_map(|r| Some((r.baseline? as f64, r.candidate? as f64, r.pp_share?)))
                    .collect::<Vec<_>>();
                Summary {
                    group: g.into(),
                    reference_players: references[i].len(),
                    paired_players: paired.len(),
                    unavailable_players: all.len() - paired.len(),
                    up: paired.iter().filter(|r| r.delta.unwrap() > 0).count(),
                    down: paired.iter().filter(|r| r.delta.unwrap() < 0).count(),
                    unchanged: paired.iter().filter(|r| r.delta == Some(0)).count(),
                    mean_absolute_change: (!paired.is_empty()).then(|| {
                        paired
                            .iter()
                            .map(|r| r.delta.unwrap().abs() as f64)
                            .sum::<f64>()
                            / paired.len() as f64
                    }),
                    largest_change: paired.iter().map(|r| r.delta.unwrap().abs()).max(),
                    baseline_pp_correlation: correlation(
                        &pp.iter().map(|p| (p.0, p.2)).collect::<Vec<_>>(),
                    ),
                    candidate_pp_correlation: correlation(
                        &pp.iter().map(|p| (p.1, p.2)).collect::<Vec<_>>(),
                    ),
                    pp_pairs: pp.len(),
                }
            })
            .collect();
        candidates.push(Candidate {
            id: id.into(),
            label: label.into(),
            attribute: "Defending".into(),
            formula: formula.into(),
            prior_attempts: 0.,
            component_unit: if kind == 1 {
                "negative FA/60"
            } else if kind == 0 {
                "negative xGA/60"
            } else {
                "off minus on xGA/60"
            }
            .into(),
            groups,
            rows,
            outcome: Some(Outcome {
                label: "5v5 expected goals against /60".into(),
                higher_is_better: false,
                exposure: "5v5 minutes".into(),
            }),
        });
    }
    Ok(Report {schema:"rinkcraft.attribute-calibration.v2".into(),season,candidates,notes:vec![
        "Diagnostic Defending candidates only; production card scores remain unchanged. Baseline is all-strength blocks/takeaways/fewer penalties, an activity proxy.".into(),
        "Candidate references and displayed minutes are five-on-five: 10 GP and 100 five-on-five minutes, at least 20 measured same-position peers. Carryover filters also use five-on-five exposure in both seasons.".into(),
        "The common next-season target is five-on-five xGA/60, lower better. Carryover correlation reverses its sign so positive means association with fewer expected goals against.".into(),
        "MoneyPuck.com on/off results describe team play, not isolated defensive impact. Strength is controlled at five-on-five; teammate, opponent, score and deployment effects are not adjusted away.".into(),
        "PK share uses NHL all-strength time. DZ-start share is DZ/(DZ+OZ) faceoff shift starts, excluding neutral-zone and on-the-fly starts. Both are descriptive context, not corrections to grades.".into(),
        "Blend raw-component repeatability refers to its on/off component. No recipe is fitted or promoted by this study.".into(),
    ]})
}
