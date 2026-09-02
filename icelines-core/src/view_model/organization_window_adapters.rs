//! Typed adapters from sealed IceLines authorities into The Window profile contract.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::ahl_affiliate::{AhlAffiliateProjectionView, AHL_AFFILIATE_PROJECTION_SCHEMA};
use super::line_combination::{LineCombinationForecastView, LINE_COMBINATION_FORECAST_SCHEMA};
use super::management_behavior::{
    build_schedule_rest_profile, BenchScheduleLoad, ScheduleRestGameInput, ScheduleRestProfileView,
    SCHEDULE_REST_PROFILE_SCHEMA,
};
use super::organization_lineup::{
    build_organization_lineup_forecast, OrganizationLineupForecastInput,
    OrganizationLineupForecastView, ORGANIZATION_LINEUP_FORECAST_SCHEMA,
};
use super::organization_profile_history::{
    carry_forward_organization_profiles, seal_organization_profile_history,
    OrganizationProfileCarryForwardRule, OrganizationProfileHistoryView,
};
use super::organization_window::{
    build_organization_window_board, load_organization_window_profile_inventory,
    OrganizationProfileInput, OrganizationWindowBoardInput, OrganizationWindowBoardView,
    OrganizationWindowError, OrganizationWindowManifestView, WindowCohortKind,
    WindowCohortManifest, WindowDimensionManifest, WindowEvidenceView, WindowFreshness,
    WindowHorizon, WindowMissingPolicy, WindowNormalizationMethod, WindowProfileStatus,
    WindowProfileWeight, WindowRankState, WindowSignalFamilyCap,
    ORGANIZATION_WINDOW_CLASSIFICATION_METHOD, ORGANIZATION_WINDOW_MANIFEST_SCHEMA,
};
use super::organization_window_registry::{
    load_organization_window_registry_lifecycle, seal_new_organization_window_manifest,
    WindowManifestLifecyclePolicy,
};
use super::prospect_conversion::{ProspectConversionBoardView, PROSPECT_CONVERSION_BOARD_SCHEMA};
use super::prospect_study::{ProspectProgramBoardView, PROSPECT_PROGRAM_BOARD_SCHEMA};
use super::team_game_forecast::{TeamGameForecastView, TEAM_GAME_FORECAST_SCHEMA};
use super::team_lineup::{
    TeamLineupPlayerView, TeamLineupProjectionView, TeamLineupSpecialTeamsUnitView,
    TEAM_LINEUP_PROJECTION_SCHEMA,
};
use super::team_season_forecast::{
    TeamSeasonForecastHistoryView, TeamSeasonForecastView, TEAM_SEASON_FORECAST_HISTORY_SCHEMA,
    TEAM_SEASON_FORECAST_SCHEMA,
};
use super::training_camp::{TrainingCampLeagueForecastView, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA};
use crate::teams::CANONICAL_TEAMS;

pub const ORGANIZATION_WINDOW_BALANCED_MANIFEST_ID: &str = "balanced.v1";
pub const ORGANIZATION_WINDOW_FORECAST_HISTORY_MANIFEST_ID: &str = "icecast_forecast_history.v1";
pub const ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA: &str = "organization_window_source_package.v1";
pub const ORGANIZATION_WINDOW_SOURCE_PACKAGE_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_source_package.v1.schema.json");
pub const ORGANIZATION_WINDOW_SOURCE_COVERAGE_SCHEMA: &str =
    "organization_window_source_coverage.v1";
pub const ORGANIZATION_WINDOW_SOURCE_COVERAGE_JSON_SCHEMA: &str =
    include_str!("../../../design/schemas/organization_window_source_coverage.v1.schema.json");
const TEAM_CATALOG_VERSION: &str = "nhl_32.v1";
const BALANCED_MANIFEST_CREATED_AT: &str = "2026-07-27";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrganizationWindowAdapterContext {
    pub season: u32,
    pub season_type: String,
    pub as_of: NaiveDate,
    pub horizon: WindowHorizon,
    pub organization_identity_version: String,
}

/// Sealed upstream documents used by the official first Frame. Missing
/// documents remain visible as missing observations when the board is built.
#[derive(Debug, Clone, Copy, Default)]
pub struct OrganizationWindowSourceSet<'a> {
    pub team_season_forecast: Option<&'a TeamSeasonForecastView>,
    pub team_game_forecast: Option<&'a TeamGameForecastView>,
    pub team_lineups: &'a [TeamLineupProjectionView],
    pub ahl_affiliates: &'a [AhlAffiliateProjectionView],
    pub organization_lineups: &'a [OrganizationLineupForecastView],
    pub prospect_program: Option<&'a ProspectProgramBoardView>,
    pub prospect_conversion: Option<&'a ProspectConversionBoardView>,
    pub training_camp: Option<&'a TrainingCampLeagueForecastView>,
    pub schedule_rest: &'a [ScheduleRestProfileView],
    pub profile_history: Option<&'a OrganizationProfileHistoryView>,
}

/// One portable, sealed authority package for a balanced Window board. The
/// package owns upstream documents while `OrganizationWindowSourceSet` remains
/// the zero-copy adapter view used by the scorer.
#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct OrganizationWindowSourcePackageView {
    pub schema: String,
    pub season: u32,
    pub season_type: String,
    pub as_of: NaiveDate,
    pub organization_identity_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_season_forecast: Option<TeamSeasonForecastView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub team_game_forecast: Option<TeamGameForecastView>,
    #[serde(default)]
    pub team_lineups: Vec<TeamLineupProjectionView>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ahl_affiliates: Vec<AhlAffiliateProjectionView>,
    #[serde(default)]
    pub organization_lineups: Vec<OrganizationLineupForecastView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prospect_program: Option<ProspectProgramBoardView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prospect_conversion: Option<ProspectConversionBoardView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_camp: Option<TrainingCampLeagueForecastView>,
    #[serde(default)]
    pub schedule_rest: Vec<ScheduleRestProfileView>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile_history: Option<OrganizationProfileHistoryView>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct WindowSourceProfileCoverageView {
    pub profile_key: String,
    pub method_version: String,
    pub required: bool,
    pub organizations_with_observation: usize,
    pub organizations_with_value: usize,
    pub missing_organizations: Vec<String>,
    pub complete: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, serde::Deserialize)]
pub struct OrganizationWindowSourceCoverageView {
    pub schema: String,
    pub season: u32,
    pub as_of: NaiveDate,
    pub package_fingerprint: String,
    pub board_fingerprint: String,
    pub expected_organizations: usize,
    pub profiles: Vec<WindowSourceProfileCoverageView>,
    pub complete_required_profiles: usize,
    pub required_profiles: usize,
    pub rank_eligible_organizations: usize,
    #[serde(default)]
    pub carry_forward_observations: usize,
    pub production_ranked: bool,
    pub disclosures: Vec<String>,
}

