//! Descriptive EDGE exposure comparisons, never capability grades or feat awards.
use super::rinkcraft_edge::EdgeEvidence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExposureRecord {
    pub player_id: u32,
    pub season: u32,
    pub name: String,
    pub group: String,
    pub games: u32,
    pub minutes: f64,
    pub peak_mph: Option<f64>,
    pub bursts: Option<f64>,
    pub bursts_per_game: Option<f64>,
    pub bursts_per_60: Option<f64>,
    pub edge_sha256: String,
}

pub fn record(edge: &EdgeEvidence, name: &str, minutes: f64) -> Result<ExposureRecord, String> {
    if edge.schema != "icelines.rinkcraft-edge.v1"
        || edge.strength_scope != "all strengths"
        || edge.season_type != "regular"
        || !matches!(edge.group.as_str(), "Forwards" | "Defense")
        || edge.games == 0
        || !minutes.is_finite()
        || minutes <= 0.
    {
        return Err("positive skater GP and all-strength minutes required".into());
    }
    let get = |key: &str, unit: &str| -> Result<Option<f64>, String> {
        let found: Vec<_> = edge.measures.iter().filter(|m| m.key == key).collect();
        if found.len() > 1 {
            return Err("duplicate EDGE measure".into());
        }
        match found.first() {
            Some(m) if m.unit == unit && m.value.is_finite() && m.value >= 0. => Ok(Some(m.value)),
            Some(_) => Err("invalid EDGE measurement/unit".into()),
            None => Ok(None),
        }
    };
    let peak_mph = get("max-skating-speed", "Mph")?;
    let bursts = get("speed-bursts-over-20", "Count")?;
    if peak_mph == Some(0.) || bursts.is_some_and(|x| x.fract() != 0.) {
        return Err("invalid physical maximum or burst count".into());
    }
    let bursts_per_game = bursts.map(|n| n / f64::from(edge.games));
    let bursts_per_60 = bursts.map(|n| n * 60. / minutes);
    if bursts_per_60.is_some_and(|x| !x.is_finite()) {
        return Err("nonfinite rate".into());
    }
    Ok(ExposureRecord {
        player_id: edge.player_id,
        season: edge.season,
        name: name.into(),
        group: edge.group.clone(),
        games: edge.games,
        minutes,
        peak_mph,
        bursts,
        bursts_per_game,
        bursts_per_60,
        edge_sha256: edge.source.sha256.clone(),
    })
}

#[derive(Debug, Serialize)]
pub struct Comparison {
    pub metric: String,
    pub value: f64,
    /// IceLines within-season positional midrank, 0..100; not NHL's percentile.
    pub percentile: f64,
    pub peers: usize,
}

#[derive(Debug, Serialize)]
pub struct RankedRecord {
    pub record: ExposureRecord,
    pub eligible: bool,
    pub comparisons: Vec<Comparison>,
}

