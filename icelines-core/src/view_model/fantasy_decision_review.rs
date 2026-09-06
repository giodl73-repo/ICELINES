//! Deterministic review of frozen fantasy decisions and later observations.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::fantasy_pickup_sequence::{FantasyPickupSequenceRow, FantasyPickupSequenceView};
use super::fantasy_today::FantasyTodayState;
use crate::model::Position;

pub const FANTASY_DECISION_OUTCOME_SCHEMA: &str = "fantasy_decision_outcome.v1";
pub const FANTASY_DECISION_REVIEW_SCHEMA: &str = "fantasy_decision_review.v1";
pub const DISPLAY_ALIGNMENT_TOLERANCE: f64 = 1.0;
pub const DESCRIPTIVE_READY_MINIMUM: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionOutcomeLane {
    Execution,
    ActiveValue,
    Matchup,
    Reserve,
}

impl FantasyDecisionOutcomeLane {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Execution => "execution",
            Self::ActiveValue => "active_value",
            Self::Matchup => "matchup",
            Self::Reserve => "reserve",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionOutcomeCompleteness {
    Provisional,
    Final,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionOutcomeSource {
    Manager,
    PlatformImport,
    DerivedBoxscores,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionMatchupResult {
    Win,
    Loss,
    Tie,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDecisionOutcome {
    pub schema: String,
    pub decision_id: String,
    pub lane: FantasyDecisionOutcomeLane,
    pub completeness: FantasyDecisionOutcomeCompleteness,
    pub source: FantasyDecisionOutcomeSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_observed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_active_points_delta: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_usable_starts_delta: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matchup_result: Option<FantasyDecisionMatchupResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_final_points: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opponent_final_points: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_needed: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve_used: Option<bool>,
}

impl FantasyDecisionOutcome {
    pub fn validate(&self) -> Result<(), FantasyDecisionReviewError> {
        if self.schema != FANTASY_DECISION_OUTCOME_SCHEMA || self.decision_id.trim().is_empty() {
            return Err(FantasyDecisionReviewError::InvalidOutcome(
                "schema and decision ID are required".to_owned(),
            ));
        }
        for (name, value) in [
            (
                "actual_active_points_delta",
                self.actual_active_points_delta,
            ),
            ("user_final_points", self.user_final_points),
            ("opponent_final_points", self.opponent_final_points),
        ] {
            if value.is_some_and(|value| !value.is_finite()) {
                return Err(FantasyDecisionReviewError::InvalidOutcome(format!(
                    "{name} must be finite"
                )));
            }
        }
        let fields_match_lane = match self.lane {
            FantasyDecisionOutcomeLane::Execution => {
                self.executed.is_some()
                    && self.actual_active_points_delta.is_none()
                    && self.actual_usable_starts_delta.is_none()
                    && self.matchup_result.is_none()
                    && self.user_final_points.is_none()
                    && self.opponent_final_points.is_none()
                    && self.reserve_needed.is_none()
                    && self.reserve_used.is_none()
            }
            FantasyDecisionOutcomeLane::ActiveValue => {
                self.executed.is_none()
                    && (self.actual_active_points_delta.is_some()
                        || self.actual_usable_starts_delta.is_some())
                    && self.matchup_result.is_none()
                    && self.user_final_points.is_none()
                    && self.opponent_final_points.is_none()
                    && self.reserve_needed.is_none()
                    && self.reserve_used.is_none()
            }
            FantasyDecisionOutcomeLane::Matchup => {
                self.executed.is_none()
                    && self.actual_active_points_delta.is_none()
                    && self.actual_usable_starts_delta.is_none()
                    && self.matchup_result.is_some()
                    && self.reserve_needed.is_none()
                    && self.reserve_used.is_none()
            }
            FantasyDecisionOutcomeLane::Reserve => {
                self.executed.is_none()
                    && self.actual_active_points_delta.is_none()
                    && self.actual_usable_starts_delta.is_none()
                    && self.matchup_result.is_none()
                    && self.user_final_points.is_none()
                    && self.opponent_final_points.is_none()
                    && (self.reserve_needed.is_some() || self.reserve_used.is_some())
            }
        };
        if !fields_match_lane {
            return Err(FantasyDecisionReviewError::InvalidOutcome(format!(
                "payload fields do not match {} lane",
                self.lane.as_str()
            )));
        }
        if self.lane == FantasyDecisionOutcomeLane::Matchup
            && self.user_final_points.is_some() != self.opponent_final_points.is_some()
        {
            return Err(FantasyDecisionReviewError::InvalidOutcome(
                "matchup final points must be supplied together".to_owned(),
            ));
        }
        Ok(())
    }

    pub fn material_fingerprint(&self) -> Result<String, FantasyDecisionReviewError> {
        self.validate()?;
        let bytes = serde_json::to_vec(self)
            .map_err(|error| FantasyDecisionReviewError::Serialization(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

pub fn validate_fantasy_decision_outcome_timing(
    outcome: &FantasyDecisionOutcome,
    projection: &FantasyPickupSequenceView,
    inserted_at: DateTime<Utc>,
) -> Result<(), FantasyDecisionReviewError> {
    outcome.validate()?;
    let observed_at = outcome.source_observed_at.unwrap_or(inserted_at);
    if observed_at < projection.context.evaluated_at {
        return Err(FantasyDecisionReviewError::InvalidOutcome(
            "source observation time cannot precede the frozen evaluation".to_owned(),
        ));
    }
    let needs_closed_week = outcome.completeness == FantasyDecisionOutcomeCompleteness::Final
        && matches!(
            outcome.lane,
            FantasyDecisionOutcomeLane::ActiveValue | FantasyDecisionOutcomeLane::Matchup
        );
    if needs_closed_week {
        let timezone = projection.context.timezone.parse::<Tz>().map_err(|_| {
            FantasyDecisionReviewError::InvalidOutcome(format!(
                "frozen projection timezone '{}' is invalid",
                projection.context.timezone
            ))
        })?;
        if observed_at.with_timezone(&timezone).date_naive() <= projection.context.week_end {
            return Err(FantasyDecisionReviewError::InvalidOutcome(
                "final active-value and matchup observations require the frozen week to end; record this observation as provisional"
                    .to_owned(),
            ));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyDecisionStoredOutcomeInput {
    pub id: String,
    pub observed_at: DateTime<Utc>,
    pub outcome: FantasyDecisionOutcome,
    pub correction_of: Option<String>,
    pub private_notes: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyDecisionStoredInput {
    pub id: String,
    pub kind: String,
    pub recommendation_id: String,
    pub recommendation_fingerprint: String,
    pub recorded_at: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
    pub chosen_alternative: usize,
    pub manager_rationale: Option<String>,
    pub projection_json: String,
    pub outcomes: Vec<FantasyDecisionStoredOutcomeInput>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FantasyDecisionReviewBuildInput {
    pub league_id: String,
    pub league_name: String,
    pub generated_at: DateTime<Utc>,
    pub week: Option<NaiveDate>,
    pub season: Option<String>,
    pub include_private: bool,
    pub decisions: Vec<FantasyDecisionStoredInput>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionLane {
    NoMove,
    SkaterOnly,
    GoalieOnly,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionProcessAssessment {
    Supported,
    Unsupported,
    InsufficientEvidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionResultAssessment {
    Positive,
    Neutral,
    Negative,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FantasyDecisionProjectionAssessment {
    Aligned,
    Above,
    Below,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDecisionEffectiveOutcome {
    pub execution: Option<FantasyDecisionOutcome>,
    pub active_value: Option<FantasyDecisionOutcome>,
    pub matchup: Option<FantasyDecisionOutcome>,
    pub reserve: Option<FantasyDecisionOutcome>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDecisionReviewItem {
    pub decision_id: String,
    pub kind: String,
    pub recommendation_id: String,
    pub recommendation_fingerprint: String,
    pub recorded_at: DateTime<Utc>,
    pub evaluated_at: DateTime<Utc>,
    pub week_start: Option<NaiveDate>,
    pub week_end: Option<NaiveDate>,
    pub stats_season: Option<String>,
    pub competition_mode: Option<String>,
    pub projection_schema: Option<String>,
    pub chosen_alternative: usize,
    pub decision_lane: FantasyDecisionLane,
    pub selected_sequence_id: Option<String>,
    pub projected_active_points_delta: Option<f64>,
    pub projected_net_value_delta: Option<f64>,
    pub projected_usable_starts_delta: Option<i32>,
    pub decision_state: Option<FantasyTodayState>,
    pub process: FantasyDecisionProcessAssessment,
    pub result: FantasyDecisionResultAssessment,
    pub projection: FantasyDecisionProjectionAssessment,
    pub active_points_error: Option<f64>,
    pub usable_starts_error: Option<i32>,
    pub effective_outcome: FantasyDecisionEffectiveOutcome,
    pub outcome_rows: usize,
    pub superseded_outcome_rows: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manager_rationale: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub private_outcome_notes: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDecisionCalibrationRow {
    pub kind: String,
    pub projection_schema: String,
    pub competition_mode: String,
    pub stats_season: String,
    pub decision_lane: FantasyDecisionLane,
    pub comparable_observations: usize,
    pub mean_signed_error: Option<f64>,
    pub mean_absolute_error: Option<f64>,
    pub root_mean_square_error: Option<f64>,
    pub aligned: usize,
    pub above: usize,
    pub below: usize,
    pub descriptive_ready: bool,
    pub retuning_blocked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FantasyDecisionReviewSummary {
    pub decisions: usize,
    pub with_effective_outcomes: usize,
    pub supported_process: usize,
    pub positive_results: usize,
    pub comparable_projections: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FantasyDecisionReviewView {
    pub schema: String,
    pub league_id: String,
    pub league_name: String,
    pub generated_at: DateTime<Utc>,
    pub week: Option<NaiveDate>,
    pub season: Option<String>,
    pub display_alignment_tolerance: f64,
    pub summary: FantasyDecisionReviewSummary,
    pub items: Vec<FantasyDecisionReviewItem>,
    pub calibration: Vec<FantasyDecisionCalibrationRow>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FantasyDecisionReviewError {
    #[error("week filter must be a Monday")]
    InvalidWeek,
    #[error("week and season filters cannot be combined")]
    ConflictingFilters,
    #[error("invalid fantasy decision outcome: {0}")]
    InvalidOutcome(String),
    #[error("invalid correction graph: {0}")]
    InvalidCorrection(String),
    #[error("cannot serialize fantasy decision review: {0}")]
    Serialization(String),
}

pub fn build_fantasy_decision_review(
    input: FantasyDecisionReviewBuildInput,
) -> Result<FantasyDecisionReviewView, FantasyDecisionReviewError> {
    if input
        .week
        .is_some_and(|week| week.weekday() != chrono::Weekday::Mon)
    {
        return Err(FantasyDecisionReviewError::InvalidWeek);
    }
    if input.week.is_some() && input.season.is_some() {
        return Err(FantasyDecisionReviewError::ConflictingFilters);
    }
    let mut items = Vec::new();
    let mut warnings = Vec::new();
    for decision in input.decisions {
        match build_item(decision, input.include_private) {
            Ok(item) => {
                if input.week.is_some_and(|week| item.week_start != Some(week)) {
                    continue;
                }
                if input
                    .season
                    .as_ref()
                    .is_some_and(|season| item.stats_season.as_ref() != Some(season))
                {
                    continue;
                }
                items.push(item);
            }
            Err(error) => warnings.push(error.to_string()),
        }
    }
    items.sort_by(|left, right| {
        right
            .week_start
            .cmp(&left.week_start)
            .then_with(|| right.evaluated_at.cmp(&left.evaluated_at))
            .then_with(|| left.decision_id.cmp(&right.decision_id))
    });
    let calibration = build_calibration(&items);
    let summary = FantasyDecisionReviewSummary {
        decisions: items.len(),
        with_effective_outcomes: items
            .iter()
            .filter(|item| effective_count(&item.effective_outcome) > 0)
            .count(),
        supported_process: items
            .iter()
            .filter(|item| item.process == FantasyDecisionProcessAssessment::Supported)
            .count(),
        positive_results: items
            .iter()
            .filter(|item| item.result == FantasyDecisionResultAssessment::Positive)
            .count(),
        comparable_projections: calibration
            .iter()
            .map(|row| row.comparable_observations)
            .sum(),
    };
    Ok(FantasyDecisionReviewView {
        schema: FANTASY_DECISION_REVIEW_SCHEMA.to_owned(),
        league_id: input.league_id,
        league_name: input.league_name,
        generated_at: input.generated_at,
        week: input.week,
        season: input.season,
        display_alignment_tolerance: DISPLAY_ALIGNMENT_TOLERANCE,
        summary,
        items,
        calibration,
        warnings,
    })
}

fn build_item(
    decision: FantasyDecisionStoredInput,
    include_private: bool,
) -> Result<FantasyDecisionReviewItem, FantasyDecisionReviewError> {
    let mut warnings = Vec::new();
    let projection =
        match serde_json::from_str::<FantasyPickupSequenceView>(&decision.projection_json) {
            Ok(view)
                if view.schema
                    == super::fantasy_pickup_sequence::FANTASY_PICKUP_SEQUENCE_SCHEMA =>
            {
                Some(view)
            }
            Ok(view) => {
                warnings.push(format!(
                    "unsupported frozen projection schema '{}'",
                    view.schema
                ));
                None
            }
            Err(error) => {
                warnings.push(format!("frozen projection is opaque: {error}"));
                None
            }
        };
    let selected = projection
        .as_ref()
        .and_then(|view| select_sequence(view, decision.chosen_alternative));
    if projection.is_some() && selected.is_none() {
        warnings.push("chosen alternative is absent from the frozen projection".to_owned());
    }
    let private_outcome_notes = if include_private {
        decision
            .outcomes
            .iter()
            .filter_map(|row| row.private_notes.clone())
            .collect()
    } else {
        Vec::new()
    };
    let (effective_outcome, superseded) = reduce_outcomes(&decision.id, &decision.outcomes)?;
    let actual_points = effective_outcome
        .active_value
        .as_ref()
        .and_then(|outcome| outcome.actual_active_points_delta);
    let actual_starts = effective_outcome
        .active_value
        .as_ref()
        .and_then(|outcome| outcome.actual_usable_starts_delta);
    let projected_points = selected.map(|row| row.active_points_delta);
    let projected_starts = selected.map(|row| row.incremental_usable_starts);
    let active_points_error = actual_points.zip(projected_points).map(|(a, p)| a - p);
    let usable_starts_error = actual_starts.zip(projected_starts).map(|(a, p)| a - p);
    let process = assess_process(projection.as_ref(), selected);
    let result = assess_result(actual_points, effective_outcome.matchup.as_ref());
    let projection_assessment = assess_projection(active_points_error);
    let decision_lane = projection
        .as_ref()
        .zip(selected)
        .map_or(FantasyDecisionLane::Unknown, |(view, selected)| {
            derive_lane(view, selected)
        });
    Ok(FantasyDecisionReviewItem {
        decision_id: decision.id,
        kind: decision.kind,
        recommendation_id: decision.recommendation_id,
        recommendation_fingerprint: decision.recommendation_fingerprint,
        recorded_at: decision.recorded_at,
        evaluated_at: decision.evaluated_at,
        week_start: projection.as_ref().map(|view| view.context.week_start),
        week_end: projection.as_ref().map(|view| view.context.week_end),
        stats_season: projection
            .as_ref()
            .map(|view| view.context.stats_season.clone()),
        competition_mode: projection
            .as_ref()
            .map(|view| view.context.competition_mode.clone()),
        projection_schema: projection.as_ref().map(|view| view.schema.clone()),
        chosen_alternative: decision.chosen_alternative,
        decision_lane,
        selected_sequence_id: selected.map(|row| row.sequence_id.clone()),
        projected_active_points_delta: projected_points,
        projected_net_value_delta: selected.map(|row| row.projected_value_delta),
        projected_usable_starts_delta: projected_starts,
        decision_state: projection.as_ref().map(|view| view.state),
        process,
        result,
        projection: projection_assessment,
        active_points_error,
        usable_starts_error,
        effective_outcome,
        outcome_rows: decision.outcomes.len(),
        superseded_outcome_rows: superseded,
        manager_rationale: include_private
            .then_some(decision.manager_rationale)
            .flatten(),
        private_outcome_notes,
        warnings,
    })
}

fn select_sequence(
    view: &FantasyPickupSequenceView,
    chosen: usize,
) -> Option<&FantasyPickupSequenceRow> {
    if chosen == 0 {
        Some(&view.primary_sequence)
    } else {
        view.alternatives.get(chosen - 1)
    }
}

fn reduce_outcomes(
    decision_id: &str,
    rows: &[FantasyDecisionStoredOutcomeInput],
) -> Result<(FantasyDecisionEffectiveOutcome, usize), FantasyDecisionReviewError> {
    let mut by_id = BTreeMap::new();
    let mut corrected = BTreeSet::new();
    for row in rows {
        row.outcome.validate()?;
        if row.outcome.decision_id != decision_id || by_id.insert(row.id.clone(), row).is_some() {
            return Err(FantasyDecisionReviewError::InvalidCorrection(
                "outcome decision mismatch or duplicate outcome ID".to_owned(),
            ));
        }
    }
    for row in rows {
        if let Some(parent_id) = &row.correction_of {
            let parent = by_id.get(parent_id).ok_or_else(|| {
                FantasyDecisionReviewError::InvalidCorrection(format!(
                    "outcome '{}' has missing correction parent '{parent_id}'",
                    row.id
                ))
            })?;
            if parent.outcome.lane != row.outcome.lane || !corrected.insert(parent_id.clone()) {
                return Err(FantasyDecisionReviewError::InvalidCorrection(
                    "corrections must be linear and remain in one lane".to_owned(),
                ));
            }
        }
    }
    for row in rows {
        let mut seen = BTreeSet::new();
        let mut cursor = Some(row);
        while let Some(current) = cursor {
            if !seen.insert(current.id.as_str()) {
                return Err(FantasyDecisionReviewError::InvalidCorrection(
                    "correction chain contains a cycle".to_owned(),
                ));
            }
            cursor = current
                .correction_of
                .as_ref()
                .and_then(|parent| by_id.get(parent).copied());
        }
    }
    let mut leaves =
        BTreeMap::<FantasyDecisionOutcomeLane, &FantasyDecisionStoredOutcomeInput>::new();
    for row in rows.iter().filter(|row| !corrected.contains(&row.id)) {
        if leaves.insert(row.outcome.lane, row).is_some() {
            return Err(FantasyDecisionReviewError::InvalidCorrection(format!(
                "{} lane has multiple effective leaves",
                row.outcome.lane.as_str()
            )));
        }
    }
    let effective = FantasyDecisionEffectiveOutcome {
        execution: leaves
            .remove(&FantasyDecisionOutcomeLane::Execution)
            .map(|row| row.outcome.clone()),
        active_value: leaves
            .remove(&FantasyDecisionOutcomeLane::ActiveValue)
            .map(|row| row.outcome.clone()),
        matchup: leaves
            .remove(&FantasyDecisionOutcomeLane::Matchup)
            .map(|row| row.outcome.clone()),
        reserve: leaves
            .remove(&FantasyDecisionOutcomeLane::Reserve)
            .map(|row| row.outcome.clone()),
    };
    Ok((effective, corrected.len()))
}

fn assess_process(
    view: Option<&FantasyPickupSequenceView>,
    selected: Option<&FantasyPickupSequenceRow>,
) -> FantasyDecisionProcessAssessment {
    let (Some(view), Some(selected)) = (view, selected) else {
        return FantasyDecisionProcessAssessment::InsufficientEvidence;
    };
    if selected.projected_value_delta < 0.0 {
        return FantasyDecisionProcessAssessment::Unsupported;
    }
    if view.state != FantasyTodayState::Ready
        || selected.moves.iter().any(|row| row.firmness != "firm")
    {
        FantasyDecisionProcessAssessment::InsufficientEvidence
    } else {
        FantasyDecisionProcessAssessment::Supported
    }
}

fn assess_result(
    actual_points: Option<f64>,
    matchup: Option<&FantasyDecisionOutcome>,
) -> FantasyDecisionResultAssessment {
    if let Some(value) = actual_points {
        return if value > 0.0 {
            FantasyDecisionResultAssessment::Positive
        } else if value < 0.0 {
            FantasyDecisionResultAssessment::Negative
        } else {
            FantasyDecisionResultAssessment::Neutral
        };
    }
    match matchup.and_then(|outcome| outcome.matchup_result) {
        Some(FantasyDecisionMatchupResult::Win) => FantasyDecisionResultAssessment::Positive,
        Some(FantasyDecisionMatchupResult::Loss) => FantasyDecisionResultAssessment::Negative,
        Some(FantasyDecisionMatchupResult::Tie) => FantasyDecisionResultAssessment::Neutral,
        None => FantasyDecisionResultAssessment::Unknown,
    }
}

fn assess_projection(error: Option<f64>) -> FantasyDecisionProjectionAssessment {
    match error {
        Some(error) if error > DISPLAY_ALIGNMENT_TOLERANCE => {
            FantasyDecisionProjectionAssessment::Above
        }
        Some(error) if error < -DISPLAY_ALIGNMENT_TOLERANCE => {
            FantasyDecisionProjectionAssessment::Below
        }
        Some(_) => FantasyDecisionProjectionAssessment::Aligned,
        None => FantasyDecisionProjectionAssessment::Unknown,
    }
}

fn derive_lane(
    _view: &FantasyPickupSequenceView,
    selected: &FantasyPickupSequenceRow,
) -> FantasyDecisionLane {
    if selected.moves.is_empty() {
        return FantasyDecisionLane::NoMove;
    }
    let goalies = selected
        .moves
        .iter()
        .flat_map(|row| &row.add_positions)
        .any(|position| *position == Position::Goalie);
    let skaters = selected
        .moves
        .iter()
        .flat_map(|row| &row.add_positions)
        .any(|position| *position != Position::Goalie);
    match (goalies, skaters) {
        (true, false) => FantasyDecisionLane::GoalieOnly,
        (false, true) => FantasyDecisionLane::SkaterOnly,
        (true, true) => FantasyDecisionLane::Mixed,
        (false, false) => FantasyDecisionLane::Unknown,
    }
}

fn effective_count(outcome: &FantasyDecisionEffectiveOutcome) -> usize {
    [
        outcome.execution.is_some(),
        outcome.active_value.is_some(),
        outcome.matchup.is_some(),
        outcome.reserve.is_some(),
    ]
    .into_iter()
    .filter(|present| *present)
    .count()
}

fn build_calibration(items: &[FantasyDecisionReviewItem]) -> Vec<FantasyDecisionCalibrationRow> {
    type Key = (String, String, String, String, FantasyDecisionLane);
    let mut groups = BTreeMap::<Key, Vec<(f64, FantasyDecisionProjectionAssessment)>>::new();
    for item in items {
        let (Some(schema), Some(mode), Some(season), Some(error)) = (
            item.projection_schema.clone(),
            item.competition_mode.clone(),
            item.stats_season.clone(),
            item.active_points_error,
        ) else {
            continue;
        };
        if item
            .effective_outcome
            .active_value
            .as_ref()
            .is_none_or(|outcome| outcome.completeness != FantasyDecisionOutcomeCompleteness::Final)
        {
            continue;
        }
        if item.decision_lane == FantasyDecisionLane::Unknown {
            continue;
        }
        groups
            .entry((item.kind.clone(), schema, mode, season, item.decision_lane))
            .or_default()
            .push((error, item.projection));
    }
    groups
        .into_iter()
        .map(|((kind, schema, mode, season, lane), values)| {
            let n = values.len();
            let signed = values.iter().map(|(error, _)| error).sum::<f64>() / n as f64;
            let absolute = values.iter().map(|(error, _)| error.abs()).sum::<f64>() / n as f64;
            let rms =
                (values.iter().map(|(error, _)| error * error).sum::<f64>() / n as f64).sqrt();
            FantasyDecisionCalibrationRow {
                kind,
                projection_schema: schema,
                competition_mode: mode,
                stats_season: season,
                decision_lane: lane,
                comparable_observations: n,
                mean_signed_error: Some(signed),
                mean_absolute_error: Some(absolute),
                root_mean_square_error: Some(rms),
                aligned: values
                    .iter()
                    .filter(|(_, state)| *state == FantasyDecisionProjectionAssessment::Aligned)
                    .count(),
                above: values
                    .iter()
                    .filter(|(_, state)| *state == FantasyDecisionProjectionAssessment::Above)
                    .count(),
                below: values
                    .iter()
                    .filter(|(_, state)| *state == FantasyDecisionProjectionAssessment::Below)
                    .count(),
                descriptive_ready: n >= DESCRIPTIVE_READY_MINIMUM,
                retuning_blocked: true,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::{
        build_fantasy_pickup_sequence, FantasyAssistantRules, FantasyPickupSequenceContext,
        FantasyPickupSequenceInput, FantasyWeekBudgetView,
    };
    use chrono::TimeZone;

    use super::*;

    fn at(day: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, day, 12, 0, 0)
            .single()
            .expect("fixture instant")
    }

    fn outcome(lane: FantasyDecisionOutcomeLane) -> FantasyDecisionOutcome {
        FantasyDecisionOutcome {
            schema: FANTASY_DECISION_OUTCOME_SCHEMA.to_owned(),
            decision_id: "decision".to_owned(),
            lane,
            completeness: FantasyDecisionOutcomeCompleteness::Final,
            source: FantasyDecisionOutcomeSource::Manager,
            source_observed_at: Some(at(16)),
            executed: (lane == FantasyDecisionOutcomeLane::Execution).then_some(true),
            actual_active_points_delta: (lane == FantasyDecisionOutcomeLane::ActiveValue)
                .then_some(5.0),
            actual_usable_starts_delta: None,
            matchup_result: (lane == FantasyDecisionOutcomeLane::Matchup)
                .then_some(FantasyDecisionMatchupResult::Win),
            user_final_points: None,
            opponent_final_points: None,
            reserve_needed: (lane == FantasyDecisionOutcomeLane::Reserve).then_some(false),
            reserve_used: None,
        }
    }

    #[test]
    fn l0_outcome_lanes_reject_cross_lane_fields_and_non_finite_values() {
        let mut row = outcome(FantasyDecisionOutcomeLane::Execution);
        row.actual_active_points_delta = Some(1.0);
        assert!(row.validate().is_err());
        let mut row = outcome(FantasyDecisionOutcomeLane::ActiveValue);
        row.actual_active_points_delta = Some(f64::NAN);
        assert!(row.validate().is_err());
    }

    #[test]
    fn l0_projection_alignment_honors_exact_display_boundary() {
        assert_eq!(
            assess_projection(Some(-1.0)),
            FantasyDecisionProjectionAssessment::Aligned
        );
        assert_eq!(
            assess_projection(Some(1.0)),
            FantasyDecisionProjectionAssessment::Aligned
        );
        assert_eq!(
            assess_projection(Some(1.01)),
            FantasyDecisionProjectionAssessment::Above
        );
    }

    #[test]
    fn l0_final_value_waits_for_the_frozen_week_to_close() {
        let monday = NaiveDate::from_ymd_opt(2026, 11, 9).unwrap();
        let projection = build_fantasy_pickup_sequence(FantasyPickupSequenceInput {
            context: FantasyPickupSequenceContext {
                league_id: "league".to_owned(),
                league_name: "League".to_owned(),
                fantasy_team_id: "team".to_owned(),
                fantasy_team_name: "Team".to_owned(),
                stats_season: "20262027".to_owned(),
                season_type: crate::season_stats::SeasonType::Regular,
                competition_mode: "points".to_owned(),
                week_start: monday,
                week_end: monday + chrono::Duration::days(6),
                timezone: "America/Los_Angeles".to_owned(),
                generated_at: at(9),
                evaluated_at: at(9),
            },
            rules: FantasyAssistantRules::configured_2026(),
            budget: FantasyWeekBudgetView {
                schema: "fantasy_week_budget.v1".to_owned(),
                timezone: "America/Los_Angeles".to_owned(),
                week_start: monday,
                week_end: monday + chrono::Duration::days(6),
                acquisition_limit: 4,
                acquisitions_used: 0,
                acquisitions_remaining: 4,
                can_add: true,
                injury_reserve: 1,
                injury_reserve_active: 1,
                proactive_acquisitions_remaining: 3,
                can_proactively_add: true,
                injury_reserve_releases_on: None,
            },
            players: Vec::new(),
            transitions: Vec::new(),
            max_moves: 3,
            beam_width: 20,
            alternative_limit: 3,
            readiness: Vec::new(),
            evidence: Vec::new(),
        })
        .unwrap();
        let mut value = outcome(FantasyDecisionOutcomeLane::ActiveValue);
        value.source_observed_at = Some(at(15));
        assert!(validate_fantasy_decision_outcome_timing(&value, &projection, at(15)).is_err());
        value.completeness = FantasyDecisionOutcomeCompleteness::Provisional;
        assert!(validate_fantasy_decision_outcome_timing(&value, &projection, at(15)).is_ok());
        value.completeness = FantasyDecisionOutcomeCompleteness::Final;
        value.source_observed_at = Some(at(17));
        assert!(validate_fantasy_decision_outcome_timing(&value, &projection, at(17)).is_ok());
    }

    #[test]
    fn l0_correction_graph_rejects_branches_and_cross_lane_edges() {
        let root = FantasyDecisionStoredOutcomeInput {
            id: "root".to_owned(),
            observed_at: at(16),
            outcome: outcome(FantasyDecisionOutcomeLane::ActiveValue),
            correction_of: None,
            private_notes: None,
        };
        let mut child = root.clone();
        child.id = "child".to_owned();
        child.correction_of = Some("root".to_owned());
        let mut branch = child.clone();
        branch.id = "branch".to_owned();
        assert!(reduce_outcomes("decision", &[root.clone(), child, branch]).is_err());

        let mut wrong_lane = root.clone();
        wrong_lane.id = "wrong".to_owned();
        wrong_lane.correction_of = Some("root".to_owned());
        wrong_lane.outcome = outcome(FantasyDecisionOutcomeLane::Reserve);
        assert!(reduce_outcomes("decision", &[root, wrong_lane]).is_err());
    }

    #[test]
    fn l0_correction_graph_rejects_cycles() {
        let mut first = FantasyDecisionStoredOutcomeInput {
            id: "first".to_owned(),
            observed_at: at(16),
            outcome: outcome(FantasyDecisionOutcomeLane::Execution),
            correction_of: Some("second".to_owned()),
            private_notes: None,
        };
        let mut second = first.clone();
        second.id = "second".to_owned();
        second.correction_of = Some("first".to_owned());
        first.outcome.executed = Some(true);
        second.outcome.executed = Some(false);
        assert!(reduce_outcomes("decision", &[first, second]).is_err());
    }

    #[test]
    fn l0_calibration_uses_documented_signed_absolute_and_rms_formulas() {
        let item = |id: &str, error: f64| FantasyDecisionReviewItem {
            decision_id: id.to_owned(),
            kind: "week_plan".to_owned(),
            recommendation_id: id.to_owned(),
            recommendation_fingerprint: id.to_owned(),
            recorded_at: at(9),
            evaluated_at: at(9),
            week_start: None,
            week_end: None,
            stats_season: Some("20262027".to_owned()),
            competition_mode: Some("points".to_owned()),
            projection_schema: Some("fantasy_pickup_sequence.v1".to_owned()),
            chosen_alternative: 0,
            decision_lane: FantasyDecisionLane::NoMove,
            selected_sequence_id: Some(id.to_owned()),
            projected_active_points_delta: Some(0.0),
            projected_net_value_delta: Some(0.0),
            projected_usable_starts_delta: Some(0),
            decision_state: Some(FantasyTodayState::Ready),
            process: FantasyDecisionProcessAssessment::Supported,
            result: FantasyDecisionResultAssessment::Positive,
            projection: assess_projection(Some(error)),
            active_points_error: Some(error),
            usable_starts_error: None,
            effective_outcome: FantasyDecisionEffectiveOutcome {
                execution: None,
                active_value: Some(outcome(FantasyDecisionOutcomeLane::ActiveValue)),
                matchup: None,
                reserve: None,
            },
            outcome_rows: 1,
            superseded_outcome_rows: 0,
            manager_rationale: None,
            private_outcome_notes: Vec::new(),
            warnings: Vec::new(),
        };
        let rows = build_calibration(&[item("a", -2.0), item("b", 4.0)]);
        assert_eq!(rows.len(), 1);
        // Errors -2 and +4: bias = 1, MAE = 3, RMSE = sqrt(10).
        assert!((rows[0].mean_signed_error.unwrap() - 1.0).abs() < 1e-12);
        assert!((rows[0].mean_absolute_error.unwrap() - 3.0).abs() < 1e-12);
        assert!((rows[0].root_mean_square_error.unwrap() - 10_f64.sqrt()).abs() < 1e-12);
        assert!(!rows[0].descriptive_ready);
        assert!(rows[0].retuning_blocked);
    }
}
