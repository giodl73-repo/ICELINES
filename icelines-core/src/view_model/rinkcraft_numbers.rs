//! Experimental numerical evidence vectors. No quote inputs or class thresholds.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA: &str = "icelines.rinkcraft-numerical.v1";
pub const PROTOCOL: &str = "rinkcraft.numerical-evidence.0.1.0-experimental";
#[derive(Debug, Clone)]
pub struct InputRow {
    pub player_id: u32,
    pub name: String,
    pub group: String,
    pub games: u32,
    pub seconds: f64,
    pub values: BTreeMap<String, f64>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub url: String,
    pub sha256: String,
    pub bytes: usize,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericalCatalog {
    pub schema: String,
    pub protocol: String,
    pub season: u32,
    pub competition: String,
    pub source_credit: String,
    pub sources: Vec<Source>,
    pub profiles: Vec<NumericalProfile>,
    pub missing_players: Vec<u32>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericalProfile {
    #[serde(default)]
    pub style_evidence: Option<super::rinkcraft_style::StyleEvidence>,
    pub player_id: u32,
    pub name: String,
    pub group: String,
    pub situation: String,
    pub games: u32,
    pub minutes: f64,
    pub minimum_minutes: f64,
    pub eligible_peers: usize,
    pub metrics: Vec<Measurement>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Measurement {
    pub key: String,
    pub label: String,
    pub domain: String,
    pub interpretation: String,
    pub unit: String,
    pub value: Option<f64>,
    pub percentile: Option<f64>,
    pub numerator: Option<f64>,
    pub denominator: Option<f64>,
    pub scale: f64,
    pub measured_peers: usize,
    pub reference_sha256: String,
    pub state: String,
    pub formula: String,
}
struct Definition {
    key: &'static str,
    label: &'static str,
    domain: &'static str,
    a: &'static str,
    subtract: Option<&'static str>,
    denominator: &'static str,
    scale: f64,
    unit: &'static str,
    interpretation: &'static str,
}
fn definitions(goalie: bool) -> Vec<Definition> {
    let rate = |key, label, domain, a, interpretation| Definition {
        key,
        label,
        domain,
        a,
        subtract: None,
        denominator: "seconds",
        scale: 3600.,
        unit: "Per60",
        interpretation,
    };
    if goalie {
        return vec![
            Definition{key:"gsax-per-60",label:"Goals saved above expected / 60",domain:"Saving outcomes",a:"xga",subtract:Some("goals_against"),denominator:"seconds",scale:3600.,unit:"Per60",interpretation:"Model-relative observed saving; positive means fewer goals conceded than expected. Not a stable talent estimate."},
            Definition{key:"gsax",label:"Goals saved above expected",domain:"Saving outcomes",a:"xga",subtract:Some("goals_against"),denominator:"one",scale:1.,unit:"Goals",interpretation:"Observed cumulative saving versus this provider model; depends on workload."},
            Definition{key:"save-fraction",label:"Observed save percentage",domain:"Saving outcomes",a:"shots_against",subtract:Some("goals_against"),denominator:"shots_against",scale:1.,unit:"Pct",interpretation:"Observed saves / shots on goal faced. Unadjusted outcome."},
            rate("goalie-threat-rate","Expected goals against / 60","Threat environment","xga","Model-estimated threat faced; not a goalie positioning or ability grade."),
            rate("goalie-shot-workload","Shots on goal faced / 60","Threat environment","shots_against","Shot workload per actual goalie ice time; not save quality."),
        ];
    }
    vec![
        rate("primary-assists-per-60","Primary assists / 60","Passing outcomes","primary_assists","Recorded primary-assist output; does not count all dangerous passes or failed attempts."),
        rate("secondary-assists-per-60","Secondary assists / 60","Passing outcomes","secondary_assists","Recorded secondary-assist output; not independent creation credit."),
        rate("shots-per-60","Shots on goal / 60","Shooting behavior","shots","Recorded shooting involvement per ice time."),
        rate("ixg-per-60","Individual expected goals / 60","Shooting behavior","ixg","Model-estimated shot opportunity volume; not finishing talent."),
        Definition{key:"shot-quality",label:"Expected goals / unblocked attempt",domain:"Shooting behavior",a:"ixg",subtract:None,denominator:"unblocked_shots",scale:1.,unit:"Pct",interpretation:"Average modeled scoring probability of unblocked attempts, including misses. Not observed conversion."},
        rate("high-danger-per-60","High-danger attempts / 60","Shooting behavior","high_danger_shots","Provider-defined high-danger unblocked attempts; not a screen or net-front skill measure."),
        rate("goals-per-60","Goals / 60","Scoring outcomes","goals","Recorded goal output per ice time."),
        Definition{key:"goals-above-xg-per-60",label:"Goals minus expected goals / 60",domain:"Scoring outcomes",a:"goals",subtract:Some("ixg"),denominator:"seconds",scale:3600.,unit:"Per60",interpretation:"Observed residual against a model; one season does not establish persistent finishing talent."},
        rate("contact-per-60","Recorded hits / 60","Recorded contests","hits","Recorded contact frequency; not pressure, puck recovery or contact success."),
        rate("takeaways-per-60","Recorded takeaways / 60","Recorded contests","takeaways","Recorded takeaway frequency; incomplete proxy for possession acquisition."),
        rate("giveaways-per-60","Recorded giveaways / 60","Recorded puck losses","giveaways","Recorded giveaway frequency per ice time; not normalized by touches or possession demand."),
        rate("penalties-drawn-per-60","Penalties drawn / 60","Penalty outcomes","penalties_drawn","Recorded penalties drawn per ice time; does not identify the mechanism."),
        rate("penalties-taken-per-60","Penalties taken / 60","Penalty outcomes","penalties_taken","Recorded penalties taken per ice time; lower frequency is not a complete discipline or defensive grade."),
        rate("on-ice-xgf-per-60","On-ice expected goals for / 60","On-ice environment","on_ice_xgf","Team attacking threat while on ice; includes teammates, opponents and deployment."),
        rate("on-ice-xga-per-60","On-ice expected goals against / 60","On-ice environment","on_ice_xga","Team threat conceded while on ice; lower is not proof of individual defensive skill."),
    ]
}
fn parts(row: &InputRow, d: &Definition) -> (Option<f64>, Option<f64>, Option<f64>) {
    let get = |k: &str| match k {
        "seconds" => Some(row.seconds),
        "one" => Some(1.),
        _ => row.values.get(k).copied(),
    };
    let n = get(d.a).and_then(|a| match d.subtract {
        Some(b) => get(b).map(|b| a - b),
        None => Some(a),
    });
    let den = get(d.denominator);
    let value = n
        .zip(den)
        .filter(|(_, d)| *d > 0.)
        .map(|(n, den)| n / den * d.scale)
        .filter(|v| v.is_finite());
    (n, den, value)
}
pub fn build(
    rows: &[InputRow],
    season: u32,
    selected: &[u32],
    sources: Vec<Source>,
) -> Result<NumericalCatalog, String> {
    let mut ids = BTreeSet::new();
    for r in rows {
        if !ids.insert(r.player_id)
            || r.player_id == 0
            || !matches!(r.group.as_str(), "Forwards" | "Defense" | "Goalies")
            || !r.seconds.is_finite()
            || r.seconds < 0.
            || r.values.values().any(|v| !v.is_finite() || *v < 0.)
        {
            return Err("invalid or duplicate numerical input row".into());
        }
    }
    let mut profiles = Vec::new();
    let targets: BTreeSet<_> = selected.iter().copied().collect();
    for row in rows
        .iter()
        .filter(|r| targets.is_empty() || targets.contains(&r.player_id))
    {
        let goalie = row.group == "Goalies";
        let minimum_minutes = if goalie { 600. } else { 300. };
        let minimum_games = if goalie { 5 } else { 10 };
        let minimum_peers = if goalie { 10 } else { 20 };
        let eligible: Vec<_> = rows
            .iter()
            .filter(|r| {
                r.group == row.group
                    && r.seconds >= minimum_minutes * 60.
                    && r.games >= minimum_games
            })
            .collect();
        let metrics = definitions(goalie)
            .iter()
            .map(|d| {
                let (numerator, denominator, value) = parts(row, d);
                let mut reference: Vec<_> = eligible
                    .iter()
                    .filter_map(|r| parts(r, d).2.map(|v| (r.player_id, v)))
                    .collect();
                reference.sort_by_key(|(id, _)| *id);
                let state = if value.is_none() {
                    "unavailable"
                } else if row.seconds < minimum_minutes * 60. || row.games < minimum_games {
                    "insufficient_exposure"
                } else if reference.len() < minimum_peers {
                    "insufficient_reference"
                } else {
                    "ranked"
                };
                let percentile = value.filter(|_| state == "ranked").map(|v| {
                    100. * (reference.iter().filter(|(_, x)| *x < v).count() as f64
                        + 0.5 * reference.iter().filter(|(_, x)| *x == v).count() as f64)
                        / reference.len() as f64
                });
                let mut hash = Sha256::new();
                hash.update(PROTOCOL);
                hash.update(season.to_le_bytes());
                hash.update(row.group.as_bytes());
                hash.update(d.key);
                for (id, v) in &reference {
                    hash.update(id.to_le_bytes());
                    hash.update(v.to_bits().to_le_bytes());
                }
                Measurement {
                    key: d.key.into(),
                    label: d.label.into(),
                    domain: d.domain.into(),
                    interpretation: d.interpretation.into(),
                    unit: d.unit.into(),
                    value,
                    percentile,
                    numerator,
                    denominator,
                    scale: d.scale,
                    measured_peers: reference.len(),
                    reference_sha256: format!("{:x}", hash.finalize()),
                    state: state.into(),
                    formula: format!(
                        "({}{}) / {} * {}",
                        d.a,
                        d.subtract.map(|s| format!(" - {s}")).unwrap_or_default(),
                        d.denominator,
                        d.scale
                    ),
                }
            })
            .collect();
        profiles.push(NumericalProfile {
            style_evidence: None,
            player_id: row.player_id,
            name: row.name.clone(),
            group: row.group.clone(),
            situation: if goalie { "all" } else { "5on5" }.into(),
            games: row.games,
            minutes: row.seconds / 60.,
            minimum_minutes,
            eligible_peers: eligible.len(),
            metrics,
        });
    }
    profiles.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(NumericalCatalog{schema:SCHEMA.into(),protocol:PROTOCOL.into(),season,competition:"regular".into(),source_credit:"MoneyPuck.com — noncommercial research; provider model and event definitions apply.".into(),sources,profiles,missing_players:targets.difference(&ids).copied().collect(),limitations:vec![
        "Experimental numerical evidence, without quote inputs, composite ability scores or automatic class assignments.".into(),
        "Skaters: five-on-five; goalies: all strengths. Percentiles use same-season Forwards, Defense or Goalies, never just the showcase.".into(),
        "P means raw-value midrank percentile: a higher number means a larger value, not necessarily better performance.".into(),
        "Eligibility: skaters 300 minutes and 10 GP; goalies 600 minutes and 5 GP; at least 20/10 measured peers. These exploratory filters are not reliability thresholds.".into(),
        "On-ice outcomes are unadjusted team context. No teammate/opponent adjustment, uncertainty interval or persistent-talent estimate.".into(),
        "Passing choices, controlled entries/exits, dump/chase choices, direction, pressure, screens, assignments and goalie technique remain unmeasured.".into(),
    ]})
}

/// Presentation projection over already computed measures; no new hockey score.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCard {
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub finishing_research: Option<super::rinkcraft_finishing_evidence::Readout>,
    #[serde(default)]
    pub context_evidence: Option<super::rinkcraft_context::ContextEvidence>,
    #[serde(default)]
    pub feature_context: Vec<super::rinkcraft_attribute_evidence::FeatureContext>,
    #[serde(default)]
    pub style_evidence: Option<super::rinkcraft_style::StyleEvidence>,
    pub schema: String,
    pub season: u32,
    pub player_id: u32,
    pub name: String,
    pub group: String,
    pub situation: String,
    pub games: u32,
    pub minutes: f64,
    pub eligible_peers: usize,
    pub facets: Vec<EvidenceFacet>,
    pub leading: Vec<String>,
    pub metrics: Vec<Measurement>,
    pub source_credit: String,
    pub sources: Vec<Source>,
    pub unknowns: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceFacet {
    #[serde(default)]
    pub explanation: Option<super::rinkcraft_attribute_evidence::AttributeExplanation>,
    pub label: String,
    pub measurement: Measurement,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericalComparison {
    #[serde(default)]
    pub contrast: Option<super::rinkcraft_contrast::ContrastSummary>,
    #[serde(default)]
    pub style: Option<super::rinkcraft_style::StyleComparison>,
    pub season: u32,
    pub left: EvidenceCard,
    pub right: EvidenceCard,
    pub rows: Vec<ComparisonRow>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonRow {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub left: Option<f64>,
    pub right: Option<f64>,
    pub left_percentile: Option<f64>,
    pub right_percentile: Option<f64>,
    pub difference: Option<f64>,
    pub percentile_difference: Option<f64>,
    pub interpretation: String,
}
pub(crate) fn primary_facets(goalie: bool) -> &'static [(&'static str, &'static str)] {
    if goalie {
        &[
            ("gsax-per-60", "Saving vs model"),
            ("save-fraction", "Save outcome"),
            ("goalie-threat-rate", "Threat faced"),
            ("goalie-shot-workload", "Shot workload"),
        ]
    } else {
        &[
            ("primary-assists-per-60", "Primary-assist output"),
            ("shots-per-60", "Shot volume"),
            ("ixg-per-60", "Shot-opportunity volume"),
            ("shot-quality", "Average shot quality"),
            ("contact-per-60", "Recorded contact"),
            ("goals-above-xg-per-60", "Observed conversion residual"),
        ]
    }
}
impl NumericalCatalog {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA || self.protocol != PROTOCOL || self.competition != "regular" {
            return Err("unsupported numerical catalog".into());
        }
        let year = self.season / 10000;
        if !(1900..=2200).contains(&year) || self.season % 10000 != year + 1 {
            return Err("invalid numerical season".into());
        }
        let mut ids = BTreeSet::new();
        for source in &self.sources {
            if !source.url.starts_with("https://")
                || source.sha256.len() != 64
                || !source.sha256.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Err("invalid numerical source".into());
            }
        }
        for p in &self.profiles {
            if let Some(s) = &p.style_evidence {
                super::rinkcraft_style::validate(s, self.season)?;
            }
            if !ids.insert(p.player_id)
                || p.player_id == 0
                || !matches!(p.group.as_str(), "Forwards" | "Defense" | "Goalies")
                || !p.minutes.is_finite()
                || p.minutes < 0.
                || p.situation != if p.group == "Goalies" { "all" } else { "5on5" }
            {
                return Err("invalid numerical profile".into());
            }
            let mut keys = BTreeSet::new();
            for m in &p.metrics {
                if !keys.insert(&m.key)
                    || [m.value, m.percentile, m.numerator, m.denominator]
                        .into_iter()
                        .flatten()
                        .any(|v| !v.is_finite())
                    || m.percentile.is_some_and(|v| !(0.0..=100.0).contains(&v))
                    || m.measured_peers > p.eligible_peers
                {
                    return Err("invalid numerical measurement".into());
                }
            }
        }
        Ok(())
    }
    /// Latest supplied snapshot must not be later than the caller's active window.
    pub fn card(&self, pid: u32, ceiling: Option<u32>) -> Result<Option<EvidenceCard>, String> {
        self.validate()?;
        if ceiling.is_some_and(|s| self.season > s) {
            return Ok(None);
        }
        let Some(p) = self.profiles.iter().find(|p| p.player_id == pid) else {
            return Ok(None);
        };
        let keys = primary_facets(p.group == "Goalies");
        let facets = keys
            .iter()
            .filter_map(|(key, label)| {
                p.metrics
                    .iter()
                    .find(|m| m.key == *key)
                    .map(|m| EvidenceFacet {
                        explanation: super::rinkcraft_attribute_evidence::explanation(key),
                        label: (*label).into(),
                        measurement: m.clone(),
                    })
            })
            .collect();
        let leader_keys = if p.group == "Goalies" {
            vec!["gsax-per-60"]
        } else {
            vec![
                "primary-assists-per-60",
                "shots-per-60",
                "ixg-per-60",
                "contact-per-60",
            ]
        };
        let mut leaders: Vec<_> = leader_keys
            .iter()
            .enumerate()
            .filter_map(|(i, key)| {
                p.metrics
                    .iter()
                    .find(|m| m.key == *key && m.state == "ranked")
                    .and_then(|m| m.percentile.map(|rank| (i, *key, rank)))
            })
            .collect();
        leaders.sort_by(|a, b| b.2.total_cmp(&a.2).then(a.0.cmp(&b.0)));
        let leading = leaders
            .into_iter()
            .take(2)
            .map(|(_, key, _)| key.to_string())
            .collect();
        Ok(Some(EvidenceCard {
            finishing_research: None,
            context_evidence: Some(super::rinkcraft_context::build(p)),
            feature_context: super::rinkcraft_attribute_evidence::feature_context(&p.metrics),
            style_evidence: p.style_evidence.clone(),
            schema: "icelines.rinkcraft-evidence-card.v1".into(),
            season: self.season,
            player_id: p.player_id,
            name: p.name.clone(),
            group: p.group.clone(),
            situation: p.situation.clone(),
            games: p.games,
            minutes: p.minutes,
            eligible_peers: p.eligible_peers,
            facets,
            leading,
            metrics: p.metrics.clone(),
            source_credit: self.source_credit.clone(),
            sources: self.sources.clone(),
            unknowns: if p.group == "Goalies" {
                vec![
                    "Depth, movement and recovery mechanics",
                    "Rebound decisions and puck exchanges",
                ]
            } else {
                vec![
                    "Controlled entries/exits and carry/pass/dump choice",
                    "Direction, passing deception and off-puck options",
                    "Coverage duties and teammate relationships",
                ]
            }
            .into_iter()
            .map(str::to_string)
            .collect(),
        }))
    }
    pub fn compare(&self, left: u32, right: u32) -> Result<NumericalComparison, String> {
        if left == right {
            return Err("choose two different players".into());
        }
        let left = self
            .card(left, None)?
            .ok_or("left player has no numerical snapshot")?;
        let right = self
            .card(right, None)?
            .ok_or("right player has no numerical snapshot")?;
        if left.group != right.group || left.situation != right.situation {
            return Err("compare players in the same position group and situation".into());
        }
        let mut rows = Vec::new();
        for a in &left.metrics {
            let Some(b) = right.metrics.iter().find(|m| m.key == a.key) else {
                continue;
            };
            if a.unit != b.unit
                || a.reference_sha256 != b.reference_sha256
                || a.formula != b.formula
                || a.scale != b.scale
            {
                return Err("comparison requires the same definition, scale, units and reference population".into());
            }
            rows.push(ComparisonRow {
                key: a.key.clone(),
                label: a.label.clone(),
                unit: a.unit.clone(),
                left: a.value,
                right: b.value,
                left_percentile: a.percentile,
                right_percentile: b.percentile,
                difference: a.value.zip(b.value).map(|(a, b)| a - b),
                percentile_difference: a.percentile.zip(b.percentile).map(|(a, b)| a - b),
                interpretation: a.interpretation.clone(),
            });
        }
        Ok(NumericalComparison {
            contrast: Some(super::rinkcraft_contrast::build(&left, &right, &rows)),
            style: super::rinkcraft_style::compare(&left, &right),
            season: self.season,
            left,
            right,
            rows,
        })
    }
}
pub fn parse_catalog(text: &str) -> Result<NumericalCatalog, String> {
    if text.len() > 2 * 1024 * 1024 {
        return Err("numerical catalog exceeds 2 MiB".into());
    }
    let c: NumericalCatalog = serde_json::from_str(text).map_err(|e| e.to_string())?;
    c.validate()?;
    Ok(c)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn numerical_card_and_comparison_keep_scope_and_do_not_create_ability_scores() {
        let c = build(&rows(), 20242025, &[], vec![]).unwrap();
        let card = c.card(25, Some(20252026)).unwrap().unwrap();
        assert_eq!(card.leading[0], "primary-assists-per-60");
        assert_eq!(card.facets.len(), 6);
        assert!(c.card(25, Some(20232024)).unwrap().is_none());
        assert!(c.card(999, None).unwrap().is_none());
        let comparison = c.compare(25, 1).unwrap();
        assert_eq!(comparison.rows[0].difference, Some(3.6));
        assert_eq!(comparison.rows[0].percentile_difference, Some(96.));
        assert!(c.compare(1, 1).is_err());
        let mut other = c.clone();
        other.profiles[0].metrics[0].reference_sha256 = "different".into();
        assert!(other.compare(25, 1).is_err());
        let mut invalid = c;
        invalid.profiles[0].metrics[0].percentile = Some(101.);
        assert!(invalid.validate().is_err());
    }
    fn rows() -> Vec<InputRow> {
        (1..=25)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Player {id}"),
                group: "Forwards".into(),
                games: 20,
                seconds: 24000.,
                values: BTreeMap::from([
                    ("primary_assists".into(), id as f64),
                    ("shots".into(), 10.),
                    ("goals".into(), 0.),
                    ("ixg".into(), 2.),
                ]),
            })
            .collect()
    }
    #[test]
    fn numerical_rates_rank_league_peers_and_preserve_negative_residuals_and_missingness() {
        let c = build(&rows(), 20242025, &[25], vec![]).unwrap();
        let p = &c.profiles[0];
        assert_eq!(p.eligible_peers, 25);
        let m = &p.metrics[0];
        assert_eq!(m.value, Some(3.75));
        assert_eq!(m.percentile, Some(98.));
        assert_eq!(m.numerator, Some(25.));
        assert_eq!(m.denominator, Some(24000.));
        assert!(
            p.metrics
                .iter()
                .find(|m| m.key == "goals-above-xg-per-60")
                .unwrap()
                .value
                .unwrap()
                < 0.
        );
        assert_eq!(
            p.metrics
                .iter()
                .find(|m| m.key == "contact-per-60")
                .unwrap()
                .value,
            None
        );
        assert_eq!(
            p.metrics
                .iter()
                .find(|m| m.key == "shots-per-60")
                .unwrap()
                .percentile,
            Some(50.)
        );
    }
    #[test]
    fn numerical_inputs_reject_duplicates_and_keep_sparse_exposure_unranked() {
        let mut r = rows();
        r.push(r[0].clone());
        assert!(build(&r, 20242025, &[], vec![]).is_err());
        r.pop();
        r[0].seconds = 0.;
        let c = build(&r, 20242025, &[1, 999], vec![]).unwrap();
        assert_eq!(c.missing_players, vec![999]);
        assert_eq!(c.profiles[0].metrics[0].state, "unavailable");
        r[0].seconds = 100.;
        let c = build(&r, 20242025, &[1], vec![]).unwrap();
        assert_eq!(c.profiles[0].metrics[0].state, "insufficient_exposure");
        assert_eq!(c.profiles[0].metrics[0].percentile, None);
    }
    #[test]
    fn numerical_goalie_uses_model_residual_and_actual_ice_time() {
        let r = InputRow {
            player_id: 1,
            name: "Synthetic Goalie".into(),
            group: "Goalies".into(),
            games: 20,
            seconds: 72000.,
            values: BTreeMap::from([
                ("xga".into(), 42.),
                ("goals_against".into(), 40.),
                ("shots_against".into(), 500.),
            ]),
        };
        let c = build(&[r], 20242025, &[], vec![]).unwrap();
        let p = &c.profiles[0];
        assert_eq!(p.situation, "all");
        assert_eq!(p.metrics[0].value, Some(0.1));
        assert_eq!(p.metrics[1].value, Some(2.));
        assert_eq!(p.metrics[2].value, Some(0.92));
        assert_eq!(p.metrics[0].state, "insufficient_reference");
    }
}
