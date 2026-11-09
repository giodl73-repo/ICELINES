//! Season-specific evidence, not a pooled ability or persistence estimate.
use super::rinkcraft_numbers::{Measurement, NumericalCatalog, Source};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const SCHEMA: &str = "icelines.rinkcraft-numerical-history.v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericalHistory {
    pub schema: String,
    pub catalogs: Vec<NumericalCatalog>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryCard {
    #[serde(default)]
    pub assessment: Option<super::rinkcraft_assessment::MeasuredAssessment>,
    pub schema: String,
    pub player_id: u32,
    pub name: String,
    pub seasons: Vec<HistorySeason>,
    pub rows: Vec<HistoryRow>,
    pub limitations: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistorySeason {
    pub season: u32,
    pub group: Option<String>,
    pub situation: Option<String>,
    pub games: Option<u32>,
    pub minutes: Option<f64>,
    pub state: String,
    pub sources: Vec<Source>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryRow {
    pub key: String,
    pub label: String,
    pub points: Vec<HistoryPoint>,
    pub ranked_seasons: usize,
    pub upper_quintile_seasons: Option<usize>,
    pub comparability: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoryPoint {
    pub season: u32,
    pub state: String,
    pub measurement: Option<Measurement>,
}
impl NumericalHistory {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA || self.catalogs.is_empty() || self.catalogs.len() > 20 {
            return Err("unsupported or unbounded numerical history".into());
        }
        let mut seasons = BTreeSet::new();
        for c in &self.catalogs {
            c.validate()?;
            if !seasons.insert(c.season) {
                return Err("duplicate history season".into());
            }
        }
        Ok(())
    }
    pub fn card(
        &self,
        pid: u32,
        first: Option<u32>,
        last: Option<u32>,
    ) -> Result<Option<HistoryCard>, String> {
        self.validate()?;
        if first.zip(last).is_some_and(|(a, b)| a > b) {
            return Ok(None);
        }
        let mut frames = self
            .catalogs
            .iter()
            .filter(|c| first.is_none_or(|s| c.season >= s) && last.is_none_or(|s| c.season <= s))
            .collect::<Vec<_>>();
        frames.sort_by_key(|c| c.season);
        let Some(target) = frames
            .iter()
            .rev()
            .find_map(|c| c.profiles.iter().find(|p| p.player_id == pid))
        else {
            return Ok(None);
        };
        let mut keys = BTreeSet::new();
        let mut rows = Vec::new();
        for m in frames
            .iter()
            .rev()
            .filter_map(|c| c.profiles.iter().find(|p| p.player_id == pid))
            .flat_map(|p| &p.metrics)
        {
            if !keys.insert(&m.key) {
                continue;
            }
            let mut contexts = BTreeSet::new();
            let points = frames
                .iter()
                .map(|c| {
                    let p = c.profiles.iter().find(|p| p.player_id == pid);
                    let metric = p.and_then(|p| p.metrics.iter().find(|v| v.key == m.key));
                    if let Some((p, v)) = p.zip(metric) {
                        contexts.insert((
                            p.group.clone(),
                            p.situation.clone(),
                            v.unit.clone(),
                            v.formula.clone(),
                        ));
                    }
                    HistoryPoint {
                        season: c.season,
                        state: metric.map(|v| v.state.clone()).unwrap_or_else(|| {
                            if p.is_none() {
                                "missing_player".into()
                            } else {
                                "missing_metric".into()
                            }
                        }),
                        measurement: metric.cloned(),
                    }
                })
                .collect::<Vec<_>>();
            let ranked = points
                .iter()
                .filter_map(|p| p.measurement.as_ref())
                .filter(|m| m.state == "ranked")
                .filter_map(|m| m.percentile)
                .collect::<Vec<_>>();
            let compatible = contexts.len() == 1;
            rows.push(HistoryRow {
                key: m.key.clone(),
                label: m.label.clone(),
                ranked_seasons: ranked.len(),
                upper_quintile_seasons: compatible
                    .then(|| ranked.iter().filter(|p| **p >= 80.).count()),
                comparability: if compatible {
                    "Same definition/group/situation; each season has its own peer population."
                } else {
                    "Definition, group, unit or situation changed; repetition count withheld."
                }
                .into(),
                points,
            });
        }
        let seasons: Vec<_> = frames
            .iter()
            .map(|c| {
                let p = c.profiles.iter().find(|p| p.player_id == pid);
                HistorySeason {
                    season: c.season,
                    group: p.map(|p| p.group.clone()),
                    situation: p.map(|p| p.situation.clone()),
                    games: p.map(|p| p.games),
                    minutes: p.map(|p| p.minutes),
                    state: if p.is_some() {
                        "available"
                    } else {
                        "missing_player"
                    }
                    .into(),
                    sources: c.sources.clone(),
                }
            })
            .collect();
        let assessment = Some(super::rinkcraft_assessment::build(&seasons, &rows));
        Ok(Some(HistoryCard {assessment,schema:"icelines.rinkcraft-history-card.v1".into(),player_id:pid,name:target.name.clone(),seasons,rows,limitations:vec![
            "P >= 80 counts repeated upper-quintile raw values among ranked seasons; higher is not always better. This is a descriptive display threshold, not skill or class validity.".into(),
            "Seasonal ranks use different annual peers; they are not averaged or pooled. Missing seasons and unranked exposure stay explicit.".into(),
            "Historical snapshots were acquired retrospectively; model vintages, opportunity adjustment and statistical persistence are not established.".into(),
        ]}))
    }
}
pub fn parse(text: &str) -> Result<NumericalHistory, String> {
    if text.len() > 8 * 1024 * 1024 {
        return Err("numerical history exceeds 8 MiB".into());
    }
    let h: NumericalHistory = serde_json::from_str(text).map_err(|e| e.to_string())?;
    h.validate()?;
    Ok(h)
}

#[cfg(test)]
mod tests {
    use super::super::rinkcraft_numbers::{self, InputRow};
    use super::*;
    #[test]
    fn histories_keep_missing_seasons_annual_peers_and_bounds() {
        let rows = (1..=25)
            .map(|id| InputRow {
                player_id: id,
                name: format!("Synthetic {id}"),
                group: "Forwards".into(),
                games: 30,
                seconds: 30000.,
                values: [("primary_assists".into(), id as f64)]
                    .into_iter()
                    .collect(),
            })
            .collect::<Vec<_>>();
        let a = rinkcraft_numbers::build(&rows, 20232024, &[], vec![]).unwrap();
        let b = rinkcraft_numbers::build(&rows, 20242025, &[], vec![]).unwrap();
        let mut c = rinkcraft_numbers::build(&rows, 20252026, &[], vec![]).unwrap();
        c.profiles.retain(|p| p.player_id != 25);
        let mut h = NumericalHistory {
            schema: SCHEMA.into(),
            catalogs: vec![c, a, b],
        };
        let card = h.card(25, None, None).unwrap().unwrap();
        assert_eq!(card.seasons.len(), 3);
        assert_eq!(card.rows[0].upper_quintile_seasons, Some(2));
        assert_eq!(card.rows[0].ranked_seasons, 2);
        assert_eq!(card.rows[0].points[2].state, "missing_player");
        assert!(card.rows[0].points[2].measurement.is_none());
        assert_eq!(
            h.card(25, None, Some(20242025))
                .unwrap()
                .unwrap()
                .seasons
                .len(),
            2
        );
        assert!(h.card(25, None, Some(20222023)).unwrap().is_none());
        assert!(h
            .card(25, Some(20252026), Some(20242025))
            .unwrap()
            .is_none());
        h.catalogs[2]
            .profiles
            .iter_mut()
            .find(|p| p.player_id == 25)
            .unwrap()
            .group = "Defense".into();
        assert!(h.card(25, None, None).unwrap().unwrap().rows[0]
            .upper_quintile_seasons
            .is_none());
        h.catalogs.push(h.catalogs[0].clone());
        assert!(h.validate().is_err());
    }
}
