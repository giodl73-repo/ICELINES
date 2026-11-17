//! Season-specific evidence, not a pooled ability or persistence estimate.
use super::rinkcraft_numbers::{Measurement, NumericalCatalog, Source};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
pub const SCHEMA: &str = "icelines.rinkcraft-numerical-history.v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NumericalHistory {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finishing_research: Option<serde_json::Value>,
    pub schema: String,
    pub catalogs: Vec<NumericalCatalog>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotSelection {
    pub selected_season: Option<u32>,
    pub newest_supplied_season: Option<u32>,
    pub requested_season: Option<u32>,
    pub state: String,
    pub note: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotResult {
    pub selection: SnapshotSelection,
    pub card: Option<super::rinkcraft_numbers::EvidenceCard>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogView {
    pub catalog: NumericalCatalog,
    pub seasons: Vec<u32>,
}
fn period(s: u32) -> String {
    format!("{}–{:02}", s / 10000, s % 100)
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
    /// Comparisons use exactly one supplied season, never a per-player fallback.
    pub fn catalog_view(&self, season: Option<u32>) -> Result<CatalogView, String> {
        self.validate()?;
        let mut seasons = self.catalogs.iter().map(|c| c.season).collect::<Vec<_>>();
        seasons.sort_unstable();
        let selected = season.unwrap_or(*seasons.last().expect("validated nonempty history"));
        let catalog = self
            .catalogs
            .iter()
            .find(|c| c.season == selected)
            .ok_or_else(|| format!("No numerical source for {}", period(selected)))?
            .clone();
        Ok(CatalogView { catalog, seasons })
    }
    /// Filter first; exact requests cannot fall back to a different year.
    pub fn snapshot(
        &self,
        pid: u32,
        first: Option<u32>,
        last: Option<u32>,
        exact: Option<u32>,
    ) -> Result<SnapshotResult, String> {
        self.validate()?;
        let mut frames = self
            .catalogs
            .iter()
            .filter(|c| first.is_none_or(|s| c.season >= s) && last.is_none_or(|s| c.season <= s))
            .collect::<Vec<_>>();
        frames.sort_by_key(|c| c.season);
        let newest = frames.last().map(|c| c.season);
        let selected = frames.iter().rev().find(|c| {
            exact.is_none_or(|s| c.season == s) && c.profiles.iter().any(|p| p.player_id == pid)
        });
        let selected_season = selected.map(|c| c.season);
        let mut card = selected.map(|c| c.card(pid, last)).transpose()?.flatten();
        if let (Some(raw), Some(card)) = (&self.finishing_research, &mut card) {
            card.finishing_research = Some(super::rinkcraft_finishing_evidence::project(
                raw,
                card,
                &self.catalogs,
            ));
        }
        let (state, note) = match selected_season {
            None => ("unavailable", exact.map(|s| format!("No numerical snapshot for {} within the requested window. No fallback used.", period(s)))
                .unwrap_or_else(|| "No numerical snapshot within the requested window.".into())),
            Some(s) if exact.is_some() => ("exact", format!("Selected {} numerical snapshot. Historical request; no fallback used.", period(s))),
            Some(s) if Some(s) != newest => ("older_fallback", format!("No player row in latest supplied {} source; showing {}. Latest available numerical snapshot within the requested window.", period(newest.expect("selected frame implies newest")), period(s))),
            Some(s) => ("latest", format!("Latest available numerical snapshot within the requested window: {}.", period(s))),
        };
        Ok(SnapshotResult {
            selection: SnapshotSelection {
                selected_season,
                newest_supplied_season: newest,
                requested_season: exact,
                state: state.into(),
                note,
            },
            card,
        })
    }
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
/// Accept an archive or the legacy single-season source without reserializing floats.
pub fn parse_source(text: &str) -> Result<NumericalHistory, String> {
    if text.len() > 8 * 1024 * 1024 {
        return Err("numerical source exceeds 8 MiB".into());
    }
    #[derive(Deserialize)]
    struct Header {
        schema: String,
    }
    let header: Header = serde_json::from_str(text).map_err(|e| e.to_string())?;
    if header.schema == SCHEMA {
        return parse(text);
    }
    let catalog = super::rinkcraft_numbers::parse_catalog(text)?;
    Ok(NumericalHistory {
        finishing_research: None,
        schema: SCHEMA.into(),
        catalogs: vec![catalog],
    })
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
            finishing_research: None,
            schema: SCHEMA.into(),
            catalogs: vec![c, a, b],
        };
        let latest = h.snapshot(1, None, None, None).unwrap();
        assert_eq!(latest.selection.state, "latest");
        assert_eq!(latest.card.unwrap().season, 20252026);
        let fallback = h.snapshot(25, None, None, None).unwrap();
        assert_eq!(fallback.selection.state, "older_fallback");
        assert_eq!(fallback.selection.newest_supplied_season, Some(20252026));
        assert_eq!(fallback.card.unwrap().season, 20242025);
        let exact = h.snapshot(25, None, None, Some(20252026)).unwrap();
        assert_eq!(exact.selection.state, "unavailable");
        assert!(exact.card.is_none());
        assert!(h
            .snapshot(25, Some(20252026), None, None)
            .unwrap()
            .card
            .is_none());
        assert!(h
            .snapshot(1, Some(20252026), Some(20242025), None)
            .unwrap()
            .card
            .is_none());
        assert!(h
            .snapshot(1, Some(20242025), None, Some(20232024))
            .unwrap()
            .card
            .is_none());
        let earlier = h.snapshot(25, None, Some(20242025), None).unwrap();
        assert_eq!(earlier.selection.state, "latest");
        assert_eq!(earlier.card.unwrap().season, 20242025);
        assert_eq!(h.catalog_view(None).unwrap().catalog.season, 20252026);
        assert!(h
            .catalog_view(None)
            .unwrap()
            .catalog
            .compare(1, 25)
            .is_err());
        assert!(h
            .catalog_view(Some(20242025))
            .unwrap()
            .catalog
            .compare(1, 25)
            .is_ok());
        assert!(h.catalog_view(Some(20222023)).is_err());
        let legacy = serde_json::to_string(&h.catalogs[1]).unwrap();
        assert_eq!(
            parse_source(&legacy).unwrap().catalogs,
            vec![rinkcraft_numbers::parse_catalog(&legacy).unwrap()]
        );
        let archive = serde_json::to_string(&h).unwrap();
        assert_eq!(parse_source(&archive).unwrap(), parse(&archive).unwrap());
        assert!(parse_source(r#"{"schema":"unknown"}"#).is_err());
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
