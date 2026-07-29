//! Sealed official standings authority for historical Window evaluation.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{Datelike, NaiveDate};
use icelines_core::{
    build_organization_window_board, calibrate_organization_window,
    load_organization_window_profile_inventory, load_organization_window_registry_lifecycle,
    seal_new_organization_window_manifest, OrganizationProfileInput, OrganizationWindowBoardInput,
    OrganizationWindowBoardView, OrganizationWindowCalibrationView, OrganizationWindowManifestView,
    WindowCalibrationEvaluationOriginInput, WindowCalibrationOriginInput,
    WindowCalibrationOriginRole, WindowCohortKind, WindowCohortManifest, WindowDimensionManifest,
    WindowEvidenceView, WindowFreshness, WindowHorizon, WindowLeakageAuditRow,
    WindowManifestLifecyclePolicy, WindowMissingPolicy, WindowNormalizationMethod,
    WindowOutcomeRow, WindowProfileStatus, WindowProfileWeight, WindowTrialNoiseInput,
    CANONICAL_TEAMS, ORGANIZATION_WINDOW_CLASSIFICATION_METHOD,
    ORGANIZATION_WINDOW_MANIFEST_SCHEMA,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::{
    nhl_api::NhlStandingsRow,
    schema::{SkaterBio, SkaterStats},
};

pub const ORGANIZATION_WINDOW_STANDINGS_SCHEMA: &str = "organization_window_standings_snapshot.v1";
pub const ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION: &str = "nhl_franchise_continuity_32.v1";
pub const NHL_STANDINGS_SOURCE_BASE: &str = "https://api-web.nhle.com/v1/standings";
pub const ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA: &str =
    "organization_window_historical_origin.v1";
pub const ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/organization_window_historical_origin.v1.schema.json");
pub const ORGANIZATION_WINDOW_FUTURE_HOLDOUT_REGISTRATION_SCHEMA: &str =
    "organization_window_future_holdout_registration.v1";
pub const ORGANIZATION_WINDOW_FUTURE_HOLDOUT_REGISTRATION_JSON_SCHEMA: &str = include_str!(
    "../../design/schemas/organization_window_future_holdout_registration.v1.schema.json"
);
pub const ORGANIZATION_WINDOW_FUTURE_HOLDOUT_RESULT_SCHEMA: &str =
    "organization_window_future_holdout_result.v1";
pub const ORGANIZATION_WINDOW_FUTURE_HOLDOUT_RESULT_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/organization_window_future_holdout_result.v1.schema.json");
pub const ORGANIZATION_WINDOW_STANDINGS_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/organization_window_standings_snapshot.v1.schema.json");
pub const ORGANIZATION_WINDOW_HISTORICAL_MANIFEST_ID: &str = "observed_history.v1";
const HISTORICAL_SOURCE_SCHEMA: &str = "bundled_nhl_season_stats.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowStandingRow {
    /// Stable Window organization identity.
    pub organization: String,
    /// Abbreviation reported by the NHL endpoint at the historical date.
    pub observed_team: String,
    pub games_played: u32,
    pub wins: u32,
    pub losses: u32,
    pub overtime_losses: u32,
    pub points: u32,
    pub points_percentage: f64,
    pub goal_differential: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowStandingsSnapshot {
    pub schema: String,
    pub target_season: u32,
    pub effective_date: NaiveDate,
    pub captured_at: String,
    pub source_url: String,
    /// SHA-256 of the sorted, normalized standings projection used here.
    pub source_projection_fingerprint: String,
    pub organization_identity_version: String,
    pub rows: Vec<OrganizationWindowStandingRow>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl OrganizationWindowStandingsSnapshot {
    pub fn outcomes(&self) -> Vec<WindowOutcomeRow> {
        let mut order = self.rows.iter().collect::<Vec<_>>();
        order.sort_by(|left, right| {
            right
                .points_percentage
                .total_cmp(&left.points_percentage)
                .then_with(|| left.organization.cmp(&right.organization))
        });
        let mut percentiles = BTreeMap::new();
        let mut start = 0;
        while start < order.len() {
            let mut end = start + 1;
            while end < order.len()
                && order[end].points_percentage == order[start].points_percentage
            {
                end += 1;
            }
            let average_rank = (start + 1 + end) as f64 / 2.0;
            let percentile = if order.len() == 1 {
                50.0
            } else {
                100.0 * (order.len() as f64 - average_rank) / (order.len() as f64 - 1.0)
            };
            for row in &order[start..end] {
                percentiles.insert(row.organization.as_str(), percentile);
            }
            start = end;
        }
        self.rows
            .iter()
            .map(|row| WindowOutcomeRow {
                organization: row.organization.clone(),
                target_value: percentiles[row.organization.as_str()],
            })
            .collect()
    }

    pub fn validate(&self) -> Result<(), OrganizationWindowHistoryError> {
        if self.schema != ORGANIZATION_WINDOW_STANDINGS_SCHEMA
            || self.rows.len() != CANONICAL_TEAMS.len()
            || self.fingerprint != snapshot_fingerprint(self)?
        {
            return Err(OrganizationWindowHistoryError::InvalidStandingsSnapshot);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowHistoricalOriginArtifact {
    pub schema: String,
    pub source_season: u32,
    pub target_season: u32,
    pub as_of: NaiveDate,
    pub role: WindowCalibrationOriginRole,
    pub origin: WindowCalibrationOriginInput,
    pub source_fingerprints: Vec<String>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowHoldoutAcceptanceRule {
    pub minimum_sample_size: usize,
    pub require_mae_below_baseline: bool,
    pub minimum_rank_correlation: f64,
    pub require_complete_leakage_audit: bool,
}

/// Outcome-free commitment for one genuinely future Window holdout.
///
/// The complete feature board, baseline, target, cutoff, and acceptance rule
/// are sealed before the target outcome is observable. Outcomes deliberately
/// have no field in this document.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowFutureHoldoutRegistration {
    pub schema: String,
    pub registration_id: String,
    pub registered_at: String,
    pub source_season: u32,
    pub target_season: u32,
    pub feature_cutoff: NaiveDate,
    pub outcome_not_before: NaiveDate,
    pub target: String,
    pub origin_id: String,
    pub board: OrganizationWindowBoardView,
    pub leakage_audit: Vec<WindowLeakageAuditRow>,
    pub baseline_value: f64,
    pub acceptance: OrganizationWindowHoldoutAcceptanceRule,
    pub source_fingerprints: Vec<String>,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl OrganizationWindowFutureHoldoutRegistration {
    pub fn validate(&self) -> Result<(), OrganizationWindowHistoryError> {
        let inventory = load_organization_window_profile_inventory()
            .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
        let registered_date = chrono::DateTime::parse_from_rfc3339(&self.registered_at)
            .map(|timestamp| timestamp.date_naive())
            .map_err(|_| OrganizationWindowHistoryError::InvalidFutureHoldoutRegistration)?;
        let expected_audit = self
            .board
            .manifest
            .dimensions
            .iter()
            .flat_map(|dimension| &dimension.profiles)
            .map(|profile| {
                (
                    profile.profile_key.as_str(),
                    profile.method_version.as_str(),
                )
            })
            .collect::<BTreeSet<_>>();
        let actual_audit = self
            .leakage_audit
            .iter()
            .map(|row| (row.profile_key.as_str(), row.method_version.as_str()))
            .collect::<BTreeSet<_>>();
        let mut expected_sources = self
            .board
            .source_fingerprints
            .iter()
            .filter(|fingerprint| fingerprint.starts_with("sha256:"))
            .cloned()
            .collect::<Vec<_>>();
        expected_sources.sort();
        let mut actual_sources = self.source_fingerprints.clone();
        actual_sources.sort();
        if self.schema != ORGANIZATION_WINDOW_FUTURE_HOLDOUT_REGISTRATION_SCHEMA
            || self.registration_id
                != format!(
                    "future-holdout:{}-to-{}",
                    self.source_season, self.target_season
                )
            || self.source_season % 10_000 != self.target_season / 10_000
            || self.feature_cutoff >= self.outcome_not_before
            || registered_date < self.feature_cutoff
            || registered_date >= self.outcome_not_before
            || self.target != "next-season-final-standings-percentile"
            || self.origin_id != format!("{}-to-{}", self.source_season, self.target_season)
            || self.board.season != self.target_season
            || self.board.as_of != self.feature_cutoff
            || self.board.generated_at != self.registered_at
            || self.board.manifest.manifest_id != ORGANIZATION_WINDOW_HISTORICAL_MANIFEST_ID
            || self.board.organizations.len() != CANONICAL_TEAMS.len()
            || self
                .board
                .organizations
                .iter()
                .any(|organization| organization.overall.rank.is_none())
            || expected_audit != actual_audit
            || expected_audit.len() != self.leakage_audit.len()
            || self
                .leakage_audit
                .iter()
                .any(|row| !row.point_in_time_safe || row.evidence.trim().is_empty())
            || self.baseline_value != 50.0
            || self.acceptance.minimum_sample_size != CANONICAL_TEAMS.len()
            || !self.acceptance.require_mae_below_baseline
            || self.acceptance.minimum_rank_correlation != 0.3
            || !self.acceptance.require_complete_leakage_audit
            || self.source_fingerprints.len() < 2
            || expected_sources != actual_sources
            || self.disclosures.iter().any(|row| row.trim().is_empty())
            || self.fingerprint != future_holdout_registration_fingerprint(self)?
        {
            return Err(OrganizationWindowHistoryError::InvalidFutureHoldoutRegistration);
        }
        icelines_core::validate_organization_window_board(&self.board, &inventory)
            .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowFutureHoldoutResult {
    pub schema: String,
    pub registration: OrganizationWindowFutureHoldoutRegistration,
    pub standings: OrganizationWindowStandingsSnapshot,
    pub scored_at: String,
    pub calibration: OrganizationWindowCalibrationView,
    pub acceptance_passed: bool,
    pub disclosures: Vec<String>,
    pub fingerprint: String,
}

impl OrganizationWindowFutureHoldoutResult {
    pub fn validate(&self) -> Result<(), OrganizationWindowHistoryError> {
        self.registration.validate()?;
        self.standings.validate()?;
        let scored_date = chrono::DateTime::parse_from_rfc3339(&self.scored_at)
            .map(|timestamp| timestamp.date_naive())
            .map_err(|_| OrganizationWindowHistoryError::InvalidFutureHoldoutResult)?;
        let captured_date = chrono::DateTime::parse_from_rfc3339(&self.standings.captured_at)
            .map(|timestamp| timestamp.date_naive())
            .map_err(|_| OrganizationWindowHistoryError::InvalidFutureHoldoutResult)?;
        let outcomes = self.standings.outcomes();
        let expected = calibrate_organization_window(
            &self.registration.target,
            &self.registration.board,
            &outcomes,
            &self.registration.leakage_audit,
        )
        .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
        let registered_baseline_mae = outcomes
            .iter()
            .map(|row| (row.target_value - self.registration.baseline_value).abs())
            .sum::<f64>()
            / outcomes.len() as f64;
        let expected_acceptance = holdout_acceptance_passed(&self.registration, &expected);
        if self.schema != ORGANIZATION_WINDOW_FUTURE_HOLDOUT_RESULT_SCHEMA
            || self.standings.target_season != self.registration.target_season
            || self.standings.effective_date < self.registration.outcome_not_before
            || captured_date < self.standings.effective_date
            || scored_date < captured_date
            || self.calibration != expected
            || (self.calibration.overall.baseline_mean_absolute_error - registered_baseline_mae)
                .abs()
                > 1e-9
            || self.acceptance_passed != expected_acceptance
            || self.disclosures.iter().any(|row| row.trim().is_empty())
            || self.fingerprint != future_holdout_result_fingerprint(self)?
        {
            return Err(OrganizationWindowHistoryError::InvalidFutureHoldoutResult);
        }
        Ok(())
    }
}

impl OrganizationWindowHistoricalOriginArtifact {
    pub fn evaluation_input(&self) -> WindowCalibrationEvaluationOriginInput {
        WindowCalibrationEvaluationOriginInput {
            role: self.role,
            origin: self.origin.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), OrganizationWindowHistoryError> {
        let inventory = load_organization_window_profile_inventory()
            .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
        if self.schema != ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA
            || self.source_season % 10_000 != self.target_season / 10_000
            || self.origin.board.season != self.target_season
            || self.origin.board.as_of != self.as_of
            || self.source_fingerprints.len() < 3
            || self.fingerprint != historical_origin_fingerprint(self)?
        {
            return Err(OrganizationWindowHistoryError::InvalidHistoricalOrigin);
        }
        icelines_core::validate_organization_window_board(&self.origin.board, &inventory)
            .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowHistoryError {
    #[error("historical standings capture timestamp is empty")]
    EmptyCapturedAt,
    #[error("historical standings cohort does not match the canonical 32 organizations")]
    InvalidCohort,
    #[error("historical standings contains duplicate organization {0}")]
    DuplicateOrganization(String),
    #[error("historical standings row is invalid for {0}")]
    InvalidStanding(String),
    #[error("historical standings serialization failed: {0}")]
    Serialization(String),
    #[error("historical standings snapshot is invalid or its fingerprint does not match")]
    InvalidStandingsSnapshot,
    #[error("historical Window source and target seasons are not consecutive")]
    NonConsecutiveSeasons,
    #[error("historical Window outcome is not after the feature cutoff")]
    OutcomeBeforeFeatureCutoff,
    #[error("historical Window source data is incomplete: {0}")]
    IncompleteSource(String),
    #[error("historical Window board construction failed: {0}")]
    Board(String),
    #[error("historical Window origin is invalid or its fingerprint does not match")]
    InvalidHistoricalOrigin,
    #[error("future Window holdout registration is invalid or its fingerprint does not match")]
    InvalidFutureHoldoutRegistration,
    #[error("future Window holdout result is invalid, early, or its fingerprint does not match")]
    InvalidFutureHoldoutResult,
}

#[derive(Debug, Default)]
struct HistoricalTeamAggregate {
    included_player_games: u64,
    excluded_multi_team_player_games: u64,
    points: u64,
    power_play_points: Vec<u32>,
    player_points: Vec<u32>,
    contributors: u64,
    birth_dated_points: u64,
    young_points: u64,
    players: u64,
}

struct HistoricalFeatureBundle {
    board: OrganizationWindowBoardView,
    leakage_audit: Vec<WindowLeakageAuditRow>,
    stats_fingerprint: String,
    bios_fingerprint: String,
}

pub fn build_organization_window_standings_snapshot(
    target_season: u32,
    effective_date: NaiveDate,
    captured_at: &str,
    standings: &[NhlStandingsRow],
) -> Result<OrganizationWindowStandingsSnapshot, OrganizationWindowHistoryError> {
    if captured_at.trim().is_empty() {
        return Err(OrganizationWindowHistoryError::EmptyCapturedAt);
    }
    let mut rows = standings
        .iter()
        .map(|standing| OrganizationWindowStandingRow {
            organization: historical_franchise_organization(&standing.team),
            observed_team: standing.team.to_ascii_uppercase(),
            games_played: standing.games_played,
            wins: standing.wins,
            losses: standing.losses,
            overtime_losses: standing.overtime_losses,
            points: standing.points,
            points_percentage: f64::from(standing.points_percentage),
            goal_differential: standing.goal_differential,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.organization.cmp(&right.organization));

    let mut organizations = BTreeSet::new();
    for row in &rows {
        if !organizations.insert(row.organization.as_str()) {
            return Err(OrganizationWindowHistoryError::DuplicateOrganization(
                row.organization.clone(),
            ));
        }
        let decisions = row.wins + row.losses + row.overtime_losses;
        let calculated_percentage = row.points as f64 / (row.games_played * 2) as f64;
        if row.games_played == 0
            || decisions != row.games_played
            || !row.points_percentage.is_finite()
            || !(0.0..=1.0).contains(&row.points_percentage)
            || row.points != row.wins * 2 + row.overtime_losses
            || (row.points_percentage - calculated_percentage).abs() > 0.000_01
        {
            return Err(OrganizationWindowHistoryError::InvalidStanding(
                row.observed_team.clone(),
            ));
        }
    }
    let expected = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| *team)
        .collect::<BTreeSet<_>>();
    if organizations != expected {
        return Err(OrganizationWindowHistoryError::InvalidCohort);
    }

    let source_projection_fingerprint = sha256_json(&rows)?;
    let mut snapshot = OrganizationWindowStandingsSnapshot {
        schema: ORGANIZATION_WINDOW_STANDINGS_SCHEMA.to_owned(),
        target_season,
        effective_date,
        captured_at: captured_at.to_owned(),
        source_url: format!("{NHL_STANDINGS_SOURCE_BASE}/{effective_date}"),
        source_projection_fingerprint,
        organization_identity_version: ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION.to_owned(),
        rows,
        disclosures: vec![
            "The source retains final official regular-season point percentage; calibration outcomes transform it to an empirical 0..100 league percentile matching the Window score scale.".to_owned(),
            "ARI and PHX observations map to the stable UTA franchise identity; observed_team preserves the historical abbreviation.".to_owned(),
            "The source projection fingerprint seals normalized parsed rows, not the byte-for-byte HTTP response.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    snapshot.fingerprint = snapshot_fingerprint(&snapshot)?;
    Ok(snapshot)
}

pub fn historical_franchise_organization(team: &str) -> String {
    match team.trim().to_ascii_uppercase().as_str() {
        "ARI" | "PHX" => "UTA".to_owned(),
        normalized => normalized.to_owned(),
    }
}

pub fn historical_organization_window_manifest(
    created_at: &str,
) -> Result<OrganizationWindowManifestView, OrganizationWindowHistoryError> {
    let inventory = load_organization_window_profile_inventory()
        .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
    let expected_organizations = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| (*team).to_owned())
        .collect::<Vec<_>>();
    let manifest = OrganizationWindowManifestView {
        schema: ORGANIZATION_WINDOW_MANIFEST_SCHEMA.to_owned(),
        manifest_id: ORGANIZATION_WINDOW_HISTORICAL_MANIFEST_ID.to_owned(),
        label: "Observed historical organization Window".to_owned(),
        description: "A retrospective point-in-time Frame built only from prior-season bundled NHL skater facts.".to_owned(),
        manifest_version: "1.0.0".to_owned(),
        comparison_cohort: WindowCohortManifest {
            kind: WindowCohortKind::SeasonCanonical,
            team_catalog_version: ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION.to_owned(),
            expected_organizations,
        },
        normalization_method: WindowNormalizationMethod::EmpiricalPercentile,
        primary_horizon: WindowHorizon::OneYear,
        dimensions: vec![
            historical_dimension(
                "nhl_strength",
                "NHL strength",
                0.35,
                "history.nhl_skater_scoring",
                "historical_observed_skater_scoring.v1",
            ),
            historical_dimension(
                "deployment",
                "Deployment",
                0.15,
                "history.power_play_breadth",
                "historical_observed_power_play_breadth.v1",
            ),
            historical_dimension(
                "pipeline",
                "Pipeline",
                0.20,
                "history.young_contribution_share",
                "historical_observed_young_share.v1",
            ),
            historical_dimension(
                "development_system",
                "Development system",
                0.15,
                "history.contributor_depth",
                "historical_observed_contributor_depth.v1",
            ),
            historical_dimension(
                "resilience",
                "Resilience",
                0.15,
                "history.roster_concentration",
                "historical_observed_roster_concentration.v1",
            ),
        ],
        missing_policy: WindowMissingPolicy::WithholdRank,
        classification_method: ORGANIZATION_WINDOW_CLASSIFICATION_METHOD.to_owned(),
        created_at: created_at.to_owned(),
        fingerprint: String::new(),
    };
    let lifecycle = load_organization_window_registry_lifecycle(&inventory)
        .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
    seal_new_organization_window_manifest(
        manifest,
        &inventory,
        &lifecycle,
        &WindowManifestLifecyclePolicy::evaluation(),
    )
    .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
pub fn build_historical_organization_window_origin(
    source_season: u32,
    target_season: u32,
    as_of: NaiveDate,
    generated_at: &str,
    role: WindowCalibrationOriginRole,
    stats: &[SkaterStats],
    bios: &[SkaterBio],
    outcomes: &OrganizationWindowStandingsSnapshot,
) -> Result<OrganizationWindowHistoricalOriginArtifact, OrganizationWindowHistoryError> {
    outcomes.validate()?;
    if outcomes.target_season != target_season {
        return Err(OrganizationWindowHistoryError::IncompleteSource(format!(
            "standings target {} does not match {target_season}",
            outcomes.target_season
        )));
    }
    if outcomes.effective_date <= as_of {
        return Err(OrganizationWindowHistoryError::OutcomeBeforeFeatureCutoff);
    }
    let features = build_historical_feature_bundle(
        source_season,
        target_season,
        as_of,
        generated_at,
        stats,
        bios,
    )?;
    let outcome_fingerprint = format!("sha256:{}", outcomes.fingerprint);
    let origin = WindowCalibrationOriginInput {
        origin_id: format!("{source_season}-to-{target_season}"),
        board: features.board,
        outcomes: outcomes.outcomes(),
        leakage_audit: features.leakage_audit,
        baseline_value: 50.0,
        trial_noise: WindowTrialNoiseInput::NotApplicable,
    };
    let mut artifact = OrganizationWindowHistoricalOriginArtifact {
        schema: ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA.to_owned(),
        source_season,
        target_season,
        as_of,
        role,
        origin,
        source_fingerprints: vec![
            features.stats_fingerprint,
            features.bios_fingerprint,
            outcome_fingerprint,
        ],
        disclosures: vec![
            "This is a retrospective observed Frame and is separate from the current balanced.v1 descriptive Frame.".to_owned(),
            "Features use only the completed source season; final target-season standings are attached after the feature cutoff.".to_owned(),
            "The frozen comparison baseline is the neutral 50.0 league percentile for every organization.".to_owned(),
            "Multi-team aggregate skater rows are omitted from team profiles rather than allocated without stint-level authority.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    artifact.fingerprint = historical_origin_fingerprint(&artifact)?;
    Ok(artifact)
}

#[allow(clippy::too_many_arguments)]
pub fn build_organization_window_future_holdout_registration(
    source_season: u32,
    target_season: u32,
    feature_cutoff: NaiveDate,
    outcome_not_before: NaiveDate,
    registered_at: &str,
    stats: &[SkaterStats],
    bios: &[SkaterBio],
) -> Result<OrganizationWindowFutureHoldoutRegistration, OrganizationWindowHistoryError> {
    if feature_cutoff >= outcome_not_before
        || chrono::DateTime::parse_from_rfc3339(registered_at).is_err()
    {
        return Err(OrganizationWindowHistoryError::InvalidFutureHoldoutRegistration);
    }
    let features = build_historical_feature_bundle(
        source_season,
        target_season,
        feature_cutoff,
        registered_at,
        stats,
        bios,
    )?;
    let mut registration = OrganizationWindowFutureHoldoutRegistration {
        schema: ORGANIZATION_WINDOW_FUTURE_HOLDOUT_REGISTRATION_SCHEMA.to_owned(),
        registration_id: format!("future-holdout:{source_season}-to-{target_season}"),
        registered_at: registered_at.to_owned(),
        source_season,
        target_season,
        feature_cutoff,
        outcome_not_before,
        target: "next-season-final-standings-percentile".to_owned(),
        origin_id: format!("{source_season}-to-{target_season}"),
        board: features.board,
        leakage_audit: features.leakage_audit,
        baseline_value: 50.0,
        acceptance: OrganizationWindowHoldoutAcceptanceRule {
            minimum_sample_size: CANONICAL_TEAMS.len(),
            require_mae_below_baseline: true,
            minimum_rank_correlation: 0.3,
            require_complete_leakage_audit: true,
        },
        source_fingerprints: vec![features.stats_fingerprint, features.bios_fingerprint],
        disclosures: vec![
            "This outcome-free registration freezes the feature board before target-season results are observed.".to_owned(),
            "The target, neutral baseline, cohort, leakage audit, and acceptance rule may not be changed after registration.".to_owned(),
            "Scoring occurs once, no earlier than outcome_not_before, against a separately sealed official final-standings snapshot.".to_owned(),
            "A failed or inconclusive holdout is retained; it is not replaced or reclassified as development evidence.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    registration.fingerprint = future_holdout_registration_fingerprint(&registration)?;
    registration.validate()?;
    Ok(registration)
}

pub fn score_organization_window_future_holdout(
    registration: &OrganizationWindowFutureHoldoutRegistration,
    standings: &OrganizationWindowStandingsSnapshot,
    scored_at: &str,
) -> Result<OrganizationWindowFutureHoldoutResult, OrganizationWindowHistoryError> {
    registration.validate()?;
    standings.validate()?;
    let calibration = calibrate_organization_window(
        &registration.target,
        &registration.board,
        &standings.outcomes(),
        &registration.leakage_audit,
    )
    .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
    let acceptance_passed = holdout_acceptance_passed(registration, &calibration);
    let mut result = OrganizationWindowFutureHoldoutResult {
        schema: ORGANIZATION_WINDOW_FUTURE_HOLDOUT_RESULT_SCHEMA.to_owned(),
        registration: registration.clone(),
        standings: standings.clone(),
        scored_at: scored_at.to_owned(),
        calibration,
        acceptance_passed,
        disclosures: vec![
            "This result scores the exact preregistered feature board and acceptance rule; neither is refit after outcomes.".to_owned(),
            "The separately sealed official final-standings snapshot is retained inside the result for replay.".to_owned(),
            "The result remains evidence when it fails or is inconclusive and may not be replaced by a later development split.".to_owned(),
        ],
        fingerprint: String::new(),
    };
    result.fingerprint = future_holdout_result_fingerprint(&result)?;
    result.validate()?;
    Ok(result)
}

fn holdout_acceptance_passed(
    registration: &OrganizationWindowFutureHoldoutRegistration,
    calibration: &OrganizationWindowCalibrationView,
) -> bool {
    calibration.overall.sample_size >= registration.acceptance.minimum_sample_size
        && (!registration.acceptance.require_mae_below_baseline
            || calibration.overall.mean_absolute_error
                < calibration.overall.baseline_mean_absolute_error)
        && calibration
            .overall
            .rank_correlation
            .is_some_and(|correlation| {
                correlation >= registration.acceptance.minimum_rank_correlation
            })
        && (!registration.acceptance.require_complete_leakage_audit
            || calibration
                .leakage_audit
                .iter()
                .all(|row| row.point_in_time_safe))
}

fn build_historical_feature_bundle(
    source_season: u32,
    target_season: u32,
    as_of: NaiveDate,
    generated_at: &str,
    stats: &[SkaterStats],
    bios: &[SkaterBio],
) -> Result<HistoricalFeatureBundle, OrganizationWindowHistoryError> {
    if source_season % 10_000 != target_season / 10_000 {
        return Err(OrganizationWindowHistoryError::NonConsecutiveSeasons);
    }
    if stats.is_empty() || bios.is_empty() {
        return Err(OrganizationWindowHistoryError::IncompleteSource(
            "skater stats or bios are empty".to_owned(),
        ));
    }
    if stats
        .iter()
        .any(|row| row.season_id.is_some_and(|season| season != source_season))
        || bios
            .iter()
            .any(|row| row.season_id.is_some_and(|season| season != source_season))
    {
        return Err(OrganizationWindowHistoryError::IncompleteSource(
            "row season does not match source season".to_owned(),
        ));
    }

    let stats_fingerprint = format!("sha256:{}", sha256_json(stats)?);
    let bios_fingerprint = format!("sha256:{}", sha256_json(bios)?);
    let birth_dates = bios
        .iter()
        .filter_map(|bio| {
            let birth_date = bio
                .birth_date
                .as_deref()
                .and_then(|value| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())?;
            Some((bio.player_id, birth_date))
        })
        .collect::<BTreeMap<_, _>>();
    let mut teams = CANONICAL_TEAMS
        .iter()
        .map(|(team, _)| ((*team).to_owned(), HistoricalTeamAggregate::default()))
        .collect::<BTreeMap<_, _>>();

    for row in stats {
        let team_stints = row
            .team_abbrevs
            .as_deref()
            .unwrap_or_default()
            .split(',')
            .map(historical_franchise_organization)
            .filter(|team| teams.contains_key(team))
            .collect::<BTreeSet<_>>();
        if team_stints.len() != 1 {
            for team in team_stints {
                teams
                    .get_mut(&team)
                    .expect("filtered historical team")
                    .excluded_multi_team_player_games += u64::from(row.games_played);
            }
            continue;
        }
        let team = team_stints.into_iter().next().expect("one historical team");
        let aggregate = teams.get_mut(&team).expect("filtered historical team");
        aggregate.included_player_games += u64::from(row.games_played);
        aggregate.points += u64::from(row.points);
        aggregate.power_play_points.push(row.pp_points);
        aggregate.player_points.push(row.points);
        aggregate.players += 1;
        if row.games_played >= 20 && row.points >= 10 {
            aggregate.contributors += 1;
        }
        if let Some(birth_date) = birth_dates.get(&row.player_id) {
            aggregate.birth_dated_points += u64::from(row.points);
            if age_on(*birth_date, as_of) <= 23 {
                aggregate.young_points += u64::from(row.points);
            }
        }
    }

    let manifest = historical_organization_window_manifest(generated_at)?;
    let stats_evidence = WindowEvidenceView {
        source_schema: HISTORICAL_SOURCE_SCHEMA.to_owned(),
        source_id: format!("bundled:{source_season}:stats.json"),
        captured_at: Some(generated_at.to_owned()),
        as_of: Some(as_of),
        freshness: WindowFreshness::Current,
        source_url: None,
    };
    let bios_evidence = WindowEvidenceView {
        source_schema: HISTORICAL_SOURCE_SCHEMA.to_owned(),
        source_id: format!("bundled:{source_season}:bios.json"),
        captured_at: Some(generated_at.to_owned()),
        as_of: Some(as_of),
        freshness: WindowFreshness::Current,
        source_url: None,
    };
    let mut profile_inputs = Vec::with_capacity(teams.len() * 5);
    for (organization, mut aggregate) in teams {
        if aggregate.included_player_games == 0 || aggregate.points == 0 {
            return Err(OrganizationWindowHistoryError::IncompleteSource(format!(
                "no assignable skater contribution for {organization}"
            )));
        }
        let allocation_coverage = aggregate.included_player_games as f64
            / (aggregate.included_player_games + aggregate.excluded_multi_team_player_games) as f64;
        let scoring_rate = aggregate.points as f64 / aggregate.included_player_games as f64 * 82.0;
        let power_play_total = aggregate.power_play_points.iter().sum::<u32>();
        let power_play_breadth = if power_play_total == 0 {
            0.0
        } else {
            1.0 - aggregate
                .power_play_points
                .iter()
                .map(|points| (f64::from(*points) / f64::from(power_play_total)).powi(2))
                .sum::<f64>()
        };
        let age_coverage = aggregate.birth_dated_points as f64 / aggregate.points as f64;
        let young_share = if aggregate.birth_dated_points == 0 {
            0.0
        } else {
            aggregate.young_points as f64 / aggregate.birth_dated_points as f64 * 100.0
        };
        aggregate
            .player_points
            .sort_unstable_by(|left, right| right.cmp(left));
        let top_five = aggregate.player_points.iter().take(5).sum::<u32>();
        let concentration = f64::from(top_five) / aggregate.points as f64 * 100.0;
        let allocation_limitation = (allocation_coverage < 0.999).then(|| {
            "Aggregate multi-team season rows are omitted because their contribution cannot be allocated point-in-time by team; coverage is conservative.".to_owned()
        });

        profile_inputs.push(historical_profile_input(
            "history.nhl_skater_scoring",
            "historical_observed_skater_scoring.v1",
            &organization,
            target_season,
            as_of,
            scoring_rate,
            "points_per_82_player_games",
            aggregate.included_player_games,
            allocation_coverage,
            vec![stats_evidence.clone()],
            allocation_limitation.clone().into_iter().collect(),
            vec![stats_fingerprint.clone()],
        ));
        profile_inputs.push(historical_profile_input(
            "history.power_play_breadth",
            "historical_observed_power_play_breadth.v1",
            &organization,
            target_season,
            as_of,
            power_play_breadth,
            "inverse_concentration",
            aggregate.players,
            allocation_coverage,
            vec![stats_evidence.clone()],
            allocation_limitation.clone().into_iter().collect(),
            vec![stats_fingerprint.clone()],
        ));
        profile_inputs.push(historical_profile_input(
            "history.young_contribution_share",
            "historical_observed_young_share.v1",
            &organization,
            target_season,
            as_of,
            young_share,
            "points_share_pct",
            aggregate.players,
            allocation_coverage * age_coverage,
            vec![stats_evidence.clone(), bios_evidence.clone()],
            allocation_limitation
                .clone()
                .into_iter()
                .chain((age_coverage < 0.999).then(|| "Players without parseable birth dates are excluded from the age-share denominator.".to_owned()))
                .collect(),
            vec![stats_fingerprint.clone(), bios_fingerprint.clone()],
        ));
        profile_inputs.push(historical_profile_input(
            "history.contributor_depth",
            "historical_observed_contributor_depth.v1",
            &organization,
            target_season,
            as_of,
            aggregate.contributors as f64,
            "qualified_skaters",
            aggregate.players,
            allocation_coverage,
            vec![stats_evidence.clone()],
            allocation_limitation.clone().into_iter().collect(),
            vec![stats_fingerprint.clone()],
        ));
        profile_inputs.push(historical_profile_input(
            "history.roster_concentration",
            "historical_observed_roster_concentration.v1",
            &organization,
            target_season,
            as_of,
            concentration,
            "top_five_points_share_pct",
            aggregate.players,
            allocation_coverage,
            vec![stats_evidence.clone()],
            allocation_limitation.into_iter().collect(),
            vec![stats_fingerprint.clone()],
        ));
    }

    let inventory = load_organization_window_profile_inventory()
        .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
    let board = build_organization_window_board(
        OrganizationWindowBoardInput {
            season: target_season,
            season_type: "regular".to_owned(),
            as_of,
            generated_at: generated_at.to_owned(),
            manifest,
            profile_inputs,
            source_fingerprints: vec![
                stats_fingerprint.clone(),
                bios_fingerprint.clone(),
                format!(
                    "registry-lifecycle:{}",
                    load_organization_window_registry_lifecycle(&inventory)
                        .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?
                        .fingerprint
                ),
            ],
        },
        &inventory,
    )
    .map_err(|error| OrganizationWindowHistoryError::Board(error.to_string()))?;
    let leakage_audit = board
        .manifest
        .dimensions
        .iter()
        .flat_map(|dimension| &dimension.profiles)
        .map(|profile| WindowLeakageAuditRow {
            profile_key: profile.profile_key.clone(),
            method_version: profile.method_version.clone(),
            point_in_time_safe: true,
            evidence: format!(
                "uses only sealed {source_season} stats/bios available by {as_of}; target-season outcomes are excluded from feature construction"
            ),
        })
        .collect();
    Ok(HistoricalFeatureBundle {
        board,
        leakage_audit,
        stats_fingerprint,
        bios_fingerprint,
    })
}

fn historical_dimension(
    key: &str,
    label: &str,
    weight: f64,
    profile_key: &str,
    method_version: &str,
) -> WindowDimensionManifest {
    WindowDimensionManifest {
        key: key.to_owned(),
        label: label.to_owned(),
        weight,
        minimum_coverage: 0.50,
        rank_required: true,
        profiles: vec![WindowProfileWeight {
            profile_key: profile_key.to_owned(),
            method_version: method_version.to_owned(),
            weight: 1.0,
            required: true,
        }],
        signal_family_caps: Vec::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn historical_profile_input(
    profile_key: &str,
    method_version: &str,
    organization: &str,
    target_season: u32,
    as_of: NaiveDate,
    raw_value: f64,
    raw_unit: &str,
    sample_size: u64,
    coverage: f64,
    evidence: Vec<WindowEvidenceView>,
    limitations: Vec<String>,
    source_fingerprints: Vec<String>,
) -> OrganizationProfileInput {
    OrganizationProfileInput {
        profile_key: profile_key.to_owned(),
        method_version: method_version.to_owned(),
        organization: organization.to_owned(),
        organization_identity_version: ORGANIZATION_WINDOW_HISTORICAL_IDENTITY_VERSION.to_owned(),
        season: target_season,
        season_type: "regular".to_owned(),
        as_of,
        horizon: WindowHorizon::OneYear,
        raw_value: Some(raw_value),
        raw_unit: raw_unit.to_owned(),
        sample_size,
        confidence: coverage.clamp(0.0, 1.0),
        coverage: coverage.clamp(0.0, 1.0),
        status: WindowProfileStatus::Observed,
        evidence,
        limitations,
        source_fingerprints,
    }
}

fn age_on(birth_date: NaiveDate, as_of: NaiveDate) -> i32 {
    as_of.year()
        - birth_date.year()
        - i32::from((as_of.month(), as_of.day()) < (birth_date.month(), birth_date.day()))
}

fn historical_origin_fingerprint(
    artifact: &OrganizationWindowHistoricalOriginArtifact,
) -> Result<String, OrganizationWindowHistoryError> {
    let mut canonical = artifact.clone();
    canonical.fingerprint.clear();
    canonical.source_fingerprints.sort();
    canonical.disclosures.sort();
    let wire = serde_json::to_vec(&canonical)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    let normalized: OrganizationWindowHistoricalOriginArtifact = serde_json::from_slice(&wire)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    sha256_json(&normalized)
}

fn future_holdout_registration_fingerprint(
    registration: &OrganizationWindowFutureHoldoutRegistration,
) -> Result<String, OrganizationWindowHistoryError> {
    let mut canonical = registration.clone();
    canonical.fingerprint.clear();
    canonical.leakage_audit.sort_by(|left, right| {
        (&left.profile_key, &left.method_version).cmp(&(&right.profile_key, &right.method_version))
    });
    canonical.source_fingerprints.sort();
    canonical.disclosures.sort();
    let wire = serde_json::to_vec(&canonical)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    let normalized: OrganizationWindowFutureHoldoutRegistration = serde_json::from_slice(&wire)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    sha256_json(&normalized)
}

fn future_holdout_result_fingerprint(
    result: &OrganizationWindowFutureHoldoutResult,
) -> Result<String, OrganizationWindowHistoryError> {
    let mut canonical = result.clone();
    canonical.fingerprint.clear();
    canonical.disclosures.sort();
    let wire = serde_json::to_vec(&canonical)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    let normalized: OrganizationWindowFutureHoldoutResult = serde_json::from_slice(&wire)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    sha256_json(&normalized)
}

fn snapshot_fingerprint(
    snapshot: &OrganizationWindowStandingsSnapshot,
) -> Result<String, OrganizationWindowHistoryError> {
    let mut canonical = snapshot.clone();
    canonical.fingerprint.clear();
    canonical
        .rows
        .sort_by(|left, right| left.organization.cmp(&right.organization));
    canonical.disclosures.sort();
    let wire = serde_json::to_vec(&canonical)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    let normalized: OrganizationWindowStandingsSnapshot = serde_json::from_slice(&wire)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    sha256_json(&normalized)
}

fn sha256_json(
    value: &(impl Serialize + ?Sized),
) -> Result<String, OrganizationWindowHistoryError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| OrganizationWindowHistoryError::Serialization(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_historical_window_schemas_are_embedded_json_documents() {
        let standings: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_STANDINGS_JSON_SCHEMA).unwrap();
        let origin: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_JSON_SCHEMA).unwrap();
        let registration: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_FUTURE_HOLDOUT_REGISTRATION_JSON_SCHEMA)
                .unwrap();
        let result: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_FUTURE_HOLDOUT_RESULT_JSON_SCHEMA).unwrap();
        assert_eq!(
            standings["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_STANDINGS_SCHEMA
        );
        assert_eq!(
            origin["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_HISTORICAL_ORIGIN_SCHEMA
        );
        assert_eq!(
            registration["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_FUTURE_HOLDOUT_REGISTRATION_SCHEMA
        );
        assert_eq!(
            result["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_FUTURE_HOLDOUT_RESULT_SCHEMA
        );
    }

    fn standing(team: &str, points: u32) -> NhlStandingsRow {
        let wins = points / 2;
        let overtime_losses = points % 2;
        NhlStandingsRow {
            team: team.to_owned(),
            conference: None,
            division: None,
            games_played: 82,
            wins,
            losses: 82 - wins - overtime_losses,
            overtime_losses,
            points,
            points_percentage: points as f32 / 164.0,
            regulation_wins: None,
            goal_differential: 0,
            league_rank: None,
            conference_rank: None,
            division_rank: None,
            wild_card_rank: None,
        }
    }

    fn historical_cohort() -> Vec<NhlStandingsRow> {
        CANONICAL_TEAMS
            .iter()
            .enumerate()
            .map(|(index, (team, _))| {
                standing(if *team == "UTA" { "ARI" } else { team }, 70 + index as u32)
            })
            .collect()
    }

    #[test]
    fn l0_snapshot_seals_canonical_outcomes_and_preserves_historical_identity() {
        let rows = historical_cohort();
        let snapshot = build_organization_window_standings_snapshot(
            20232024,
            NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
            "2026-07-28T08:00:00Z",
            &rows,
        )
        .unwrap();
        assert_eq!(snapshot.rows.len(), 32);
        let utah = snapshot
            .rows
            .iter()
            .find(|row| row.organization == "UTA")
            .unwrap();
        assert_eq!(utah.observed_team, "ARI");
        let outcomes = snapshot.outcomes();
        assert_eq!(outcomes.len(), 32);
        assert_eq!(
            outcomes
                .iter()
                .map(|row| row.target_value)
                .max_by(f64::total_cmp),
            Some(100.0)
        );
        assert_eq!(
            outcomes
                .iter()
                .map(|row| row.target_value)
                .min_by(f64::total_cmp),
            Some(0.0)
        );
        assert_eq!(snapshot.fingerprint.len(), 64);
        let wire: OrganizationWindowStandingsSnapshot =
            serde_json::from_str(&serde_json::to_string(&snapshot).unwrap()).unwrap();
        wire.validate().unwrap();

        let mut reversed = rows;
        reversed.reverse();
        let same = build_organization_window_standings_snapshot(
            20232024,
            NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
            "2026-07-28T08:00:00Z",
            &reversed,
        )
        .unwrap();
        assert_eq!(snapshot.fingerprint, same.fingerprint);
    }

    #[test]
    fn l0_snapshot_rejects_incomplete_or_duplicate_franchise_cohorts() {
        let mut incomplete = historical_cohort();
        incomplete.pop();
        assert_eq!(
            build_organization_window_standings_snapshot(
                20232024,
                NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
                "2026-07-28T08:00:00Z",
                &incomplete,
            ),
            Err(OrganizationWindowHistoryError::InvalidCohort)
        );

        let mut duplicate = historical_cohort();
        duplicate.push(standing("UTA", 90));
        assert_eq!(
            build_organization_window_standings_snapshot(
                20232024,
                NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
                "2026-07-28T08:00:00Z",
                &duplicate,
            ),
            Err(OrganizationWindowHistoryError::DuplicateOrganization(
                "UTA".to_owned()
            ))
        );
    }

    #[test]
    fn l1_bundled_modern_seasons_build_complete_ranked_point_in_time_origins() {
        let cases = [
            (
                20212022,
                20222023,
                NaiveDate::from_ymd_opt(2022, 6, 30).unwrap(),
                NaiveDate::from_ymd_opt(2023, 4, 14).unwrap(),
                WindowCalibrationOriginRole::Training,
            ),
            (
                20222023,
                20232024,
                NaiveDate::from_ymd_opt(2023, 6, 30).unwrap(),
                NaiveDate::from_ymd_opt(2024, 4, 18).unwrap(),
                WindowCalibrationOriginRole::Training,
            ),
            (
                20232024,
                20242025,
                NaiveDate::from_ymd_opt(2024, 6, 30).unwrap(),
                NaiveDate::from_ymd_opt(2025, 4, 17).unwrap(),
                WindowCalibrationOriginRole::Validation,
            ),
            (
                20242025,
                20252026,
                NaiveDate::from_ymd_opt(2025, 6, 30).unwrap(),
                NaiveDate::from_ymd_opt(2026, 4, 17).unwrap(),
                WindowCalibrationOriginRole::RetrospectiveHoldout,
            ),
        ];
        for (source, target, as_of, outcome_date, role) in cases {
            let stats = crate::bundled::get_stats(&source.to_string()).unwrap();
            let bios = crate::bundled::get_bios(&source.to_string()).unwrap();
            let outcomes = build_organization_window_standings_snapshot(
                target,
                outcome_date,
                "2026-07-28T08:00:00Z",
                &historical_cohort(),
            )
            .unwrap();
            let artifact = build_historical_organization_window_origin(
                source,
                target,
                as_of,
                "2026-07-28T08:00:00Z",
                role,
                &stats,
                &bios,
                &outcomes,
            )
            .unwrap_or_else(|error| panic!("{source}->{target}: {error}"));
            assert_eq!(artifact.origin.board.organizations.len(), 32);
            assert!(artifact
                .origin
                .board
                .organizations
                .iter()
                .all(|team| team.overall.rank.is_some()));
            assert!(artifact
                .origin
                .board
                .source_fingerprints
                .iter()
                .any(|fingerprint| fingerprint.starts_with("registry-lifecycle:")
                    && fingerprint.len() == "registry-lifecycle:".len() + 64));
            assert_eq!(artifact.origin.leakage_audit.len(), 5);
            assert_eq!(
                artifact.origin.trial_noise,
                WindowTrialNoiseInput::NotApplicable
            );
            assert_eq!(artifact.fingerprint.len(), 64);
            let wire: OrganizationWindowHistoricalOriginArtifact =
                serde_json::from_str(&serde_json::to_string(&artifact).unwrap()).unwrap();
            wire.validate()
                .unwrap_or_else(|error| panic!("{source}->{target} wire artifact: {error}"));
            let inventory = load_organization_window_profile_inventory().unwrap();
            icelines_core::validate_organization_window_board(&wire.origin.board, &inventory)
                .unwrap_or_else(|error| panic!("{source}->{target} wire board: {error}"));
            if source == 20212022 {
                let mut tampered = wire;
                tampered.role = WindowCalibrationOriginRole::RetrospectiveHoldout;
                assert_eq!(
                    tampered.validate(),
                    Err(OrganizationWindowHistoryError::InvalidHistoricalOrigin)
                );
            }
        }
    }

    #[test]
    fn l1_future_holdout_registration_freezes_features_without_outcomes() {
        let source = 20252026;
        let target = 20262027;
        let stats = crate::bundled::get_stats(&source.to_string()).unwrap();
        let bios = crate::bundled::get_bios(&source.to_string()).unwrap();
        let registration = build_organization_window_future_holdout_registration(
            source,
            target,
            NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
            NaiveDate::from_ymd_opt(2027, 4, 11).unwrap(),
            "2026-07-29T12:00:00Z",
            &stats,
            &bios,
        )
        .unwrap();
        registration.validate().unwrap();
        assert_eq!(registration.board.organizations.len(), 32);
        assert!(registration
            .board
            .organizations
            .iter()
            .all(|team| team.overall.rank.is_some()));
        assert_eq!(registration.leakage_audit.len(), 5);
        assert_eq!(registration.acceptance.minimum_rank_correlation, 0.3);
        assert_eq!(registration.fingerprint.len(), 64);
        let value = serde_json::to_value(&registration).unwrap();
        assert!(value.get("outcomes").is_none());
        assert!(value.get("claim_status").is_none());

        let standings = build_organization_window_standings_snapshot(
            target,
            NaiveDate::from_ymd_opt(2027, 4, 11).unwrap(),
            "2027-04-11T12:00:00Z",
            &historical_cohort(),
        )
        .unwrap();
        let result = score_organization_window_future_holdout(
            &registration,
            &standings,
            "2027-04-11T13:00:00Z",
        )
        .unwrap();
        result.validate().unwrap();
        assert_eq!(result.calibration.overall.sample_size, 32);
        assert_eq!(result.fingerprint.len(), 64);

        let early = build_organization_window_standings_snapshot(
            target,
            NaiveDate::from_ymd_opt(2027, 4, 10).unwrap(),
            "2027-04-10T12:00:00Z",
            &historical_cohort(),
        )
        .unwrap();
        assert_eq!(
            score_organization_window_future_holdout(&registration, &early, "2027-04-10T13:00:00Z"),
            Err(OrganizationWindowHistoryError::InvalidFutureHoldoutResult)
        );

        let mut tampered = registration;
        tampered.acceptance.minimum_rank_correlation = 0.2;
        assert_eq!(
            tampered.validate(),
            Err(OrganizationWindowHistoryError::InvalidFutureHoldoutRegistration)
        );
    }
}
