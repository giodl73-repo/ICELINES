//! UI-neutral matchup card for one or more sealed prediction vintages.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::*;
use crate::view_model::{
    Completeness, EvidenceLabel, MetricCell, MetricUnit, MetricValue, SemanticToken, SourceKind,
    StatKey, TeamGameForecastVintage, TeamGamePredictionEdgeGameRow, TeamGamePredictionEdgeView,
    ValuePrecision, ViewContext, ViewWarning, WarningKind, TEAM_GAME_PREDICTION_EDGE_METHOD,
};

pub const TEAM_GAME_PREDICTION_EDGE_CARD_VERSION: &str = "team_game_prediction_edge_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionEdgeCardInput {
    /// One to three sealed edge documents for the same baseline and game.
    pub edges: Vec<TeamGamePredictionEdgeView>,
    pub game_id: u64,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
    /// Optional closing-market consensus used only as a retrospective
    /// benchmark. It never enters an IceLines probability calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_benchmark: Option<TeamGamePredictionMarketBenchmarkInput>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TeamGamePredictionMarketBenchmarkInput {
    pub label: String,
    pub captured_at: DateTime<Utc>,
    pub home_win_probability: f64,
    pub source_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TeamGamePredictionEdgeCardError {
    #[error("prediction edge card requires at least one sealed vintage")]
    MissingEdges,
    #[error("prediction edge team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("prediction edge card requires a team name")]
    MissingTeamName,
    #[error("prediction edge season {edge} does not match view season {view}")]
    SeasonMismatch { edge: u32, view: u32 },
    #[error("prediction edge source or model differs between vintages")]
    IncomparableVintages,
    #[error("duplicate prediction edge vintage: {0:?}")]
    DuplicateVintage(TeamGameForecastVintage),
    #[error("prediction edge game {0} is absent")]
    MissingGame(u64),
    #[error("prediction edge game identity differs between vintages")]
    GameIdentityMismatch,
    #[error("team {team} does not participate in game {game_id}")]
    TeamNotInGame { team: String, game_id: u64 },
    #[error("invalid sealed prediction edge: {0}")]
    InvalidEdge(String),
    #[error("invalid closing-market benchmark: {0}")]
    InvalidMarketBenchmark(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_team_game_prediction_edge_card(
    mut input: TeamGamePredictionEdgeCardInput,
) -> Result<CardDocumentView, TeamGamePredictionEdgeCardError> {
    if input.edges.is_empty() {
        return Err(TeamGamePredictionEdgeCardError::MissingEdges);
    }
    let team = input.focus_team.trim().to_ascii_uppercase();
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(TeamGamePredictionEdgeCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(TeamGamePredictionEdgeCardError::MissingTeamName);
    }
    input.edges.sort_by_key(|edge| edge.vintage);
    let first = &input.edges[0];
    let season = first.season;
    if season != input.view.window.season.0 {
        return Err(TeamGamePredictionEdgeCardError::SeasonMismatch {
            edge: season,
            view: input.view.window.season.0,
        });
    }
    let mut vintages = BTreeSet::new();
    for edge in &input.edges {
        edge.validate()
            .map_err(|error| TeamGamePredictionEdgeCardError::InvalidEdge(error.to_string()))?;
        if edge.season != season {
            return Err(TeamGamePredictionEdgeCardError::SeasonMismatch {
                edge: edge.season,
                view: input.view.window.season.0,
            });
        }
        if edge.source_forecast_fingerprint != first.source_forecast_fingerprint
            || edge.model != first.model
        {
            return Err(TeamGamePredictionEdgeCardError::IncomparableVintages);
        }
        if !vintages.insert(edge.vintage) {
            return Err(TeamGamePredictionEdgeCardError::DuplicateVintage(
                edge.vintage,
            ));
        }
    }
    let games = input
        .edges
        .iter()
        .map(|edge| {
            edge.games
                .iter()
                .find(|game| game.game_id == input.game_id)
                .ok_or(TeamGamePredictionEdgeCardError::MissingGame(input.game_id))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let game = games[0];
    if games.iter().skip(1).any(|candidate| {
        candidate.date != game.date
            || candidate.away_team != game.away_team
            || candidate.home_team != game.home_team
    }) {
        return Err(TeamGamePredictionEdgeCardError::GameIdentityMismatch);
    }
    let focus_is_home = if team == game.home_team {
        true
    } else if team == game.away_team {
        false
    } else {
        return Err(TeamGamePredictionEdgeCardError::TeamNotInGame {
            team,
            game_id: input.game_id,
        });
    };
    let latest_edge = input.edges.last().expect("non-empty edges");
    let latest_game = *games.last().expect("one game per edge");
    if let Some(benchmark) = &input.market_benchmark {
        let forecast_at = latest_game.forecast_at.ok_or_else(|| {
            TeamGamePredictionEdgeCardError::InvalidMarketBenchmark(
                "latest vintage has no forecast timestamp".to_owned(),
            )
        })?;
        if benchmark.label.trim().is_empty()
            || !benchmark.home_win_probability.is_finite()
            || !(0.0..=1.0).contains(&benchmark.home_win_probability)
            || benchmark.captured_at < forecast_at
            || !valid_card_sha256(&benchmark.source_fingerprint)
        {
            return Err(TeamGamePredictionEdgeCardError::InvalidMarketBenchmark(
                "label, probability, capture boundary, or source fingerprint is invalid".to_owned(),
            ));
        }
    }
    let opening_probability = focused_probability(game, focus_is_home);
    let latest_probability = focused_probability(latest_game, focus_is_home);
    let evidence_label = vintage_evidence_label(latest_edge.vintage);
    let opponent = if focus_is_home {
        &game.away_team
    } else {
        &game.home_team
    };

    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert("prediction_edge".to_string(), latest_edge.schema.clone());
    methodology_versions.insert(
        "prediction_model".to_string(),
        latest_edge.model.method.clone(),
    );
    methodology_versions.insert(
        "card_projection".to_string(),
        TEAM_GAME_PREDICTION_EDGE_CARD_VERSION.to_string(),
    );
    let warnings = input
        .edges
        .iter()
        .flat_map(|edge| edge.warnings.iter())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|message| ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::Snapshot),
            message,
            recovery: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut provenance = input
        .edges
        .iter()
        .map(|edge| CardProvenanceView {
            id: format!("edge-{}", vintage_key(edge.vintage)),
            source: SourceKind::Snapshot,
            label: format!("{} sealed prediction edge", vintage_label(edge.vintage)),
            state: Completeness::Complete,
            observed_at: Some(edge.generated_at),
            fingerprint: Some(card_fingerprint(&edge.fingerprint)),
            note: Some(format!(
                "{} games; model {} ({:?})",
                edge.games.len(),
                edge.model.model_id,
                edge.model.authority
            )),
        })
        .collect::<Vec<_>>();
    if let Some(benchmark) = &input.market_benchmark {
        provenance.push(CardProvenanceView {
            id: "closing-market-benchmark".to_owned(),
            source: SourceKind::Snapshot,
            label: benchmark.label.trim().to_owned(),
            state: Completeness::Complete,
            observed_at: Some(benchmark.captured_at),
            fingerprint: Some(card_fingerprint(&benchmark.source_fingerprint)),
            note: Some(
                "Benchmark only; excluded from model features and forecast vintages.".to_owned(),
            ),
        });
    }
    let provenance_ids = provenance.iter().map(|row| row.id.clone()).collect();

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::TeamGamePredictionEdge,
        document_id: format!("prediction-edge:{}:{}:{}", season, input.game_id, team),
        fingerprint: String::new(),
        title: format!("{} at {}", game.away_team, game.home_team),
        subtitle: Some(format!(
            "{} · {} read · {} {:.1}%",
            game.date,
            vintage_label(latest_edge.vintage),
            team,
            latest_probability * 100.0
        )),
        context: CardContextView {
            view: input.view.clone(),
            evidence_at: input.evidence_at.or(Some(latest_edge.generated_at)),
            evidence_label,
            builder_version: TEAM_GAME_PREDICTION_EDGE_CARD_VERSION.to_string(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                scenario_comparison_key: Some(
                    input
                        .edges
                        .iter()
                        .map(|edge| edge.fingerprint.as_str())
                        .collect::<Vec<_>>()
                        .join(":"),
                ),
                team_ids: vec![game.away_team.clone(), game.home_team.clone()],
                game_ids: vec![input.game_id.to_string()],
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some(latest_edge.model.model_id.clone()),
                model_version: Some(latest_edge.model.method.clone()),
                parameter_fingerprint: latest_edge.model.training_fingerprint.clone(),
                seed: None,
                trials: None,
            },
        },
        theme: nhl_team_card_theme(&team),
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "line".to_string(),
                literal_label: "Matchup probability and forecast-vintage movement".to_string(),
                display_label: Some("The Line".to_string()),
                order: 1,
                accessible_summary: format!(
                    "{} win probability against {}, its movement across forecast vintages, and the evidence factors behind the latest read.",
                    input.team_name.trim(), opponent
                ),
                sections: line_sections(
                    &input,
                    &games,
                    focus_is_home,
                    opening_probability,
                    latest_probability,
                    evidence_label,
                    input.market_benchmark.as_ref(),
                ),
            },
            CardPageView {
                id: "insider".to_string(),
                literal_label: "Prediction methodology, authority, and provenance".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary: "How IceCast freezes each forecast vintage, separates evidence from outcomes, and labels model authority.".to_string(),
                sections: vec![
                    CardSectionView::Methodology(MethodologySectionView {
                        id: "edge-methodology".to_string(),
                        title: "How the line moves".to_string(),
                        methods: vec![CardMethodologyItemView {
                            key: "elo-evidence-logit".to_string(),
                            label: "Elo plus dated evidence".to_string(),
                            version: TEAM_GAME_PREDICTION_EDGE_METHOD.to_string(),
                            summary: "The opening game probability is blended with Elo, then dated roster, availability, goalie, xG, special-teams, and matchup evidence moves log odds. Each vintage is sealed before outcomes are joined.".to_string(),
                        }],
                        limitations: latest_edge.disclosures.clone(),
                    }),
                    CardSectionView::Provenance(ProvenanceSectionView {
                        id: "edge-sources".to_string(),
                        title: "Sealed forecast vintages".to_string(),
                        provenance_ids,
                    }),
                ],
            },
        ],
        assets: Vec::new(),
        provenance,
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| TeamGamePredictionEdgeCardError::Document(error.to_string()))
}

