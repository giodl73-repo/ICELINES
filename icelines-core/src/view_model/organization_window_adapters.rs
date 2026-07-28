//! Typed adapters from sealed IceLines authorities into The Window profile contract.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::line_combination::{LineCombinationForecastView, LINE_COMBINATION_FORECAST_SCHEMA};
use super::management_behavior::{ScheduleRestProfileView, SCHEDULE_REST_PROFILE_SCHEMA};
use super::organization_lineup::{
    OrganizationLineupForecastView, ORGANIZATION_LINEUP_FORECAST_SCHEMA,
};
use super::organization_window::{
    build_organization_window_board, load_organization_window_profile_inventory,
    OrganizationProfileInput, OrganizationWindowBoardInput, OrganizationWindowBoardView,
    OrganizationWindowError, OrganizationWindowManifestView, WindowCohortKind,
    WindowCohortManifest, WindowDimensionManifest, WindowEvidenceView, WindowFreshness,
    WindowHorizon, WindowMissingPolicy, WindowNormalizationMethod, WindowProfileStatus,
    WindowProfileWeight, WindowSignalFamilyCap, ORGANIZATION_WINDOW_CLASSIFICATION_METHOD,
    ORGANIZATION_WINDOW_MANIFEST_SCHEMA,
};
use super::prospect_conversion::{ProspectConversionBoardView, PROSPECT_CONVERSION_BOARD_SCHEMA};
use super::prospect_study::{ProspectProgramBoardView, PROSPECT_PROGRAM_BOARD_SCHEMA};
use super::team_lineup::{
    TeamLineupPlayerView, TeamLineupProjectionView, TeamLineupSpecialTeamsUnitView,
    TEAM_LINEUP_PROJECTION_SCHEMA,
};
use super::team_season_forecast::{TeamSeasonForecastView, TEAM_SEASON_FORECAST_SCHEMA};
use super::training_camp::{TrainingCampLeagueForecastView, TRAINING_CAMP_LEAGUE_FORECAST_SCHEMA};
use crate::teams::CANONICAL_TEAMS;

pub const ORGANIZATION_WINDOW_BALANCED_MANIFEST_ID: &str = "balanced.v1";
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
    pub team_lineups: &'a [TeamLineupProjectionView],
    pub organization_lineups: &'a [OrganizationLineupForecastView],
    pub prospect_program: Option<&'a ProspectProgramBoardView>,
    pub prospect_conversion: Option<&'a ProspectConversionBoardView>,
    pub training_camp: Option<&'a TrainingCampLeagueForecastView>,
    pub schedule_rest: &'a [ScheduleRestProfileView],
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
        sources.organization_lineups,
        &mut output,
    )?;
    adapt_prospect_program(context, &expected, sources.prospect_program, &mut output)?;
    adapt_prospect_conversion(context, &expected, sources.prospect_conversion, &mut output)?;
    adapt_training_camp(context, &expected, sources.training_camp, &mut output)?;
    adapt_schedule(context, &expected, sources.schedule_rest, &mut output)?;
    Ok(output)
}

pub fn build_balanced_organization_window_board(
    context: OrganizationWindowAdapterContext,
    generated_at: impl Into<String>,
    sources: OrganizationWindowSourceSet<'_>,
) -> Result<OrganizationWindowBoardView, OrganizationWindowError> {
    let generated_at = generated_at.into();
    let inventory = load_organization_window_profile_inventory()?;
    let profile_inputs = adapt_balanced_organization_window_sources(&context, sources)?;
    let source_fingerprints = profile_inputs
        .iter()
        .flat_map(|row| row.source_fingerprints.iter().cloned())
        .collect();
    build_organization_window_board(
        OrganizationWindowBoardInput {
            season: context.season,
            season_type: context.season_type,
            as_of: context.as_of,
            generated_at: generated_at.clone(),
            manifest: balanced_organization_window_manifest(BALANCED_MANIFEST_CREATED_AT),
            profile_inputs,
            source_fingerprints,
        },
        &inventory,
    )
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

fn adapt_organization_lineups(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    forecasts: &[OrganizationLineupForecastView],
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let mut seen = BTreeSet::new();
    for forecast in forecasts {
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
            .map(|row| row.recall_readiness.unwrap_or(row.projected_score))
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

fn adapt_schedule(
    context: &OrganizationWindowAdapterContext,
    expected: &BTreeSet<String>,
    profiles: &[ScheduleRestProfileView],
    output: &mut Vec<OrganizationProfileInput>,
) -> Result<(), OrganizationWindowError> {
    let mut seen = BTreeSet::new();
    for row in profiles {
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
    use crate::view_model::organization_window::{
        seal_organization_window_manifest, WindowProfileReadiness,
    };

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
    }

    #[test]
    fn line_combination_adapter_preserves_evaluation_boundary() {
        let forecast: LineCombinationForecastView = serde_json::from_str(include_str!(
            "../../../examples/icecast-nyr-line-combinations.json"
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
