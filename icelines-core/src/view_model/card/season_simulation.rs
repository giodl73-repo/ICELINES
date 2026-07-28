//! Focused, UI-neutral projections of a sealed league-wide IceCast run.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    Completeness, EvidenceLabel, MetricCell, MetricUnit, MetricValue, SemanticToken, SourceKind,
    StatKey, TeamSeasonForecastRow, TeamSeasonForecastView, TeamSeasonReplayCheckpointTeamRow,
    TeamSeasonStretchKind, ValuePrecision, ViewContext, ViewWarning, WarningKind,
};

pub const SEASON_SIMULATION_CARD_VERSION: &str = "season_simulation_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SeasonSimulationCardInput {
    /// The complete league run. It is fingerprinted before any team filtering.
    pub forecast: TeamSeasonForecastView,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
    pub calendar_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SeasonSimulationCardError {
    #[error("season simulation team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("season simulation requires a team name")]
    MissingTeamName,
    #[error("forecast season {forecast} does not match view season {view}")]
    SeasonMismatch { forecast: u32, view: u32 },
    #[error("forecast has no row for team {0}")]
    MissingForecastTeam(String),
    #[error("serialize league forecast: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_season_simulation_card(
    input: SeasonSimulationCardInput,
) -> Result<CardDocumentView, SeasonSimulationCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(SeasonSimulationCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(SeasonSimulationCardError::MissingTeamName);
    }
    if input.forecast.season != input.view.window.season.0 {
        return Err(SeasonSimulationCardError::SeasonMismatch {
            forecast: input.forecast.season,
            view: input.view.window.season.0,
        });
    }
    let row = input
        .forecast
        .teams
        .iter()
        .find(|row| row.team == team)
        .ok_or_else(|| SeasonSimulationCardError::MissingForecastTeam(team.clone()))?;
    let league_run_fingerprint = fingerprint(&input.forecast)?;
    let scenario_name = input
        .forecast
        .scenario
        .as_ref()
        .map(|s| s.name.as_str())
        .unwrap_or("Baseline");
    let impact = input
        .forecast
        .scenario_impacts
        .iter()
        .find(|row| row.team == team);

    let warnings = input
        .forecast
        .warnings
        .iter()
        .map(|message| ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::Schedule),
            message: message.clone(),
            recovery: vec![],
        })
        .collect::<Vec<_>>();
    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert("season_forecast".to_string(), input.forecast.schema.clone());
    methodology_versions.insert(
        "card_projection".to_string(),
        SEASON_SIMULATION_CARD_VERSION.to_string(),
    );

    let provenance = vec![CardProvenanceView {
        id: "league-run".to_string(),
        source: SourceKind::Schedule,
        label: "Sealed 32-team IceCast run".to_string(),
        state: Completeness::Complete,
        observed_at: input.evidence_at,
        fingerprint: Some(league_run_fingerprint.clone()),
        note: Some(format!(
            "{} scheduled games; focused only after the complete run was sealed",
            input.forecast.schedule_games
        )),
    }];
    let scenario_id = input
        .forecast
        .scenario_reference
        .as_ref()
        .map(|r| r.id.clone());
    let document_id = match input.forecast.as_of_date {
        Some(date) => format!(
            "season-simulation:{}:{}:through:{}",
            team, input.forecast.season, date
        ),
        None => format!("season-simulation:{}:{}", team, input.forecast.season),
    };
    let subtitle = match input.forecast.as_of_date {
        Some(date) => format!(
            "{scenario_name} · through {date} · {} trials · seed {}",
            input.forecast.trials, input.forecast.seed
        ),
        None => format!(
            "{scenario_name} · {} trials · seed {}",
            input.forecast.trials, input.forecast.seed
        ),
    };
    let document = CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::SeasonSimulation,
        document_id,
        fingerprint: String::new(),
        title: format!("{} {} IceCast", input.team_name.trim(), season_label(input.forecast.season)),
        subtitle: Some(subtitle),
        context: CardContextView {
            view: input.view,
            evidence_at: input.evidence_at,
            evidence_label: EvidenceLabel::Simulated,
            builder_version: SEASON_SIMULATION_CARD_VERSION.to_string(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                calendar_fingerprint: input.calendar_fingerprint,
                scenario_id,
                team_ids: vec![team.clone()],
                game_ids: input.forecast.games.iter().map(|g| g.game_id.to_string()).collect(),
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("icecast".to_string()),
                model_version: Some(input.forecast.schema.clone()),
                parameter_fingerprint: Some(league_run_fingerprint.clone()),
                seed: Some(input.forecast.seed),
                trials: Some(u64::from(input.forecast.trials)),
            },
        },
        theme: team_theme(&team),
        required_capabilities: vec![CardRendererCapability::Timelines],
        pages: vec![
            CardPageView {
                id: "scoreboard".to_string(),
                literal_label: "Season distribution and championship odds".to_string(),
                display_label: Some("The Scoreboard".to_string()),
                order: 1,
                accessible_summary: format!("{} projected record, points range, playoff odds, Cup odds, and streak outlook.", input.team_name.trim()),
                sections: scoreboard_sections(
                    &team,
                    input.team_name.trim(),
                    row,
                    impact,
                    input.forecast.as_of_date,
                    input
                        .forecast
                        .replay_checkpoint
                        .as_ref()
                        .and_then(|checkpoint| checkpoint.teams.iter().find(|row| row.team == team)),
                ),
            },
            CardPageView {
                id: "insider".to_string(),
                literal_label: "Schedule pressure, pivotal games, scenario events, and methodology".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary: format!("Why the {} projection moves: schedule stretches, pivotal games, injuries, trades, and model limits.", team),
                sections: insider_sections(&input.forecast, &team, scenario_name, &league_run_fingerprint),
            },
        ],
        assets: vec![],
        provenance,
        warnings,
        empty_state: None,
    };
    document
        .seal()
        .map_err(|error| SeasonSimulationCardError::Document(error.to_string()))
}