fn line_sections(
    input: &TeamGamePredictionEdgeCardInput,
    games: &[&TeamGamePredictionEdgeGameRow],
    focus_is_home: bool,
    opening_probability: f64,
    latest_probability: f64,
    evidence_label: EvidenceLabel,
    market_benchmark: Option<&TeamGamePredictionMarketBenchmarkInput>,
) -> Vec<CardSectionView> {
    let first_game = games[0];
    let latest_game = *games.last().expect("one game per edge");
    let team = input.focus_team.trim().to_ascii_uppercase();
    let vintage_metrics = input
        .edges
        .iter()
        .zip(games)
        .map(|(edge, game)| {
            probability_metric(
                vintage_key(edge.vintage),
                vintage_label(edge.vintage),
                focused_probability(game, focus_is_home),
                opening_probability,
                vintage_evidence_label(edge.vintage),
            )
        })
        .collect();
    let factor_metrics = latest_game
        .factors
        .iter()
        .filter(|factor| factor.available || factor.key == "calibration")
        .map(|factor| {
            let delta = if focus_is_home {
                factor.home_win_probability_delta
            } else {
                -factor.home_win_probability_delta
            };
            signed_probability_metric(
                &format!("factor_{}", factor.key),
                &factor_label(&factor.key),
                delta,
                evidence_label,
            )
        })
        .collect::<Vec<_>>();
    let strongest = latest_game
        .factors
        .iter()
        .filter(|factor| factor.available || factor.key == "calibration")
        .max_by(|left, right| {
            left.home_win_probability_delta
                .abs()
                .total_cmp(&right.home_win_probability_delta.abs())
        });
    let mut rationale = vec![format!(
        "The latest read moved {:+.1} percentage points from the opening vintage.",
        (latest_probability - opening_probability) * 100.0
    )];
    if let Some(factor) = strongest {
        let delta = if focus_is_home {
            factor.home_win_probability_delta
        } else {
            -factor.home_win_probability_delta
        };
        rationale.push(format!(
            "Largest latest-vintage factor: {} ({:+.1} percentage points).",
            factor_label(&factor.key),
            delta * 100.0
        ));
    }
    rationale.push(format!(
        "Evidence coverage: {}/{} factors.",
        latest_game.available_features, latest_game.expected_features
    ));
    let stability_metrics = stability_metrics(latest_game, focus_is_home, evidence_label);
    let mut sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "matchup".to_string(),
            eyebrow: Some("IceCast game prediction".to_string()),
            title: input.team_name.trim().to_string(),
            subtitle: Some(format!(
                "{} at {} · game {}",
                first_game.away_team, first_game.home_team, input.game_id
            )),
            identities: vec![
                CardIdentityView {
                    kind: CardIdentityKind::Team,
                    subject_id: first_game.away_team.clone(),
                    label: first_game.away_team.clone(),
                    asset_id: None,
                },
                CardIdentityView {
                    kind: CardIdentityKind::Team,
                    subject_id: first_game.home_team.clone(),
                    label: first_game.home_team.clone(),
                    asset_id: None,
                },
            ],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "vintage-probabilities".to_string(),
            title: Some(format!("{team} win probability")),
            metrics: vintage_metrics,
        }),
        CardSectionView::Decision(DecisionSectionView {
            id: "latest-read".to_string(),
            title: "Latest read".to_string(),
            recommendation: format!("{team} {:.1}%", latest_probability * 100.0),
            rationale,
            alternatives: Vec::new(),
            action_id: None,
            token: SemanticToken::DecisionHighlight,
            evidence_label,
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "factor-movement".to_string(),
            title: Some("What moved the line in the latest vintage".to_string()),
            metrics: factor_metrics,
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "evidence-stability".to_string(),
            title: Some("Evidence-stability range (not a confidence interval)".to_string()),
            metrics: stability_metrics,
        }),
    ];
    if let Some(benchmark) = market_benchmark {
        let market_probability = if focus_is_home {
            benchmark.home_win_probability
        } else {
            1.0 - benchmark.home_win_probability
        };
        sections.push(CardSectionView::MetricStrip(MetricStripSectionView {
            id: "market-benchmark".to_owned(),
            title: Some("Closing-market benchmark only (not a model input)".to_owned()),
            metrics: vec![probability_metric(
                "closing_market",
                benchmark.label.trim(),
                market_probability,
                latest_probability,
                EvidenceLabel::Confirmed,
            )],
        }));
    }
    sections
}

