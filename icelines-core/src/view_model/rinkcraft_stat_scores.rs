//! Simple, owner-selected stat approximations for the six skater attributes.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub player_id: u32,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    /// Count metrics are rates per60; skating uses mph and bursts per60.
    pub metrics: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Part {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub weight: f64,
    pub lower_is_better: bool,
    pub value: Option<f64>,
    pub percentile: Option<f64>,
    pub favorable_percentile: Option<f64>,
    pub peers: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Score {
    pub label: String,
    pub score: Option<u8>,
    pub stat_points: Option<f64>,
    pub parts: Vec<Part>,
    pub note: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub scope: String,
    pub small_sample: bool,
    pub scores: Vec<Score>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordMetric {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub value: Option<f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstimatedMetric {
    pub key: String,
    pub label: String,
    pub value: Option<f64>,
    pub unit: String,
    pub per_game: Option<f64>,
    pub projected_total: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstimatedRecord {
    pub season: u32,
    pub target_games: u32,
    pub basis: String,
    pub metrics: Vec<EstimatedMetric>,
}
/// Straight season pace, not an injury/selection forecast. Ratios remain ratios.
pub fn estimate_record(
    season: u32,
    games: u32,
    minutes: f64,
    max_games: Option<u32>,
    metrics: &[RecordMetric],
) -> EstimatedRecord {
    let target = max_games
        .unwrap_or(if season >= 20262027 { 84 } else { 82 })
        .max(games);
    let metrics = metrics
        .iter()
        .map(|m| {
            let raw = m.value.filter(|v| v.is_finite());
            let projected = matches!(m.unit.as_str(), "PerGame" | "Per60" | "Count" | "Goals");
            let rate = if games == 0 {
                None
            } else {
                match m.unit.as_str() {
                    "PerGame" => raw,
                    "Per60" if minutes.is_finite() && minutes >= 0. => {
                        raw.map(|v| v * minutes / (60. * games as f64))
                    }
                    "Count" | "Goals" => raw.map(|v| v / games as f64),
                    _ => None,
                }
            };
            EstimatedMetric {
                key: m.key.clone(),
                label: m.label.replace(" / GP", "").replace(" / 60", ""),
                value: if projected {
                    rate.map(|v| v * target as f64).filter(|v| v.is_finite())
                } else {
                    raw
                },
                unit: if projected {
                    "Count".into()
                } else {
                    m.unit.clone()
                },
                per_game: rate,
                projected_total: projected,
            }
        })
        .collect();
    EstimatedRecord {
        season,
        target_games: target,
        basis: format!("{target}-game pace"),
        metrics,
    }
}
#[derive(Debug, Deserialize)]
pub struct Input {
    pub season: u32,
    pub scope: String,
    pub players: Vec<Player>,
}
pub fn evaluate_json(input: &str) -> Result<String, String> {
    let input: Input = serde_json::from_str(input).map_err(|e| e.to_string())?;
    serde_json::to_string(&build(input.season, &input.scope, &input.players)?)
        .map_err(|e| e.to_string())
}
type Term = (&'static str, &'static str, &'static str, f64, bool);
struct Recipe {
    label: &'static str,
    terms: Vec<Term>,
    note: &'static str,
}
fn recipes() -> Vec<Recipe> {
    vec![
        Recipe { label: "Skating", terms: vec![("peak-speed", "Top speed", "mph", 0.5, false), ("bursts", "20+ mph bursts", "/60", 0.5, false)], note: "Speed-based approximation; turning and pivots are not scored." },
        Recipe { label: "Handling", terms: vec![("puck-loss-share", "Giveaway share of recorded offensive actions", "%", 0.5, true), ("drawn", "Penalties drawn", "/60", 0.5, false)], note: "Puck-management proxy. Loss share = giveaways / (shots + assists + giveaways); these actions are not a touch count." },
        Recipe { label: "Passing", terms: vec![("assists", "Assists", "/60", 1., false)], note: "Assist-production approximation; includes teammate finishing and deployment." },
        Recipe { label: "Shooting", terms: vec![("goals", "Goals", "/60", 0.5, false), ("shots", "Shots on goal", "/60", 0.3, false), ("conversion", "Goals / shots on goal", "%", 0.2, false)], note: "Scoring, shot volume and conversion; not a release-mechanics assessment." },
        Recipe { label: "Reading", terms: vec![("takeaways", "Takeaways", "/60", 0.5, false), ("assists", "Assists", "/60", 0.3, false), ("taken", "Penalties taken", "/60", 0.2, true)], note: "Awareness proxy from takeaways, assists and discipline; not a measured hockey-IQ score." },
        Recipe { label: "Defending", terms: vec![("blocks", "Blocked shots", "/60", 0.5, false), ("takeaways", "Takeaways", "/60", 0.3, false), ("taken", "Penalties taken", "/60", 0.2, true)], note: "Defensive-event approximation; blocks and takeaways depend on role and opportunity." },
    ]
}
pub fn level(points: f64) -> u8 {
    (points / 5.).round().clamp(1., 20.) as u8
}

/// Same-season F/D ranks; 10GP/100min reference, with scores for smaller samples
/// visibly marked. Missing terms use the available weights; no missing=zero.
pub fn build(season: u32, scope: &str, players: &[Player]) -> Result<Vec<Profile>, String> {
    let mut ids = BTreeSet::new();
    let mut rows = players.to_vec();
    for p in &mut rows {
        if p.player_id == 0
            || !ids.insert(p.player_id)
            || !["Forwards", "Defense"].contains(&p.group.as_str())
            || !p.minutes.is_finite()
            || p.minutes < 0.
            || p.metrics.values().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("Invalid or duplicate skater stat input".into());
        }
        if let (Some(g), Some(s)) = (p.metrics.get("goals"), p.metrics.get("shots")) {
            if *s > 0. && g <= s {
                p.metrics.insert("conversion".into(), g / s);
            }
        }
        if let (Some(g), Some(s), Some(a)) = (
            p.metrics.get("giveaways"),
            p.metrics.get("shots"),
            p.metrics.get("assists"),
        ) {
            let total = g + s + a;
            if total > 0. {
                p.metrics.insert("puck-loss-share".into(), g / total);
            }
        }
    }
    let mut output = Vec::new();
    for p in &rows {
        let active = p.games > 0 && p.minutes > 0.;
        let scores = recipes()
            .into_iter()
            .map(|recipe| {
                let parts = recipe
                    .terms
                    .iter()
                    .map(|&(key, label, unit, weight, lower)| {
                        let reference = rows
                            .iter()
                            .filter(|r| r.group == p.group && r.games >= 10 && r.minutes >= 100.)
                            .filter_map(|r| r.metrics.get(key).copied())
                            .collect::<Vec<_>>();
                        let value = p.metrics.get(key).copied();
                        let percentile =
                            value.filter(|_| active && reference.len() >= 20).map(|v| {
                                100. * (reference.iter().filter(|x| **x < v).count() as f64
                                    + 0.5 * reference.iter().filter(|x| **x == v).count() as f64)
                                    / reference.len() as f64
                            });
                        Part {
                            key: key.into(),
                            label: label.into(),
                            unit: unit.into(),
                            weight,
                            lower_is_better: lower,
                            value,
                            percentile,
                            favorable_percentile: percentile
                                .map(|v| if lower { 100. - v } else { v }),
                            peers: reference.len(),
                        }
                    })
                    .collect::<Vec<_>>();
                let used_weight = parts
                    .iter()
                    .filter(|t| t.favorable_percentile.is_some())
                    .map(|t| t.weight)
                    .sum::<f64>();
                let points = (used_weight > 0.).then(|| {
                    parts
                        .iter()
                        .filter_map(|t| t.favorable_percentile.map(|v| v * t.weight))
                        .sum::<f64>()
                        / used_weight
                });
                Score {
                    label: recipe.label.into(),
                    score: points.map(level),
                    stat_points: points,
                    parts,
                    note: recipe.note.into(),
                }
            })
            .collect();
        output.push(Profile {
            schema: "rinkcraft.simple-stat-attributes.v1".into(),
            player_id: p.player_id,
            season,
            group: p.group.clone(),
            scope: scope.into(),
            small_sample: p.games < 10 || p.minutes < 100.,
            scores,
        });
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn rows() -> Vec<Player> {
        (1..=40)
            .map(|i| Player {
                player_id: i,
                group: if i <= 20 { "Forwards" } else { "Defense" }.into(),
                games: 20,
                minutes: 200.,
                metrics: [
                    "goals",
                    "shots",
                    "assists",
                    "drawn",
                    "giveaways",
                    "takeaways",
                    "taken",
                    "blocks",
                    "peak-speed",
                    "bursts",
                ]
                .into_iter()
                .map(|k| (k.into(), i as f64))
                .collect(),
            })
            .collect()
    }
    #[test]
    fn six_recipes_use_position_peers_and_reverse_negative_stats() {
        let r = build(20252026, "all strengths", &rows()).unwrap();
        assert!(r
            .iter()
            .all(|p| p.scores.len() == 6 && p.scores.iter().all(|s| s.score.is_some())));
        assert_eq!(r[19].scores[2].score, Some(20));
        assert_eq!(r[20].scores[2].score, Some(1));
        let taken = &r[19].scores[4].parts[2];
        assert_eq!(taken.percentile, Some(97.5));
        assert_eq!(taken.favorable_percentile, Some(2.5));
        assert_eq!(
            (level(0.), level(50.), level(90.), level(100.)),
            (1, 10, 18, 20)
        );
    }
    #[test]
    fn missing_zero_and_small_samples_are_distinct() {
        let mut input = rows();
        input[0].metrics.remove("bursts");
        input[0].metrics.insert("drawn".into(), 0.);
        input[0].games = 1;
        let r = build(20252026, "all strengths", &input).unwrap();
        assert!(r[0].small_sample);
        assert_eq!(r[0].scores[0].parts[1].percentile, None);
        // Removing a reference player leaves 19 measured peers: no fabricated ranks.
        assert_eq!(r[0].scores[0].score, None);
        assert_eq!(r[0].scores[1].parts[1].value, Some(0.));
        assert!(build(20252026, "all", &[input[0].clone(), input[0].clone()]).is_err());
    }
}
