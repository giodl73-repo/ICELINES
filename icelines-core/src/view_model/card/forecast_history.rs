//! UI-neutral card projection of a sealed multi-checkpoint IceCast history.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    Completeness, EvidenceLabel, MetricCell, MetricUnit, MetricValue, SourceKind, StatKey,
    TeamSeasonForecastHistoryTeamRow, TeamSeasonForecastHistoryView, ValuePrecision, ViewContext,
    TEAM_SEASON_FORECAST_HISTORY_SCHEMA,
};

pub const FORECAST_HISTORY_CARD_VERSION: &str = "forecast_history_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastHistoryCardInput {
    pub history: TeamSeasonForecastHistoryView,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ForecastHistoryCardError {
    #[error("forecast history team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("forecast history card requires a team name")]
    MissingTeamName,
    #[error("history season {history} does not match view season {view}")]
    SeasonMismatch { history: u32, view: u32 },
    #[error("unsupported forecast history schema: {0}")]
    UnsupportedSchema(String),
    #[error("forecast history requires at least two checkpoints")]
    InsufficientCheckpoints,
    #[error("history has no row for team {0}")]
    MissingHistoryTeam(String),
    #[error("history checkpoint structure is inconsistent for team {0}")]
    InconsistentCheckpoints(String),
    #[error("history source fingerprint is invalid at checkpoint {0}")]
    InvalidSourceFingerprint(usize),
    #[error("serialize forecast history: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_forecast_history_card(
    input: ForecastHistoryCardInput,
) -> Result<CardDocumentView, ForecastHistoryCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(ForecastHistoryCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(ForecastHistoryCardError::MissingTeamName);
    }
    if input.history.season != input.view.window.season.0 {
        return Err(ForecastHistoryCardError::SeasonMismatch {
            history: input.history.season,
            view: input.view.window.season.0,
        });
    }
    if input.history.schema != TEAM_SEASON_FORECAST_HISTORY_SCHEMA {
        return Err(ForecastHistoryCardError::UnsupportedSchema(
            input.history.schema,
        ));
    }
    if input.history.checkpoints.len() < 2 {
        return Err(ForecastHistoryCardError::InsufficientCheckpoints);
    }
    for (index, checkpoint) in input.history.checkpoints.iter().enumerate() {
        if !valid_fingerprint(&checkpoint.fingerprint) {
            return Err(ForecastHistoryCardError::InvalidSourceFingerprint(
                index + 1,
            ));
        }
    }
    if input
        .history
        .checkpoints
        .windows(2)
        .any(|pair| pair[0].as_of_date >= pair[1].as_of_date)
    {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    let team_history = input
        .history
        .teams
        .iter()
        .find(|row| row.team == team)
        .ok_or_else(|| ForecastHistoryCardError::MissingHistoryTeam(team.clone()))?;
    if team_history.checkpoints.len() != input.history.checkpoints.len()
        || team_history
            .checkpoints
            .iter()
            .zip(&input.history.checkpoints)
            .any(|(team, league)| team.as_of_date != league.as_of_date)
        || team_history
            .checkpoints
            .iter()
            .enumerate()
            .any(|(index, point)| {
                if index == 0 {
                    point.average_points_delta_from_previous.is_some()
                        || point.playoff_probability_delta_from_previous.is_some()
                        || point.stanley_cup_probability_delta_from_previous.is_some()
                        || point.completed_games_delta_from_previous.is_some()
                        || point
                            .prior_expected_points_for_completed_interval_from_previous
                            .is_some()
                        || point
                            .realized_points_vs_prior_remaining_pace_from_previous
                            .is_some()
                        || point.remaining_outlook_revaluation_from_previous.is_some()
                        || point
                            .pace_attribution_reconciliation_error_from_previous
                            .is_some()
                } else {
                    point.average_points_delta_from_previous.is_none()
                        || point.playoff_probability_delta_from_previous.is_none()
                        || point.stanley_cup_probability_delta_from_previous.is_none()
                        || point.completed_games_delta_from_previous.is_none()
                }
            })
        || team_history.checkpoints.windows(2).any(|pair| {
            pair[1].completed_games < pair[0].completed_games
                || pair[1].remaining_games > pair[0].remaining_games
        })
    {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    for pair in team_history.checkpoints.windows(2) {
        let earlier = &pair[0];
        let later = &pair[1];
        let completed_delta = later.completed_games - earlier.completed_games;
        let observed_delta = i64::from(later.observed_standings_points)
            - i64::from(earlier.observed_standings_points);
        let prior_pace = (earlier.remaining_games > 0)
            .then(|| earlier.expected_remaining_points / earlier.remaining_games as f64);
        let expected_interval = prior_pace.map(|pace| pace * completed_delta as f64);
        let realized_vs_pace = expected_interval.map(|expected| observed_delta as f64 - expected);
        let outlook_revaluation = expected_interval.map(|expected| {
            later.expected_remaining_points - (earlier.expected_remaining_points - expected)
        });
        let residual = realized_vs_pace
            .zip(outlook_revaluation)
            .map(|(realized, revaluation)| {
                (later.average_points - earlier.average_points) - (realized + revaluation)
            });
        if !optional_approximately_equal(
            later.average_points_delta_from_previous,
            Some(later.average_points - earlier.average_points),
        ) || !optional_approximately_equal(
            later.playoff_probability_delta_from_previous,
            Some(later.playoff_probability - earlier.playoff_probability),
        ) || !optional_approximately_equal(
            later.stanley_cup_probability_delta_from_previous,
            Some(later.stanley_cup_probability - earlier.stanley_cup_probability),
        ) || later.completed_games_delta_from_previous != Some(completed_delta)
            || !optional_approximately_equal(
                later.prior_expected_points_for_completed_interval_from_previous,
                expected_interval,
            )
            || !optional_approximately_equal(
                later.realized_points_vs_prior_remaining_pace_from_previous,
                realized_vs_pace,
            )
            || !optional_approximately_equal(
                later.remaining_outlook_revaluation_from_previous,
                outlook_revaluation,
            )
            || !optional_approximately_equal(
                later.pace_attribution_reconciliation_error_from_previous,
                residual,
            )
            || later
                .pace_attribution_reconciliation_error_from_previous
                .is_some_and(|error| error.abs() > 1e-6)
        {
            return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
        }
    }
    let first_team_checkpoint = team_history
        .checkpoints
        .first()
        .expect("validated history checkpoints");
    let last_team_checkpoint = team_history
        .checkpoints
        .last()
        .expect("validated history checkpoints");
    if !approximately_equal(
        team_history.average_points_delta_first_to_last,
        last_team_checkpoint.average_points - first_team_checkpoint.average_points,
    ) || !approximately_equal(
        team_history.playoff_probability_delta_first_to_last,
        last_team_checkpoint.playoff_probability - first_team_checkpoint.playoff_probability,
    ) || !approximately_equal(
        team_history.stanley_cup_probability_delta_first_to_last,
        last_team_checkpoint.stanley_cup_probability
            - first_team_checkpoint.stanley_cup_probability,
    ) {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    let expected_observed_delta = i64::from(last_team_checkpoint.observed_standings_points)
        - i64::from(first_team_checkpoint.observed_standings_points);
    let expected_remainder_delta = last_team_checkpoint.expected_remaining_points
        - first_team_checkpoint.expected_remaining_points;
    let expected_reconciliation = team_history.average_points_delta_first_to_last
        - (expected_observed_delta as f64 + expected_remainder_delta);
    if team_history.observed_standings_points_delta_first_to_last != expected_observed_delta
        || !approximately_equal(
            team_history.expected_remaining_points_delta_first_to_last,
            expected_remainder_delta,
        )
        || !approximately_equal(
            team_history.points_movement_reconciliation_error,
            expected_reconciliation,
        )
        || team_history.points_movement_reconciliation_error.abs() > 1e-6
    {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    let expected_completed_delta =
        last_team_checkpoint.completed_games - first_team_checkpoint.completed_games;
    let expected_prior_pace = (first_team_checkpoint.remaining_games > 0).then(|| {
        first_team_checkpoint.expected_remaining_points
            / first_team_checkpoint.remaining_games as f64
    });
    let expected_interval = expected_prior_pace.map(|pace| pace * expected_completed_delta as f64);
    let expected_realized_vs_pace =
        expected_interval.map(|expected| expected_observed_delta as f64 - expected);
    let expected_revaluation = expected_interval.map(|expected| {
        last_team_checkpoint.expected_remaining_points
            - (first_team_checkpoint.expected_remaining_points - expected)
    });
    let expected_pace_residual =
        expected_realized_vs_pace
            .zip(expected_revaluation)
            .map(|(realized, revaluation)| {
                team_history.average_points_delta_first_to_last - (realized + revaluation)
            });
    if team_history.completed_games_delta_first_to_last != expected_completed_delta
        || !optional_approximately_equal(
            team_history.prior_expected_points_per_remaining_game,
            expected_prior_pace,
        )
        || !optional_approximately_equal(
            team_history.prior_expected_points_for_completed_interval,
            expected_interval,
        )
        || !optional_approximately_equal(
            team_history.realized_points_vs_prior_remaining_pace,
            expected_realized_vs_pace,
        )
        || !optional_approximately_equal(
            team_history.remaining_outlook_revaluation,
            expected_revaluation,
        )
        || !optional_approximately_equal(
            team_history.pace_attribution_reconciliation_error,
            expected_pace_residual,
        )
        || team_history
            .pace_attribution_reconciliation_error
            .is_some_and(|value| value.abs() > 1e-6)
    {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    if team_history.league_team_count != input.history.teams.len()
        || team_history.projected_points_movement_rank == 0
        || team_history.projected_points_movement_rank > team_history.league_team_count
    {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    let Some(largest_swing_pair) = team_history.checkpoints.windows(2).find(|pair| {
        pair[0].as_of_date == team_history.largest_swing_from_date
            && pair[1].as_of_date == team_history.largest_swing_to_date
    }) else {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    };
    if !approximately_equal(
        team_history.largest_projected_points_swing,
        largest_swing_pair[1].average_points - largest_swing_pair[0].average_points,
    ) {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }
    let first_width = f64::from(
        first_team_checkpoint
            .points_p90
            .saturating_sub(first_team_checkpoint.points_p10),
    );
    let last_width = f64::from(
        last_team_checkpoint
            .points_p90
            .saturating_sub(last_team_checkpoint.points_p10),
    );
    let expected_width = (first_width + last_width) / 2.0;
    let expected_share = (expected_width > 0.0)
        .then(|| team_history.average_points_delta_first_to_last.abs() / expected_width);
    if !approximately_equal(
        team_history.average_first_last_points_range_width,
        expected_width,
    ) || !optional_approximately_equal(
        team_history.net_points_movement_share_of_range,
        expected_share,
    ) {
        return Err(ForecastHistoryCardError::InconsistentCheckpoints(team));
    }

    let history_fingerprint = json_fingerprint(&input.history)?;
    let first = input
        .history
        .checkpoints
        .first()
        .expect("validated history");
    let last = input.history.checkpoints.last().expect("validated history");
    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert("forecast_history".to_string(), input.history.schema.clone());
    methodology_versions.insert(
        "card_projection".to_string(),
        FORECAST_HISTORY_CARD_VERSION.to_string(),
    );
    let provenance = input
        .history
        .checkpoints
        .iter()
        .enumerate()
        .map(|(index, checkpoint)| CardProvenanceView {
            id: format!("checkpoint-{}", index + 1),
            source: SourceKind::Schedule,
            label: format!(
                "Sealed IceCast checkpoint through {}",
                checkpoint.as_of_date
            ),
            state: Completeness::Complete,
            observed_at: if index + 1 == input.history.checkpoints.len() {
                input.evidence_at
            } else {
                None
            },
            fingerprint: Some(checkpoint.fingerprint.clone()),
            note: Some(format!(
                "{} league games complete; {} remaining",
                checkpoint.league_completed_games, checkpoint.league_remaining_games
            )),
        })
        .collect::<Vec<_>>();

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::ForecastHistory,
        document_id: format!(
            "forecast-history:{}:{}:{}:{}",
            team,
            input.history.season,
            first.as_of_date.format("%Y%m%d"),
            last.as_of_date.format("%Y%m%d")
        ),
        fingerprint: String::new(),
        title: format!("{} forecast history", input.team_name.trim()),
        subtitle: Some(format!(
            "{} → {} · {} checkpoints · {} trials · seed {}",
            first.as_of_date,
            last.as_of_date,
            input.history.checkpoints.len(),
            input.history.trials,
            input.history.seed
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: input.evidence_at,
            evidence_label: EvidenceLabel::Simulated,
            builder_version: FORECAST_HISTORY_CARD_VERSION.to_string(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                scenario_comparison_key: Some(format!(
                    "history:{}:{}",
                    first.as_of_date, last.as_of_date
                )),
                team_ids: vec![team.clone()],
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("icecast-history".to_string()),
                model_version: Some(input.history.schema.clone()),
                parameter_fingerprint: Some(history_fingerprint),
                seed: Some(input.history.seed),
                trials: Some(u64::from(input.history.trials)),
            },
        },
        theme: nhl_team_card_theme(&team),
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "tape".to_string(),
                literal_label: "Chronological forecast levels and movement".to_string(),
                display_label: Some("The Tape".to_string()),
                order: 1,
                accessible_summary: format!(
                    "{} forecast levels and consecutive movement across {} sealed checkpoints.",
                    input.team_name.trim(),
                    input.history.checkpoints.len()
                ),
                sections: tape_sections(
                    &team,
                    input.team_name.trim(),
                    team_history,
                ),
            },
            CardPageView {
                id: "insider".to_string(),
                literal_label: "Forecast history methodology and source authority".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary:
                    "How chronological checkpoints are aligned and which sealed league runs support every level and delta."
                        .to_string(),
                sections: vec![
                    CardSectionView::Methodology(MethodologySectionView {
                        id: "history-methodology".to_string(),
                        title: "How to read The Tape".to_string(),
                        methods: vec![CardMethodologyItemView {
                            key: "chronological-checkpoint-history".to_string(),
                            label: "Chronological checkpoint history".to_string(),
                            version: input.history.schema.clone(),
                            summary: "Absolute values come from each sealed league forecast. Change values are computed in core against the immediately preceding checkpoint.".to_string(),
                        }],
                        limitations: input.history.disclosures.clone(),
                    }),
                    CardSectionView::Provenance(ProvenanceSectionView {
                        id: "history-sources".to_string(),
                        title: "Checkpoint authority".to_string(),
                        provenance_ids: provenance.iter().map(|row| row.id.clone()).collect(),
                    }),
                ],
            },
        ],
        assets: Vec::new(),
        provenance,
        warnings: Vec::new(),
        empty_state: None,
    }
    .seal()
    .map_err(|error| ForecastHistoryCardError::Document(error.to_string()))
}

fn tape_sections(
    team: &str,
    team_name: &str,
    history: &TeamSeasonForecastHistoryTeamRow,
) -> Vec<CardSectionView> {
    let checkpoints = &history.checkpoints;
    let mut sections = vec![CardSectionView::IdentityHeader(IdentityHeaderSectionView {
        id: "history-team".to_string(),
        eyebrow: Some("IceCast forecast history".to_string()),
        title: team_name.to_string(),
        subtitle: Some(format!("{} sealed checkpoints", checkpoints.len())),
        identities: vec![CardIdentityView {
            kind: CardIdentityKind::Team,
            subject_id: team.to_string(),
            label: team_name.to_string(),
            asset_id: None,
        }],
    })];
    sections.extend(checkpoints.iter().enumerate().map(|(index, point)| {
        let mut metrics = vec![
            metric(
                "projected_points",
                "Projected points",
                point.average_points,
                MetricUnit::Points,
                EvidenceLabel::Simulated,
            ),
            text_metric(
                "projected_points_distribution",
                "Projected points P10 / P50 / P90",
                &format!(
                    "{} / {} / {}",
                    point.points_p10, point.points_p50, point.points_p90
                ),
            ),
            probability(
                "playoff_probability",
                "Playoff odds",
                point.playoff_probability,
            ),
            probability(
                "cup_probability",
                "Stanley Cup odds",
                point.stanley_cup_probability,
            ),
            metric(
                "observed_points",
                "Observed standings points",
                f64::from(point.observed_standings_points),
                MetricUnit::Points,
                EvidenceLabel::Confirmed,
            ),
            metric(
                "expected_remaining_points",
                "Expected remaining points",
                point.expected_remaining_points,
                MetricUnit::Points,
                EvidenceLabel::Simulated,
            ),
        ];
        if let (
            Some(completed_games),
            Some(expected_interval),
            Some(realized_vs_pace),
            Some(outlook_revaluation),
        ) = (
            point.completed_games_delta_from_previous,
            point.prior_expected_points_for_completed_interval_from_previous,
            point.realized_points_vs_prior_remaining_pace_from_previous,
            point.remaining_outlook_revaluation_from_previous,
        ) {
            metrics.extend([
                signed(
                    "prior_expected_points_for_completed_interval_from_previous",
                    &format!("Prior expected points for {completed_games} newly completed games"),
                    expected_interval,
                    MetricUnit::Points,
                ),
                signed_with_evidence(
                    "realized_points_vs_prior_remaining_pace_from_previous",
                    "Realized points versus prior checkpoint pace",
                    realized_vs_pace,
                    MetricUnit::Points,
                    EvidenceLabel::Confirmed,
                ),
                signed(
                    "remaining_outlook_revaluation_from_previous",
                    "Still-unplayed outlook revaluation from prior checkpoint",
                    outlook_revaluation,
                    MetricUnit::Points,
                ),
            ]);
        }
        if let Some(value) = point.average_points_delta_from_previous {
            metrics.push(signed(
                "projected_points_delta",
                "Change in projected points",
                value,
                MetricUnit::Points,
            ));
        }
        if let Some(value) = point.playoff_probability_delta_from_previous {
            metrics.push(signed(
                "playoff_probability_delta",
                "Change in playoff odds",
                value * 100.0,
                MetricUnit::Percentage,
            ));
        }
        if let Some(value) = point.stanley_cup_probability_delta_from_previous {
            metrics.push(signed(
                "cup_probability_delta",
                "Change in Stanley Cup odds",
                value * 100.0,
                MetricUnit::Percentage,
            ));
        }
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: format!("checkpoint-{}", index + 1),
            title: Some(format!(
                "Through {} · {} games complete",
                point.as_of_date, point.completed_games
            )),
            metrics,
        })
    }));
    let mut net_metrics = vec![
        rank_metric(
            history.projected_points_movement_rank,
            history.league_team_count,
        ),
        text_metric(
            "projected_points_trend",
            "Trajectory",
            history.projected_points_trend.as_str(),
        ),
        text_metric(
            "net_points_movement_materiality",
            "Movement materiality",
            history.net_points_movement_materiality.as_str(),
        ),
        signed(
            "largest_projected_points_swing",
            &format!(
                "Largest checkpoint swing · {} → {}",
                history.largest_swing_from_date, history.largest_swing_to_date
            ),
            history.largest_projected_points_swing,
            MetricUnit::Points,
        ),
        signed(
            "net_projected_points_delta",
            "Net change in projected points",
            history.average_points_delta_first_to_last,
            MetricUnit::Points,
        ),
        signed_with_evidence(
            "observed_standings_points_delta_first_to_last",
            "Confirmed standings points gained",
            history.observed_standings_points_delta_first_to_last as f64,
            MetricUnit::Points,
            EvidenceLabel::Confirmed,
        ),
        signed(
            "expected_remaining_points_delta_first_to_last",
            "Change in expected remaining points",
            history.expected_remaining_points_delta_first_to_last,
            MetricUnit::Points,
        ),
        text_metric(
            "points_movement_bridge_status",
            "Movement bridge",
            "confirmed points + remainder change = net movement",
        ),
        signed(
            "net_playoff_probability_delta",
            "Net change in playoff odds",
            history.playoff_probability_delta_first_to_last * 100.0,
            MetricUnit::Percentage,
        ),
        signed(
            "net_cup_probability_delta",
            "Net change in Stanley Cup odds",
            history.stanley_cup_probability_delta_first_to_last * 100.0,
            MetricUnit::Percentage,
        ),
    ];
    if let (Some(expected_interval), Some(realized_vs_pace), Some(outlook_revaluation)) = (
        history.prior_expected_points_for_completed_interval,
        history.realized_points_vs_prior_remaining_pace,
        history.remaining_outlook_revaluation,
    ) {
        net_metrics.extend([
            signed(
                "prior_expected_points_for_completed_interval",
                &format!(
                    "Prior expected points for {} newly completed games",
                    history.completed_games_delta_first_to_last
                ),
                expected_interval,
                MetricUnit::Points,
            ),
            signed_with_evidence(
                "realized_points_vs_prior_remaining_pace",
                "Realized points versus prior expected pace",
                realized_vs_pace,
                MetricUnit::Points,
                EvidenceLabel::Confirmed,
            ),
            signed(
                "remaining_outlook_revaluation",
                "Still-unplayed outlook revaluation",
                outlook_revaluation,
                MetricUnit::Points,
            ),
            text_metric(
                "pace_attribution_status",
                "Pace-normalized attribution",
                "realized versus prior pace + remaining revaluation = net movement",
            ),
        ]);
    }
    if let Some(share) = history.net_points_movement_share_of_range {
        net_metrics.push(probability(
            "net_points_movement_share_of_range",
            "Net movement / average outcome spread",
            share,
        ));
    }
    sections.push(CardSectionView::MetricStrip(MetricStripSectionView {
        id: "history-net-movement".to_string(),
        title: Some(format!(
            "Net change · {} → {}",
            checkpoints
                .first()
                .expect("validated checkpoints")
                .as_of_date,
            checkpoints
                .last()
                .expect("validated checkpoints")
                .as_of_date
        )),
        metrics: net_metrics,
    }));
    sections
}

fn rank_metric(rank: usize, team_count: usize) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey("projected_points_movement_rank".to_string()),
            label: "League movement rank".to_string(),
            value: MetricValue::Integer(rank as i64),
            unit: MetricUnit::Count,
            precision: ValuePrecision::Integer,
            token: None,
        },
        display_text: format!("{rank} of {team_count}"),
        accessible_text: format!(
            "League projected points movement rank {rank} of {team_count}, highest gain first"
        ),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn text_metric(key: &str, label: &str, value: &str) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Text(value.to_string()),
            unit: MetricUnit::None,
            precision: ValuePrecision::Raw,
            token: None,
        },
        display_text: value.to_string(),
        accessible_text: format!("{label} {value}"),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn metric(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: cell(key, label, value, unit),
        display_text: format!("{value:.2}"),
        accessible_text: format!("{label} {value:.2}"),
        comparison: None,
        evidence_label,
    }
}