fn scoreboard_sections(
    team: &str,
    name: &str,
    row: &TeamSeasonForecastRow,
    impact: Option<&crate::view_model::TeamSeasonScenarioImpactRow>,
    as_of_date: Option<chrono::NaiveDate>,
    checkpoint: Option<&TeamSeasonReplayCheckpointTeamRow>,
) -> Vec<CardSectionView> {
    let mut sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "team".to_string(),
            eyebrow: Some("IceCast season simulation".to_string()),
            title: name.to_string(),
            subtitle: Some(format!("{} · {} · {}", team, row.division, row.conference)),
            identities: vec![CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: team.to_string(),
                label: name.to_string(),
                asset_id: None,
            }],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "season-headline".to_string(),
            title: Some("Median season".to_string()),
            metrics: vec![
                metric(
                    "wins",
                    "Wins",
                    row.average_wins,
                    MetricUnit::Games,
                    ValuePrecision::OneDecimal,
                ),
                metric(
                    "points",
                    "Points",
                    row.average_points,
                    MetricUnit::Points,
                    ValuePrecision::OneDecimal,
                ),
                metric(
                    "league_rank",
                    "League rank",
                    row.average_league_rank,
                    MetricUnit::Ranking,
                    ValuePrecision::OneDecimal,
                ),
                probability("playoff", "Playoffs", row.playoff_probability),
                probability("cup", "Stanley Cup", row.stanley_cup_probability),
            ],
        }),
        CardSectionView::ProbabilityRange(ProbabilityRangeSectionView {
            id: "points-range".to_string(),
            title: "Points distribution".to_string(),
            ranges: vec![CardProbabilityRangeView {
                key: "points_p10_p50_p90".to_string(),
                label: "Projected points".to_string(),
                low: cell(
                    "points_p10",
                    "P10",
                    f64::from(row.points_p10),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                median: cell(
                    "points_p50",
                    "P50",
                    f64::from(row.points_p50),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                high: cell(
                    "points_p90",
                    "P90",
                    f64::from(row.points_p90),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                display_text: format!(
                    "{} / {} / {}",
                    row.points_p10, row.points_p50, row.points_p90
                ),
                accessible_text: format!(
                    "10th percentile {}, median {}, 90th percentile {} points",
                    row.points_p10, row.points_p50, row.points_p90
                ),
                evidence_label: EvidenceLabel::Simulated,
            }],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "playoff-path".to_string(),
            title: Some("Playoff path".to_string()),
            metrics: vec![
                probability("round_2", "Second round", row.second_round_probability),
                probability(
                    "conference_final",
                    "Conference final",
                    row.conference_final_probability,
                ),
                probability("cup_final", "Cup final", row.stanley_cup_final_probability),
                metric(
                    "win_streak",
                    "Longest win streak",
                    row.average_longest_win_streak,
                    MetricUnit::Games,
                    ValuePrecision::OneDecimal,
                ),
            ],
        }),
    ];
    if let (Some(date), Some(checkpoint)) = (as_of_date, checkpoint) {
        sections.insert(
            1,
            CardSectionView::MetricStrip(MetricStripSectionView {
                id: "as-of-checkpoint".to_string(),
                title: Some(format!("Actual checkpoint through {date}")),
                metrics: vec![
                    observed_metric(
                        "checkpoint_games",
                        "Games played",
                        checkpoint.completed_games as f64,
                        MetricUnit::Games,
                        ValuePrecision::Integer,
                    ),
                    observed_metric(
                        "checkpoint_wins",
                        "Wins",
                        f64::from(checkpoint.wins),
                        MetricUnit::Games,
                        ValuePrecision::Integer,
                    ),
                    observed_metric(
                        "checkpoint_losses",
                        "Losses",
                        f64::from(checkpoint.losses),
                        MetricUnit::Games,
                        ValuePrecision::Integer,
                    ),
                    observed_metric(
                        "checkpoint_ot_losses",
                        "OT losses",
                        f64::from(checkpoint.overtime_losses),
                        MetricUnit::Games,
                        ValuePrecision::Integer,
                    ),
                    observed_metric(
                        "checkpoint_points",
                        "Points",
                        f64::from(checkpoint.standings_points),
                        MetricUnit::Points,
                        ValuePrecision::Integer,
                    ),
                    observed_metric(
                        "checkpoint_remaining",
                        "Games remaining",
                        checkpoint.remaining_games as f64,
                        MetricUnit::Games,
                        ValuePrecision::Integer,
                    ),
                ],
            }),
        );
        sections.insert(
            2,
            CardSectionView::MetricStrip(MetricStripSectionView {
                id: "projected-remainder".to_string(),
                title: Some(format!("Expected rest of season after {date}")),
                metrics: vec![
                    metric(
                        "remaining_wins",
                        "Wins",
                        checkpoint.expected_remaining_wins,
                        MetricUnit::Games,
                        ValuePrecision::OneDecimal,
                    ),
                    metric(
                        "remaining_losses",
                        "Losses",
                        checkpoint.expected_remaining_losses,
                        MetricUnit::Games,
                        ValuePrecision::OneDecimal,
                    ),
                    metric(
                        "remaining_ot_losses",
                        "OT losses",
                        checkpoint.expected_remaining_overtime_losses,
                        MetricUnit::Games,
                        ValuePrecision::OneDecimal,
                    ),
                    metric(
                        "remaining_points",
                        "Points added",
                        checkpoint.expected_remaining_points,
                        MetricUnit::Points,
                        ValuePrecision::OneDecimal,
                    ),
                ],
            }),
        );
    }
    if let Some(impact) = impact {
        sections.push(CardSectionView::ScenarioBridge(ScenarioBridgeSectionView {
            id: "scenario-delta".to_string(),
            title: "Scenario delta from baseline".to_string(),
            from_label: "Baseline".to_string(),
            to_label: "Scenario".to_string(),
            metrics: vec![
                signed(
                    "points_delta",
                    "Points",
                    impact.average_points_delta,
                    MetricUnit::Points,
                ),
                signed(
                    "playoff_delta",
                    "Playoffs",
                    impact.playoff_probability_delta * 100.0,
                    MetricUnit::Percentage,
                ),
                signed(
                    "cup_delta",
                    "Stanley Cup",
                    impact.stanley_cup_probability_delta * 100.0,
                    MetricUnit::Percentage,
                ),
            ],
            evidence_label: EvidenceLabel::Simulated,
        }));
    }
    sections
}

fn insider_sections(
    forecast: &TeamSeasonForecastView,
    team: &str,
    scenario_name: &str,
    fingerprint: &str,
) -> Vec<CardSectionView> {
    let mut sections = Vec::new();
    let stretches = forecast
        .schedule_stretches
        .iter()
        .filter(|s| s.team == team)
        .map(|s| {
            let kind = match s.kind {
                TeamSeasonStretchKind::Hardest => "Hardest",
                TeamSeasonStretchKind::Easiest => "Easiest",
            };
            CardTimelineItemView {
                id: format!("stretch:{}:{}", kind.to_lowercase(), s.start_date),
                effective_at: at_utc(s.start_date),
                observed_at: None,
                label: format!("{kind} stretch: {} to {}", s.start_date, s.end_date),
                detail: Some(format!(
                    "{} games, {:.1} expected wins, {} away, {} back-to-backs, {:.0} km",
                    s.opponents.len(),
                    s.expected_wins,
                    s.away_games,
                    s.back_to_backs,
                    s.travel_km
                )),
                evidence_label: EvidenceLabel::Simulated,
                token: if matches!(s.kind, TeamSeasonStretchKind::Hardest) {
                    SemanticToken::Risk
                } else {
                    SemanticToken::ScheduleEdge
                },
            }
        })
        .collect::<Vec<_>>();
    if !stretches.is_empty() {
        sections.push(CardSectionView::Timeline(TimelineSectionView {
            id: "schedule-stretches".to_string(),
            title: "Schedule pressure".to_string(),
            items: stretches,
        }));
    }
    let pivotal = forecast
        .pivotal_games
        .iter()
        .filter(|g| g.away_team == team || g.home_team == team)
        .take(8)
        .map(|g| CardTimelineItemView {
            id: format!("game:{}", g.game_id),
            effective_at: at_utc(g.date),
            observed_at: None,
            label: format!("{} at {}", g.away_team, g.home_team),
            detail: Some(format!(
                "Hunt {:.1}% · spoiler {:.1}%",
                g.hunt_probability * 100.0,
                g.spoiler_probability * 100.0
            )),
            evidence_label: EvidenceLabel::Simulated,
            token: SemanticToken::DecisionHighlight,
        })
        .collect::<Vec<_>>();
    if !pivotal.is_empty() {
        sections.push(CardSectionView::Timeline(TimelineSectionView {
            id: "pivotal-games".to_string(),
            title: "Pivotal games".to_string(),
            items: pivotal,
        }));
    }
    let events = forecast
        .scenario
        .as_ref()
        .into_iter()
        .flat_map(|s| &s.events)
        .filter(|e| e.team == team)
        .map(|e| CardTimelineItemView {
            id: format!("event:{}", e.id),
            effective_at: at_utc(e.effective_date),
            observed_at: None,
            label: e.label.clone(),
            detail: Some(format!(
                "{:?} · strength {:+.1} · occurrence {:.0}%",
                e.kind,
                e.strength_delta,
                e.occurrence_probability * 100.0
            )),
            evidence_label: EvidenceLabel::Simulated,
            token: if e.strength_delta < 0.0 {
                SemanticToken::Risk
            } else {
                SemanticToken::Rising
            },
        })
        .collect::<Vec<_>>();
    if !events.is_empty() {
        sections.push(CardSectionView::Timeline(TimelineSectionView {
            id: "scenario-events".to_string(),
            title: format!("{scenario_name} events"),
            items: events,
        }));
    }
    sections.extend(event_path_sections(forecast, team));
    if let Some(accuracy) = &forecast.accuracy {
        sections.extend(replay_sections(forecast, team, accuracy));
    }
    sections.push(CardSectionView::Methodology(MethodologySectionView {
        id: "methodology".to_string(), title: "How to read this forecast".to_string(),
        methods: vec![CardMethodologyItemView { key: "icecast".to_string(), label: "IceCast Monte Carlo".to_string(), version: forecast.schema.clone(), summary: format!("One sealed {}-game league schedule, {} same-seed trials, seed {}. Full-run fingerprint {}.", forecast.schedule_games, forecast.trials, forecast.seed, &fingerprint[..12]) }],
        limitations: forecast.disclosures.clone(),
    }));
    sections.push(CardSectionView::Provenance(ProvenanceSectionView {
        id: "provenance".to_string(),
        title: "Auditable source run".to_string(),
        provenance_ids: vec!["league-run".to_string()],
    }));
    sections
}

fn event_path_sections(forecast: &TeamSeasonForecastView, team: &str) -> Vec<CardSectionView> {
    let mut outcomes = forecast
        .scenario_outcomes
        .iter()
        .filter(|outcome| outcome.team == team)
        .collect::<Vec<_>>();
    if outcomes.is_empty() {
        return Vec::new();
    }
    outcomes.sort_by(|left, right| left.average_points.total_cmp(&right.average_points));
    let middle = outcomes.len() / 2;
    let mut selected = vec![("downside", "Downside sampled path", outcomes[0])];
    if middle > 0 && middle + 1 < outcomes.len() {
        selected.push(("typical", "Middle sampled path", outcomes[middle]));
    }
    if outcomes.len() > 1 {
        selected.push((
            "upside",
            "Upside sampled path",
            outcomes[outcomes.len() - 1],
        ));
    }
    selected
        .into_iter()
        .map(|(id, title, outcome)| {
            CardSectionView::MetricStrip(MetricStripSectionView {
                id: format!("event-path-{id}"),
                title: Some(title.to_string()),
                metrics: vec![
                    metric(
                        "positive_events",
                        "Positive events",
                        f64::from(outcome.positive_events),
                        MetricUnit::Count,
                        ValuePrecision::Integer,
                    ),
                    metric(
                        "negative_events",
                        "Negative events",
                        f64::from(outcome.negative_events),
                        MetricUnit::Count,
                        ValuePrecision::Integer,
                    ),
                    probability("path_probability", "Path frequency", outcome.probability),
                    metric(
                        "path_points",
                        "Average points",
                        outcome.average_points,
                        MetricUnit::Points,
                        ValuePrecision::OneDecimal,
                    ),
                    probability("path_playoffs", "Playoffs", outcome.playoff_probability),
                    probability("path_cup", "Stanley Cup", outcome.stanley_cup_probability),
                ],
            })
        })
        .collect()
}

fn replay_sections(
    forecast: &TeamSeasonForecastView,
    team: &str,
    accuracy: &crate::view_model::TeamGameForecastAccuracySummary,
) -> Vec<CardSectionView> {
    let team_games = forecast
        .games
        .iter()
        .filter(|game| game.away_team == team || game.home_team == team)
        .filter(|game| game.actual_winner.is_some())
        .collect::<Vec<_>>();
    let correct = team_games
        .iter()
        .filter(|game| game.pick_correct == Some(true))
        .count();
    let mut wins = 0_u16;
    let mut losses = 0_u16;
    let mut overtime_losses = 0_u16;
    for game in &team_games {
        if game.actual_winner.as_deref() == Some(team) {
            wins += 1;
        } else if matches!(game.actual_ending.as_deref(), Some("OT" | "SO")) {
            overtime_losses += 1;
        } else {
            losses += 1;
        }
    }
    let actual_points = wins * 2 + overtime_losses;
    let team_pick_accuracy = if team_games.is_empty() {
        0.0
    } else {
        correct as f64 / team_games.len() as f64
    };
    let mut calibration_metrics = vec![
        observed_metric(
            "evaluated_games",
            "Evaluated games",
            accuracy.final_games as f64,
            MetricUnit::Games,
            ValuePrecision::Integer,
        ),
        observed_probability(
            "pick_accuracy",
            "League picks correct",
            accuracy.pick_accuracy,
        ),
        observed_metric(
            "brier_score",
            "Brier score",
            accuracy.brier_score,
            MetricUnit::Score,
            ValuePrecision::ThreeDecimals,
        ),
        observed_probability(
            "calibration_error",
            "Expected calibration error",
            accuracy.expected_calibration_error,
        ),
        observed_probability(
            "brier_skill",
            "Skill vs coin flip",
            accuracy.brier_skill_vs_coinflip,
        ),
    ];
    if let (Some(intercept), Some(slope)) =
        (accuracy.calibration_intercept, accuracy.calibration_slope)
    {
        calibration_metrics.extend([
            observed_metric(
                "calibration_intercept",
                "Calibration intercept · ideal 0",
                intercept,
                MetricUnit::Score,
                ValuePrecision::ThreeDecimals,
            ),
            observed_metric(
                "calibration_slope",
                "Calibration slope · ideal 1",
                slope,
                MetricUnit::Score,
                ValuePrecision::ThreeDecimals,
            ),
        ]);
        if let (
            Some(intercept_lower),
            Some(intercept_upper),
            Some(slope_lower),
            Some(slope_upper),
        ) = (
            accuracy.calibration_intercept_ci95_lower,
            accuracy.calibration_intercept_ci95_upper,
            accuracy.calibration_slope_ci95_lower,
            accuracy.calibration_slope_ci95_upper,
        ) {
            calibration_metrics.extend([
                observed_metric(
                    "calibration_intercept_ci95_lower",
                    "Calibration intercept 95% lower",
                    intercept_lower,
                    MetricUnit::Score,
                    ValuePrecision::ThreeDecimals,
                ),
                observed_metric(
                    "calibration_intercept_ci95_upper",
                    "Calibration intercept 95% upper",
                    intercept_upper,
                    MetricUnit::Score,
                    ValuePrecision::ThreeDecimals,
                ),
                observed_metric(
                    "calibration_slope_ci95_lower",
                    "Calibration slope 95% lower",
                    slope_lower,
                    MetricUnit::Score,
                    ValuePrecision::ThreeDecimals,
                ),
                observed_metric(
                    "calibration_slope_ci95_upper",
                    "Calibration slope 95% upper",
                    slope_upper,
                    MetricUnit::Score,
                    ValuePrecision::ThreeDecimals,
                ),
            ]);
        }
    }
    let mut sections = vec![
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "actual-team-result".to_string(),
            title: Some(match forecast.as_of_date {
                Some(date) => format!("Actual team result through {date}"),
                None => "Actual team result".to_string(),
            }),
            metrics: vec![
                observed_metric(
                    "actual_wins",
                    "Wins",
                    f64::from(wins),
                    MetricUnit::Games,
                    ValuePrecision::Integer,
                ),
                observed_metric(
                    "actual_losses",
                    "Losses",
                    f64::from(losses),
                    MetricUnit::Games,
                    ValuePrecision::Integer,
                ),
                observed_metric(
                    "actual_ot_losses",
                    "OT losses",
                    f64::from(overtime_losses),
                    MetricUnit::Games,
                    ValuePrecision::Integer,
                ),
                observed_metric(
                    "actual_points",
                    "Points",
                    f64::from(actual_points),
                    MetricUnit::Points,
                    ValuePrecision::Integer,
                ),
                observed_probability(
                    "team_pick_accuracy",
                    "Team picks correct",
                    team_pick_accuracy,
                ),
            ],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "replay-calibration".to_string(),
            title: Some(match forecast.as_of_date {
                Some(date) => format!("Calibration through {date}"),
                None => "Completed-season calibration".to_string(),
            }),
            metrics: calibration_metrics,
        }),
    ];
    if let Some(blend) = &accuracy.best_elo_blend_by_brier {
        sections.push(CardSectionView::MetricStrip(MetricStripSectionView {
            id: "replay-best-blend".to_string(),
            title: Some("Best chronological Elo blend".to_string()),
            metrics: vec![
                observed_probability("elo_weight", "Elo weight", blend.elo_weight),
                observed_metric(
                    "blend_brier",
                    "Brier score",
                    blend.brier_score,
                    MetricUnit::Score,
                    ValuePrecision::ThreeDecimals,
                ),
                observed_metric(
                    "blend_brier_gain",
                    "Brier improvement",
                    blend.brier_improvement_vs_model,
                    MetricUnit::Score,
                    ValuePrecision::ThreeDecimals,
                ),
            ],
        }));
    }
    sections
}