impl OrganizationWindowSourcePackageView {
    pub fn as_source_set(&self) -> OrganizationWindowSourceSet<'_> {
        OrganizationWindowSourceSet {
            team_season_forecast: self.team_season_forecast.as_ref(),
            team_game_forecast: self.team_game_forecast.as_ref(),
            team_lineups: &self.team_lineups,
            ahl_affiliates: &self.ahl_affiliates,
            organization_lineups: &self.organization_lineups,
            prospect_program: self.prospect_program.as_ref(),
            prospect_conversion: self.prospect_conversion.as_ref(),
            training_camp: self.training_camp.as_ref(),
            schedule_rest: &self.schedule_rest,
            profile_history: self.profile_history.as_ref(),
        }
    }

    pub fn calculate_fingerprint(&self) -> Result<String, OrganizationWindowError> {
        let mut canonical = self.clone();
        canonical.fingerprint.clear();
        canonical
            .team_lineups
            .sort_by(|left, right| left.team.cmp(&right.team));
        canonical
            .organization_lineups
            .sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
        canonical
            .ahl_affiliates
            .sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
        canonical
            .schedule_rest
            .sort_by(|left, right| left.team.cmp(&right.team));
        if let Some(history) = canonical.profile_history.take() {
            canonical.profile_history = Some(
                seal_organization_profile_history(history)
                    .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?,
            );
        }
        let wire = serde_json::to_vec(&canonical)
            .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
        let normalized: Self = serde_json::from_slice(&wire)
            .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
        let bytes = serde_json::to_vec(&normalized)
            .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

pub fn seal_organization_window_source_package(
    mut package: OrganizationWindowSourcePackageView,
) -> Result<OrganizationWindowSourcePackageView, OrganizationWindowError> {
    if package.schema != ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA {
        return Err(OrganizationWindowError::UnsupportedSchema {
            contract: "source package",
            found: package.schema,
        });
    }
    if package.season_type != "regular"
        || package.organization_identity_version != TEAM_CATALOG_VERSION
    {
        return Err(OrganizationWindowError::ContextMismatch(
            "source package requires regular season and nhl_32.v1 identity".to_owned(),
        ));
    }
    let context = OrganizationWindowAdapterContext {
        season: package.season,
        season_type: package.season_type.clone(),
        as_of: package.as_of,
        horizon: WindowHorizon::Current,
        organization_identity_version: package.organization_identity_version.clone(),
    };
    validate_context(&context)?;
    package
        .team_lineups
        .sort_by(|left, right| left.team.cmp(&right.team));
    package
        .organization_lineups
        .sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    package
        .ahl_affiliates
        .sort_by(|left, right| left.nhl_team.cmp(&right.nhl_team));
    package
        .schedule_rest
        .sort_by(|left, right| left.team.cmp(&right.team));
    if let Some(history) = package.profile_history.take() {
        let history = seal_organization_profile_history(history)
            .map_err(|error| OrganizationWindowError::InvalidProfileInput(error.to_string()))?;
        if history.organization_identity_version != package.organization_identity_version {
            return Err(context_error("profile history organization identity"));
        }
        package.profile_history = Some(history);
    }
    // Running the adapters validates every nested schema, axis, team identity,
    // and duplicate before the package receives a trusted fingerprint.
    adapt_balanced_organization_window_sources(&context, package.as_source_set())?;
    let supplied = std::mem::take(&mut package.fingerprint);
    let calculated = package.calculate_fingerprint()?;
    if !supplied.is_empty() && supplied != calculated {
        return Err(OrganizationWindowError::InvalidProfileInput(
            "organization Window source package fingerprint mismatch".to_owned(),
        ));
    }
    package.fingerprint = calculated;
    Ok(package)
}

pub fn build_balanced_organization_window_board_from_package(
    package: &OrganizationWindowSourcePackageView,
    generated_at: impl Into<String>,
) -> Result<OrganizationWindowBoardView, OrganizationWindowError> {
    let package = seal_organization_window_source_package(package.clone())?;
    build_balanced_organization_window_board(
        OrganizationWindowAdapterContext {
            season: package.season,
            season_type: package.season_type.clone(),
            as_of: package.as_of,
            horizon: WindowHorizon::Current,
            organization_identity_version: package.organization_identity_version.clone(),
        },
        generated_at,
        package.as_source_set(),
    )
}

pub fn audit_organization_window_source_package(
    package: &OrganizationWindowSourcePackageView,
    generated_at: impl Into<String>,
) -> Result<OrganizationWindowSourceCoverageView, OrganizationWindowError> {
    let package = seal_organization_window_source_package(package.clone())?;
    let context = OrganizationWindowAdapterContext {
        season: package.season,
        season_type: package.season_type.clone(),
        as_of: package.as_of,
        horizon: WindowHorizon::Current,
        organization_identity_version: package.organization_identity_version.clone(),
    };
    let inputs = adapt_balanced_organization_window_sources(&context, package.as_source_set())?;
    let expected = canonical_teams();
    let manifest = balanced_organization_window_manifest(BALANCED_MANIFEST_CREATED_AT);
    let mut profiles = manifest
        .dimensions
        .iter()
        .flat_map(|dimension| &dimension.profiles)
        .map(|configured| {
            let matching = inputs
                .iter()
                .filter(|input| {
                    input.profile_key == configured.profile_key
                        && input.method_version == configured.method_version
                })
                .collect::<Vec<_>>();
            let observed = matching
                .iter()
                .map(|input| input.organization.clone())
                .collect::<BTreeSet<_>>();
            let valued = matching
                .iter()
                .filter(|input| {
                    input.raw_value.is_some()
                        && matches!(
                            input.status,
                            WindowProfileStatus::Observed
                                | WindowProfileStatus::Modeled
                                | WindowProfileStatus::Provisional
                        )
                })
                .map(|input| input.organization.clone())
                .collect::<BTreeSet<_>>();
            let missing_organizations = expected.difference(&valued).cloned().collect::<Vec<_>>();
            WindowSourceProfileCoverageView {
                profile_key: configured.profile_key.clone(),
                method_version: configured.method_version.clone(),
                required: configured.required,
                organizations_with_observation: observed.len(),
                organizations_with_value: valued.len(),
                complete: missing_organizations.is_empty(),
                missing_organizations,
            }
        })
        .collect::<Vec<_>>();
    profiles.sort_by(|left, right| {
        left.profile_key
            .cmp(&right.profile_key)
            .then_with(|| left.method_version.cmp(&right.method_version))
    });
    let required_profiles = profiles.iter().filter(|profile| profile.required).count();
    let complete_required_profiles = profiles
        .iter()
        .filter(|profile| profile.required && profile.complete)
        .count();
    let board = build_balanced_organization_window_board_from_package(&package, generated_at)?;
    let carry_forward_observations = inputs
        .iter()
        .filter(|input| {
            input
                .source_fingerprints
                .iter()
                .any(|fingerprint| fingerprint.starts_with("organization-profile-history:"))
        })
        .count();
    let rank_eligible_organizations = board
        .organizations
        .iter()
        .filter(|organization| organization.overall.rank_status.state == WindowRankState::Ranked)
        .count();
    Ok(OrganizationWindowSourceCoverageView {
        schema: ORGANIZATION_WINDOW_SOURCE_COVERAGE_SCHEMA.to_owned(),
        season: package.season,
        as_of: package.as_of,
        package_fingerprint: package.fingerprint,
        board_fingerprint: board.fingerprint,
        expected_organizations: expected.len(),
        profiles,
        complete_required_profiles,
        required_profiles,
        rank_eligible_organizations,
        carry_forward_observations,
        production_ranked: rank_eligible_organizations == expected.len()
            && carry_forward_observations == 0,
        disclosures: vec![
            "Cohort presence is not profile completeness; each adapter-ready profile is audited independently across the canonical league.".to_owned(),
            "Carry-forward observations can complete a preseason rank but cannot satisfy the confirmed-authority production gate.".to_owned(),
            "Production-ranked means every organization passed the balanced.v1 rank gate without historical carry-forward; it is not a calibration or predictive claim.".to_owned(),
        ],
    })
}

/// Validate a persisted source-coverage audit before another lifecycle
/// document relies on it. The audit is intentionally derived evidence rather
/// than an authority of its own, so validation checks every declared count and
/// profile against the canonical `balanced.v1` manifest.
pub fn validate_organization_window_source_coverage(
    coverage: &OrganizationWindowSourceCoverageView,
) -> Result<(), OrganizationWindowError> {
    if coverage.schema != ORGANIZATION_WINDOW_SOURCE_COVERAGE_SCHEMA {
        return Err(OrganizationWindowError::UnsupportedSchema {
            contract: "organization Window source coverage",
            found: coverage.schema.clone(),
        });
    }
    let expected_organizations = canonical_teams();
    if coverage.expected_organizations != expected_organizations.len()
        || coverage.package_fingerprint.trim().is_empty()
        || coverage.board_fingerprint.trim().is_empty()
        || coverage.rank_eligible_organizations > expected_organizations.len()
        || coverage.disclosures.is_empty()
        || coverage
            .disclosures
            .iter()
            .any(|disclosure| disclosure.trim().is_empty())
    {
        return Err(OrganizationWindowError::InvalidBoard(
            "source coverage has invalid cohort, fingerprints, rank count, or disclosures"
                .to_owned(),
        ));
    }

    let manifest = balanced_organization_window_manifest(BALANCED_MANIFEST_CREATED_AT);
    let expected_profiles = manifest
        .dimensions
        .iter()
        .flat_map(|dimension| &dimension.profiles)
        .map(|profile| {
            (
                (profile.profile_key.clone(), profile.method_version.clone()),
                profile.required,
            )
        })
        .collect::<BTreeMap<_, _>>();
    if coverage.profiles.len() != expected_profiles.len() {
        return Err(OrganizationWindowError::InvalidBoard(format!(
            "source coverage declares {} profiles but balanced.v1 requires {}",
            coverage.profiles.len(),
            expected_profiles.len()
        )));
    }

    let mut seen = BTreeSet::new();
    let mut complete_required_profiles = 0usize;
    for profile in &coverage.profiles {
        let id = (profile.profile_key.clone(), profile.method_version.clone());
        let Some(expected_required) = expected_profiles.get(&id).copied() else {
            return Err(OrganizationWindowError::UnknownProfileMethod(format!(
                "{}@{}",
                profile.profile_key, profile.method_version
            )));
        };
        if !seen.insert(id)
            || profile.required != expected_required
            || profile.organizations_with_observation > expected_organizations.len()
            || profile.organizations_with_value > profile.organizations_with_observation
        {
            return Err(OrganizationWindowError::InvalidBoard(format!(
                "source coverage row {}@{} has duplicate identity, invalid requirement, or impossible counts",
                profile.profile_key, profile.method_version
            )));
        }
        let missing = profile
            .missing_organizations
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        if missing.len() != profile.missing_organizations.len()
            || !missing.is_subset(&expected_organizations)
            || missing.len() + profile.organizations_with_value != expected_organizations.len()
            || profile.complete != missing.is_empty()
        {
            return Err(OrganizationWindowError::InvalidBoard(format!(
                "source coverage row {}@{} has inconsistent missing-team evidence",
                profile.profile_key, profile.method_version
            )));
        }
        if profile.required && profile.complete {
            complete_required_profiles += 1;
        }
    }

    let required_profiles = expected_profiles
        .values()
        .filter(|required| **required)
        .count();
    let expected_production_ranked = coverage.rank_eligible_organizations
        == expected_organizations.len()
        && coverage.carry_forward_observations == 0
        && complete_required_profiles == required_profiles;
    if coverage.required_profiles != required_profiles
        || coverage.complete_required_profiles != complete_required_profiles
        || coverage.production_ranked != expected_production_ranked
    {
        return Err(OrganizationWindowError::InvalidBoard(
            "source coverage summary does not match its canonical profile rows".to_owned(),
        ));
    }
    Ok(())
}

/// Fail closed when a caller asks to publish a production-ranked balanced
/// board but any organization still has a withheld rank.
pub fn require_ranked_balanced_organization_window_board(
    board: &OrganizationWindowBoardView,
) -> Result<(), OrganizationWindowError> {
    if board.manifest.manifest_id != ORGANIZATION_WINDOW_BALANCED_MANIFEST_ID {
        return Err(OrganizationWindowError::InvalidBoard(
            "production rank gate requires the balanced.v1 Frame".to_owned(),
        ));
    }
    let withheld = board
        .organizations
        .iter()
        .filter(|organization| organization.overall.rank_status.state != WindowRankState::Ranked)
        .map(|organization| {
            let reasons = organization.overall.rank_status.reasons.join("; ");
            format!("{} ({reasons})", organization.organization)
        })
        .collect::<Vec<_>>();
    if withheld.is_empty() {
        Ok(())
    } else {
        Err(OrganizationWindowError::InvalidBoard(format!(
            "balanced Window is not production-ranked; {} organization(s) withheld: {}",
            withheld.len(),
            withheld.into_iter().take(5).collect::<Vec<_>>().join(", ")
        )))
    }
}

/// Adapt a sealed line-combination forecast for a custom/evaluation Frame.
/// “Competitive” means a non-baseline candidate whose modeled strength is at
/// least the baseline's; this method does not claim shift-derived chemistry.
pub fn adapt_line_combination_window_profile(
    context: &OrganizationWindowAdapterContext,
    forecast: &LineCombinationForecastView,
) -> Result<OrganizationProfileInput, OrganizationWindowError> {
    require_schema(
        "line combination",
        &forecast.schema,
        LINE_COMBINATION_FORECAST_SCHEMA,
    )?;
    if forecast.roster_season != context.season {
        return Err(context_error("line-combination season"));
    }
    if !CANONICAL_TEAMS
        .iter()
        .any(|(abbreviation, _)| *abbreviation == forecast.team.as_str())
    {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "line-combination team {} is outside the canonical cohort",
            forecast.team
        )));
    }
    let source_evidence = evidence(LINE_COMBINATION_FORECAST_SCHEMA, forecast, None, context)?;
    let alternatives = forecast
        .candidates
        .iter()
        .filter(|candidate| !candidate.is_baseline)
        .collect::<Vec<_>>();
    let competitive = alternatives
        .iter()
        .filter(|candidate| candidate.strength_delta >= 0.0)
        .count();
    let confidence = mean(
        &alternatives
            .iter()
            .map(|candidate| candidate.score.talent_confidence)
            .collect::<Vec<_>>(),
    )
    .unwrap_or(0.0)
    .clamp(0.0, 1.0);
    Ok(input(
        context,
        "deployment.lineup_optionality",
        "line_combination_optionality.v1",
        &forecast.team,
        "competitive_combinations",
        Some(competitive as f64),
        alternatives.len() as u64,
        confidence,
        if alternatives.is_empty() { 0.0 } else { 1.0 },
        if alternatives.is_empty() {
            WindowProfileStatus::Blocked
        } else {
            WindowProfileStatus::Modeled
        },
        source_evidence,
        vec![
            "Competitive combinations are modeled alternatives at or above baseline strength; this is not full shift-derived chemistry."
                .to_owned(),
        ],
    ))
}

