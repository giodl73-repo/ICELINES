//! Transparent descriptions of measured history, never player-class inference.
use super::rinkcraft_history::{HistoryRow, HistorySeason};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MeasuredAssessment {
    pub schema: String,
    pub first_season: u32,
    pub last_season: u32,
    pub supplied_seasons: usize,
    pub reported_seasons: usize,
    pub shortlist: Vec<String>,
    pub repeated: Vec<RepeatedSignal>,
    pub repeated_reason: String,
    pub recent: Option<RecentChange>,
    pub recent_reason: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RepeatedSignal {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub seasons: usize,
    pub minimum_percentile: f64,
    pub maximum_percentile: f64,
    pub latest_percentile: f64,
    pub latest_value: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecentChange {
    pub key: String,
    pub label: String,
    pub unit: String,
    pub from_season: u32,
    pub to_season: u32,
    pub previous_value: f64,
    pub latest_value: f64,
    pub previous_percentile: f64,
    pub latest_percentile: f64,
    pub difference: f64,
    pub percentile_difference: f64,
}
pub fn build(seasons: &[HistorySeason], rows: &[HistoryRow]) -> MeasuredAssessment {
    let goalie = seasons.iter().rev().find_map(|s| s.group.as_deref()) == Some("Goalies");
    let shortlist = if goalie {
        vec!["gsax-per-60", "save-fraction", "goalie-shot-workload"]
    } else {
        vec![
            "primary-assists-per-60",
            "shots-per-60",
            "ixg-per-60",
            "contact-per-60",
        ]
    };
    let supplied = seasons.len();
    let reported = seasons.iter().filter(|s| s.state == "available").count();
    let mut repeated = Vec::new();
    if supplied >= 2 && reported == supplied {
        for (order, key) in shortlist.iter().enumerate() {
            let Some(row) = rows.iter().find(|r| r.key == *key) else {
                continue;
            };
            if row.ranked_seasons != supplied || row.upper_quintile_seasons != Some(supplied) {
                continue;
            }
            let Some(latest) = row.points.last().and_then(|p| p.measurement.as_ref()) else {
                continue;
            };
            let Some((latest_value, latest_percentile)) = latest.value.zip(latest.percentile)
            else {
                continue;
            };
            let ranks = row
                .points
                .iter()
                .filter_map(|p| {
                    p.measurement
                        .as_ref()
                        .filter(|m| m.state == "ranked" && m.value.is_some())
                        .and_then(|m| m.percentile)
                })
                .collect::<Vec<_>>();
            if ranks.len() != supplied {
                continue;
            }
            repeated.push((
                order,
                RepeatedSignal {
                    key: row.key.clone(),
                    label: row.label.clone(),
                    unit: latest.unit.clone(),
                    seasons: supplied,
                    minimum_percentile: ranks.iter().copied().fold(f64::INFINITY, f64::min),
                    maximum_percentile: ranks.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                    latest_percentile,
                    latest_value,
                },
            ));
        }
    }
    repeated.sort_by(|(a, x), (b, y)| {
        y.minimum_percentile
            .total_cmp(&x.minimum_percentile)
            .then(a.cmp(b))
    });
    let repeated = repeated
        .into_iter()
        .take(2)
        .map(|(_, s)| s)
        .collect::<Vec<_>>();
    let repeated_reason=if !repeated.is_empty(){""}else if supplied<2{"At least two supplied ranked seasons are needed."}else if reported<supplied{"Incomplete seasonal source coverage; repetition headline withheld."}else{"No fully comparable shortlist signal has ranked P >= 80 in every supplied season; missing or unranked values cannot qualify."}.into();
    let mut recent: Option<RecentChange> = None;
    let mut recent_reason =
        "Two consecutive supplied seasons with comparable ranked measurements are needed."
            .to_string();
    if seasons.len() >= 2 {
        let a = &seasons[seasons.len() - 2];
        let b = &seasons[seasons.len() - 1];
        if a.state != "available" || b.state != "available" {
            recent_reason =
                "Latest supplied pair has missing player evidence; no older pair is substituted."
                    .into();
        } else if a.group != b.group || a.situation != b.situation {
            recent_reason =
                "Latest pair changes position group or situation; rank-shift headline withheld."
                    .into();
        } else if a.season / 10000 + 1 == b.season / 10000 {
            for key in &shortlist {
                let Some(row) = rows.iter().find(|r| r.key == *key) else {
                    continue;
                };
                let x = row
                    .points
                    .iter()
                    .find(|p| p.season == a.season)
                    .and_then(|p| p.measurement.as_ref());
                let y = row
                    .points
                    .iter()
                    .find(|p| p.season == b.season)
                    .and_then(|p| p.measurement.as_ref());
                let Some((x, y)) = x.zip(y) else {
                    continue;
                };
                if x.state != "ranked"
                    || y.state != "ranked"
                    || x.unit != y.unit
                    || x.formula != y.formula
                {
                    continue;
                }
                let Some((
                    (previous_value, previous_percentile),
                    (latest_value, latest_percentile),
                )) = x.value.zip(x.percentile).zip(y.value.zip(y.percentile))
                else {
                    continue;
                };
                let delta = latest_percentile - previous_percentile;
                if recent
                    .as_ref()
                    .is_none_or(|v| delta.abs() > v.percentile_difference.abs())
                {
                    recent = Some(RecentChange {
                        key: row.key.clone(),
                        label: row.label.clone(),
                        unit: y.unit.clone(),
                        from_season: a.season,
                        to_season: b.season,
                        previous_value,
                        latest_value,
                        previous_percentile,
                        latest_percentile,
                        difference: latest_value - previous_value,
                        percentile_difference: delta,
                    });
                }
            }
        }
    }
    if recent.is_some() {
        recent_reason.clear();
    }
    MeasuredAssessment {
        schema: "icelines.rinkcraft-measured-assessment.v1".into(),
        first_season: seasons.first().map(|s| s.season).unwrap_or_default(),
        last_season: seasons.last().map(|s| s.season).unwrap_or_default(),
        supplied_seasons: supplied,
        reported_seasons: reported,
        shortlist: shortlist.into_iter().map(str::to_owned).collect(),
        repeated,
        repeated_reason,
        recent,
        recent_reason,
    }
}

#[cfg(test)]
mod tests {
    use crate::view_model::{
        rinkcraft_history::{NumericalHistory, SCHEMA},
        rinkcraft_numbers::{self, InputRow},
    };
    fn history() -> NumericalHistory {
        let rows = (1..=25)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: "Forwards".into(),
                games: 30,
                seconds: 30000.,
                values: [
                    ("primary_assists".into(), id as f64),
                    ("shots".into(), id as f64),
                ]
                .into_iter()
                .collect(),
            })
            .collect::<Vec<_>>();
        NumericalHistory {
            schema: SCHEMA.into(),
            catalogs: [20232024, 20242025, 20252026]
                .into_iter()
                .map(|s| rinkcraft_numbers::build(&rows, s, &[], vec![]).unwrap())
                .collect(),
        }
    }
    #[test]
    fn assessment_ties_are_deterministic_and_time_bounds_precede_selection() {
        let mut h = history();
        let a = h.card(25, None, None).unwrap().unwrap().assessment.unwrap();
        assert_eq!(a.repeated[0].key, "primary-assists-per-60");
        assert_eq!(a.repeated.len(), 2);
        let m = h.catalogs[2]
            .profiles
            .iter_mut()
            .find(|p| p.player_id == 25)
            .unwrap()
            .metrics
            .iter_mut()
            .find(|m| m.key == "primary-assists-per-60")
            .unwrap();
        m.value = Some(0.12);
        m.numerator = Some(1.);
        m.percentile = Some(4.);
        // This deliberately supplied change exercises descriptive selection, not ranking math.
        let a = h.card(25, None, None).unwrap().unwrap().assessment.unwrap();
        assert_eq!(a.repeated.len(), 1);
        assert_eq!(a.repeated[0].key, "shots-per-60");
        let change = a.recent.unwrap();
        assert_eq!(change.key, "primary-assists-per-60");
        assert_eq!(change.percentile_difference, -94.);
        assert!((change.difference + 2.88).abs() < 1e-10);
        let past = h
            .card(25, None, Some(20242025))
            .unwrap()
            .unwrap()
            .assessment
            .unwrap();
        assert_eq!(past.repeated.len(), 2);
        assert_eq!(past.last_season, 20242025);
    }
    #[test]
    fn assessment_missing_latest_and_nonconsecutive_pairs_do_not_substitute_old_changes() {
        let mut h = history();
        h.catalogs[2].profiles.retain(|p| p.player_id != 25);
        let a = h.card(25, None, None).unwrap().unwrap().assessment.unwrap();
        assert_eq!(a.reported_seasons, 2);
        assert!(a.repeated.is_empty());
        assert!(a.recent.is_none());
        assert!(a.recent_reason.contains("missing"));
        let mut h = history();
        h.catalogs.remove(1);
        assert!(h
            .card(25, None, None)
            .unwrap()
            .unwrap()
            .assessment
            .unwrap()
            .recent
            .is_none());
        let h = history();
        let a = h
            .card(25, Some(20252026), None)
            .unwrap()
            .unwrap()
            .assessment
            .unwrap();
        assert!(a.repeated.is_empty());
        assert!(a.recent.is_none());
    }
    #[test]
    fn assessment_includes_exact_p80_and_withholds_incompatible_latest_groups() {
        let rows = (1..=20)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: "Forwards".into(),
                games: 30,
                seconds: 30000.,
                values: [
                    (
                        "primary_assists".into(),
                        if id == 17 { 16. } else { id as f64 },
                    ),
                    ("shots".into(), id as f64),
                ]
                .into_iter()
                .collect(),
            })
            .collect::<Vec<_>>();
        let mut h = NumericalHistory {
            schema: SCHEMA.into(),
            catalogs: [20232024, 20242025, 20252026]
                .into_iter()
                .map(|s| rinkcraft_numbers::build(&rows, s, &[], vec![]).unwrap())
                .collect(),
        };
        let a = h.card(17, None, None).unwrap().unwrap().assessment.unwrap();
        assert_eq!(
            a.repeated
                .iter()
                .find(|s| s.key == "primary-assists-per-60")
                .unwrap()
                .minimum_percentile,
            80.
        );
        h.catalogs[2]
            .profiles
            .iter_mut()
            .find(|p| p.player_id == 17)
            .unwrap()
            .group = "Defense".into();
        let a = h.card(17, None, None).unwrap().unwrap().assessment.unwrap();
        assert!(a.repeated.is_empty());
        assert!(a.recent.is_none());
        assert!(a.recent_reason.contains("group"));
    }
}