fn probability(key: &str, label: &str, value: f64) -> CardMetricView {
    let value = value * 100.0;
    CardMetricView {
        metric: cell(key, label, value, MetricUnit::Percentage),
        display_text: format!("{value:.1}%"),
        accessible_text: format!("{label} {value:.1} percent"),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn signed(key: &str, label: &str, value: f64, unit: MetricUnit) -> CardMetricView {
    signed_with_evidence(key, label, value, unit, EvidenceLabel::Simulated)
}

fn signed_with_evidence(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let suffix = if unit == MetricUnit::Percentage {
        " pp"
    } else {
        ""
    };
    CardMetricView {
        metric: cell(key, label, value, unit),
        display_text: format!("{value:+.2}{suffix}"),
        accessible_text: format!("{label} {value:+.2}{suffix}"),
        comparison: Some(CardMetricComparisonView {
            label: "change from previous checkpoint".to_string(),
            baseline: MetricValue::Decimal(0.0),
            delta: MetricValue::Decimal(value),
        }),
        evidence_label,
    }
}

fn cell(key: &str, label: &str, value: f64, unit: MetricUnit) -> MetricCell {
    MetricCell {
        key: StatKey(key.to_string()),
        label: label.to_string(),
        value: MetricValue::Decimal(value),
        unit,
        precision: ValuePrecision::TwoDecimals,
        token: None,
    }
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn approximately_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9
}

fn optional_approximately_equal(left: Option<f64>, right: Option<f64>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => approximately_equal(left, right),
        (None, None) => true,
        _ => false,
    }
}

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, ForecastHistoryCardError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| ForecastHistoryCardError::Serialize(error.to_string()))
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{
        model::Season, season_stats::SeasonType, TeamSeasonForecastHistoryCheckpointRow,
        TeamSeasonForecastHistoryMateriality, TeamSeasonForecastHistoryPointRow,
        TeamSeasonForecastHistoryTeamRow, TeamSeasonForecastHistoryTrend, ViewWindow,
    };

    fn history() -> TeamSeasonForecastHistoryView {
        let dates = [
            NaiveDate::from_ymd_opt(2025, 1, 31).unwrap(),
            NaiveDate::from_ymd_opt(2025, 2, 28).unwrap(),
        ];
        TeamSeasonForecastHistoryView {
            schema: TEAM_SEASON_FORECAST_HISTORY_SCHEMA.to_string(),
            season: 20242025,
            trials: 1_000,
            seed: 20_242_025,
            checkpoints: vec![
                TeamSeasonForecastHistoryCheckpointRow {
                    as_of_date: dates[0],
                    fingerprint: "a".repeat(64),
                    league_completed_games: 800,
                    league_remaining_games: 512,
                },
                TeamSeasonForecastHistoryCheckpointRow {
                    as_of_date: dates[1],
                    fingerprint: "b".repeat(64),
                    league_completed_games: 900,
                    league_remaining_games: 412,
                },
            ],
            teams: vec![TeamSeasonForecastHistoryTeamRow {
                team: "NYR".to_string(),
                checkpoints: vec![
                    point(dates[0], 90.0, None),
                    point(dates[1], 92.25, Some(2.25)),
                ],
                average_points_delta_first_to_last: 2.25,
                playoff_probability_delta_first_to_last: 0.04,
                stanley_cup_probability_delta_first_to_last: 0.005,
                projected_points_movement_rank: 1,
                league_team_count: 1,
                projected_points_trend: TeamSeasonForecastHistoryTrend::Improving,
                largest_projected_points_swing: 2.25,
                largest_swing_from_date: dates[0],
                largest_swing_to_date: dates[1],
                average_first_last_points_range_width: 20.0,
                net_points_movement_share_of_range: Some(0.1125),
                net_points_movement_materiality: TeamSeasonForecastHistoryMateriality::Moderate,
                observed_standings_points_delta_first_to_last: 10,
                expected_remaining_points_delta_first_to_last: -7.75,
                points_movement_reconciliation_error: 0.0,
                completed_games_delta_first_to_last: 9,
                prior_expected_points_per_remaining_game: Some(1.125),
                prior_expected_points_for_completed_interval: Some(10.125),
                realized_points_vs_prior_remaining_pace: Some(-0.125),
                remaining_outlook_revaluation: Some(2.375),
                pace_attribution_reconciliation_error: Some(0.0),
            }],
            biggest_risers: Vec::new(),
            biggest_fallers: Vec::new(),
            disclosures: vec!["Same sealed simulation identity.".to_string()],
        }
    }

    fn point(
        date: NaiveDate,
        points: f64,
        delta: Option<f64>,
    ) -> TeamSeasonForecastHistoryPointRow {
        let observed_standings_points = if delta.is_some() { 64 } else { 54 };
        TeamSeasonForecastHistoryPointRow {
            as_of_date: date,
            average_points: points,
            points_p10: 80,
            points_p50: 90,
            points_p90: 100,
            playoff_probability: if delta.is_some() { 0.54 } else { 0.5 },
            stanley_cup_probability: if delta.is_some() { 0.025 } else { 0.02 },
            average_longest_win_streak: 5.0,
            completed_games: if delta.is_some() { 59 } else { 50 },
            remaining_games: if delta.is_some() { 23 } else { 32 },
            observed_standings_points,
            expected_remaining_points: points - f64::from(observed_standings_points),
            average_points_delta_from_previous: delta,
            playoff_probability_delta_from_previous: delta.map(|_| 0.04),
            stanley_cup_probability_delta_from_previous: delta.map(|_| 0.005),
            completed_games_delta_from_previous: delta.map(|_| 9),
            prior_expected_points_for_completed_interval_from_previous: delta.map(|_| 10.125),
            realized_points_vs_prior_remaining_pace_from_previous: delta.map(|_| -0.125),
            remaining_outlook_revaluation_from_previous: delta.map(|_| 2.375),
            pace_attribution_reconciliation_error_from_previous: delta.map(|_| 0.0),
        }
    }

    fn input() -> ForecastHistoryCardInput {
        ForecastHistoryCardInput {
            history: history(),
            focus_team: "nyr".to_string(),
            team_name: "New York Rangers".to_string(),
            view: ViewContext::new(ViewWindow::new(Season(20242025), SeasonType::Regular)),
            evidence_at: None,
        }
    }

    #[test]
    fn history_card_preserves_all_checkpoint_sources_and_levels() {
        let card = build_forecast_history_card(input()).unwrap();
        assert_eq!(card.card_kind, CardKind::ForecastHistory);
        assert_eq!(card.pages[0].display_label.as_deref(), Some("The Tape"));
        assert_eq!(card.provenance.len(), 2);
        assert_eq!(card.pages[0].sections.len(), 4);
        let CardSectionView::MetricStrip(second) = &card.pages[0].sections[2] else {
            panic!("expected second checkpoint metrics");
        };
        assert!(second
            .metrics
            .iter()
            .any(|metric| metric.display_text == "+2.25"));
        assert!(second.metrics.iter().any(|metric| {
            metric.accessible_text == "Realized points versus prior checkpoint pace -0.12"
        }));
        let CardSectionView::MetricStrip(net) = &card.pages[0].sections[3] else {
            panic!("expected net movement metrics");
        };
        assert!(net
            .metrics
            .iter()
            .any(|metric| metric.metric.label == "Net change in projected points"));
        assert!(net
            .metrics
            .iter()
            .any(|metric| metric.display_text == "1 of 1"));
        assert!(net
            .metrics
            .iter()
            .any(|metric| metric.accessible_text == "Trajectory improving"));
        assert!(net
            .metrics
            .iter()
            .any(|metric| metric.accessible_text == "Movement materiality moderate"));
        assert!(net
            .metrics
            .iter()
            .any(|metric| metric.accessible_text == "Confirmed standings points gained +10.00"));
        assert!(net.metrics.iter().any(|metric| {
            metric.accessible_text == "Realized points versus prior expected pace -0.12"
        }));
        card.validate().unwrap();
    }

    #[test]
    fn history_card_rejects_inconsistent_team_dates_and_progress() {
        let mut inconsistent_date = input();
        inconsistent_date.history.teams[0].checkpoints[1].as_of_date =
            NaiveDate::from_ymd_opt(2025, 2, 27).unwrap();
        assert_eq!(
            build_forecast_history_card(inconsistent_date),
            Err(ForecastHistoryCardError::InconsistentCheckpoints(
                "NYR".to_string()
            ))
        );

        let mut regressed_progress = input();
        regressed_progress.history.teams[0].checkpoints[1].completed_games = 49;
        assert_eq!(
            build_forecast_history_card(regressed_progress),
            Err(ForecastHistoryCardError::InconsistentCheckpoints(
                "NYR".to_string()
            ))
        );
    }
}
