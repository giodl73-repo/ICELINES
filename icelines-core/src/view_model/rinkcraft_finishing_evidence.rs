//! Opt-in research readout. Missing/invalid research never suppresses ordinary statistics.
use super::{
    rinkcraft_finishing as f,
    rinkcraft_numbers::{EvidenceCard, NumericalCatalog, Source},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceRecord {
    pub season: u32,
    pub source: Source,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub target: f::Target,
    pub history: Vec<f::HistoryRow>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResearchInput {
    pub schema: String,
    pub model_id: String,
    pub model_config_sha256: String,
    pub model: f::Model,
    pub interval_probability: f64,
    pub minimum_target_games: u32,
    pub minimum_target_minutes: f64,
    pub sources: Vec<SourceRecord>,
    pub records: Vec<Record>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Readout {
    pub status: String,
    pub note: String,
    pub input_sha256: String,
    pub model_id: String,
    pub model_config_sha256: String,
    pub qualification: String,
    pub target_season: u32,
    pub calibration_season: Option<u32>,
    pub history_seasons: Vec<u32>,
    pub history_goals: Option<u64>,
    pub history_xg: Option<f64>,
    pub mean: Option<f64>,
    pub lower: Option<f64>,
    pub upper: Option<f64>,
    pub probability: Option<f64>,
    pub sources: Vec<SourceRecord>,
    pub limits: Vec<String>,
}
fn hash(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn metric(c: &NumericalCatalog, pid: u32, key: &str) -> Option<f64> {
    c.profiles
        .iter()
        .find(|p| p.player_id == pid)?
        .metrics
        .iter()
        .find(|m| m.key == key)?
        .numerator
}
fn same(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite() && (a - b).abs() <= 1e-10
}
pub fn project(raw: &Value, card: &EvidenceCard, catalogs: &[NumericalCatalog]) -> Readout {
    let mut out = Readout {
        status: "unavailable".into(),
        note: String::new(),
        input_sha256: String::new(),
        model_id: String::new(),
        model_config_sha256: String::new(),
        qualification: String::new(),
        target_season: card.season,
        calibration_season: None,
        history_seasons: vec![],
        history_goals: None,
        history_xg: None,
        mean: None,
        lower: None,
        upper: None,
        probability: None,
        sources: vec![],
        limits: vec![
            "Experimental conditional finishing component, not a complete Shooting attribute or a 1–20 grade.".into(),
            "Position-relative baseline = 1. Equal-tailed parameter interval, not a goal-count prediction interval.".into(),
            "Frozen prior/factors/provider xG; excludes their estimation/model error, context confounding and game/team dependence.".into(),
        ],
    };
    if card.group == "Goalies" {
        out.status = "not_applicable".into();
        out.note = "Skater finishing does not assess goalie skills.".into();
        return out;
    }
    if !matches!(card.group.as_str(), "Forwards" | "Defense") || card.situation != "5on5" {
        out.note = "Research component requires a matching 5-on-5 skater snapshot.".into();
        return out;
    }
    let result = (|| -> Result<(), String> {
        let input: ResearchInput = serde_json::from_value(raw.clone())
            .map_err(|_| "Invalid supplied finishing research data")?;
        out.input_sha256 = format!(
            "{:x}",
            Sha256::digest(
                serde_json::to_vec(&input).map_err(|_| "Research input serialization failed")?
            )
        );
        if input.schema != "icelines.finishing-research-input.v1"
            || input.model_id.is_empty()
            || input.model_id.len() > 160
            || !hash(&input.model_config_sha256)
            || input.records.is_empty()
            || input.records.len() > 5000
            || input.sources.is_empty()
            || input.sources.len() > 20
            || input.minimum_target_games == 0
            || !input.minimum_target_minutes.is_finite()
            || input.minimum_target_minutes <= 0.
        {
            return Err("Unsupported or unbounded research input".into());
        }
        let mut source_seasons = BTreeSet::new();
        for s in &input.sources {
            if !source_seasons.insert(s.season)
                || !hash(&s.source.sha256)
                || s.source.bytes == 0
                || !s.source.url.starts_with("https://")
            {
                return Err("Invalid research source record".into());
            }
            let c = catalogs
                .iter()
                .find(|c| c.season == s.season)
                .ok_or("Research source season absent from statistical history")?;
            if !c.sources.iter().any(|v| v == &s.source) {
                return Err("Research source fingerprint differs from statistical history".into());
            }
        }
        if !source_seasons.contains(&input.model.calibration_season) {
            return Err("Missing calibration source fingerprint".into());
        }
        let mut ids = BTreeSet::new();
        for r in &input.records {
            if r.history.len() > 20 || !ids.insert((r.target.player_id, r.target.season)) {
                return Err("Unbounded history or duplicate research target".into());
            }
            f::predict(&input.model, &r.target, &r.history)?;
        }
        let Some(record) = input
            .records
            .iter()
            .find(|r| r.target.player_id == card.player_id && r.target.season == card.season)
        else {
            out.status = "missing".into();
            return Err("No exact player/target-season research record; no fallback used".into());
        };
        if card.games < input.minimum_target_games || card.minutes < input.minimum_target_minutes {
            return Err("Selected snapshot is outside the declared research qualification".into());
        }
        if (record.target.position == f::Position::D) != (card.group == "Defense") {
            return Err("Research position group differs from statistical snapshot".into());
        }
        let target = catalogs
            .iter()
            .find(|c| c.season == card.season)
            .ok_or("Selected source season unavailable")?;
        if !source_seasons.contains(&card.season)
            || !input
                .sources
                .iter()
                .any(|s| s.season == card.season && card.sources.contains(&s.source))
            || !card
                .metrics
                .iter()
                .find(|m| m.key == "ixg-per-60")
                .and_then(|m| m.numerator)
                .is_some_and(|x| same(x, record.target.xg))
            || !metric(target, card.player_id, "ixg-per-60")
                .is_some_and(|x| same(x, record.target.xg))
        {
            return Err("Research target exposure differs from statistical snapshot".into());
        }
        for source in input
            .sources
            .iter()
            .filter(|s| s.season >= input.model.calibration_season && s.season < card.season)
        {
            if catalogs
                .iter()
                .find(|c| c.season == source.season)
                .is_some_and(|c| c.profiles.iter().any(|p| p.player_id == card.player_id))
                && !record.history.iter().any(|h| h.season == source.season)
            {
                return Err(
                    "Available earlier source evidence omitted from research history".into(),
                );
            }
        }
        for row in &record.history {
            let c = catalogs
                .iter()
                .find(|c| c.season == row.season)
                .ok_or("Historical source season unavailable")?;
            let p = c
                .profiles
                .iter()
                .find(|p| p.player_id == row.player_id)
                .ok_or("Historical player source unavailable")?;
            if !source_seasons.contains(&row.season)
                || c.profiles.is_empty()
                || p.situation != "5on5"
                || (row.position == f::Position::D) != (p.group == "Defense")
                || !metric(c, row.player_id, "ixg-per-60").is_some_and(|x| same(x, row.xg))
                || !metric(c, row.player_id, "goals-per-60")
                    .is_some_and(|g| same(g, row.goals as f64))
            {
                return Err(
                    "Historical research evidence differs from statistical source rows".into(),
                );
            }
        }
        let prediction = f::predict(&input.model, &record.target, &record.history)?;
        let interval =
            f::credible_interval(&prediction.position_residual, input.interval_probability)?;
        out.model_id = input.model_id;
        out.qualification=format!("Target qualification: at least {} games and {:.0} five-on-five minutes. History uses available earlier source rows.",input.minimum_target_games,input.minimum_target_minutes);
        out.model_config_sha256 = input.model_config_sha256;
        out.calibration_season = Some(input.model.calibration_season);
        out.history_seasons = record.history.iter().map(|h| h.season).collect();
        out.history_seasons.sort_unstable();
        out.history_goals = Some(record.history.iter().map(|h| h.goals as u64).sum());
        out.history_xg = Some(record.history.iter().map(|h| h.xg).sum());
        out.mean = Some(prediction.position_residual.mean);
        out.lower = Some(interval.lower);
        out.upper = Some(interval.upper);
        out.probability = Some(interval.probability);
        out.sources = input
            .sources
            .into_iter()
            .filter(|s| {
                s.season == card.season
                    || Some(s.season) == out.calibration_season
                    || out.history_seasons.contains(&s.season)
            })
            .collect();
        out.status = if out.history_xg == Some(0.) {
            "baseline_only"
        } else {
            "experimental"
        }
        .into();
        out.note=if out.status=="baseline_only"{"No informative earlier player exposure; model baseline only, with conditional uncertainty."}else{"Earlier-history multiplier relative to the frozen position-adjusted baseline. Target-season goals do not enter the posterior."}.into();
        Ok(())
    })();
    if let Err(reason) = result {
        out.note = reason;
    }
    out
}