/// Minimum exposure gates are authored sensitivity settings, not reliability claims.
/// Missing values have no rank. An ineligible player never receives a comparison.
pub fn compare(
    records: &[ExposureRecord],
    min_games: u32,
    min_minutes: f64,
) -> Result<Vec<RankedRecord>, String> {
    if min_games == 0 || !min_minutes.is_finite() || min_minutes <= 0. {
        return Err("positive exposure thresholds required".into());
    }
    let eligible = |r: &ExposureRecord| r.games >= min_games && r.minutes >= min_minutes;
    let metrics: [(&str, fn(&ExposureRecord) -> Option<f64>); 3] = [
        ("peak_mph", |r| r.peak_mph),
        ("bursts", |r| r.bursts),
        ("bursts_per_60", |r| r.bursts_per_60),
    ];
    let mut unique = std::collections::BTreeSet::new();
    for r in records {
        if !unique.insert((r.season, r.player_id))
            || !matches!(r.group.as_str(), "Forwards" | "Defense")
            || !r.minutes.is_finite()
            || r.minutes <= 0.
            || r.games == 0
            || [r.peak_mph, r.bursts, r.bursts_per_60, r.bursts_per_game]
                .iter()
                .flatten()
                .any(|x| !x.is_finite() || *x < 0.)
        {
            return Err("duplicate or invalid exposure record".into());
        }
    }
    Ok(records
        .iter()
        .map(|r| {
            let mut comparisons = Vec::new();
            if eligible(r) {
                for (key, getter) in metrics {
                    if let Some(value) = getter(r) {
                        let peers: Vec<_> = records
                            .iter()
                            .filter(|p| p.season == r.season && p.group == r.group && eligible(p))
                            .filter_map(getter)
                            .collect();
                        let below = peers.iter().filter(|p| **p < value).count();
                        let tied = peers.iter().filter(|p| **p == value).count();
                        comparisons.push(Comparison {
                            metric: key.into(),
                            value,
                            percentile: 100. * (below as f64 + 0.5 * tied as f64)
                                / peers.len() as f64,
                            peers: peers.len(),
                        });
                    }
                }
            }
            RankedRecord {
                record: r.clone(),
                eligible: eligible(r),
                comparisons,
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_measurements_produce_rates_without_inventing_missing_speed() {
        use super::super::rinkcraft_edge::{EdgeMeasure, EdgeSource};
        let mut edge = EdgeEvidence {
            schema: "icelines.rinkcraft-edge.v1".into(),
            player_id: 1,
            season: 20252026,
            season_type: "regular".into(),
            group: "Forwards".into(),
            games: 20,
            captured_at: String::new(),
            strength_scope: "all strengths".into(),
            source: EdgeSource {
                url: String::new(),
                sha256: String::new(),
                bytes: 1,
            },
            measures: vec![EdgeMeasure {
                key: "speed-bursts-over-20".into(),
                label: String::new(),
                value: 100.,
                unit: "Count".into(),
                provider_percentile: None,
                provider_average: None,
                source_field: String::new(),
                attribute_id: None,
                component_id: None,
                interpretation: String::new(),
            }],
            locations: vec![],
            notes: vec![],
        };
        let r = record(&edge, "Test", 200.).unwrap();
        assert_eq!(r.bursts_per_game, Some(5.));
        assert_eq!(r.bursts_per_60, Some(30.));
        assert_eq!(r.peak_mph, None);
        assert!(record(&edge, "Test", 0.).is_err());
        edge.measures[0].value = 0.;
        assert_eq!(record(&edge, "Test", 200.).unwrap().bursts_per_60, Some(0.));
        edge.measures[0].value = 0.5;
        assert!(record(&edge, "Test", 200.).is_err());
        edge.measures[0].value = 1.;
        edge.measures.push(edge.measures[0].clone());
        assert!(record(&edge, "Test", 200.).is_err());
        edge.measures.pop();
        edge.group = "Goalies".into();
        assert!(record(&edge, "Test", 200.).is_err());
    }
    fn sample(
        id: u32,
        group: &str,
        games: u32,
        burst: Option<f64>,
        minutes: f64,
    ) -> ExposureRecord {
        ExposureRecord {
            player_id: id,
            season: 20252026,
            name: String::new(),
            group: group.into(),
            games,
            minutes,
            peak_mph: Some(22.),
            bursts: burst,
            bursts_per_game: burst.map(|n| n / f64::from(games)),
            bursts_per_60: burst.map(|n| n * 60. / minutes),
            edge_sha256: String::new(),
        }
    }
    #[test]
    fn exposure_changes_order_and_ties_use_midrank() {
        let rows = [
            sample(1, "Forwards", 20, Some(100.), 200.),
            sample(2, "Forwards", 20, Some(150.), 600.),
            sample(3, "Defense", 20, Some(999.), 400.),
            sample(4, "Forwards", 2, Some(200.), 40.),
        ];
        let result = compare(&rows, 10, 100.).unwrap();
        assert_eq!(result[0].comparisons[0].percentile, 50.);
        assert_eq!(result[0].comparisons[1].percentile, 25.);
        assert_eq!(result[0].comparisons[2].percentile, 75.);
        assert_eq!(result[0].comparisons[2].peers, 2);
        assert!(result[3].comparisons.is_empty());
    }
    #[test]
    fn zero_missing_and_bad_inputs_are_distinct() {
        let a = sample(1, "Forwards", 20, Some(0.), 200.);
        let b = sample(2, "Forwards", 20, None, 200.);
        let result = compare(&[a.clone(), b], 10, 100.).unwrap();
        assert_eq!(result[0].comparisons[2].value, 0.);
        assert_eq!(result[0].comparisons[2].peers, 1);
        assert_eq!(result[1].comparisons.len(), 1);
        assert!(compare(&[a.clone(), a], 10, 100.).is_err());
        assert!(compare(&[], 0, 100.).is_err());
        assert!(compare(&[], 10, f64::NAN).is_err());
    }
}
