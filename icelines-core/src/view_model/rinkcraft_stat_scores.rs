//! Simple public-stat estimates with separate six-attribute skater and goalie families.
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub player_id: u32,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    /// Counts use per-60 rates; skating uses mph/bursts; goalies also use save fractions and a rebound ratio.
    pub metrics: BTreeMap<String, f64>,
}
/// Join recorded counts to official all-strength TOI before scoring.
/// A 5v5 evidence card's minutes must never be used for all-strength counts.
pub fn observed_player(
    record: &super::rinkcraft_achievements::AchievementProfile,
    usage: &super::rinkcraft_deployment::DeploymentEvidence,
    edge: Option<&super::rinkcraft_edge::EdgeEvidence>,
) -> Result<Player, String> {
    use crate::season_stats::SeasonType;
    if record.player_id == 0
        || record.games == 0
        || !["Forwards", "Defense"].contains(&record.group.as_str())
        || record.season_type != "regular"
        || usage.goalie
        || usage.player_id.0 != record.player_id
        || usage.season.0 != record.season
        || usage.season_type != SeasonType::Regular
        || usage.games != record.games
    {
        return Err(
            "Stat attributes require matching regular-season skater counts and workload".into(),
        );
    }
    let totals = usage
        .rows
        .iter()
        .filter(|r| r.key == "total")
        .collect::<Vec<_>>();
    if totals.len() != 1 || totals[0].seconds == 0 {
        return Err("Stat attributes require one positive all-strength TOI total".into());
    }
    let minutes = f64::from(totals[0].seconds) / 60.;
    let mut counts = BTreeMap::new();
    for c in record.feats.iter().flat_map(|f| &f.conditions) {
        if ![
            "goals",
            "shots",
            "assists",
            "giveaways",
            "drawn",
            "takeaways",
            "taken",
            "blocks",
        ]
        .contains(&c.key.as_str())
        {
            continue;
        }
        let Some(count) = c.numerator else { continue };
        if !count.is_finite()
            || count < 0.
            || c.unit != "PerGame"
            || c.denominator != Some(f64::from(record.games))
            || c.value.is_none_or(|v| {
                !v.is_finite() || (v - count / f64::from(record.games)).abs() > 1e-10
            })
        {
            return Err(format!("Incoherent recorded count: {}", c.key));
        }
        if counts
            .insert(c.key.clone(), count)
            .is_some_and(|old| old != count)
        {
            return Err(format!("Conflicting recorded counts: {}", c.key));
        }
    }
    let mut metrics = counts
        .into_iter()
        .map(|(key, count)| (key, count * 60. / minutes))
        .collect::<BTreeMap<_, _>>();
    if let Some(edge) = edge {
        if edge.schema != "icelines.rinkcraft-edge.v1"
            || edge.player_id != record.player_id
            || edge.season != record.season
            || edge.season_type != "regular"
            || edge.group != record.group
            || edge.games != record.games
            || edge.strength_scope != "all strengths"
        {
            return Err("EDGE and recorded workload must describe the same player/window".into());
        }
        for m in &edge.measures {
            let (key, unit, rate) = match m.key.as_str() {
                "max-skating-speed" => ("peak-speed", "Mph", m.value),
                "speed-bursts-over-20" => ("bursts", "Count", m.value * 60. / minutes),
                _ => continue,
            };
            if m.unit != unit
                || !m.value.is_finite()
                || m.value < 0.
                || metrics.insert(key.into(), rate).is_some()
            {
                return Err(format!("Invalid or duplicate EDGE stat: {}", m.key));
            }
        }
    }
    Ok(Player {
        player_id: record.player_id,
        group: record.group.clone(),
        games: record.games,
        minutes,
        metrics,
    })
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub production_mix: Option<super::rinkcraft_production_mix::Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub penalty_balance: Option<super::rinkcraft_penalty_balance::Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_method_sample: Option<super::rinkcraft_shot_methods::Sample>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shot_methods: Option<super::rinkcraft_shot_methods::Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub skating_detail: Option<super::rinkcraft_skating_detail::Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit_links: Option<super::rinkcraft_unit_links::Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub under_pressure: Option<super::rinkcraft_under_pressure::Record>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_record: Option<EstimatedRecord>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub skater_context:Option<super::rinkcraft_skater_context::Context>,
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub shot_context:Option<super::rinkcraft_goalie_shots::Context>,
    pub schema: String,
    pub player_id: u32,
    pub season: u32,
    pub group: String,
    pub scope: String,
    #[serde(default)]
    pub games: u32,
    #[serde(default)]
    pub minutes: f64,
    pub small_sample: bool,
    pub scores: Vec<Score>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonChange {
    pub player_id: u32,
    pub season: u32,
    pub previous_season: u32,
    /// Estimate-point differences, not measured changes in technical skill.
    pub deltas: Vec<Option<i16>>,
}

/// Compare adjacent supplied windows for each actor after catalog validation.
/// Do not skip over an intervening missing or incompatible measurement.
pub fn season_changes(profiles: &[Profile]) -> Vec<SeasonChange> {
    let mut ordered = profiles.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|p| (p.player_id, p.season));
    ordered
        .windows(2)
        .filter_map(|pair| {
            let (old, new) = (pair[0], pair[1]);
            if old.player_id != new.player_id || old.season == new.season {
                return None;
            }
            let comparable = old.group == new.group && old.scope == new.scope;
            let deltas = new
                .scores
                .iter()
                .enumerate()
                .map(|(index, score)| {
                    let previous = old.scores.get(index)?;
                    if !comparable
                        || score.label != previous.label
                        || score.parts.len() != previous.parts.len()
                        || score.parts.iter().zip(&previous.parts).any(|(a, b)| {
                            a.key != b.key
                                || a.unit != b.unit
                                || a.weight != b.weight
                                || a.lower_is_better != b.lower_is_better
                                || a.favorable_percentile.is_some()
                                    != b.favorable_percentile.is_some()
                        })
                    {
                        return None;
                    }
                    Some(i16::from(score.score?) - i16::from(previous.score?))
                })
                .collect();
            Some(SeasonChange {
                player_id: new.player_id,
                season: new.season,
                previous_season: old.season,
                deltas,
            })
        })
        .collect()
}
/// Portable estimates for the ordinary native and browser profile paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    pub schema: String,
    pub profiles: Vec<Profile>,
}
impl Catalog {
    pub fn from_json(source: &str) -> Result<Self, String> {
        if source.len() > 32 * 1024 * 1024 {
            return Err("Stat attribute catalog exceeds 32 MiB".into());
        }
        let catalog: Self = serde_json::from_str(source).map_err(|e| e.to_string())?;
        catalog.validate()?;
        Ok(catalog)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.stat-attribute-catalog.v1" {
            return Err("Unsupported stat attribute catalog".into());
        }
        let mut seen = BTreeSet::new();
        for p in &self.profiles {
            if let Some(r)=&p.production_mix {super::rinkcraft_production_mix::validate(p,r)?;}
            if let Some(r)=&p.penalty_balance {super::rinkcraft_penalty_balance::validate(p,r)?;}
            if let Some(s)=&p.shot_method_sample {super::rinkcraft_shot_methods::validate_sample(p,s)?;}
            if let Some(r)=&p.shot_methods {super::rinkcraft_shot_methods::validate(p,r)?;}
            if let Some(r)=&p.skating_detail {super::rinkcraft_skating_detail::validate(p,r)?;}
            if let Some(r)=&p.unit_links {super::rinkcraft_unit_links::validate(p,r)?;}
            if let Some(r)=&p.under_pressure {super::rinkcraft_under_pressure::validate(p,r)?;}
            if let Some(r) = &p.estimated_record {
                if r.season != p.season || r.observed_games.is_some_and(|g|g!=p.games) || r.target_games < p.games || r.target_games == 0 || r.metrics.len() != 7
                    || r.metrics.iter().any(|m| m.value.is_some_and(|v| !v.is_finite() || v < 0.) || m.per_game.is_some_and(|v| !v.is_finite() || v < 0.)) {
                    return Err("Estimated record does not match profile window".into());
                }
            }
            if let Some(r)=p.skater_context.as_ref().and_then(|c|c.shooting.as_ref()){if r.player_id!=p.player_id||r.season!=p.season||r.group!=p.group||r.games!=p.games||(r.minutes-p.minutes).abs()>1e-9{return Err("Shooting record does not match profile window".into());}}
            if let Some(r)=p.skater_context.as_ref().and_then(|c|c.passing.as_ref()){if r.player_id!=p.player_id||r.season!=p.season||r.group!=p.group||r.games!=p.games||(r.minutes-p.minutes).abs()>1e-9{return Err("Passing record does not match profile window".into());}}
            if let Some(r)=p.skater_context.as_ref().and_then(|c|c.reading.as_ref()){if r.player_id!=p.player_id||r.season!=p.season||r.group!=p.group||r.games!=p.games||(r.minutes-p.minutes).abs()>1e-9{return Err("Reading record does not match profile window".into());}}
            if let Some(r)=p.skater_context.as_ref().and_then(|c|c.handling.as_ref()){if r.player_id!=p.player_id||r.season!=p.season||r.group!=p.group||r.games!=p.games||(r.minutes-p.minutes).abs()>1e-9{return Err("Handling record does not match profile window".into());}}
            if let Some(r)=p.skater_context.as_ref().and_then(|c|c.defending.as_ref()){if r.player_id!=p.player_id||r.season!=p.season||r.group!=p.group||r.games!=p.games||r.minutes_5v5.is_some_and(|m|m>p.minutes+1./60.){return Err("Defending record does not match profile window".into());}}
            if let Some(context)=&p.skater_context {if p.group=="Goalies"{return Err("Goalie has skater context".into());}context.validate()?;}
            if let Some(context)=&p.shot_context {if p.group!="Goalies"{return Err("Skater has goalie shot context".into());}context.validate()?;}
            let labels = attribute_labels(&p.group);
            let year = p.season / 10000;
            if p.schema != "rinkcraft.simple-stat-attributes.v1"
                || p.player_id == 0
                || !(1900..=2200).contains(&year)
                || p.season % 10000 != year + 1
                || !seen.insert((p.player_id, p.season))
                || !["Forwards", "Defense", "Goalies"].contains(&p.group.as_str())
                || !p.minutes.is_finite()
                || p.minutes < 0.
                || p.scope.trim().is_empty()
                || p.scores.len() != labels.len()
                || p.scores.iter().zip(labels).any(|(s, label)| {
                    s.label != label
                        || s.score != s.stat_points.map(level)
                        || s.score.is_some_and(|v| !(1..=20).contains(&v))
                        || s.stat_points
                            .is_some_and(|v| !v.is_finite() || !(0. ..=100.).contains(&v))
                        || s.parts.iter().any(|part| {
                            part.value.is_some_and(|v| !v.is_finite() || v < 0.)
                                || [part.percentile, part.favorable_percentile]
                                    .into_iter()
                                    .flatten()
                                    .any(|v| !v.is_finite() || !(0. ..=100.).contains(&v))
                        })
                })
                || p.scores.iter().any(|s| s.score.is_some()) && (p.games == 0 || p.minutes == 0.)
            {
                return Err("Invalid or duplicate stat attribute profile".into());
            }
        }
        Ok(())
    }
    /// A season match alone is insufficient when the active snapshot has advanced.
    pub fn profile(&self, player: u32, season: u32, games: u32) -> Option<&Profile> {
        self.profiles
            .iter()
            .find(|p| p.player_id == player && p.season == season && p.games == games)
    }
}
pub fn catalog_profile_json(
    source: &str,
    player: u32,
    season: u32,
    games: u32,
) -> Result<String, String> {
    let catalog = Catalog::from_json(source)?;
    serde_json::to_string(&catalog.profile(player, season, games)).map_err(|e| e.to_string())
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub calculation: Option<String>,
    pub key: String,
    pub label: String,
    pub value: Option<f64>,
    pub unit: String,
    pub per_game: Option<f64>,
    pub projected_total: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EstimatedRecord {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_games: Option<u32>,
    pub season: u32,
    pub target_games: u32,
    pub basis: String,
    pub metrics: Vec<EstimatedMetric>,
}
impl EstimatedRecord {
    pub fn has_calculations(&self) -> bool { self.metrics.iter().any(|m| m.calculation.is_some()) }
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
            let recorded_count = rate.map(|r| r * games as f64).filter(|v| v.is_finite() && *v >= 0. && *v <= u32::MAX as f64 && (*v-v.round()).abs() <= 1e-6).map(|v|v.round() as u32);
            let calculation = rate.filter(|r| r.is_finite()).map(|r| match recorded_count {
                Some(n) => format!("{n} / {games} GP x {target} games = {:.2}", r*target as f64),
                None => format!("Observed per-game rate x {target} games = {:.2}; full-precision rate used.",r*target as f64),
            });
            EstimatedMetric {
                recorded_count,
                calculation,
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
        observed_games: Some(games),
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
pub fn attribute_labels(group: &str) -> [&'static str; 6] {
    if group == "Goalies" {
        [
            "Tracking",
            "Positioning",
            "Movement",
            "Rebounds",
            "Recovery",
            "Puck Play",
        ]
    } else {
        [
            "Skating",
            "Handling",
            "Passing",
            "Shooting",
            "Reading",
            "Defending",
        ]
    }
}
fn recipes(goalie: bool) -> Vec<Recipe> {
    if goalie {
        return vec![
        Recipe {label:"Tracking",terms:vec![("goalie-save-fraction","NHL saves / shots on goal","%",1.,false)],note:"Overall shot-stopping approximation from NHL counts; not a measurement of visual tracking."},
        Recipe {label:"Positioning",terms:vec![("goalie-low-save","Low-danger shots on goal saved","%",1.,false)],note:"MoneyPuck.com low-danger saving outcome (xG < 0.08); not measured positioning or angle control."},
        Recipe {label:"Movement",terms:vec![("goalie-high-save","High-danger shots on goal saved","%",0.5,false),("goalie-cross-save","Cross-slot rebound shots on goal saved","%",0.5,false)],note:"MoneyPuck.com difficult-shot and royal-road rebound saving approximation; not measured lateral speed or technique."},
        Recipe {label:"Rebounds",terms:vec![("goalie-rebound-ratio","Rebounds generated / expected rebounds","ratio",1.,true)],note:"MoneyPuck.com follow-up rebound attempts relative to its expected-rebound model. Includes team effects; not a direct rebound-placement assessment."},
        Recipe {label:"Recovery",terms:vec![("goalie-rebound-save","Rebound shots on goal saved","%",1.,false)],note:"MoneyPuck.com rebound-shot saving approximation (provider three-second rebound flag); not measured reset time."},
        Recipe {label:"Puck Play",terms:vec![("goalie-assists","NHL assists","/60",1.,false)],note:"NHL assist-production approximation. Rare events create many tied scores; this does not measure pass completion or every puck touch."},
    ];
    }
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
// Far finer than the source measurements: prevents unit-conversion rounding
// noise from splitting ties. Keep the original values in the displayed parts.
pub(super) fn rank_value(value: f64) -> f64 {
    let scaled = value * 1e9;
    if scaled.is_finite() {
        scaled.round() / 1e9
    } else {
        value
    }
}

/// Same-season F/D/G ranks; 10GP/100min reference, with scores for smaller samples
/// visibly marked. Missing terms use the available weights; no missing=zero.
pub fn build(season: u32, scope: &str, players: &[Player]) -> Result<Vec<Profile>, String> {
    let mut ids = BTreeSet::new();
    let mut rows = players.to_vec();
    for p in &mut rows {
        if p.player_id == 0
            || !ids.insert(p.player_id)
            || !["Forwards", "Defense", "Goalies"].contains(&p.group.as_str())
            || !p.minutes.is_finite()
            || p.minutes < 0.
            || p.metrics.values().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("Invalid or duplicate stat input".into());
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
        let scores = recipes(p.group == "Goalies")
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
                            .map(rank_value)
                            .collect::<Vec<_>>();
                        let value = p.metrics.get(key).copied();
                        let percentile =
                            value.filter(|_| active && reference.len() >= 20).map(|v| {
                                let v = rank_value(v);
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
            production_mix: None,
            penalty_balance: None,
            shot_method_sample: None,
            shot_methods: None,
            skating_detail: None,
            unit_links: None,
            under_pressure: None,
            estimated_record: None,
            skater_context:None,
            shot_context:None,
            schema: "rinkcraft.simple-stat-attributes.v1".into(),
            player_id: p.player_id,
            season,
            group: p.group.clone(),
            scope: scope.into(),
            games: p.games,
            minutes: p.minutes,
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