fn stability_metrics(
    game: &TeamGamePredictionEdgeGameRow,
    focus_is_home: bool,
    evidence_label: EvidenceLabel,
) -> Vec<CardMetricView> {
    let (low, high) = if focus_is_home {
        (
            game.stability_low_home_win_probability,
            game.stability_high_home_win_probability,
        )
    } else {
        (
            game.stability_high_home_win_probability
                .map(|value| 1.0 - value),
            game.stability_low_home_win_probability
                .map(|value| 1.0 - value),
        )
    };
    [
        ("stability_low", "Low", low),
        ("stability_high", "High", high),
        (
            "evidence_confidence",
            "Evidence confidence",
            game.evidence_confidence,
        ),
    ]
    .into_iter()
    .filter_map(|(key, label, value)| {
        value.map(|value| plain_percentage_metric(key, label, value, evidence_label))
    })
    .collect()
}

fn focused_probability(game: &TeamGamePredictionEdgeGameRow, focus_is_home: bool) -> f64 {
    if focus_is_home {
        game.enhanced_home_win_probability
    } else {
        1.0 - game.enhanced_home_win_probability
    }
}

fn probability_metric(
    key: &str,
    label: &str,
    value: f64,
    baseline: f64,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let percentage = value * 100.0;
    let delta = (value - baseline) * 100.0;
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(percentage),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::OneDecimal,
            token: None,
        },
        display_text: format!("{percentage:.1}%"),
        accessible_text: format!(
            "{label}: {percentage:.1} percent, {delta:+.1} percentage points from opening"
        ),
        comparison: Some(CardMetricComparisonView {
            label: "change from opening vintage".to_string(),
            baseline: MetricValue::Decimal(baseline * 100.0),
            delta: MetricValue::Decimal(delta),
        }),
        evidence_label,
    }
}

