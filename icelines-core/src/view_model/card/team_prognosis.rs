//! Core-owned two-page team prognosis cards and comparison sets.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    team_lineup_card_assets, team_lineup_card_section, IsolatedEventImpactRow, IsolatedImpactView,
    MetricUnit, MetricValue, RecoveryAction, SemanticToken, StatKey, TeamLineupPlayerView,
    TeamLineupProjectionView, TeamSeasonForecastRow, TeamSeasonForecastView, ValuePrecision,
    ViewWarning, WarningKind,
};

pub const TEAM_PROGNOSIS_BUILDER_VERSION: &str = "team_prognosis_card.v1";
pub const CARD_COMPARISON_SET_SCHEMA: &str = "card_comparison_set.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamPrognosisEventProjection {
    pub event_id: String,
    pub hit_score: Option<f64>,
    pub current_role: Option<String>,
    pub hit_role: Option<String>,
    pub evidence_label: EvidenceLabel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamPrognosisCardInput {
    pub team_name: String,
    pub team_abbreviation: String,
    pub lineup: TeamLineupProjectionView,
    pub forecast: TeamSeasonForecastView,
    pub isolated_impact: IsolatedImpactView,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
    pub roster_snapshot_id: Option<String>,
    pub calendar_fingerprint: Option<String>,
    pub scenario_id: Option<String>,
    pub scenario_comparison_key: Option<String>,
    pub event_projections: Vec<TeamPrognosisEventProjection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TeamPrognosisCardError {
    #[error("team prognosis requires a team name")]
    MissingTeamName,
    #[error("team prognosis team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("lineup team {actual} does not match requested team {expected}")]
    LineupTeamMismatch { expected: String, actual: String },
    #[error("forecast season {forecast} does not match view season {view}")]
    SeasonMismatch { forecast: u32, view: u32 },
    #[error("isolated-impact season/trials/seed do not match the forecast")]
    IsolatedSimulationMismatch,
    #[error("forecast has no row for team {0}")]
    MissingForecastTeam(String),
    #[error("isolated impact has no baseline row for team {0}")]
    MissingIsolatedBaseline(String),
    #[error("duplicate event projection: {0}")]
    DuplicateEventProjection(String),
    #[error("event projection does not match an isolated event: {0}")]
    UnknownEventProjection(String),
    #[error("card document validation failed: {0}")]
    Document(String),
    #[error("serialize prognosis provenance: {0}")]
    Serialize(String),
}

pub fn build_team_prognosis_card(
    input: TeamPrognosisCardInput,
) -> Result<CardDocumentView, TeamPrognosisCardError> {
    let team = input.team_abbreviation.trim().to_ascii_uppercase();
    if input.team_name.trim().is_empty() {
        return Err(TeamPrognosisCardError::MissingTeamName);
    }
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(TeamPrognosisCardError::InvalidTeam(team));
    }
    if input.lineup.team != team {
        return Err(TeamPrognosisCardError::LineupTeamMismatch {
            expected: team,
            actual: input.lineup.team,
        });
    }
    if input.forecast.season != input.view.window.season.0 {
        return Err(TeamPrognosisCardError::SeasonMismatch {
            forecast: input.forecast.season,
            view: input.view.window.season.0,
        });
    }
    if input.isolated_impact.season != input.forecast.season
        || input.isolated_impact.trials != input.forecast.trials
        || input.isolated_impact.seed != input.forecast.seed
    {
        return Err(TeamPrognosisCardError::IsolatedSimulationMismatch);
    }
    let baseline = input
        .forecast
        .teams
        .iter()
        .find(|row| row.team == team)
        .ok_or_else(|| TeamPrognosisCardError::MissingForecastTeam(team.clone()))?;
    if !input
        .isolated_impact
        .baseline
        .iter()
        .any(|row| row.team == team)
    {
        return Err(TeamPrognosisCardError::MissingIsolatedBaseline(team));
    }

    let event_ids = input
        .isolated_impact
        .isolated_events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect::<BTreeSet<_>>();
    let mut projections = BTreeMap::new();
    for projection in &input.event_projections {
        if !event_ids.contains(projection.event_id.as_str()) {
            return Err(TeamPrognosisCardError::UnknownEventProjection(
                projection.event_id.clone(),
            ));
        }
        if projections
            .insert(projection.event_id.as_str(), projection)
            .is_some()
        {
            return Err(TeamPrognosisCardError::DuplicateEventProjection(
                projection.event_id.clone(),
            ));
        }
    }

    let forced_impact = input
        .isolated_impact
        .forced_ceiling_impacts
        .iter()
        .find(|row| row.team == team);
    let forced_path = input
        .isolated_impact
        .forced_ceiling_paths
        .iter()
        .find(|row| row.team == team);
    let team_events = input
        .isolated_impact
        .isolated_events
        .iter()
        .filter(|event| event.team == team)
        .collect::<Vec<_>>();
    let players = lineup_players(&input.lineup);
    let (breakouts, downturns): (Vec<_>, Vec<_>) = team_events
        .into_iter()
        .partition(|event| event.raw_team_strength_delta > 0.0);

    let lineup_fingerprint = json_fingerprint(&input.lineup)?;
    let forecast_fingerprint = json_fingerprint(&input.forecast)?;
    let theme = team_theme(&team);
    let warnings = prognosis_warnings(&input.lineup);
    let scenario_name = input
        .forecast
        .scenario
        .as_ref()
        .map(|scenario| scenario.name.as_str())
        .unwrap_or("Baseline");
    let path_label = forced_path
        .map(|path| path.display_label.as_str())
        .unwrap_or("No positive-event path");

    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert(
        "lineup_score".to_string(),
        input.lineup.score_method.clone(),
    );
    methodology_versions.insert(
        "isolated_impact".to_string(),
        input.isolated_impact.method.clone(),
    );

    let document = CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::TeamPrognosis,
        document_id: format!("team-prognosis:{}:{}", team, input.forecast.season),
        fingerprint: String::new(),
        title: format!(
            "{} {} prognosis",
            input.team_name.trim(),
            format_season(input.forecast.season)
        ),
        subtitle: Some(format!("Baseline and {path_label}")),
        context: CardContextView {
            view: input.view.clone(),
            evidence_at: input.evidence_at,
            evidence_label: EvidenceLabel::Simulated,
            builder_version: TEAM_PROGNOSIS_BUILDER_VERSION.to_string(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                roster_snapshot_id: input.roster_snapshot_id,
                calendar_fingerprint: input.calendar_fingerprint,
                scenario_id: input.scenario_id,
                scenario_comparison_key: input.scenario_comparison_key,
                team_ids: vec![team.clone()],
                player_ids: players
                    .iter()
                    .map(|player| player.player_id.to_string())
                    .collect(),
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("icecast".to_string()),
                model_version: Some("team_season_forecast.v1".to_string()),
                parameter_fingerprint: Some(input.isolated_impact.input_fingerprint.clone()),
                seed: Some(input.forecast.seed),
                trials: Some(u64::from(input.forecast.trials)),
            },
        },
        theme,
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "depth-chart".to_string(),
                literal_label: "Projected lineup and player scores".to_string(),
                display_label: Some("The Depth Chart".to_string()),
                order: 1,
                accessible_summary: format!(
                    "{} projected lineup with names and IceLines player scores.",
                    input.team_name.trim()
                ),
                sections: vec![
                    identity_section(&team, input.team_name.trim(), input.forecast.season),
                    CardSectionView::Lineup(team_lineup_card_section(&input.lineup)),
                    CardSectionView::MetricStrip(MetricStripSectionView {
                        id: "baseline-headlines".to_string(),
                        title: Some("Baseline prognosis".to_string()),
                        metrics: baseline_metrics(baseline),
                    }),
                ],
            },
            CardPageView {
                id: "insider".to_string(),
                literal_label: "Scenario prognosis and evidence".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary: format!(
                    "{} baseline, upside, downside, and isolated player impacts.",
                    input.team_name.trim()
                ),
                sections: insider_sections(
                    baseline,
                    forced_impact,
                    forced_path,
                    &breakouts,
                    &downturns,
                    &players,
                    &projections,
                    &input.forecast,
                    scenario_name,
                    &warnings,
                ),
            },
        ],
        assets: team_lineup_card_assets(&input.lineup),
        provenance: vec![
            CardProvenanceView {
                id: "lineup-projection".to_string(),
                source: SourceKind::Roster,
                label: "IceLines projected lineup".to_string(),
                state: input.view.completeness,
                observed_at: input.evidence_at,
                fingerprint: Some(lineup_fingerprint),
                note: Some(
                    "Assignments retain actual, reported, estimated, or scenario evidence labels."
                        .to_string(),
                ),
            },
            CardProvenanceView {
                id: "season-forecast".to_string(),
                source: SourceKind::Schedule,
                label: "IceCast season simulation".to_string(),
                state: Completeness::Complete,
                observed_at: input.evidence_at,
                fingerprint: Some(forecast_fingerprint),
                note: Some(format!("Scenario: {scenario_name}")),
            },
            CardProvenanceView {
                id: "isolated-impact".to_string(),
                source: SourceKind::Snapshot,
                label: "Paired same-seed isolated impact".to_string(),
                state: Completeness::Complete,
                observed_at: input.evidence_at,
                fingerprint: Some(input.isolated_impact.input_fingerprint),
                note: Some(
                    "Conditional one-event effects and forced positive-event ceiling.".to_string(),
                ),
            },
        ],
        warnings,
        empty_state: None,
    };
    document
        .seal()
        .map_err(|error| TeamPrognosisCardError::Document(error.to_string()))
}