pub fn balanced_organization_window_manifest(
    created_at: impl Into<String>,
) -> OrganizationWindowManifestView {
    let teams = CANONICAL_TEAMS
        .iter()
        .map(|(abbreviation, _)| (*abbreviation).to_owned())
        .collect();
    OrganizationWindowManifestView {
        schema: ORGANIZATION_WINDOW_MANIFEST_SCHEMA.to_owned(),
        manifest_id: ORGANIZATION_WINDOW_BALANCED_MANIFEST_ID.to_owned(),
        label: "Balanced organization Window".to_owned(),
        description: "A descriptive, evidence-aware balance of current NHL strength, deployment, pipeline, development, and resilience.".to_owned(),
        manifest_version: "1.0.0".to_owned(),
        comparison_cohort: WindowCohortManifest {
            kind: WindowCohortKind::CurrentNhl,
            team_catalog_version: TEAM_CATALOG_VERSION.to_owned(),
            expected_organizations: teams,
        },
        normalization_method: WindowNormalizationMethod::EmpiricalPercentile,
        primary_horizon: WindowHorizon::Current,
        dimensions: vec![
            dimension(
                "nhl_strength",
                "NHL strength",
                0.35,
                0.75,
                vec![
                    profile("nhl.team_strength", "icecast_team_strength.v1", 0.20, true),
                    profile("nhl.expected_points", "icecast_expected_points.v1", 0.20, true),
                    profile("nhl.forward_depth", "lineup_forward_depth.v1", 0.20, true),
                    profile("nhl.defense_depth", "lineup_defense_depth.v1", 0.20, true),
                    profile("nhl.goalie_quality", "lineup_goalie_quality.v1", 0.20, true),
                ],
                vec![cap("lineup_depth", 0.40)],
            ),
            dimension(
                "deployment",
                "Deployment",
                0.15,
                0.60,
                vec![
                    profile("deployment.power_play_depth", "lineup_power_play_depth.v1", 0.50, true),
                    profile("deployment.penalty_kill_depth", "lineup_penalty_kill_depth.v1", 0.50, true),
                ],
                vec![cap("special_teams", 1.0)],
            ),
            dimension(
                "pipeline",
                "Pipeline",
                0.20,
                0.65,
                vec![
                    profile("pipeline.prospect_pool", "prospect_pool_score.v1", 0.30, true),
                    profile("pipeline.prospect_development", "prospect_development_score.v1", 0.25, true),
                    profile("pipeline.prospect_readiness", "prospect_readiness_score.v1", 0.25, true),
                    profile("pipeline.training_camp_arrival", "training_camp_arrival.v1", 0.20, true),
                ],
                vec![cap("prospect_program", 0.80)],
            ),
            dimension(
                "development_system",
                "Development system",
                0.15,
                0.60,
                vec![
                    profile("development.organization_depth", "organization_lineup_depth.v1", 0.40, true),
                    profile("development.recall_depth", "organization_recall_depth.v1", 0.30, true),
                    profile("development.prospect_conversion", "prospect_conversion_efficiency.v1", 0.30, false),
                ],
                vec![cap("organization_depth", 0.70)],
            ),
            dimension(
                "resilience",
                "Resilience",
                0.15,
                0.60,
                vec![
                    profile("resilience.goalie_dependency", "lineup_goalie_dependency.v1", 0.30, true),
                    profile("resilience.schedule_fatigue", "schedule_fatigue_exposure.v1", 0.30, true),
                    profile("resilience.roster_concentration", "lineup_value_concentration.v1", 0.40, true),
                ],
                vec![cap("lineup_depth", 0.40)],
            ),
        ],
        missing_policy: WindowMissingPolicy::WithholdRank,
        classification_method: ORGANIZATION_WINDOW_CLASSIFICATION_METHOD.to_owned(),
        created_at: created_at.into(),
        fingerprint: String::new(),
    }
}

pub fn adapt_balanced_organization_window_sources(
    context: &OrganizationWindowAdapterContext,
    sources: OrganizationWindowSourceSet<'_>,
) -> Result<Vec<OrganizationProfileInput>, OrganizationWindowError> {
    validate_context(context)?;
    let expected = canonical_teams();
    let mut output = Vec::new();

    if let Some(forecast) = sources.team_season_forecast {
        require_schema(
            "team season forecast",
            &forecast.schema,
            TEAM_SEASON_FORECAST_SCHEMA,
        )?;
        if forecast.season != context.season {
            return Err(context_error("team season forecast season"));
        }
        let evidence = evidence(
            TEAM_SEASON_FORECAST_SCHEMA,
            forecast,
            forecast.as_of_date,
            context,
        )?;
        let mut seen_strength = BTreeSet::new();
        for row in &forecast.opening_strengths {
            require_team(&row.team, &expected, &mut seen_strength, "opening strength")?;
            output.push(input(
                context,
                "nhl.team_strength",
                "icecast_team_strength.v1",
                &row.team,
                "strength_points",
                Some(row.strength),
                row.valued_players as u64,
                row.value_coverage,
                row.value_coverage,
                WindowProfileStatus::Modeled,
                evidence.clone(),
                Vec::new(),
            ));
        }
        if forecast.opening_strengths.is_empty()
            && forecast.replay_checkpoint.is_none()
            && forecast
                .games
                .iter()
                .all(|game| game.evidence_cutoff_date.is_none())
        {
            let mut frozen_strengths = BTreeMap::<String, Vec<f64>>::new();
            for game in &forecast.games {
                for (team, strength) in [
                    (&game.away_team, game.away_strength),
                    (&game.home_team, game.home_strength),
                ] {
                    if !expected.contains(team) || !strength.is_finite() {
                        return Err(OrganizationWindowError::InvalidProfileInput(
                            "frozen team-season game strengths require canonical teams and finite values"
                                .to_owned(),
                        ));
                    }
                    frozen_strengths
                        .entry(team.clone())
                        .or_default()
                        .push(strength);
                }
            }
            for (team, values) in frozen_strengths {
                let first = values[0];
                if values.iter().any(|value| (value - first).abs() > 1e-9) {
                    return Err(OrganizationWindowError::InvalidProfileInput(format!(
                        "frozen team-season forecast has dynamic strength for {team}"
                    )));
                }
                require_team(&team, &expected, &mut seen_strength, "frozen strength")?;
                let coverage = (values.len() as f64 / 84.0).min(1.0);
                output.push(input(
                    context,
                    "nhl.team_strength",
                    "icecast_team_strength.v1",
                    &team,
                    "strength_points",
                    Some(first),
                    values.len() as u64,
                    coverage,
                    coverage,
                    WindowProfileStatus::Modeled,
                    evidence.clone(),
                    vec![
                        "Strength is the frozen preseason IceCast game feature; rolling replay and dated evidence-cutoff rows are excluded to prevent future-result leakage."
                            .to_owned(),
                    ],
                ));
            }
        }
        let mut seen_points = BTreeSet::new();
        for row in &forecast.teams {
            require_team(&row.team, &expected, &mut seen_points, "season forecast")?;
            output.push(input(
                context,
                "nhl.expected_points",
                "icecast_expected_points.v1",
                &row.team,
                "standings_points",
                Some(row.average_points),
                forecast.trials as u64,
                trial_confidence(forecast.trials),
                1.0,
                WindowProfileStatus::Modeled,
                evidence.clone(),
                Vec::new(),
            ));
        }
    }

    adapt_lineups(context, &expected, sources.team_lineups, &mut output)?;
    adapt_organization_lineups(
        context,
        &expected,
        sources.team_lineups,
        sources.ahl_affiliates,
        sources.organization_lineups,
        &mut output,
    )?;
    adapt_profile_history_fallback(context, sources.profile_history, &mut output)?;
    adapt_prospect_program(context, &expected, sources.prospect_program, &mut output)?;
    adapt_prospect_conversion(context, &expected, sources.prospect_conversion, &mut output)?;
    adapt_training_camp(context, &expected, sources.training_camp, &mut output)?;
    adapt_schedule(
        context,
        &expected,
        sources.team_game_forecast,
        sources.schedule_rest,
        &mut output,
    )?;
    Ok(output)
}

fn adapt_profile_history_fallback(
    context: &OrganizationWindowAdapterContext,
    history: Option<&OrganizationProfileHistoryView>,
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let Some(history) = history else {
        return Ok(());
    };
    let rules = [
        OrganizationProfileCarryForwardRule {
            profile_key: "development.organization_depth".to_owned(),
            method_version: "organization_lineup_depth.v1".to_owned(),
            maximum_season_age: 1,
            annual_confidence_decay: 0.75,
        },
        OrganizationProfileCarryForwardRule {
            profile_key: "development.recall_depth".to_owned(),
            method_version: "organization_recall_depth.v1".to_owned(),
            maximum_season_age: 1,
            annual_confidence_decay: 0.70,
        },
    ];
    let carried =
        carry_forward_organization_profiles(history, context.season, context.as_of, &rules)
            .map_err(|error| OrganizationWindowError::InvalidProfileInput(error.to_string()))?;
    let present = output
        .iter()
        .map(|row| {
            (
                row.organization.clone(),
                row.profile_key.clone(),
                row.method_version.clone(),
            )
        })
        .collect::<BTreeSet<_>>();
    output.extend(carried.into_iter().filter(|row| {
        !present.contains(&(
            row.organization.clone(),
            row.profile_key.clone(),
            row.method_version.clone(),
        ))
    }));
    Ok(())
}

pub fn build_balanced_organization_window_board(
    context: OrganizationWindowAdapterContext,
    generated_at: impl Into<String>,
    sources: OrganizationWindowSourceSet<'_>,
) -> Result<OrganizationWindowBoardView, OrganizationWindowError> {
    let generated_at = generated_at.into();
    let inventory = load_organization_window_profile_inventory()?;
    let profile_inputs = adapt_balanced_organization_window_sources(&context, sources)?;
    let mut source_fingerprints = profile_inputs
        .iter()
        .flat_map(|row| row.source_fingerprints.iter().cloned())
        .collect::<Vec<_>>();
    let lifecycle = load_organization_window_registry_lifecycle(&inventory)
        .map_err(|error| OrganizationWindowError::InvalidManifest(error.to_string()))?;
    let manifest = seal_new_organization_window_manifest(
        balanced_organization_window_manifest(BALANCED_MANIFEST_CREATED_AT),
        &inventory,
        &lifecycle,
        &WindowManifestLifecyclePolicy::official(),
    )
    .map_err(|error| OrganizationWindowError::InvalidManifest(error.to_string()))?;
    source_fingerprints.push(format!("registry-lifecycle:{}", lifecycle.fingerprint));
    build_organization_window_board(
        OrganizationWindowBoardInput {
            season: context.season,
            season_type: context.season_type,
            as_of: context.as_of,
            generated_at: generated_at.clone(),
            manifest,
            profile_inputs,
            source_fingerprints,
        },
        &inventory,
    )
}

