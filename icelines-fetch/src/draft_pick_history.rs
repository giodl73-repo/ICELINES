//! Survivorship-safe historical draft-pick outcome assembly.
//!
//! The complete official draft class is the denominator. NHL skater and
//! goalie bios contribute appearances by draft coordinates; absent players
//! remain legitimate zero outcomes instead of disappearing from the sample.

use crate::{bundled::get_bios, NhlApiClient};
use chrono::{DateTime, Utc};
use icelines_core::{
    build_draft_pick_value_curve, source_facts::ContentHash, DraftPickOutcomeObservation,
    DraftPickValueConfig, DraftPickValueCurve,
};
use icelines_sources::{
    adapter::{SourceAdapter, SourceInput},
    nhl::draft_picks::OfficialNhlDraftPicksAdapter,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

pub const DRAFT_PICK_OUTCOME_SET_SCHEMA: &str = "draft_pick_outcome_set.v1";
pub const DRAFT_PICK_CURVE_ACQUISITION_SCHEMA: &str = "draft_pick_curve_acquisition.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalDraftSelection {
    pub overall_pick: u16,
    pub position_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalDraftClass {
    pub draft_year: u16,
    pub terminal: bool,
    pub source_url: String,
    pub selections: Vec<HistoricalDraftSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftPickSeasonAppearance {
    pub player_id: u32,
    pub draft_year: u16,
    pub overall_pick: u16,
    /// NHL season ID such as 20182019.
    pub season_id: u32,
    pub games_played: u32,
    pub position_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftPickOutcomeBuildConfig {
    pub training_start_year: u16,
    pub training_cutoff_year: u16,
    pub completed_season_start_year: u16,
    pub outcome_horizon_years: u8,
    pub max_overall_pick: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickOutcomeSet {
    pub schema: String,
    pub outcome_measure: String,
    pub training_start_year: u16,
    pub training_cutoff_year: u16,
    pub outcome_horizon_years: u8,
    pub max_overall_pick: u16,
    pub draft_classes: usize,
    pub selections: usize,
    pub zero_outcomes: usize,
    pub deduplicated_source_rows: usize,
    pub observations: Vec<DraftPickOutcomeObservation>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DraftPickCurveAcquisitionView {
    pub schema: String,
    pub generated_at: DateTime<Utc>,
    pub outcome_set: DraftPickOutcomeSet,
    pub curve: DraftPickValueCurve,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum DraftPickOutcomeBuildError {
    #[error("draft-pick outcome input is invalid: {0}")]
    InvalidInput(String),
}

/// Acquire complete official draft classes, combine them with bundled skater
/// bios and official goalie bios, and fit the UI-neutral pick-value curve.
pub async fn acquire_draft_pick_value_curve(
    client: &NhlApiClient,
    config: DraftPickOutcomeBuildConfig,
    annual_future_discount: f64,
    generated_at: DateTime<Utc>,
) -> Result<DraftPickCurveAcquisitionView, DraftPickOutcomeBuildError> {
    validate_config(&config)?;
    let mut classes = Vec::new();
    for year in config.training_start_year..=config.training_cutoff_year {
        let source_url = format!("https://api-web.nhle.com/v1/draft/picks/{year}/all");
        let bytes = client
            .fetch_source_bytes(&source_url)
            .await
            .map_err(|error| {
                DraftPickOutcomeBuildError::InvalidInput(format!(
                    "acquire official draft class {year}: {error}"
                ))
            })?;
        let adapter = OfficialNhlDraftPicksAdapter::new(year, generated_at)
            .map_err(DraftPickOutcomeBuildError::InvalidInput)?;
        let hash = ContentHash::try_new(format!("{:x}", Sha256::digest(&bytes)))
            .map_err(|error| DraftPickOutcomeBuildError::InvalidInput(error.to_string()))?;
        let output = adapter
            .parse(SourceInput::new(
                &bytes,
                adapter.descriptor().source_id,
                hash,
            ))
            .map_err(|error| DraftPickOutcomeBuildError::InvalidInput(error.to_string()))?;
        classes.push(HistoricalDraftClass {
            draft_year: year,
            terminal: true,
            source_url,
            selections: output
                .selections
                .into_iter()
                .map(|selection| HistoricalDraftSelection {
                    overall_pick: selection.overall,
                    position_code: selection.position_code,
                })
                .collect(),
        });
    }

    let last_season = config.training_cutoff_year + u16::from(config.outcome_horizon_years) - 1;
    let mut appearances = Vec::new();
    for season_start in config.training_start_year..=last_season {
        let season_id = u32::from(season_start) * 10_000 + u32::from(season_start + 1);
        let season = season_id.to_string();
        let skaters = get_bios(&season).ok_or_else(|| {
            DraftPickOutcomeBuildError::InvalidInput(format!(
                "bundled skater bios are unavailable for {season}"
            ))
        })?;
        appearances.extend(skaters.into_iter().filter_map(|row| {
            draft_appearance(
                row.player_id,
                row.draft_year,
                row.draft_overall,
                season_id,
                row.games_played,
                row.position_code,
            )
        }));
        let goalies = client
            .fetch_all_goalie_bios(&season, icelines_core::season_stats::SeasonType::Regular)
            .await
            .map_err(|error| {
                DraftPickOutcomeBuildError::InvalidInput(format!(
                    "acquire official goalie bios for {season}: {error}"
                ))
            })?;
        appearances.extend(goalies.into_iter().filter_map(|row| {
            draft_appearance(
                row.player_id,
                row.draft_year,
                row.draft_overall,
                season_id,
                row.games_played,
                "G".to_owned(),
            )
        }));
    }
    let outcome_set = build_draft_pick_outcome_set(classes, appearances, config.clone())?;
    let curve = build_draft_pick_value_curve(
        outcome_set.observations.clone(),
        DraftPickValueConfig {
            training_cutoff_year: config.training_cutoff_year,
            outcome_horizon_years: config.outcome_horizon_years,
            max_overall_pick: config.max_overall_pick,
            outcome_measure: outcome_set.outcome_measure.clone(),
            annual_future_discount,
        },
    )
    .map_err(|error| DraftPickOutcomeBuildError::InvalidInput(error.to_string()))?;
    Ok(DraftPickCurveAcquisitionView {
        schema: DRAFT_PICK_CURVE_ACQUISITION_SCHEMA.to_owned(),
        generated_at,
        outcome_set,
        curve,
    })
}

fn draft_appearance(
    player_id: u32,
    draft_year: Option<u32>,
    draft_overall: Option<u32>,
    season_id: u32,
    games_played: u32,
    position_code: String,
) -> Option<DraftPickSeasonAppearance> {
    Some(DraftPickSeasonAppearance {
        player_id,
        draft_year: u16::try_from(draft_year?).ok()?,
        overall_pick: u16::try_from(draft_overall?).ok()?,
        season_id,
        games_played,
        position_code,
    })
}

pub fn build_draft_pick_outcome_set(
    classes: Vec<HistoricalDraftClass>,
    appearances: Vec<DraftPickSeasonAppearance>,
    config: DraftPickOutcomeBuildConfig,
) -> Result<DraftPickOutcomeSet, DraftPickOutcomeBuildError> {
    validate_config(&config)?;
    let expected_years =
        (config.training_start_year..=config.training_cutoff_year).collect::<BTreeSet<_>>();
    let mut classes_by_year = BTreeMap::new();
    for class in classes {
        if !expected_years.contains(&class.draft_year) {
            continue;
        }
        if !class.terminal || class.source_url.trim().is_empty() {
            return invalid(format!(
                "draft class {} must be terminal and source-backed",
                class.draft_year
            ));
        }
        if classes_by_year.insert(class.draft_year, class).is_some() {
            return invalid("duplicate draft class year".to_owned());
        }
    }
    let actual_years = classes_by_year.keys().copied().collect::<BTreeSet<_>>();
    if actual_years != expected_years {
        let missing = expected_years
            .difference(&actual_years)
            .copied()
            .collect::<Vec<_>>();
        return invalid(format!("missing terminal draft classes: {missing:?}"));
    }

    let mut population = BTreeMap::<(u16, u16), String>::new();
    for class in classes_by_year.values() {
        if class.selections.is_empty() {
            return invalid(format!(
                "draft class {} has no selections",
                class.draft_year
            ));
        }
        for selection in &class.selections {
            if selection.overall_pick == 0
                || selection.overall_pick > config.max_overall_pick
                || selection.position_code.trim().is_empty()
            {
                continue;
            }
            if population
                .insert(
                    (class.draft_year, selection.overall_pick),
                    selection.position_code.clone(),
                )
                .is_some()
            {
                return invalid(format!(
                    "duplicate selection {}:{}",
                    class.draft_year, selection.overall_pick
                ));
            }
        }
    }

    let mut player_by_pick = BTreeMap::<(u16, u16), u32>::new();
    let mut seen_player_seasons = BTreeMap::new();
    let mut deduplicated_source_rows = 0;
    let mut games_by_pick = BTreeMap::<(u16, u16), u32>::new();
    for row in appearances {
        let key = (row.draft_year, row.overall_pick);
        if !population.contains_key(&key) {
            continue;
        }
        if row.player_id == 0 || row.position_code.trim().is_empty() {
            return invalid("appearance requires player identity and position".to_owned());
        }
        let season_start = season_start_year(row.season_id)?;
        let horizon_end = row.draft_year + u16::from(config.outcome_horizon_years);
        if season_start < row.draft_year || season_start >= horizon_end {
            continue;
        }
        if let Some(existing) = player_by_pick.insert(key, row.player_id) {
            if existing != row.player_id {
                return invalid(format!(
                    "draft selection {}:{} maps to multiple NHL player IDs",
                    row.draft_year, row.overall_pick
                ));
            }
        }
        if let Some(existing) = seen_player_seasons.get(&(row.player_id, row.season_id)) {
            if existing == &row {
                deduplicated_source_rows += 1;
                continue;
            }
            return invalid(format!(
                "conflicting season appearances for player {} in {}",
                row.player_id, row.season_id
            ));
        }
        seen_player_seasons.insert((row.player_id, row.season_id), row.clone());
        *games_by_pick.entry(key).or_default() += row.games_played;
    }

    let observations = population
        .keys()
        .map(|&(draft_year, overall_pick)| DraftPickOutcomeObservation {
            draft_year,
            overall_pick,
            outcome_value: f64::from(*games_by_pick.get(&(draft_year, overall_pick)).unwrap_or(&0)),
            observed_horizon_years: config.outcome_horizon_years,
        })
        .collect::<Vec<_>>();
    let zero_outcomes = observations
        .iter()
        .filter(|row| row.outcome_value == 0.0)
        .count();
    Ok(DraftPickOutcomeSet {
        schema: DRAFT_PICK_OUTCOME_SET_SCHEMA.to_owned(),
        outcome_measure: "nhl_regular_season_games_played".to_owned(),
        training_start_year: config.training_start_year,
        training_cutoff_year: config.training_cutoff_year,
        outcome_horizon_years: config.outcome_horizon_years,
        max_overall_pick: config.max_overall_pick,
        draft_classes: classes_by_year.len(),
        selections: observations.len(),
        zero_outcomes,
        deduplicated_source_rows,
        observations,
        disclosures: vec![
            "The denominator is every non-forfeited selection in each terminal official NHL draft ledger, including players with zero NHL games.".to_owned(),
            "Skater and goalie appearances join by official draft year and overall pick; player names are not used for identity resolution.".to_owned(),
            "Outcomes include regular-season NHL games in the first fixed number of seasons beginning with the draft year.".to_owned(),
            format!("{} byte-identical duplicate season rows were collapsed before aggregation.", deduplicated_source_rows),
        ],
    })
}

fn validate_config(config: &DraftPickOutcomeBuildConfig) -> Result<(), DraftPickOutcomeBuildError> {
    if config.training_start_year > config.training_cutoff_year
        || config.outcome_horizon_years == 0
        || config.max_overall_pick == 0
    {
        return invalid("year range, horizon, and maximum pick must be positive".to_owned());
    }
    let required_last_season =
        config.training_cutoff_year + u16::from(config.outcome_horizon_years) - 1;
    if required_last_season > config.completed_season_start_year {
        return invalid(format!(
            "cutoff cohort requires season {required_last_season}-{} but completed data ends at {}-{}",
            required_last_season + 1,
            config.completed_season_start_year,
            config.completed_season_start_year + 1
        ));
    }
    Ok(())
}

fn season_start_year(season_id: u32) -> Result<u16, DraftPickOutcomeBuildError> {
    let start = season_id / 10_000;
    let end = season_id % 10_000;
    if end != start + 1 || !(1900..=2200).contains(&start) {
        return invalid(format!("invalid NHL season ID {season_id}"));
    }
    Ok(start as u16)
}

fn invalid<T>(message: String) -> Result<T, DraftPickOutcomeBuildError> {
    Err(DraftPickOutcomeBuildError::InvalidInput(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class(year: u16) -> HistoricalDraftClass {
        HistoricalDraftClass {
            draft_year: year,
            terminal: true,
            source_url: format!("https://api-web.nhle.com/v1/draft/picks/{year}/all"),
            selections: vec![
                HistoricalDraftSelection {
                    overall_pick: 1,
                    position_code: "C".to_owned(),
                },
                HistoricalDraftSelection {
                    overall_pick: 2,
                    position_code: "G".to_owned(),
                },
            ],
        }
    }

    fn config() -> DraftPickOutcomeBuildConfig {
        DraftPickOutcomeBuildConfig {
            training_start_year: 2010,
            training_cutoff_year: 2011,
            completed_season_start_year: 2017,
            outcome_horizon_years: 7,
            max_overall_pick: 2,
        }
    }

    #[test]
    fn complete_population_preserves_zero_outcomes_and_goalies() {
        let output = build_draft_pick_outcome_set(
            vec![class(2010), class(2011)],
            vec![DraftPickSeasonAppearance {
                player_id: 8_470_001,
                draft_year: 2010,
                overall_pick: 2,
                season_id: 20102011,
                games_played: 25,
                position_code: "G".to_owned(),
            }],
            config(),
        )
        .unwrap();
        assert_eq!(output.selections, 4);
        assert_eq!(output.zero_outcomes, 3);
        assert_eq!(output.observations[1].outcome_value, 25.0);
    }

    #[test]
    fn missing_terminal_class_fails_closed() {
        let error = build_draft_pick_outcome_set(vec![class(2010)], vec![], config()).unwrap_err();
        assert!(error.to_string().contains("missing terminal draft classes"));
    }

    #[test]
    fn identical_duplicate_player_season_is_collapsed() {
        let row = DraftPickSeasonAppearance {
            player_id: 8_470_001,
            draft_year: 2010,
            overall_pick: 1,
            season_id: 20102011,
            games_played: 10,
            position_code: "C".to_owned(),
        };
        let output = build_draft_pick_outcome_set(
            vec![class(2010), class(2011)],
            vec![row.clone(), row],
            config(),
        )
        .unwrap();
        assert_eq!(output.deduplicated_source_rows, 1);
        assert_eq!(output.observations[0].outcome_value, 10.0);
    }

    #[test]
    fn conflicting_duplicate_player_season_fails_closed() {
        let row = DraftPickSeasonAppearance {
            player_id: 8_470_001,
            draft_year: 2010,
            overall_pick: 1,
            season_id: 20102011,
            games_played: 10,
            position_code: "C".to_owned(),
        };
        let mut conflict = row.clone();
        conflict.games_played = 11;
        let error = build_draft_pick_outcome_set(
            vec![class(2010), class(2011)],
            vec![row, conflict],
            config(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("conflicting season appearances"));
    }
}