fn signed_probability_metric(
    key: &str,
    label: &str,
    delta: f64,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let percentage_points = delta * 100.0;
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(percentage_points),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::OneDecimal,
            token: Some(if percentage_points >= 0.0 {
                SemanticToken::Rising
            } else {
                SemanticToken::Risk
            }),
        },
        display_text: format!("{percentage_points:+.1} pp"),
        accessible_text: format!("{label}: {percentage_points:+.1} percentage points"),
        comparison: Some(CardMetricComparisonView {
            label: "factor contribution".to_string(),
            baseline: MetricValue::Decimal(0.0),
            delta: MetricValue::Decimal(percentage_points),
        }),
        evidence_label,
    }
}

fn plain_percentage_metric(
    key: &str,
    label: &str,
    value: f64,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let percentage = value * 100.0;
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(percentage),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::OneDecimal,
            token: None,
        },
        display_text: format!("{percentage:.1}%"),
        accessible_text: format!("{label}: {percentage:.1} percent"),
        comparison: None,
        evidence_label,
    }
}

fn vintage_key(vintage: TeamGameForecastVintage) -> &'static str {
    vintage.as_str()
}

fn vintage_label(vintage: TeamGameForecastVintage) -> &'static str {
    match vintage {
        TeamGameForecastVintage::Preseason => "Preseason",
        TeamGameForecastVintage::GameMorning => "Game morning",
        TeamGameForecastVintage::PregameConfirmed => "Pregame confirmed",
    }
}