/// Project a sealed IceCast history into comparable, NHL-strength-only Window
/// checkpoints. This intentionally does not imply that the other Window panes
/// were observed at each cutoff.
pub fn build_forecast_history_organization_window_boards(
    history: &TeamSeasonForecastHistoryView,
    generated_at: impl Into<String>,
) -> Result<Vec<OrganizationWindowBoardView>, OrganizationWindowError> {
    if history.schema != TEAM_SEASON_FORECAST_HISTORY_SCHEMA || history.checkpoints.len() < 2 {
        return Err(OrganizationWindowError::InvalidProfileInput(
            "forecast history must contain at least two sealed checkpoints".to_owned(),
        ));
    }
    let expected = canonical_teams();
    if history.teams.len() != expected.len()
        || history
            .teams
            .iter()
            .map(|team| team.team.as_str())
            .collect::<BTreeSet<_>>()
            .len()
            != expected.len()
        || history
            .teams
            .iter()
            .any(|team| !expected.contains(&team.team))
    {
        return Err(OrganizationWindowError::InvalidProfileInput(
            "forecast history must contain the canonical 32-team cohort exactly once".to_owned(),
        ));
    }

    let generated_at = generated_at.into();
    let source = evidence(
        TEAM_SEASON_FORECAST_HISTORY_SCHEMA,
        history,
        None,
        &OrganizationWindowAdapterContext {
            season: history.season,
            season_type: "regular".to_owned(),
            as_of: history.checkpoints[0].as_of_date,
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        },
    )?;
    let source_fingerprint = source[0].source_id.clone();
    let manifest = OrganizationWindowManifestView {
        schema: ORGANIZATION_WINDOW_MANIFEST_SCHEMA.to_owned(),
        manifest_id: ORGANIZATION_WINDOW_FORECAST_HISTORY_MANIFEST_ID.to_owned(),
        label: "IceCast forecast history Window".to_owned(),
        description: "A narrow within-season Frame containing only expected standings points from sealed IceCast checkpoints.".to_owned(),
        manifest_version: "1.0.0".to_owned(),
        comparison_cohort: WindowCohortManifest {
            kind: WindowCohortKind::SeasonCanonical,
            team_catalog_version: TEAM_CATALOG_VERSION.to_owned(),
            expected_organizations: CANONICAL_TEAMS
                .iter()
                .map(|(team, _)| (*team).to_owned())
                .collect(),
        },
        normalization_method: WindowNormalizationMethod::EmpiricalPercentile,
        primary_horizon: WindowHorizon::Current,
        dimensions: vec![dimension(
            "nhl_strength",
            "NHL strength",
            1.0,
            1.0,
            vec![profile(
                "nhl.expected_points",
                "icecast_expected_points.v1",
                1.0,
                true,
            )],
            Vec::new(),
        )],
        missing_policy: WindowMissingPolicy::WithholdRank,
        classification_method: ORGANIZATION_WINDOW_CLASSIFICATION_METHOD.to_owned(),
        created_at: generated_at.clone(),
        fingerprint: String::new(),
    };
    let inventory = load_organization_window_profile_inventory()?;
    let lifecycle = load_organization_window_registry_lifecycle(&inventory)
        .map_err(|error| OrganizationWindowError::InvalidManifest(error.to_string()))?;
    let manifest = seal_new_organization_window_manifest(
        manifest,
        &inventory,
        &lifecycle,
        &WindowManifestLifecyclePolicy::evaluation(),
    )
    .map_err(|error| OrganizationWindowError::InvalidManifest(error.to_string()))?;
    let mut boards = Vec::with_capacity(history.checkpoints.len());
    for checkpoint in &history.checkpoints {
        let context = OrganizationWindowAdapterContext {
            season: history.season,
            season_type: "regular".to_owned(),
            as_of: checkpoint.as_of_date,
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let mut profile_inputs = Vec::with_capacity(expected.len());
        for team in &history.teams {
            let point = team
                .checkpoints
                .iter()
                .find(|point| point.as_of_date == checkpoint.as_of_date)
                .ok_or_else(|| {
                    OrganizationWindowError::InvalidProfileInput(format!(
                        "forecast history is missing {} at {}",
                        team.team, checkpoint.as_of_date
                    ))
                })?;
            let mut checkpoint_evidence = source.clone();
            checkpoint_evidence[0].as_of = Some(checkpoint.as_of_date);
            profile_inputs.push(input(
                &context,
                "nhl.expected_points",
                "icecast_expected_points.v1",
                &team.team,
                "standings_points",
                Some(point.average_points),
                history.trials as u64,
                trial_confidence(history.trials),
                1.0,
                WindowProfileStatus::Modeled,
                checkpoint_evidence,
                vec![
                    "This checkpoint covers expected standings points only; no other organization-health pane is inferred."
                        .to_owned(),
                ],
            ));
        }
        boards.push(build_organization_window_board(
            OrganizationWindowBoardInput {
                season: history.season,
                season_type: "regular".to_owned(),
                as_of: checkpoint.as_of_date,
                generated_at: generated_at.clone(),
                manifest: manifest.clone(),
                profile_inputs,
                source_fingerprints: vec![
                    source_fingerprint.clone(),
                    format!("registry-lifecycle:{}", lifecycle.fingerprint),
                ],
            },
            &inventory,
        )?);
    }
    Ok(boards)
}

fn adapt_lineups(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    lineups: &[TeamLineupProjectionView],
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let mut seen = BTreeSet::new();
    for lineup in lineups {
        require_schema("team lineup", &lineup.schema, TEAM_LINEUP_PROJECTION_SCHEMA)?;
        require_team(&lineup.team, expected, &mut seen, "team lineup")?;
        if lineup.roster_season != context.season {
            return Err(context_error("team lineup season"));
        }
        let evidence = evidence(TEAM_LINEUP_PROJECTION_SCHEMA, lineup, None, context)?;
        let players = lineup_players(lineup);
        let forwards = lineup
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .filter_map(Option::as_ref)
            .collect::<Vec<_>>();
        let defense = lineup
            .defense_pairs
            .iter()
            .flat_map(|pair| [&pair.left, &pair.right])
            .filter_map(Option::as_ref)
            .collect::<Vec<_>>();
        let goalies = [&lineup.goalies.starter, &lineup.goalies.backup]
            .into_iter()
            .filter_map(Option::as_ref)
            .collect::<Vec<_>>();
        add_player_average(
            context,
            &lineup.team,
            PlayerAverageProfile {
                key: "nhl.forward_depth",
                method: "lineup_forward_depth.v1",
                unit: "icelines_score",
                expected_count: 12,
            },
            &forwards,
            &evidence,
            output,
        );
        add_player_average(
            context,
            &lineup.team,
            PlayerAverageProfile {
                key: "nhl.defense_depth",
                method: "lineup_defense_depth.v1",
                unit: "icelines_score",
                expected_count: 6,
            },
            &defense,
            &evidence,
            output,
        );
        add_player_average(
            context,
            &lineup.team,
            PlayerAverageProfile {
                key: "nhl.goalie_quality",
                method: "lineup_goalie_quality.v1",
                unit: "icelines_score",
                expected_count: 2,
            },
            &goalies,
            &evidence,
            output,
        );

        let goalie_values = goalies
            .iter()
            .filter_map(|player| player.score.value)
            .collect::<Vec<_>>();
        let goalie_coverage = goalie_values.len() as f64 / 2.0;
        let goalie_dependency =
            (goalie_values.len() == 2).then(|| (goalie_values[0] - goalie_values[1]).abs());
        output.push(input(
            context,
            "resilience.goalie_dependency",
            "lineup_goalie_dependency.v1",
            &lineup.team,
            "score_concentration",
            goalie_dependency,
            goalies.iter().map(|p| p.score.sample_games as u64).sum(),
            average_confidence(&goalies),
            goalie_coverage,
            status_for(goalie_dependency, goalie_coverage),
            evidence.clone(),
            missing_limitation(goalie_dependency, "starter and backup scores are required"),
        ));

        add_unit_average(
            context,
            &lineup.team,
            "deployment.power_play_depth",
            "lineup_power_play_depth.v1",
            &lineup.special_teams.power_play,
            &evidence,
            output,
        );
        add_unit_average(
            context,
            &lineup.team,
            "deployment.penalty_kill_depth",
            "lineup_penalty_kill_depth.v1",
            &lineup.special_teams.penalty_kill,
            &evidence,
            output,
        );

        let mut values = players
            .iter()
            .filter_map(|player| player.score.value)
            .filter(|value| *value > 0.0)
            .collect::<Vec<_>>();
        values.sort_by(|a, b| b.total_cmp(a));
        let total = values.iter().sum::<f64>();
        let concentration =
            (total > 0.0).then(|| 100.0 * values.iter().take(5).sum::<f64>() / total);
        let coverage = (values.len() as f64 / 20.0).min(1.0);
        output.push(input(
            context,
            "resilience.roster_concentration",
            "lineup_value_concentration.v1",
            &lineup.team,
            "top_share_pct",
            concentration,
            players.iter().map(|p| p.score.sample_games as u64).sum(),
            average_confidence(&players),
            coverage,
            status_for(concentration, coverage),
            evidence,
            missing_limitation(concentration, "positive player scores are required"),
        ));
    }
    Ok(())
}

/// Join every represented NHL lineup to its sealed AHL affiliate projection.
/// Missing counterparts remain missing rather than producing synthetic depth.
pub fn build_organization_lineup_forecasts_from_affiliates(
    team_lineups: &[TeamLineupProjectionView],
    affiliates: &[AhlAffiliateProjectionView],
) -> Result<Vec<OrganizationLineupForecastView>, OrganizationWindowError> {
    let expected = canonical_teams();
    let mut lineups = BTreeMap::new();
    for lineup in team_lineups {
        require_schema("team lineup", &lineup.schema, TEAM_LINEUP_PROJECTION_SCHEMA)?;
        if !expected.contains(&lineup.team) {
            return Err(OrganizationWindowError::InvalidProfileInput(format!(
                "team lineup contains unknown team {}",
                lineup.team
            )));
        }
        if lineups.insert(lineup.team.as_str(), lineup).is_some() {
            return Err(OrganizationWindowError::DuplicateProfileInput(format!(
                "team lineup:{}",
                lineup.team
            )));
        }
    }
    let mut by_team = BTreeMap::new();
    for affiliate in affiliates {
        require_schema(
            "AHL affiliate",
            &affiliate.schema,
            AHL_AFFILIATE_PROJECTION_SCHEMA,
        )?;
        if !expected.contains(&affiliate.nhl_team) {
            return Err(OrganizationWindowError::InvalidProfileInput(format!(
                "AHL affiliate contains unknown NHL team {}",
                affiliate.nhl_team
            )));
        }
        if by_team
            .insert(affiliate.nhl_team.as_str(), affiliate)
            .is_some()
        {
            return Err(OrganizationWindowError::DuplicateProfileInput(format!(
                "AHL affiliate:{}",
                affiliate.nhl_team
            )));
        }
    }
    by_team
        .into_iter()
        .filter_map(|(team, affiliate)| lineups.get(team).map(|lineup| (*lineup, affiliate)))
        .map(|(lineup, affiliate)| {
            build_organization_lineup_forecast(&OrganizationLineupForecastInput {
                nhl_lineup: lineup.clone(),
                ahl_affiliate: affiliate.clone(),
            })
            .map_err(|message| {
                OrganizationWindowError::InvalidProfileInput(format!(
                    "organization lineup composition failed for {}: {message}",
                    affiliate.nhl_team
                ))
            })
        })
        .collect()
}

fn adapt_organization_lineups(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    team_lineups: &[TeamLineupProjectionView],
    affiliates: &[AhlAffiliateProjectionView],
    forecasts: &[OrganizationLineupForecastView],
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    if affiliates
        .iter()
        .any(|affiliate| affiliate.season != context.season)
    {
        return Err(context_error("AHL affiliate season"));
    }
    let derived = build_organization_lineup_forecasts_from_affiliates(team_lineups, affiliates)?;
    let mut seen = BTreeSet::new();
    for forecast in forecasts.iter().chain(derived.iter()) {
        require_schema(
            "organization lineup",
            &forecast.schema,
            ORGANIZATION_LINEUP_FORECAST_SCHEMA,
        )?;
        require_team(
            &forecast.nhl_team,
            expected,
            &mut seen,
            "organization lineup",
        )?;
        if forecast.season != context.season {
            return Err(context_error("organization lineup season"));
        }
        let evidence = evidence(ORGANIZATION_LINEUP_FORECAST_SCHEMA, forecast, None, context)?;
        let unit_values = forecast
            .units
            .iter()
            .filter_map(|unit| unit.average_score)
            .collect::<Vec<_>>();
        let unit_value = mean(&unit_values);
        let unit_coverage = if forecast.units.is_empty() {
            0.0
        } else {
            unit_values.len() as f64 / forecast.units.len() as f64
        };
        output.push(input(
            context,
            "development.organization_depth",
            "organization_lineup_depth.v1",
            &forecast.nhl_team,
            "average_unit_score",
            unit_value,
            forecast.units.len() as u64,
            unit_coverage,
            unit_coverage,
            status_for(unit_value, unit_coverage),
            evidence.clone(),
            missing_limitation(unit_value, "scored NHL/AHL units are required"),
        ));

        let recall_values = forecast
            .recall_ladder
            .iter()
            .filter(|row| row.rank <= 2)
            .filter_map(|row| row.recall_readiness)
            .collect::<Vec<_>>();
        let recall_value = mean(&recall_values);
        let recall_coverage = (forecast
            .recall_plan
            .iter()
            .filter(|plan| plan.first_recall_player_id.is_some())
            .count() as f64
            / 3.0)
            .min(1.0);
        output.push(input(
            context,
            "development.recall_depth",
            "organization_recall_depth.v1",
            &forecast.nhl_team,
            "recall_score",
            recall_value,
            recall_values.len() as u64,
            recall_coverage,
            recall_coverage,
            status_for(recall_value, recall_coverage),
            evidence,
            missing_limitation(recall_value, "recall candidates are required"),
        ));
    }
    Ok(())
}

fn adapt_prospect_program(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    board: Option<&ProspectProgramBoardView>,
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let Some(board) = board else {
        return Ok(());
    };
    require_schema(
        "prospect program",
        &board.schema,
        PROSPECT_PROGRAM_BOARD_SCHEMA,
    )?;
    if board.as_of_season > context.season {
        return Err(context_error("prospect program season"));
    }
    let evidence = evidence(PROSPECT_PROGRAM_BOARD_SCHEMA, board, None, context)?;
    let mut seen = BTreeSet::new();
    for row in &board.programs {
        require_team(&row.organization, expected, &mut seen, "prospect program")?;
        let coverage = if row.supplied_study_count == 0 {
            0.0
        } else {
            row.prospect_count as f64 / row.supplied_study_count as f64
        };
        let confidence = (row.components.confidence / 100.0).clamp(0.0, 1.0);
        for (key, method, value, unit) in [
            (
                "pipeline.prospect_pool",
                "prospect_pool_score.v1",
                row.pool_score,
                "pipeline_score",
            ),
            (
                "pipeline.prospect_development",
                "prospect_development_score.v1",
                row.development_score,
                "development_score",
            ),
            (
                "pipeline.prospect_readiness",
                "prospect_readiness_score.v1",
                row.components.readiness,
                "readiness_score",
            ),
        ] {
            output.push(input(
                context,
                key,
                method,
                &row.organization,
                unit,
                Some(value),
                row.prospect_count as u64,
                confidence,
                coverage.clamp(0.0, 1.0),
                WindowProfileStatus::Modeled,
                evidence.clone(),
                Vec::new(),
            ));
        }
    }
    Ok(())
}

fn adapt_prospect_conversion(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    board: Option<&ProspectConversionBoardView>,
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let Some(board) = board else {
        return Ok(());
    };
    require_schema(
        "prospect conversion",
        &board.schema,
        PROSPECT_CONVERSION_BOARD_SCHEMA,
    )?;
    let evidence = evidence(PROSPECT_CONVERSION_BOARD_SCHEMA, board, None, context)?;
    let mut seen = BTreeSet::new();
    for row in &board.programs {
        require_team(
            &row.organization,
            expected,
            &mut seen,
            "prospect conversion",
        )?;
        let rankable = row.conversion_rank.is_some() && row.rank_blockers.is_empty();
        output.push(input(
            context,
            "development.prospect_conversion",
            "prospect_conversion_efficiency.v1",
            &row.organization,
            "conversion_score",
            rankable.then_some(row.efficiency_index),
            row.players as u64,
            row.baseline_confidence.clamp(0.0, 1.0),
            row.outcome_coverage.clamp(0.0, 1.0),
            if rankable {
                WindowProfileStatus::Observed
            } else {
                WindowProfileStatus::Blocked
            },
            evidence.clone(),
            if rankable {
                Vec::new()
            } else {
                vec!["upstream conversion rank is blocked".to_owned()]
            },
        ));
    }
    Ok(())
}

fn adapt_training_camp(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    league: Option<&TrainingCampLeagueForecastView>,
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let Some(league) = league else {
        return Ok(());
    };
    require_schema(
        "training camp",
        &league.schema,
        TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA,
    )?;
    if league.season != context.season {
        return Err(context_error("training camp season"));
    }
    let evidence = evidence(TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA, league, None, context)?;
    let mut seen = BTreeSet::new();
    for team in &league.teams {
        require_team(&team.team, expected, &mut seen, "training camp")?;
        let Some(forecast) = &team.forecast else {
            continue;
        };
        let candidates = forecast
            .players
            .iter()
            .filter(|player| player.rookie_eligible && player.prospect)
            .collect::<Vec<_>>();
        let value = candidates
            .iter()
            .map(|player| player.make_probability)
            .sum::<f64>();
        let coverage = if forecast.trials == 0 {
            0.0
        } else {
            forecast.valid_trials as f64 / forecast.trials as f64
        };
        output.push(input(
            context,
            "pipeline.training_camp_arrival",
            "training_camp_arrival.v1",
            &team.team,
            "expected_rookies",
            Some(value),
            forecast.valid_trials as u64,
            trial_confidence(forecast.valid_trials) * coverage,
            coverage,
            WindowProfileStatus::Modeled,
            evidence.clone(),
            team.authority_warnings.clone(),
        ));
    }
    Ok(())
}

/// Derive one rest/fatigue authority per represented club from the sealed game
/// forecast. This keeps schedule interpretation in core and makes the result
/// reusable by CLI, web, TUI, and downstream renderers.
pub fn build_schedule_rest_profiles_from_game_forecast(
    forecast: &TeamGameForecastView,
) -> Result<Vec<ScheduleRestProfileView>, OrganizationWindowError> {
    require_schema(
        "team game forecast",
        &forecast.schema,
        TEAM_GAME_FORECAST_SCHEMA,
    )?;
    let expected = canonical_teams();
    let mut game_ids = BTreeSet::new();
    let mut by_team = BTreeMap::<String, Vec<ScheduleRestGameInput>>::new();
    for game in &forecast.games {
        if !game_ids.insert(game.game_id) {
            return Err(OrganizationWindowError::InvalidProfileInput(format!(
                "team game forecast repeats game {}",
                game.game_id
            )));
        }
        if game.home_team == game.away_team
            || !expected.contains(&game.home_team)
            || !expected.contains(&game.away_team)
        {
            return Err(OrganizationWindowError::InvalidProfileInput(format!(
                "team game forecast game {} requires distinct canonical NHL teams",
                game.game_id
            )));
        }
        let home_load = BenchScheduleLoad {
            is_home: true,
            back_to_back: game.home_context.back_to_back,
            third_game_in_four_nights: game.home_context.three_in_four,
            travel_km: game.home_context.travel_km,
        };
        let away_load = BenchScheduleLoad {
            is_home: false,
            back_to_back: game.away_context.back_to_back,
            third_game_in_four_nights: game.away_context.three_in_four,
            travel_km: game.away_context.travel_km,
        };
        by_team
            .entry(game.home_team.clone())
            .or_default()
            .push(ScheduleRestGameInput {
                opponent: game.away_team.clone(),
                team_load: home_load,
                opponent_load: away_load,
            });
        by_team
            .entry(game.away_team.clone())
            .or_default()
            .push(ScheduleRestGameInput {
                opponent: game.home_team.clone(),
                team_load: away_load,
                opponent_load: home_load,
            });
    }
    CANONICAL_TEAMS
        .iter()
        .filter_map(|(team, _)| by_team.remove(*team).map(|games| (*team, games)))
        .map(|(team, games)| {
            build_schedule_rest_profile(team, &games).map_err(|message| {
                OrganizationWindowError::InvalidProfileInput(format!(
                    "team game forecast schedule derivation failed for {team}: {message}"
                ))
            })
        })
        .collect()
}

fn adapt_schedule(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    forecast: Option<&TeamGameForecastView>,
    profiles: &[ScheduleRestProfileView],
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let derived = if let Some(forecast) = forecast {
        if forecast.season != context.season {
            return Err(OrganizationWindowError::ContextMismatch(format!(
                "team game forecast season {} does not match Window season {}",
                forecast.season, context.season
            )));
        }
        build_schedule_rest_profiles_from_game_forecast(forecast)?
    } else {
        Vec::new()
    };
    let mut seen = BTreeSet::new();
    for row in profiles.iter().chain(derived.iter()) {
        require_schema("schedule rest", &row.schema, SCHEDULE_REST_PROFILE_SCHEMA)?;
        require_team(&row.team, expected, &mut seen, "schedule rest")?;
        let evidence = evidence(SCHEDULE_REST_PROFILE_SCHEMA, row, None, context)?;
        let value = (row.own_back_to_backs + row.own_third_in_four) as f64;
        let coverage = (row.games as f64 / 84.0).min(1.0);
        output.push(input(
            context,
            "resilience.schedule_fatigue",
            "schedule_fatigue_exposure.v1",
            &row.team,
            "fatigue_games",
            Some(value),
            row.games as u64,
            coverage,
            coverage,
            WindowProfileStatus::Observed,
            evidence,
            row.disclosures.clone(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct PlayerAverageProfile<'a> {
    key: &'a str,
    method: &'a str,
    unit: &'a str,
    expected_count: usize,
}

fn add_player_average(
    context: &OrganizationWindowAdapterContext,
    team: &str,
    profile: PlayerAverageProfile<'_>,
    players: &[&TeamLineupPlayerView],
    evidence: &[WindowEvidenceView],
    output: &mut Vec<OrganizationProfileInput>,
) {
    let values = players
        .iter()
        .filter_map(|player| player.score.value)
        .collect::<Vec<_>>();
    let value = mean(&values);
    let coverage = (values.len() as f64 / profile.expected_count as f64).min(1.0);
    output.push(input(
        context,
        profile.key,
        profile.method,
        team,
        profile.unit,
        value,
        players
            .iter()
            .map(|player| player.score.sample_games as u64)
            .sum(),
        average_confidence(players),
        coverage,
        status_for(value, coverage),
        evidence.to_vec(),
        missing_limitation(value, "scored lineup players are required"),
    ));
}

fn add_unit_average(
    context: &OrganizationWindowAdapterContext,
    team: &str,
    key: &str,
    method: &str,
    units: &[TeamLineupSpecialTeamsUnitView],
    evidence: &[WindowEvidenceView],
    output: &mut Vec<OrganizationProfileInput>,
) {
    let values = units
        .iter()
        .filter_map(|unit| unit.average_role_score)
        .collect::<Vec<_>>();
    let value = mean(&values);
    let coverage = (values.len() as f64 / 2.0).min(1.0);
    output.push(input(
        context,
        key,
        method,
        team,
        "role_score",
        value,
        units.iter().map(|unit| unit.player_ids.len() as u64).sum(),
        coverage,
        coverage,
        status_for(value, coverage),
        evidence.to_vec(),
        missing_limitation(value, "two scored special-teams units are required"),
    ));
}

fn lineup_players(lineup: &TeamLineupProjectionView) -> Vec<&TeamLineupPlayerView> {
    let mut players = BTreeMap::new();
    for line in &lineup.forward_lines {
        for player in [&line.left_wing, &line.center, &line.right_wing]
            .into_iter()
            .filter_map(Option::as_ref)
        {
            players.insert(player.player_id, player);
        }
    }
    for pair in &lineup.defense_pairs {
        for player in [&pair.left, &pair.right]
            .into_iter()
            .filter_map(Option::as_ref)
        {
            players.insert(player.player_id, player);
        }
    }
    for player in [&lineup.goalies.starter, &lineup.goalies.backup]
        .into_iter()
        .filter_map(Option::as_ref)
    {
        players.insert(player.player_id, player);
    }
    for player in &lineup.extras {
        players.insert(player.player_id, player);
    }
    players.into_values().collect()
}

fn average_confidence(players: &[&TeamLineupPlayerView]) -> f64 {
    if players.is_empty() {
        return 0.0;
    }
    players
        .iter()
        .map(|player| player.score.coverage_pct / 100.0)
        .sum::<f64>()
        / players.len() as f64
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

fn status_for(value: Option<f64>, coverage: f64) -> WindowProfileStatus {
    if value.is_none() {
        WindowProfileStatus::Blocked
    } else if coverage + f64::EPSILON >= 1.0 {
        WindowProfileStatus::Modeled
    } else {
        WindowProfileStatus::Provisional
    }
}

fn missing_limitation(value: Option<f64>, message: &str) -> Vec<String> {
    if value.is_some() {
        Vec::new()
    } else {
        vec![message.to_owned()]
    }
}

#[allow(clippy::too_many_arguments)]
fn input(
    context: &OrganizationWindowAdapterContext,
    profile_key: &str,
    method_version: &str,
    organization: &str,
    raw_unit: &str,
    raw_value: Option<f64>,
    sample_size: u64,
    confidence: f64,
    coverage: f64,
    status: WindowProfileStatus,
    evidence: Vec<WindowEvidenceView>,
    limitations: Vec<String>,
) -> OrganizationProfileInput {
    OrganizationProfileInput {
        profile_key: profile_key.to_owned(),
        method_version: method_version.to_owned(),
        organization: organization.to_owned(),
        organization_identity_version: context.organization_identity_version.clone(),
        season: context.season,
        season_type: context.season_type.clone(),
        as_of: context.as_of,
        horizon: context.horizon,
        raw_value,
        raw_unit: raw_unit.to_owned(),
        sample_size,
        confidence: confidence.clamp(0.0, 1.0),
        coverage: coverage.clamp(0.0, 1.0),
        status,
        source_fingerprints: evidence.iter().map(|row| row.source_id.clone()).collect(),
        evidence,
        limitations,
    }
}

fn evidence<T: Serialize>(
    schema: &str,
    document: &T,
    source_as_of: Option<NaiveDate>,
    context: &OrganizationWindowAdapterContext,
) -> Result<Vec<WindowEvidenceView>, OrganizationWindowError> {
    let bytes = serde_json::to_vec(document)
        .map_err(|error| OrganizationWindowError::InvalidJson(error.to_string()))?;
    let source_id = format!("sha256:{:x}", Sha256::digest(bytes));
    Ok(vec![WindowEvidenceView {
        source_schema: schema.to_owned(),
        source_id,
        captured_at: None,
        as_of: source_as_of,
        freshness: if source_as_of.is_none_or(|date| date <= context.as_of) {
            WindowFreshness::Current
        } else {
            WindowFreshness::Unknown
        },
        source_url: None,
    }])
}

fn canonical_teams() -> BTreeSet<String> {
    CANONICAL_TEAMS
        .iter()
        .map(|(abbreviation, _)| (*abbreviation).to_owned())
        .collect()
}

fn require_team(
    team: &str,
    expected: &BTreeSet<String>,
    seen: &mut BTreeSet<String>,
    source: &str,
) -> Result<(), OrganizationWindowError> {
    if !expected.contains(team) {
        return Err(OrganizationWindowError::InvalidProfileInput(format!(
            "{source} contains unknown team {team}"
        )));
    }
    if !seen.insert(team.to_owned()) {
        return Err(OrganizationWindowError::DuplicateProfileInput(format!(
            "{source}:{team}"
        )));
    }
    Ok(())
}

fn require_schema(label: &str, found: &str, expected: &str) -> Result<(), OrganizationWindowError> {
    if found != expected {
        return Err(OrganizationWindowError::UnsupportedSchema {
            contract: "Window source adapter",
            found: format!("{label}:{found}"),
        });
    }
    Ok(())
}

fn validate_context(
    context: &OrganizationWindowAdapterContext,
) -> Result<(), OrganizationWindowError> {
    if context.season_type.trim().is_empty()
        || context.organization_identity_version.trim().is_empty()
    {
        return Err(context_error("adapter context"));
    }
    if context.horizon != WindowHorizon::Current {
        return Err(context_error("balanced.v1 requires the current horizon"));
    }
    Ok(())
}

fn context_error(label: &str) -> OrganizationWindowError {
    OrganizationWindowError::ContextMismatch(label.to_owned())
}

fn trial_confidence(trials: u32) -> f64 {
    (trials as f64 / 10_000.0).sqrt().min(1.0)
}

fn profile(key: &str, method: &str, weight: f64, required: bool) -> WindowProfileWeight {
    WindowProfileWeight {
        profile_key: key.to_owned(),
        method_version: method.to_owned(),
        weight,
        required,
    }
}

fn cap(family: &str, maximum_weight: f64) -> WindowSignalFamilyCap {
    WindowSignalFamilyCap {
        signal_family: family.to_owned(),
        maximum_weight,
    }
}

fn dimension(
    key: &str,
    label: &str,
    weight: f64,
    minimum_coverage: f64,
    profiles: Vec<WindowProfileWeight>,
    signal_family_caps: Vec<WindowSignalFamilyCap>,
) -> WindowDimensionManifest {
    WindowDimensionManifest {
        key: key.to_owned(),
        label: label.to_owned(),
        weight,
        minimum_coverage,
        rank_required: true,
        profiles,
        signal_family_caps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Position;
    use crate::view_model::ahl_affiliate::{
        build_ahl_affiliate_projection, AhlAffiliatePlayerInput, AhlAffiliateProjectionInput,
        AhlDevelopmentRuleInput,
    };
    use crate::view_model::organization_window::{
        seal_organization_window_manifest, WindowProfileReadiness,
    };
    use crate::view_model::organization_window_comparison::build_organization_window_history;
    use crate::view_model::team_game_forecast::{
        build_team_game_forecast, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput,
    };
    use crate::view_model::team_season_forecast::{
        simulate_team_season_forecast, TeamSeasonSimulationConfig,
    };

    fn test_nyr_affiliate() -> AhlAffiliateProjectionView {
        let mut players = Vec::new();
        for index in 0..12 {
            let position = if index < 4 {
                Position::Center
            } else if index % 2 == 0 {
                Position::LeftWing
            } else {
                Position::RightWing
            };
            players.push(AhlAffiliatePlayerInput {
                player_id: 9_100_000 + index,
                display_name: format!("Hartford Forward {}", index + 1),
                primary_position: position,
                eligible_positions: vec![position],
                projected_score: 70.0 - index as f64,
                prospect: true,
                recall_readiness: Some(0.9 - index as f64 * 0.02),
                professional_games_at_season_start: Some(100),
                development_rule_qualified: None,
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL".to_owned(),
            });
        }
        for index in 0..6 {
            players.push(AhlAffiliatePlayerInput {
                player_id: 9_200_000 + index,
                display_name: format!("Hartford Defense {}", index + 1),
                primary_position: Position::Defense,
                eligible_positions: vec![Position::Defense],
                projected_score: 65.0 - index as f64,
                prospect: true,
                recall_readiness: Some(0.8 - index as f64 * 0.02),
                professional_games_at_season_start: Some(100),
                development_rule_qualified: None,
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL".to_owned(),
            });
        }
        for index in 0..2 {
            players.push(AhlAffiliatePlayerInput {
                player_id: 9_300_000 + index,
                display_name: format!("Hartford Goalie {}", index + 1),
                primary_position: Position::Goalie,
                eligible_positions: vec![Position::Goalie],
                projected_score: 60.0 - index as f64,
                prospect: true,
                recall_readiness: Some(0.7 - index as f64 * 0.05),
                professional_games_at_season_start: None,
                development_rule_qualified: None,
                assigned_to_affiliate: true,
                waiver_required: false,
                source_league: "AHL".to_owned(),
            });
        }
        build_ahl_affiliate_projection(&AhlAffiliateProjectionInput {
            nhl_team: "NYR".to_owned(),
            ahl_team: "Hartford Wolf Pack".to_owned(),
            season: 20262027,
            rule: AhlDevelopmentRuleInput::default(),
            pool_authority: Default::default(),
            players,
        })
        .unwrap()
    }

    #[test]
    fn balanced_manifest_seals_and_uses_only_adapter_ready_profiles() {
        let inventory = load_organization_window_profile_inventory().unwrap();
        let manifest = seal_organization_window_manifest(
            balanced_organization_window_manifest("2026-07-27T00:00:00Z"),
            &inventory,
        )
        .unwrap();
        assert_eq!(manifest.dimensions.len(), 5);
        assert_eq!(
            manifest
                .dimensions
                .iter()
                .map(|dimension| dimension.profiles.len())
                .sum::<usize>(),
            17
        );
        for configured in manifest
            .dimensions
            .iter()
            .flat_map(|dimension| &dimension.profiles)
        {
            assert_eq!(
                inventory
                    .find(&configured.profile_key, &configured.method_version)
                    .unwrap()
                    .readiness,
                WindowProfileReadiness::ReadyForAdapter
            );
        }
        assert_eq!(manifest.fingerprint.len(), 64);
        assert!(manifest
            .fingerprint
            .chars()
            .all(|character| character.is_ascii_hexdigit()));
    }

    #[test]
    fn standing_history_fills_only_missing_ahl_profiles_for_all_32_teams() {
        let mut observations = Vec::new();
        for (index, (team, _)) in CANONICAL_TEAMS.iter().enumerate() {
            for (profile_key, method_version, raw_unit, value) in [
                (
                    "development.organization_depth",
                    "organization_lineup_depth.v1",
                    "average_unit_score",
                    40.0 + index as f64,
                ),
                (
                    "development.recall_depth",
                    "organization_recall_depth.v1",
                    "recall_score",
                    0.35 + index as f64 / 100.0,
                ),
            ] {
                observations.push(OrganizationProfileInput {
                    profile_key: profile_key.to_owned(),
                    method_version: method_version.to_owned(),
                    organization: (*team).to_owned(),
                    organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
                    season: 20252026,
                    season_type: "regular".to_owned(),
                    as_of: NaiveDate::from_ymd_opt(2026, 6, 30).unwrap(),
                    horizon: WindowHorizon::Current,
                    raw_value: Some(value),
                    raw_unit: raw_unit.to_owned(),
                    sample_size: 16,
                    confidence: 0.8,
                    coverage: 1.0,
                    status: WindowProfileStatus::Observed,
                    evidence: Vec::new(),
                    limitations: Vec::new(),
                    source_fingerprints: vec![format!("observed:{team}:{profile_key}")],
                });
            }
        }
        let history = seal_organization_profile_history(OrganizationProfileHistoryView {
            schema: super::super::organization_profile_history::ORGANIZATION_PROFILE_HISTORY_SCHEMA
                .to_owned(),
            history_id: "2025-26-standing-baseline".to_owned(),
            created_at: "2026-07-29T12:00:00Z".to_owned(),
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
            observations,
            source_fingerprints: vec!["observed-ahl-2025-26".to_owned()],
            disclosures: Vec::new(),
            fingerprint: String::new(),
        })
        .unwrap();
        let context = OrganizationWindowAdapterContext {
            season: 20262027,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt(2026, 7, 29).unwrap(),
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let inputs = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                profile_history: Some(&history),
                ..OrganizationWindowSourceSet::default()
            },
        )
        .unwrap();
        assert_eq!(inputs.len(), 64);
        assert!(inputs.iter().all(|input| {
            input.status == WindowProfileStatus::Modeled
                && input
                    .source_fingerprints
                    .iter()
                    .any(|fingerprint| fingerprint.starts_with("organization-profile-history:"))
        }));

        let mut current = vec![inputs
            .iter()
            .find(|input| {
                input.organization == "NYR" && input.profile_key == "development.organization_depth"
            })
            .unwrap()
            .clone()];
        current[0].raw_value = Some(99.0);
        current[0].source_fingerprints = vec!["current:NYR".to_owned()];
        adapt_profile_history_fallback(&context, Some(&history), &mut current).unwrap();
        assert_eq!(
            current
                .iter()
                .filter(|input| {
                    input.organization == "NYR"
                        && input.profile_key == "development.organization_depth"
                })
                .count(),
            1
        );
        assert_eq!(current[0].raw_value, Some(99.0));
    }

    #[test]
    fn empty_source_set_preserves_explicit_missingness() {
        let context = OrganizationWindowAdapterContext {
            season: 20262027,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let board = build_balanced_organization_window_board(
            context,
            "2026-10-01T00:00:00Z",
            OrganizationWindowSourceSet::default(),
        )
        .unwrap();
        assert_eq!(board.organizations.len(), 32);
        assert!(board
            .organizations
            .iter()
            .all(|row| row.overall.rank.is_none()));
        assert!(board
            .organizations
            .iter()
            .all(|row| !row.blockers.is_empty()));
        assert!(board
            .source_fingerprints
            .iter()
            .any(|fingerprint| fingerprint.starts_with("registry-lifecycle:")
                && fingerprint.len() == "registry-lifecycle:".len() + 64));
    }

    #[test]
    fn game_forecast_derives_schedule_rest_for_each_represented_team() {
        let games = vec![
            TeamForecastGameInput {
                game_id: 1,
                date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                away_team: "NYR".to_owned(),
                home_team: "SEA".to_owned(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            },
            TeamForecastGameInput {
                game_id: 2,
                date: NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
                away_team: "SEA".to_owned(),
                home_team: "NYR".to_owned(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            },
        ];
        let forecast = build_team_game_forecast(
            20262027,
            games,
            vec![
                TeamForecastStrengthInput {
                    team: "NYR".to_owned(),
                    strength: 50.0,
                },
                TeamForecastStrengthInput {
                    team: "SEA".to_owned(),
                    strength: 50.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();

        let profiles = build_schedule_rest_profiles_from_game_forecast(&forecast).unwrap();
        assert_eq!(profiles.len(), 2);
        assert!(profiles.iter().all(|profile| profile.games == 2));
        assert!(profiles
            .iter()
            .all(|profile| profile.own_back_to_backs == 1));

        let context = OrganizationWindowAdapterContext {
            season: 20262027,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let inputs = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                team_game_forecast: Some(&forecast),
                ..OrganizationWindowSourceSet::default()
            },
        )
        .unwrap();
        assert_eq!(
            inputs
                .iter()
                .filter(|input| input.profile_key == "resilience.schedule_fatigue")
                .count(),
            2
        );

        let duplicate = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                team_game_forecast: Some(&forecast),
                schedule_rest: &profiles[..1],
                ..OrganizationWindowSourceSet::default()
            },
        );
        assert!(matches!(
            duplicate,
            Err(OrganizationWindowError::DuplicateProfileInput(message))
                if message == "schedule rest:NYR"
        ));
    }

    #[test]
    fn affiliate_projection_composes_organization_depth_without_parallel_cli_logic() {
        let lineup: TeamLineupProjectionView = serde_json::from_str(include_str!(
            "../../../examples/team-lineup-alp-2026-27.json"
        ))
        .unwrap();
        let affiliate = test_nyr_affiliate();
        let forecasts = build_organization_lineup_forecasts_from_affiliates(
            std::slice::from_ref(&lineup),
            std::slice::from_ref(&affiliate),
        )
        .unwrap();
        assert_eq!(forecasts.len(), 1);
        assert_eq!(forecasts[0].nhl_team, "NYR");
        assert_eq!(forecasts[0].counts.forward_lines, 8);

        let context = OrganizationWindowAdapterContext {
            season: 20262027,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let inputs = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                team_lineups: std::slice::from_ref(&lineup),
                ahl_affiliates: std::slice::from_ref(&affiliate),
                ..OrganizationWindowSourceSet::default()
            },
        )
        .unwrap();
        assert!(inputs.iter().any(|input| {
            input.organization == "NYR"
                && input.profile_key == "development.organization_depth"
                && input.raw_value.is_some()
        }));
        assert!(inputs.iter().any(|input| {
            input.organization == "NYR"
                && input.profile_key == "development.recall_depth"
                && input.raw_value.is_some()
        }));

        let mut missing_readiness = affiliate.clone();
        for player in &mut missing_readiness.players {
            player.recall_readiness = None;
        }
        let missing_forecasts = build_organization_lineup_forecasts_from_affiliates(
            std::slice::from_ref(&lineup),
            std::slice::from_ref(&missing_readiness),
        )
        .unwrap();
        let missing_inputs = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                organization_lineups: &missing_forecasts,
                ..OrganizationWindowSourceSet::default()
            },
        )
        .unwrap();
        let recall = missing_inputs
            .iter()
            .find(|input| input.profile_key == "development.recall_depth")
            .unwrap();
        assert_eq!(recall.raw_value, None);

        let duplicate = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                team_lineups: std::slice::from_ref(&lineup),
                ahl_affiliates: std::slice::from_ref(&affiliate),
                organization_lineups: &forecasts,
                ..OrganizationWindowSourceSet::default()
            },
        );
        assert!(matches!(
            duplicate,
            Err(OrganizationWindowError::DuplicateProfileInput(message))
                if message == "organization lineup:NYR"
        ));

        let mut wrong_season = affiliate;
        wrong_season.season = 20252026;
        assert!(matches!(
            adapt_balanced_organization_window_sources(
                &context,
                OrganizationWindowSourceSet {
                    ahl_affiliates: std::slice::from_ref(&wrong_season),
                    ..OrganizationWindowSourceSet::default()
                },
            ),
            Err(OrganizationWindowError::ContextMismatch(message))
                if message == "AHL affiliate season"
        ));
    }

    #[test]
    fn frozen_game_features_supply_strength_but_dated_replay_features_do_not() {
        let games = vec![
            TeamForecastGameInput {
                game_id: 1,
                date: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                away_team: "NYR".to_owned(),
                home_team: "SEA".to_owned(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            },
            TeamForecastGameInput {
                game_id: 2,
                date: NaiveDate::from_ymd_opt(2026, 10, 2).unwrap(),
                away_team: "SEA".to_owned(),
                home_team: "NYR".to_owned(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            },
        ];
        let game_forecast = build_team_game_forecast(
            20262027,
            games,
            vec![
                TeamForecastStrengthInput {
                    team: "NYR".to_owned(),
                    strength: 62.0,
                },
                TeamForecastStrengthInput {
                    team: "SEA".to_owned(),
                    strength: 54.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();
        let partial_error = simulate_team_season_forecast(
            &game_forecast,
            TeamSeasonSimulationConfig {
                trials: 10,
                seed: 7,
            },
        )
        .unwrap_err();
        assert!(partial_error.contains("requires all 32 NHL teams"));

        let teams = CANONICAL_TEAMS
            .iter()
            .map(|(team, _)| *team)
            .collect::<Vec<_>>();
        let league_games = (0..84)
            .flat_map(|round| {
                teams.chunks_exact(2).enumerate().map(move |(pair, chunk)| {
                    let reverse = round % 2 == 1;
                    TeamForecastGameInput {
                        game_id: (round * 16 + pair) as u64,
                        date: NaiveDate::from_ymd_opt(2026, 9, 29).unwrap()
                            + chrono::Duration::days(round as i64),
                        away_team: chunk[usize::from(reverse)].to_owned(),
                        home_team: chunk[usize::from(!reverse)].to_owned(),
                        away_score: None,
                        home_score: None,
                        final_result: false,
                        last_period: None,
                    }
                })
            })
            .collect();
        let league_strengths = teams
            .iter()
            .map(|team| TeamForecastStrengthInput {
                team: (*team).to_owned(),
                strength: match *team {
                    "NYR" => 62.0,
                    "SEA" => 54.0,
                    _ => 50.0,
                },
            })
            .collect();
        let league_forecast = build_team_game_forecast(
            20262027,
            league_games,
            league_strengths,
            TeamForecastParameters::default(),
            Some(1_344),
            Some(84),
        )
        .unwrap();
        let mut season = simulate_team_season_forecast(
            &league_forecast,
            TeamSeasonSimulationConfig { trials: 2, seed: 7 },
        )
        .unwrap();
        let context = OrganizationWindowAdapterContext {
            season: 20262027,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let inputs = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                team_season_forecast: Some(&season),
                ..OrganizationWindowSourceSet::default()
            },
        )
        .unwrap();
        let strengths = inputs
            .iter()
            .filter(|input| input.profile_key == "nhl.team_strength")
            .collect::<Vec<_>>();
        assert_eq!(strengths.len(), 32);
        assert_eq!(
            strengths
                .iter()
                .find(|input| input.organization == "NYR")
                .and_then(|input| input.raw_value),
            Some(62.0)
        );

        season.games[0].evidence_cutoff_date = Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        let guarded = adapt_balanced_organization_window_sources(
            &context,
            OrganizationWindowSourceSet {
                team_season_forecast: Some(&season),
                ..OrganizationWindowSourceSet::default()
            },
        )
        .unwrap();
        assert!(!guarded
            .iter()
            .any(|input| input.profile_key == "nhl.team_strength"));
    }

    #[test]
    fn source_package_is_canonical_replayable_and_production_gated() {
        let package =
            seal_organization_window_source_package(OrganizationWindowSourcePackageView {
                schema: ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA.to_owned(),
                season: 20262027,
                season_type: "regular".to_owned(),
                as_of: NaiveDate::from_ymd_opt(2026, 10, 1).unwrap(),
                organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
                team_season_forecast: None,
                team_game_forecast: None,
                team_lineups: Vec::new(),
                ahl_affiliates: Vec::new(),
                organization_lineups: Vec::new(),
                prospect_program: None,
                prospect_conversion: None,
                training_camp: None,
                schedule_rest: Vec::new(),
                profile_history: None,
                fingerprint: String::new(),
            })
            .unwrap();
        assert_eq!(package.fingerprint.len(), 64);
        let replayed: OrganizationWindowSourcePackageView =
            serde_json::from_str(&serde_json::to_string(&package).unwrap()).unwrap();
        assert_eq!(
            seal_organization_window_source_package(replayed.clone()).unwrap(),
            package
        );

        let board = build_balanced_organization_window_board_from_package(
            &replayed,
            "2026-10-01T00:00:00Z",
        )
        .unwrap();
        let coverage =
            audit_organization_window_source_package(&replayed, "2026-10-01T00:00:00Z").unwrap();
        assert_eq!(coverage.profiles.len(), 17);
        assert_eq!(coverage.required_profiles, 16);
        assert_eq!(coverage.complete_required_profiles, 0);
        assert_eq!(coverage.rank_eligible_organizations, 0);
        assert!(!coverage.production_ranked);
        validate_organization_window_source_coverage(&coverage).unwrap();
        let mut tampered = coverage.clone();
        tampered.profiles[0].organizations_with_value += 1;
        assert!(validate_organization_window_source_coverage(&tampered).is_err());
        assert!(matches!(
            require_ranked_balanced_organization_window_board(&board),
            Err(OrganizationWindowError::InvalidBoard(message))
                if message.contains("32 organization(s) withheld")
        ));

        let mut tampered = package;
        tampered.as_of = NaiveDate::from_ymd_opt(2026, 10, 2).unwrap();
        assert!(matches!(
            seal_organization_window_source_package(tampered),
            Err(OrganizationWindowError::InvalidProfileInput(message))
                if message.contains("fingerprint mismatch")
        ));
    }

    #[test]
    fn source_package_schema_is_embedded_json() {
        let schema: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_SOURCE_PACKAGE_JSON_SCHEMA).unwrap();
        assert_eq!(
            schema["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA
        );
        assert_eq!(schema["properties"]["team_lineups"]["maxItems"], 32);
        assert_eq!(schema["properties"]["ahl_affiliates"]["maxItems"], 32);

        let coverage: serde_json::Value =
            serde_json::from_str(ORGANIZATION_WINDOW_SOURCE_COVERAGE_JSON_SCHEMA).unwrap();
        assert_eq!(
            coverage["properties"]["schema"]["const"],
            ORGANIZATION_WINDOW_SOURCE_COVERAGE_SCHEMA
        );
        assert_eq!(coverage["properties"]["profiles"]["minItems"], 17);
    }

    #[test]
    fn real_source_package_fingerprint_survives_wire_round_trip() {
        let lineup: TeamLineupProjectionView = serde_json::from_str(include_str!(
            "../../../examples/team-lineup-alp-2026-27.json"
        ))
        .unwrap();
        let camp: TrainingCampLeagueForecastView = serde_json::from_str(include_str!(
            "../../../examples/icecast-league-training-camp-2026-27.json"
        ))
        .unwrap();
        let package =
            seal_organization_window_source_package(OrganizationWindowSourcePackageView {
                schema: ORGANIZATION_WINDOW_SOURCE_PACKAGE_SCHEMA.to_owned(),
                season: 20262027,
                season_type: "regular".to_owned(),
                as_of: NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
                organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
                team_season_forecast: None,
                team_game_forecast: None,
                team_lineups: vec![lineup],
                ahl_affiliates: vec![test_nyr_affiliate()],
                organization_lineups: Vec::new(),
                prospect_program: None,
                prospect_conversion: None,
                training_camp: Some(camp),
                schedule_rest: Vec::new(),
                profile_history: None,
                fingerprint: String::new(),
            })
            .unwrap();
        let replayed: OrganizationWindowSourcePackageView =
            serde_json::from_str(&serde_json::to_string_pretty(&package).unwrap()).unwrap();
        assert_eq!(
            seal_organization_window_source_package(replayed).unwrap(),
            package
        );
    }

    #[test]
    fn real_forecast_history_becomes_three_comparable_narrow_window_checkpoints() {
        let history: TeamSeasonForecastHistoryView = serde_json::from_str(include_str!(
            "../../../examples/icecast-history-2025-01-31-to-2025-03-31.json"
        ))
        .unwrap();
        let boards =
            build_forecast_history_organization_window_boards(&history, "2026-07-28T12:00:00Z")
                .unwrap();

        assert_eq!(boards.len(), 3);
        assert_eq!(boards[0].as_of.to_string(), "2025-01-31");
        assert_eq!(boards[2].as_of.to_string(), "2025-03-31");
        assert!(boards
            .windows(2)
            .all(|pair| pair[0].manifest.fingerprint == pair[1].manifest.fingerprint));
        assert!(boards.iter().all(|board| board
            .source_fingerprints
            .iter()
            .any(|fingerprint| fingerprint.starts_with("registry-lifecycle:")
                && fingerprint.len() == "registry-lifecycle:".len() + 64)));
        assert!(boards.iter().all(|board| {
            board.organizations.iter().all(|team| {
                team.dimensions.len() == 1
                    && team.dimensions[0].profiles.len() == 1
                    && team.overall.rank.is_some()
            })
        }));

        let movement = build_organization_window_history(&boards).unwrap();
        assert_eq!(movement.checkpoint_fingerprints.len(), 3);
        assert_eq!(movement.movements.len(), 2);
        let stl = movement.movements[1]
            .organizations
            .iter()
            .find(|team| team.organization == "STL")
            .unwrap();
        assert!(stl.score_delta.is_some_and(|delta| delta > 0.0));
        assert_eq!(stl.personnel_delta, None);
    }

    #[test]
    fn line_combination_adapter_preserves_evaluation_boundary() {
        let forecast: LineCombinationForecastView = serde_json::from_str(include_str!(
            "../../../examples/icecast-alp-line-combinations.json"
        ))
        .unwrap();
        let context = OrganizationWindowAdapterContext {
            season: forecast.roster_season,
            season_type: "regular".to_owned(),
            as_of: NaiveDate::from_ymd_opt(2026, 7, 27).unwrap(),
            horizon: WindowHorizon::Current,
            organization_identity_version: TEAM_CATALOG_VERSION.to_owned(),
        };
        let input = adapt_line_combination_window_profile(&context, &forecast).unwrap();
        assert_eq!(input.profile_key, "deployment.lineup_optionality");
        assert_eq!(input.method_version, "line_combination_optionality.v1");
        assert_eq!(input.raw_unit, "competitive_combinations");
        assert!(input.raw_value.is_some());
        assert!(input
            .limitations
            .iter()
            .any(|limitation| limitation.contains("not full shift-derived chemistry")));
        assert!(input
            .source_fingerprints
            .iter()
            .all(|fingerprint| fingerprint.starts_with("sha256:")));
    }
}