#[allow(clippy::too_many_arguments)]
fn insider_sections(
    baseline: &TeamSeasonForecastRow,
    forced_impact: Option<&crate::view_model::TeamSeasonScenarioImpactRow>,
    forced_path: Option<&crate::view_model::ForcedCeilingPathRow>,
    breakouts: &[&IsolatedEventImpactRow],
    downturns: &[&IsolatedEventImpactRow],
    players: &[&TeamLineupPlayerView],
    projections: &BTreeMap<&str, &TeamPrognosisEventProjection>,
    forecast: &TeamSeasonForecastView,
    scenario_name: &str,
    warnings: &[ViewWarning],
) -> Vec<CardSectionView> {
    let mut sections = vec![
        CardSectionView::ProbabilityRange(ProbabilityRangeSectionView {
            id: "points-range".to_string(),
            title: "Baseline points distribution".to_string(),
            ranges: vec![CardProbabilityRangeView {
                key: "points_p10_p50_p90".to_string(),
                label: "Projected points".to_string(),
                low: cell(
                    "points_p10",
                    "P10",
                    f64::from(baseline.points_p10),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                median: cell(
                    "points_p50",
                    "P50",
                    f64::from(baseline.points_p50),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                high: cell(
                    "points_p90",
                    "P90",
                    f64::from(baseline.points_p90),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                display_text: format!(
                    "{} / {} / {}",
                    baseline.points_p10, baseline.points_p50, baseline.points_p90
                ),
                accessible_text: format!(
                    "Projected points: 10th percentile {}, median {}, 90th percentile {}",
                    baseline.points_p10, baseline.points_p50, baseline.points_p90
                ),
                evidence_label: EvidenceLabel::Simulated,
            }],
        }),
        CardSectionView::ScenarioBridge(ScenarioBridgeSectionView {
            id: "internal-ceiling".to_string(),
            title: forced_path.map_or_else(
                || "Internal ceiling".to_string(),
                |path| format!("Internal ceiling — {}", path.display_label),
            ),
            from_label: "Baseline".to_string(),
            to_label: "Forced positive-event ceiling".to_string(),
            metrics: ceiling_metrics(baseline, forced_impact, forced_path),
            evidence_label: EvidenceLabel::Simulated,
        }),
    ];
    if !breakouts.is_empty() {
        sections.push(CardSectionView::PlayerList(PlayerListSectionView {
            id: "breakout-upside".to_string(),
            title: "Best breakout upside".to_string(),
            rows: event_rows(breakouts, players, projections, true),
        }));
    }
    if !downturns.is_empty() {
        sections.push(CardSectionView::PlayerList(PlayerListSectionView {
            id: "downside-risks".to_string(),
            title: "Primary downside risks".to_string(),
            rows: event_rows(downturns, players, projections, false),
        }));
    }
    if !forecast.scenario_outcomes.is_empty() {
        let team = &baseline.team;
        let rows = forecast
            .scenario_outcomes
            .iter()
            .filter(|row| &row.team == team);
        let mut any_up = 0.0;
        let mut any_down = 0.0;
        for row in rows {
            if row.positive_events > 0 {
                any_up += row.probability;
            }
            if row.negative_events > 0 {
                any_down += row.probability;
            }
        }
        sections.push(CardSectionView::MetricStrip(MetricStripSectionView {
            id: "natural-realization".to_string(),
            title: Some("Naturally sampled realization".to_string()),
            metrics: vec![
                percentage_metric(
                    "any_positive_event_probability",
                    "At least one upside event",
                    any_up,
                    None,
                ),
                percentage_metric(
                    "any_negative_event_probability",
                    "At least one downside event",
                    any_down,
                    None,
                ),
            ],
        }));
    }
    if !warnings.is_empty() {
        sections.push(CardSectionView::StateNotice(StateNoticeSectionView {
            id: "source-warnings".to_string(),
            title: "Evidence warnings".to_string(),
            detail: Some(
                "Lineup and forecast limitations remain part of the document.".to_string(),
            ),
            empty_state: None,
            warnings: warnings.to_vec(),
            token: SemanticToken::Warning,
        }));
    }
    sections.extend([
        CardSectionView::Methodology(MethodologySectionView {
            id: "methodology".to_string(),
            title: "Methodology".to_string(),
            methods: vec![
                CardMethodologyItemView { key: "lineup".to_string(), label: "The Depth Chart".to_string(), version: "team_lineup_projection.v1".to_string(), summary: "Position-aware roster assignment and multi-lens player scores.".to_string() },
                CardMethodologyItemView { key: "forecast".to_string(), label: "IceCast".to_string(), version: "team_season_forecast.v1".to_string(), summary: "Seeded chronological full-league regular season and playoff simulation.".to_string() },
                CardMethodologyItemView { key: "isolation".to_string(), label: "The Insider".to_string(), version: "paired_same_seed_one_event.v1".to_string(), summary: "One authored event forced at a time against an identical baseline.".to_string() },
            ],
            limitations: vec![
                format!("Scenario events are modeled hypotheses, not reports that they will occur: {scenario_name}."),
                "Raw team strength is a model input and is not standings points.".to_string(),
                "The forced ceiling is conditional; natural realization probabilities remain separate.".to_string(),
            ],
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "sources".to_string(),
            title: "Source authority".to_string(),
            provenance_ids: vec!["lineup-projection".to_string(), "season-forecast".to_string(), "isolated-impact".to_string()],
        }),
    ]);
    sections
}

fn identity_section(team: &str, team_name: &str, season: u32) -> CardSectionView {
    CardSectionView::IdentityHeader(IdentityHeaderSectionView {
        id: "team-identity".to_string(),
        eyebrow: Some(format_season(season)),
        title: team_name.to_string(),
        subtitle: Some("Projected roster prognosis".to_string()),
        identities: vec![CardIdentityView {
            kind: CardIdentityKind::Team,
            subject_id: format!("team:{team}"),
            label: team_name.to_string(),
            asset_id: None,
        }],
    })
}

fn baseline_metrics(row: &TeamSeasonForecastRow) -> Vec<CardMetricView> {
    vec![
        number_metric(
            "baseline_points",
            "Projected points",
            row.average_points,
            MetricUnit::Points,
            ValuePrecision::OneDecimal,
            None,
        ),
        percentage_metric(
            "baseline_playoff_probability",
            "Playoffs",
            row.playoff_probability,
            None,
        ),
        percentage_metric(
            "baseline_cup_probability",
            "Stanley Cup",
            row.stanley_cup_probability,
            None,
        ),
    ]
}

fn ceiling_metrics(
    baseline: &TeamSeasonForecastRow,
    impact: Option<&crate::view_model::TeamSeasonScenarioImpactRow>,
    path: Option<&crate::view_model::ForcedCeilingPathRow>,
) -> Vec<CardMetricView> {
    let points_delta = impact.map_or(0.0, |row| row.average_points_delta);
    let playoff_delta = impact.map_or(0.0, |row| row.playoff_probability_delta);
    let cup_delta = impact.map_or(0.0, |row| row.stanley_cup_probability_delta);
    vec![
        number_metric(
            "ceiling_points",
            "Ceiling points",
            baseline.average_points + points_delta,
            MetricUnit::Points,
            ValuePrecision::OneDecimal,
            Some(("Baseline", baseline.average_points, points_delta)),
        ),
        percentage_metric(
            "ceiling_playoff_probability",
            "Ceiling playoffs",
            baseline.playoff_probability + playoff_delta,
            Some(("Baseline", baseline.playoff_probability, playoff_delta)),
        ),
        percentage_metric(
            "ceiling_cup_probability",
            "Ceiling Stanley Cup",
            baseline.stanley_cup_probability + cup_delta,
            Some(("Baseline", baseline.stanley_cup_probability, cup_delta)),
        ),
        number_metric(
            "ceiling_team_strength",
            "Path team strength",
            path.map_or(0.0, |row| row.raw_team_strength_delta_sum),
            MetricUnit::Score,
            ValuePrecision::TwoDecimals,
            Some((
                "Baseline",
                0.0,
                path.map_or(0.0, |row| row.raw_team_strength_delta_sum),
            )),
        ),
    ]
}

fn event_rows(
    events: &[&IsolatedEventImpactRow],
    players: &[&TeamLineupPlayerView],
    projections: &BTreeMap<&str, &TeamPrognosisEventProjection>,
    positive: bool,
) -> Vec<CardPlayerRowView> {
    events
        .iter()
        .map(|event| {
            let player = event.player.as_deref().and_then(|name| {
                players
                    .iter()
                    .find(|player| player.display_name == name)
                    .copied()
            });
            let projection = projections.get(event.event_id.as_str()).copied();
            let evidence = projection.map_or(EvidenceLabel::Simulated, |projection| {
                projection.evidence_label
            });
            let mut metrics = vec![
                percentage_metric(
                    "occurrence_probability",
                    "Modeled likelihood",
                    event.occurrence_probability,
                    None,
                ),
                number_metric(
                    "team_strength_delta",
                    "Team strength delta",
                    event.raw_team_strength_delta,
                    MetricUnit::Score,
                    ValuePrecision::TwoDecimals,
                    None,
                ),
                number_metric(
                    "standings_points_delta",
                    "Conditional points delta",
                    event.conditional_impact.average_points_delta,
                    MetricUnit::Points,
                    ValuePrecision::OneDecimal,
                    None,
                ),
                percentage_delta_metric(
                    "playoff_probability_delta",
                    "Conditional playoff delta",
                    event.conditional_impact.playoff_probability_delta,
                ),
                percentage_delta_metric(
                    "cup_probability_delta",
                    "Conditional Cup delta",
                    event.conditional_impact.stanley_cup_probability_delta,
                ),
            ];
            if let Some(player) = player {
                metrics.insert(
                    0,
                    optional_score_metric(
                        "current_player_score",
                        "Current IceLines score",
                        player.score.value,
                        &player.score.display,
                        player.score.evidence_label,
                    ),
                );
            }
            if let Some(projection) = projection {
                if projection.hit_score.is_some() {
                    metrics.insert(
                        1.min(metrics.len()),
                        optional_score_metric(
                            "scenario_player_score",
                            "Scenario IceLines score",
                            projection.hit_score,
                            "NR",
                            projection.evidence_label,
                        ),
                    );
                }
            }
            CardPlayerRowView {
                player_id: player.map_or_else(
                    || format!("event:{}", event.event_id),
                    |player| format!("player:{}", player.player_id),
                ),
                name: event.player.clone().unwrap_or_else(|| event.label.clone()),
                role: projection.and_then(|projection| {
                    match (&projection.current_role, &projection.hit_role) {
                        (Some(current), Some(hit)) => Some(format!("{current} → {hit}")),
                        (Some(current), None) => Some(current.clone()),
                        (None, Some(hit)) => Some(hit.clone()),
                        (None, None) => None,
                    }
                }),
                asset_id: player.map(|player| player.portrait.asset_id.clone()),
                metrics,
                tokens: vec![if positive {
                    SemanticToken::Rising
                } else {
                    SemanticToken::Risk
                }],
                evidence_label: evidence,
            }
        })
        .collect()
}

fn optional_score_metric(
    key: &str,
    label: &str,
    value: Option<f64>,
    missing_display: &str,
    evidence: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: value
                .map(MetricValue::Decimal)
                .unwrap_or(MetricValue::Missing),
            unit: MetricUnit::Score,
            precision: ValuePrecision::Integer,
            token: None,
        },
        display_text: value.map_or_else(
            || missing_display.to_string(),
            |value| format!("{value:.0}"),
        ),
        accessible_text: value.map_or_else(
            || format!("{label} not rated"),
            |value| format!("{label} {value:.0} out of 100"),
        ),
        comparison: None,
        evidence_label: evidence,
    }
}

fn number_metric(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    precision: ValuePrecision,
    comparison: Option<(&str, f64, f64)>,
) -> CardMetricView {
    let decimals = match precision {
        ValuePrecision::Integer => 0,
        ValuePrecision::OneDecimal => 1,
        _ => 2,
    };
    CardMetricView {
        metric: cell(key, label, value, unit, precision),
        display_text: format!("{value:.decimals$}"),
        accessible_text: format!("{label} {value:.decimals$}"),
        comparison: comparison.map(|(label, baseline, delta)| CardMetricComparisonView {
            label: label.to_string(),
            baseline: MetricValue::Decimal(baseline),
            delta: MetricValue::Decimal(delta),
        }),
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn percentage_metric(
    key: &str,
    label: &str,
    probability: f64,
    comparison: Option<(&str, f64, f64)>,
) -> CardMetricView {
    let value = probability * 100.0;
    CardMetricView {
        metric: cell(
            key,
            label,
            value,
            MetricUnit::Percentage,
            ValuePrecision::PercentOneDecimal,
        ),
        display_text: format!("{value:.1}%"),
        accessible_text: format!("{label} {value:.1} percent"),
        comparison: comparison.map(|(label, baseline, delta)| CardMetricComparisonView {
            label: label.to_string(),
            baseline: MetricValue::Decimal(baseline * 100.0),
            delta: MetricValue::Decimal(delta * 100.0),
        }),
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn percentage_delta_metric(key: &str, label: &str, probability_delta: f64) -> CardMetricView {
    let value = probability_delta * 100.0;
    CardMetricView {
        metric: cell(
            key,
            label,
            value,
            MetricUnit::Percentage,
            ValuePrecision::PercentOneDecimal,
        ),
        display_text: format!("{value:+.1} pp"),
        accessible_text: format!("{label} {value:+.1} percentage points"),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn cell(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    precision: ValuePrecision,
) -> MetricCell {
    MetricCell {
        key: StatKey(key.to_string()),
        label: label.to_string(),
        value: MetricValue::Decimal(value),
        unit,
        precision,
        token: None,
    }
}

fn lineup_players(lineup: &TeamLineupProjectionView) -> Vec<&TeamLineupPlayerView> {
    lineup
        .forward_lines
        .iter()
        .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
        .chain(
            lineup
                .defense_pairs
                .iter()
                .flat_map(|pair| [&pair.left, &pair.right]),
        )
        .chain([&lineup.goalies.starter, &lineup.goalies.backup])
        .filter_map(Option::as_ref)
        .chain(lineup.extras.iter())
        .collect()
}

fn prognosis_warnings(lineup: &TeamLineupProjectionView) -> Vec<ViewWarning> {
    lineup
        .warnings
        .iter()
        .map(|warning| ViewWarning {
            kind: WarningKind::EstimatedDeployment,
            source: Some(SourceKind::Roster),
            message: format!("{}: {}", warning.code, warning.message),
            recovery: Vec::<RecoveryAction>::new(),
        })
        .collect()
}

fn team_theme(team: &str) -> CardThemeView {
    let (primary, secondary, accent) = match team {
        "NYR" => ("#0038A8", "#CE1126", "#FFFFFF"),
        "SEA" => ("#001628", "#99D9D9", "#E9072B"),
        _ => ("#14213D", "#E5E5E5", "#FCA311"),
    };
    CardThemeView {
        theme_key: format!("team_{}", team.to_ascii_lowercase()),
        primary: Some(primary.to_string()),
        secondary: Some(secondary.to_string()),
        accent: Some(accent.to_string()),
        surface: Some("#FFFFFF".to_string()),
        text: Some("#111111".to_string()),
        team_abbreviation: Some(team.to_string()),
        ascii_identity: team.to_string(),
        minimum_text_contrast_x100: 450,
    }
}

fn format_season(season: u32) -> String {
    format!("{}-{:02}", season / 10_000, season % 100)
}

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, TeamPrognosisCardError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| TeamPrognosisCardError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardComparisonWarningKind {
    Season,
    Model,
    EvidenceCutoff,
    Scenario,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardComparisonWarning {
    pub kind: CardComparisonWarningKind,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardAlignedMetricRow {
    pub metric_key: String,
    pub label: String,
    pub unit: MetricUnit,
    pub values: BTreeMap<String, MetricValue>,
    pub deltas_from_first: BTreeMap<String, MetricValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardComparisonSetView {
    pub schema: String,
    pub documents: Vec<CardDocumentView>,
    pub aligned_metrics: Vec<CardAlignedMetricRow>,
    pub warnings: Vec<CardComparisonWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CardComparisonError {
    #[error("comparison requires at least two card documents")]
    TooFewDocuments,
    #[error("comparison contains duplicate document id: {0}")]
    DuplicateDocumentId(String),
    #[error("invalid card document: {0}")]
    InvalidDocument(String),
}

pub fn build_card_comparison_set(
    documents: Vec<CardDocumentView>,
) -> Result<CardComparisonSetView, CardComparisonError> {
    if documents.len() < 2 {
        return Err(CardComparisonError::TooFewDocuments);
    }
    let mut document_ids = BTreeSet::new();
    for document in &documents {
        if !document_ids.insert(document.document_id.as_str()) {
            return Err(CardComparisonError::DuplicateDocumentId(
                document.document_id.clone(),
            ));
        }
        document
            .validate()
            .map_err(|error| CardComparisonError::InvalidDocument(error.to_string()))?;
    }
    let first = &documents[0];
    let mut warnings = Vec::new();
    for document in documents.iter().skip(1) {
        if document.context.view.window != first.context.view.window {
            warnings.push(CardComparisonWarning {
                kind: CardComparisonWarningKind::Season,
                message: format!("{} uses a different season window", document.document_id),
            });
        }
        if document.context.simulation.model_id != first.context.simulation.model_id
            || document.context.simulation.model_version != first.context.simulation.model_version
        {
            warnings.push(CardComparisonWarning {
                kind: CardComparisonWarningKind::Model,
                message: format!("{} uses a different model", document.document_id),
            });
        }
        if document.context.evidence_at != first.context.evidence_at {
            warnings.push(CardComparisonWarning {
                kind: CardComparisonWarningKind::EvidenceCutoff,
                message: format!("{} uses a different evidence cutoff", document.document_id),
            });
        }
        let first_scenario_dimension = first
            .context
            .joins
            .scenario_comparison_key
            .as_ref()
            .or(first.context.joins.scenario_id.as_ref());
        let document_scenario_dimension = document
            .context
            .joins
            .scenario_comparison_key
            .as_ref()
            .or(document.context.joins.scenario_id.as_ref());
        if document_scenario_dimension != first_scenario_dimension {
            warnings.push(CardComparisonWarning {
                kind: CardComparisonWarningKind::Scenario,
                message: format!("{} uses a different scenario", document.document_id),
            });
        }
    }
    let aligned_metrics = if warnings.is_empty() {
        align_metrics(&documents)
    } else {
        Vec::new()
    };
    Ok(CardComparisonSetView {
        schema: CARD_COMPARISON_SET_SCHEMA.to_string(),
        documents,
        aligned_metrics,
        warnings,
    })
}

fn align_metrics(documents: &[CardDocumentView]) -> Vec<CardAlignedMetricRow> {
    let maps = documents.iter().map(document_metrics).collect::<Vec<_>>();
    let first = &maps[0];
    first
        .iter()
        .filter_map(|(key, metric)| {
            let peers = maps
                .iter()
                .map(|map| map.get(key))
                .collect::<Option<Vec<_>>>()?;
            if peers
                .iter()
                .any(|peer| peer.metric.unit != metric.metric.unit)
            {
                return None;
            }
            let mut values = BTreeMap::new();
            let mut deltas = BTreeMap::new();
            for (document, peer) in documents.iter().zip(peers) {
                values.insert(document.document_id.clone(), peer.metric.value.clone());
                if let (Some(base), Some(value)) = (
                    numeric_value(&metric.metric.value),
                    numeric_value(&peer.metric.value),
                ) {
                    deltas.insert(
                        document.document_id.clone(),
                        MetricValue::Decimal(value - base),
                    );
                }
            }
            Some(CardAlignedMetricRow {
                metric_key: key.clone(),
                label: metric.metric.label.clone(),
                unit: metric.metric.unit,
                values,
                deltas_from_first: deltas,
            })
        })
        .collect()
}

fn document_metrics(document: &CardDocumentView) -> BTreeMap<String, &CardMetricView> {
    let mut metrics = BTreeMap::new();
    for page in &document.pages {
        for section in &page.sections {
            let rows: Vec<&CardMetricView> = match section {
                CardSectionView::MetricStrip(section) => section.metrics.iter().collect(),
                CardSectionView::ScenarioBridge(section) => section.metrics.iter().collect(),
                _ => Vec::new(),
            };
            for metric in rows {
                metrics.entry(metric.metric.key.0.clone()).or_insert(metric);
            }
        }
    }
    metrics
}

fn numeric_value(value: &MetricValue) -> Option<f64> {
    match value {
        MetricValue::Integer(value) => Some(*value as f64),
        MetricValue::Decimal(value) => Some(*value),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;
    use crate::{
        model::Season,
        season_stats::SeasonType,
        view_model::{
            ForcedCeilingPathRow, IsolatedImpactBaselineRow, TeamSeasonScenarioImpactRow,
            ViewWindow,
        },
    };

    fn forecast() -> TeamSeasonForecastView {
        serde_json::from_value(json!({
            "schema": "team_season_forecast.v1",
            "season": 20262027,
            "trials": 10000,
            "seed": 73,
            "schedule_games": 1344,
            "scenario": { "name": "NYR development variance", "trade_deadline": "2027-03-05", "events": [] },
            "games": [],
            "accuracy": null,
            "personnel_evidence": [],
            "membership_intervals": [],
            "membership_anomalies": [],
            "teams": [{
                "team": "NYR", "conference": "Eastern", "division": "Metropolitan",
                "average_wins": 45.0, "average_losses": 29.0, "average_overtime_losses": 10.0,
                "average_points": 100.0, "points_p10": 90, "points_p50": 100, "points_p90": 110,
                "average_league_rank": 10.0, "playoff_probability": 0.72,
                "second_round_probability": 0.40, "conference_final_probability": 0.22,
                "stanley_cup_final_probability": 0.12, "stanley_cup_probability": 0.06,
                "presidents_trophy_probability": 0.03, "average_longest_win_streak": 5.0,
                "longest_win_streak_p90": 8, "longest_win_streak_leader_probability": 0.04
            }],
            "scenario_outcomes": [{
                "team": "NYR", "positive_events": 1, "negative_events": 0, "trials": 3000,
                "probability": 0.3, "average_sampled_strength_delta": 3.0187,
                "average_points": 101.0, "playoff_probability": 0.74, "stanley_cup_probability": 0.065
            }, {
                "team": "NYR", "positive_events": 0, "negative_events": 1, "trials": 4000,
                "probability": 0.4, "average_sampled_strength_delta": -4.0,
                "average_points": 98.0, "playoff_probability": 0.68, "stanley_cup_probability": 0.05
            }],
            "pivotal_games": [],
            "league_leaders": { "presidents_trophy": [], "stanley_cup": [], "longest_win_streak": [] },
            "schedule_stretches": [], "warnings": [], "disclosures": []
        })).unwrap()
    }

    fn impact(team: &str, points: f64, playoffs: f64, cup: f64) -> TeamSeasonScenarioImpactRow {
        TeamSeasonScenarioImpactRow {
            team: team.to_string(),
            average_points_delta: points,
            playoff_probability_delta: playoffs,
            second_round_probability_delta: 0.01,
            conference_final_probability_delta: 0.01,
            stanley_cup_final_probability_delta: 0.005,
            stanley_cup_probability_delta: cup,
            presidents_trophy_probability_delta: 0.001,
            average_longest_win_streak_delta: 0.1,
        }
    }

    fn isolated() -> IsolatedImpactView {
        let baseline = IsolatedImpactBaselineRow {
            team: "NYR".to_string(),
            average_points: 100.0,
            playoff_probability: 0.72,
            second_round_probability: 0.40,
            conference_final_probability: 0.22,
            stanley_cup_final_probability: 0.12,
            stanley_cup_probability: 0.06,
        };
        IsolatedImpactView {
            schema: "isolated_scenario_impact.v1".to_string(),
            method: "paired_same_seed_one_event.v1".to_string(),
            season: 20262027,
            as_of_date: None,
            trials: 10000,
            seed: 73,
            input_fingerprint: "a".repeat(64),
            scenario_fingerprint: "b".repeat(64),
            baseline: vec![baseline.clone()],
            isolated_events: vec![
                IsolatedEventImpactRow {
                    event_id: "nyr-kartye-breakout-range".to_string(),
                    team: "NYR".to_string(),
                    player: Some("Tye Kartye".to_string()),
                    label: "Kartye reaches middle-six value".to_string(),
                    occurrence_probability: 0.2389,
                    correlation_key: None,
                    raw_team_strength_delta: 3.0187,
                    conditional_impact: impact("NYR", 1.2, 0.03, 0.01),
                    conditional_outcome: IsolatedImpactBaselineRow {
                        average_points: 101.2,
                        playoff_probability: 0.75,
                        stanley_cup_probability: 0.07,
                        ..baseline.clone()
                    },
                    isolated_scenario_fingerprint: "c".repeat(64),
                },
                IsolatedEventImpactRow {
                    event_id: "nyr-goalie-downturn".to_string(),
                    team: "NYR".to_string(),
                    player: Some("Igor Shesterkin".to_string()),
                    label: "Goalie downturn".to_string(),
                    occurrence_probability: 0.40,
                    correlation_key: None,
                    raw_team_strength_delta: -4.0,
                    conditional_impact: impact("NYR", -2.0, -0.05, -0.015),
                    conditional_outcome: IsolatedImpactBaselineRow {
                        average_points: 98.0,
                        playoff_probability: 0.67,
                        stanley_cup_probability: 0.045,
                        ..baseline.clone()
                    },
                    isolated_scenario_fingerprint: "d".repeat(64),
                },
            ],
            naturally_sampled_impacts: vec![impact("NYR", -0.4, -0.01, -0.003)],
            forced_ceiling_paths: vec![ForcedCeilingPathRow {
                team: "NYR".to_string(),
                raw_team_strength_delta_sum: 15.4855,
                display_label: "+15 Path".to_string(),
                event_ids: vec!["nyr-kartye-breakout-range".to_string()],
            }],
            forced_ceiling_impacts: vec![impact("NYR", 5.5, 0.12, 0.04)],
            disclosures: vec![],
        }
    }

    fn input() -> TeamPrognosisCardInput {
        let lineup: TeamLineupProjectionView = serde_json::from_str(include_str!(
            "../../../../examples/team-lineup-nyr-2026-27.json"
        ))
        .unwrap();
        let timestamp = Utc.with_ymd_and_hms(2026, 7, 21, 17, 0, 0).unwrap();
        let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
        view.generated_at = Some(timestamp);
        TeamPrognosisCardInput {
            team_name: "New York Rangers".to_string(),
            team_abbreviation: "NYR".to_string(),
            lineup,
            forecast: forecast(),
            isolated_impact: isolated(),
            view,
            evidence_at: Some(timestamp),
            roster_snapshot_id: Some("roster-2026-07-21".to_string()),
            calendar_fingerprint: Some("calendar-2026-27".to_string()),
            scenario_id: Some("nyr-development-variance".to_string()),
            scenario_comparison_key: Some("development-variance".to_string()),
            event_projections: vec![TeamPrognosisEventProjection {
                event_id: "nyr-kartye-breakout-range".to_string(),
                hit_score: Some(78.0),
                current_role: Some("Bottom six".to_string()),
                hit_role: Some("Middle six".to_string()),
                evidence_label: EvidenceLabel::Estimated,
            }],
        }
    }

    #[test]
    fn builds_two_page_nyr_card_with_kartye_and_reconciled_ceiling() {
        let card = build_team_prognosis_card(input()).unwrap();
        assert_eq!(card.pages.len(), 2);
        assert_eq!(card.pages[0].id, "depth-chart");
        assert_eq!(card.pages[1].id, "insider");
        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("Tye Kartye"));
        assert!(json.contains("+15 Path"));
        assert!(json.contains("15.4855"));
        assert!(json.contains("current_player_score"));
        assert!(json.contains("scenario_player_score"));
        assert!(json.contains("Conditional points delta"));
        card.validate().unwrap();
    }

    #[test]
    fn comparison_aligns_compatible_cards_and_blocks_mismatched_scenarios() {
        let first = build_team_prognosis_card(input()).unwrap();
        let mut second_input = input();
        second_input.team_name = "New York Rangers B".to_string();
        let mut second = build_team_prognosis_card(second_input).unwrap();
        second.document_id = "team-prognosis:NYR-B:20262027".to_string();
        second.refresh_fingerprint().unwrap();
        let compatible = build_card_comparison_set(vec![first.clone(), second]).unwrap();
        assert!(compatible.warnings.is_empty());
        assert!(compatible
            .aligned_metrics
            .iter()
            .any(|row| row.metric_key == "baseline_points"));

        let mut mismatched = first.clone();
        mismatched.document_id = "team-prognosis:NYR-mismatched:20262027".to_string();
        mismatched.context.joins.scenario_id = Some("different-scenario".to_string());
        mismatched.context.joins.scenario_comparison_key = Some("different-family".to_string());
        mismatched.refresh_fingerprint().unwrap();
        mismatched.validate().unwrap();
        let blocked = build_card_comparison_set(vec![first, mismatched]).unwrap();
        assert_eq!(
            blocked.warnings[0].kind,
            CardComparisonWarningKind::Scenario
        );
        assert!(blocked.aligned_metrics.is_empty());
    }

    #[test]
    fn rejects_projection_for_unknown_event() {
        let mut invalid = input();
        invalid.event_projections[0].event_id = "not-authored".to_string();
        assert_eq!(
            build_team_prognosis_card(invalid),
            Err(TeamPrognosisCardError::UnknownEventProjection(
                "not-authored".to_string()
            ))
        );
    }
}