fn vintage_evidence_label(vintage: TeamGameForecastVintage) -> EvidenceLabel {
    match vintage {
        TeamGameForecastVintage::Preseason => EvidenceLabel::Estimated,
        TeamGameForecastVintage::GameMorning => EvidenceLabel::Reported,
        TeamGameForecastVintage::PregameConfirmed => EvidenceLabel::Confirmed,
    }
}

fn factor_label(key: &str) -> String {
    match key {
        "roster" => "Roster quality",
        "availability" => "Player availability",
        "lineup_impact" => "Replacement-adjusted lineup",
        "goalie" => "Starting goalie",
        "goalie_schedule" => "Goalie under schedule load",
        "goalie_form" => "Goalie recent form",
        "goalie_workload" => "Goalie workload readiness",
        "xg_form" => "Expected-goals form",
        "opponent_adjusted_xg" => "Opponent-adjusted xG form",
        "special_teams" => "Special teams",
        "matchup" => "Matchup fit",
        "calibration" => "Model calibration",
        other => other,
    }
    .to_string()
}

fn card_fingerprint(value: &str) -> String {
    value.strip_prefix("sha256:").unwrap_or(value).to_string()
}

fn valid_card_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;
    use crate::{
        build_team_game_forecast, build_team_game_prediction_edge, model::Season,
        season_stats::SeasonType, TeamForecastGameInput, TeamForecastParameters,
        TeamForecastStrengthInput, TeamGameEvidenceState, TeamGamePredictionEvidenceInput,
        TeamGamePredictionModel, TeamGamePredictionTeamEvidence, ViewWindow,
    };

    fn team(team: &str, strength: f64) -> TeamGamePredictionTeamEvidence {
        TeamGamePredictionTeamEvidence {
            team: team.to_string(),
            roster_strength: Some(strength),
            roster_state: TeamGameEvidenceState::Confirmed,
            availability_strength: Some(strength),
            availability_state: TeamGameEvidenceState::Confirmed,
            lineup_impact: None,
            lineup_impact_state: TeamGameEvidenceState::Unavailable,
            goalie_quality: Some(strength),
            goalie_state: TeamGameEvidenceState::Confirmed,
            goalie_player_id: Some(if team == "NYR" { 1 } else { 2 }),
            goalie_form_quality: Some(strength),
            goalie_form_appearances: 5,
            goalie_form_state: TeamGameEvidenceState::Confirmed,
            goalie_workload_readiness: Some(strength),
            xg_share: Some(strength / 100.0),
            xg_games: 20,
            opponent_adjusted_xg_share: None,
            opponent_adjusted_xg_games: 0,
            special_teams_strength: Some(strength),
            special_teams_games: 20,
            matchup_suitability: Some((strength - 50.0) / 10.0),
            matchup_state: TeamGameEvidenceState::Modeled,
            source_fingerprints: vec![format!("sha256:{}", "a".repeat(64))],
        }
    }

    fn edges() -> Vec<TeamGamePredictionEdgeView> {
        let source = build_team_game_forecast(
            20_262_027,
            vec![TeamForecastGameInput {
                game_id: 27,
                date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
                away_team: "SEA".to_string(),
                home_team: "NYR".to_string(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            }],
            vec![
                TeamForecastStrengthInput {
                    team: "SEA".to_string(),
                    strength: 49.0,
                },
                TeamForecastStrengthInput {
                    team: "NYR".to_string(),
                    strength: 53.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();
        let evidence = TeamGamePredictionEvidenceInput {
            game_id: 27,
            forecast_at: Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap(),
            captured_at: Utc.with_ymd_and_hms(2026, 10, 10, 15, 0, 0).unwrap(),
            away: team("SEA", 48.0),
            home: team("NYR", 55.0),
        };
        [
            TeamGameForecastVintage::GameMorning,
            TeamGameForecastVintage::PregameConfirmed,
        ]
        .into_iter()
        .map(|vintage| {
            build_team_game_prediction_edge(
                &source,
                vintage,
                Utc.with_ymd_and_hms(2026, 10, 10, 17, 0, 0).unwrap(),
                TeamGamePredictionModel::evaluation_challenger(),
                vec![evidence.clone()],
            )
            .unwrap()
        })
        .collect()
    }

    #[test]
    fn card_compares_sealed_vintages_and_inverts_factors_for_away_team() {
        let edges = edges();
        let latest_game = edges.last().unwrap().games.first().unwrap();
        let expected_low = (1.0 - latest_game.stability_high_home_win_probability.unwrap()) * 100.0;
        let expected_high = (1.0 - latest_game.stability_low_home_win_probability.unwrap()) * 100.0;
        let card = build_team_game_prediction_edge_card(TeamGamePredictionEdgeCardInput {
            edges,
            game_id: 27,
            focus_team: "sea".to_string(),
            team_name: "Seattle Kraken".to_string(),
            view: ViewContext::new(ViewWindow::new(Season(20_262_027), SeasonType::Regular)),
            evidence_at: None,
            market_benchmark: None,
        })
        .unwrap();
        assert_eq!(card.card_kind, CardKind::TeamGamePredictionEdge);
        assert_eq!(card.pages[0].display_label.as_deref(), Some("The Line"));
        assert_eq!(card.provenance.len(), 2);
        let CardSectionView::MetricStrip(factors) = &card.pages[0].sections[3] else {
            panic!("expected factor strip");
        };
        let roster = factors
            .metrics
            .iter()
            .find(|metric| metric.metric.key.0 == "factor_roster")
            .unwrap();
        assert!(matches!(roster.metric.value, MetricValue::Decimal(value) if value < 0.0));
        let CardSectionView::MetricStrip(stability) = &card.pages[0].sections[4] else {
            panic!("expected stability strip");
        };
        assert!(
            matches!(stability.metrics[0].metric.value, MetricValue::Decimal(value) if (value - expected_low).abs() < 1e-12)
        );
        assert!(
            matches!(stability.metrics[1].metric.value, MetricValue::Decimal(value) if (value - expected_high).abs() < 1e-12)
        );
        card.validate().unwrap();
    }

    #[test]
    fn card_refuses_duplicate_vintages() {
        let mut edges = edges();
        edges[1] = edges[0].clone();
        assert_eq!(
            build_team_game_prediction_edge_card(TeamGamePredictionEdgeCardInput {
                edges,
                game_id: 27,
                focus_team: "SEA".to_string(),
                team_name: "Seattle Kraken".to_string(),
                view: ViewContext::new(ViewWindow::new(Season(20_262_027), SeasonType::Regular)),
                evidence_at: None,
                market_benchmark: None,
            }),
            Err(TeamGamePredictionEdgeCardError::DuplicateVintage(
                TeamGameForecastVintage::GameMorning
            ))
        );
    }

    #[test]
    fn closing_market_is_an_inverted_benchmark_and_never_a_factor() {
        let card = build_team_game_prediction_edge_card(TeamGamePredictionEdgeCardInput {
            edges: edges(),
            game_id: 27,
            focus_team: "SEA".to_owned(),
            team_name: "Seattle Kraken".to_owned(),
            view: ViewContext::new(ViewWindow::new(Season(20_262_027), SeasonType::Regular)),
            evidence_at: None,
            market_benchmark: Some(TeamGamePredictionMarketBenchmarkInput {
                label: "Consensus close".to_owned(),
                captured_at: Utc.with_ymd_and_hms(2026, 10, 10, 18, 0, 0).unwrap(),
                home_win_probability: 0.6,
                source_fingerprint: format!("sha256:{}", "b".repeat(64)),
            }),
        })
        .unwrap();
        let CardSectionView::MetricStrip(benchmark) = card.pages[0].sections.last().unwrap() else {
            panic!("expected market benchmark strip");
        };
        assert_eq!(benchmark.id, "market-benchmark");
        assert!(
            matches!(benchmark.metrics[0].metric.value, MetricValue::Decimal(value) if (value - 40.0).abs() < 1e-12)
        );
        assert_eq!(card.provenance.len(), 3);
        assert!(card.pages[0]
            .sections
            .iter()
            .filter_map(|section| match section {
                CardSectionView::MetricStrip(strip) => Some(strip),
                _ => None,
            })
            .flat_map(|strip| &strip.metrics)
            .all(|metric| metric.metric.key.0 != "factor_closing_market"));
    }
}