fn metric(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    precision: ValuePrecision,
) -> CardMetricView {
    CardMetricView {
        metric: cell(key, label, value, unit, precision),
        display_text: match precision {
            ValuePrecision::OneDecimal => format!("{value:.1}"),
            _ => format!("{value:.0}"),
        },
        accessible_text: format!("{label} {value:.1}"),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}
fn probability(key: &str, label: &str, value: f64) -> CardMetricView {
    let pct = value * 100.0;
    CardMetricView {
        metric: cell(
            key,
            label,
            pct,
            MetricUnit::Percentage,
            ValuePrecision::PercentOneDecimal,
        ),
        display_text: format!("{pct:.1}%"),
        accessible_text: format!("{label} {pct:.1} percent"),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}
fn observed_metric(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    precision: ValuePrecision,
) -> CardMetricView {
    let display_text = match precision {
        ValuePrecision::Integer => format!("{value:.0}"),
        ValuePrecision::ThreeDecimals => format!("{value:.3}"),
        _ => format!("{value:.1}"),
    };
    CardMetricView {
        metric: cell(key, label, value, unit, precision),
        display_text,
        accessible_text: format!("{label} {value}"),
        comparison: None,
        evidence_label: EvidenceLabel::Confirmed,
    }
}
fn observed_probability(key: &str, label: &str, value: f64) -> CardMetricView {
    let pct = value * 100.0;
    CardMetricView {
        metric: cell(
            key,
            label,
            pct,
            MetricUnit::Percentage,
            ValuePrecision::PercentOneDecimal,
        ),
        display_text: format!("{pct:.1}%"),
        accessible_text: format!("{label} {pct:.1} percent"),
        comparison: None,
        evidence_label: EvidenceLabel::Confirmed,
    }
}
fn signed(key: &str, label: &str, value: f64, unit: MetricUnit) -> CardMetricView {
    CardMetricView {
        metric: cell(key, label, value, unit, ValuePrecision::OneDecimal),
        display_text: format!("{value:+.1}"),
        accessible_text: format!("{label} delta {value:+.1}"),
        comparison: Some(CardMetricComparisonView {
            label: "scenario minus same-seed baseline".to_string(),
            baseline: MetricValue::Decimal(0.0),
            delta: MetricValue::Decimal(value),
        }),
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
fn at_utc(date: chrono::NaiveDate) -> DateTime<Utc> {
    date.and_hms_opt(12, 0, 0).expect("noon is valid").and_utc()
}
fn fingerprint<T: Serialize>(value: &T) -> Result<String, SeasonSimulationCardError> {
    serde_json::to_vec(value)
        .map(|b| format!("{:x}", Sha256::digest(b)))
        .map_err(|e| SeasonSimulationCardError::Serialize(e.to_string()))
}
fn season_label(season: u32) -> String {
    format!("{}-{:02}", season / 10_000, season % 100)
}
fn team_theme(team: &str) -> CardThemeView {
    let (p, s, a) = match team {
        "NYR" => ("#0038A8", "#CE1126", "#FFFFFF"),
        "SEA" => ("#001628", "#99D9D9", "#E9072B"),
        _ => ("#14213D", "#E5E5E5", "#FCA311"),
    };
    CardThemeView {
        theme_key: format!("team_{}", team.to_ascii_lowercase()),
        primary: Some(p.into()),
        secondary: Some(s.into()),
        accent: Some(a.into()),
        surface: Some("#FFFFFF".into()),
        text: Some("#111111".into()),
        team_abbreviation: Some(team.into()),
        ascii_identity: team.into(),
        minimum_text_contrast_x100: 450,
    }
}
