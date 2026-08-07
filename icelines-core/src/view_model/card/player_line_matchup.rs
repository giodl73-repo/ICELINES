//! UI-neutral projection of one sealed player-line matchup forecast.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::*;
use crate::view_model::{
    validate_player_line_matchup_forecast, Completeness, EvidenceLabel, MetricCell, MetricUnit,
    MetricValue, PlayerForecastProfileView, PlayerLineMatchupForecastView,
    PlayerLineMatchupTeamView, PlayerLineMatchupUnitKind, SemanticToken, SourceKind, StatKey,
    TeamGameEvidenceState, TeamGamePredictionEdgeView, ValuePrecision, ViewContext, ViewWarning,
    WarningKind, PLAYER_LINE_MATCHUP_FORECAST_METHOD,
};

pub const PLAYER_LINE_MATCHUP_CARD_VERSION: &str = "player_line_matchup_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayerLineMatchupCardInput {
    pub matchup: PlayerLineMatchupForecastView,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prediction_edge: Option<TeamGamePredictionEdgeView>,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum PlayerLineMatchupCardError {
    #[error("player-line matchup team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("player-line matchup card requires a team name")]
    MissingTeamName,
    #[error("player-line matchup season {matchup} does not match view season {view}")]
    SeasonMismatch { matchup: u32, view: u32 },
    #[error("team {team} does not participate in game {game_id}")]
    TeamNotInGame { team: String, game_id: u64 },
    #[error("invalid sealed player-line matchup: {0}")]
    InvalidMatchup(String),
    #[error("invalid sealed prediction edge: {0}")]
    InvalidPredictionEdge(String),
    #[error("prediction edge does not match the player-line matchup")]
    PredictionEdgeMismatch,
    #[error("prediction edge does not cite the player-line matchup fingerprint")]
    MissingPredictionEdgeJoin,
    #[error("prediction edge does not contain an available matchup factor")]
    MissingPredictionEdgeFactor,
    #[error("card generation time cannot predate the attached prediction edge")]
    CardGeneratedBeforePredictionEdge,
    #[error("card document validation failed: {0}")]
    Document(String),
}

#[derive(Debug, Clone, PartialEq)]
struct PredictionEdgeProjection {
    fingerprint: String,
    generated_at: DateTime<Utc>,
    win_probability: f64,
    matchup_probability_delta: f64,
    evidence_confidence: Option<f64>,
    model_id: String,
    model_version: String,
    training_fingerprint: Option<String>,
}

pub fn build_player_line_matchup_card(
    input: PlayerLineMatchupCardInput,
) -> Result<CardDocumentView, PlayerLineMatchupCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(PlayerLineMatchupCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(PlayerLineMatchupCardError::MissingTeamName);
    }
    validate_player_line_matchup_forecast(&input.matchup)
        .map_err(PlayerLineMatchupCardError::InvalidMatchup)?;
    if input.matchup.season != input.view.window.season.0 {
        return Err(PlayerLineMatchupCardError::SeasonMismatch {
            matchup: input.matchup.season,
            view: input.view.window.season.0,
        });
    }
    let (focus, opponent) = if input.matchup.home.team == team {
        (&input.matchup.home, &input.matchup.away)
    } else if input.matchup.away.team == team {
        (&input.matchup.away, &input.matchup.home)
    } else {
        return Err(PlayerLineMatchupCardError::TeamNotInGame {
            team,
            game_id: input.matchup.game_id,
        });
    };
    let edge = input
        .prediction_edge
        .as_ref()
        .map(|edge| prediction_edge_projection(edge, &input.matchup, &team))
        .transpose()?;
    if edge.as_ref().is_some_and(|edge| {
        input
            .view
            .generated_at
            .is_none_or(|generated_at| generated_at < edge.generated_at)
    }) {
        return Err(PlayerLineMatchupCardError::CardGeneratedBeforePredictionEdge);
    }
    let evidence_label = evidence_label(focus.matchup_state);
    let evidence_at = input.evidence_at.or(Some(input.matchup.captured_at));
    let source_fingerprint = strip_sha256(&input.matchup.fingerprint);
    let mut methods = BTreeMap::new();
    methods.insert(
        "player_line_matchup".to_owned(),
        input.matchup.method.clone(),
    );
    methods.insert(
        "card_projection".to_owned(),
        PLAYER_LINE_MATCHUP_CARD_VERSION.to_owned(),
    );
    if let Some(edge) = input.prediction_edge.as_ref() {
        methods.insert("prediction_edge".to_owned(), edge.schema.clone());
        methods.insert("prediction_model".to_owned(), edge.model.method.clone());
    }
    let mut provenance = vec![CardProvenanceView {
        id: "sealed-player-line-matchup".to_owned(),
        source: SourceKind::Snapshot,
        label: "Sealed player-line matchup forecast".to_owned(),
        state: completeness(focus.matchup_state),
        observed_at: Some(input.matchup.captured_at),
        fingerprint: Some(source_fingerprint.clone()),
        note: Some(format!(
            "{} source seals; {} profiles and {} units per team",
            input.matchup.source_fingerprints.len(),
            focus.profiles.len(),
            focus.units.len()
        )),
    }];
    if let Some(edge) = edge.as_ref() {
        provenance.push(CardProvenanceView {
            id: "sealed-prediction-edge".to_owned(),
            source: SourceKind::Snapshot,
            label: "Sealed prediction edge".to_owned(),
            state: Completeness::Complete,
            observed_at: Some(edge.generated_at),
            fingerprint: Some(strip_sha256(&edge.fingerprint)),
            note: Some(
                "Probability and factor movement are edge-owned values; the card does not recompute them."
                    .to_owned(),
            ),
        });
    }
    let warnings = input
        .matchup
        .warnings
        .iter()
        .cloned()
        .map(|message| ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::Snapshot),
            message,
            recovery: Vec::new(),
        })
        .collect::<Vec<_>>();
    let player_ids = input
        .matchup
        .away
        .profiles
        .iter()
        .chain(&input.matchup.home.profiles)
        .map(|profile| profile.player_id.to_string())
        .collect();

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_owned(),
        card_kind: CardKind::PlayerLineMatchup,
        document_id: format!(
            "player-line-matchup:{}:{}:{}",
            input.matchup.season, input.matchup.game_id, team
        ),
        fingerprint: String::new(),
        title: format!("{} at {}", input.matchup.away.team, input.matchup.home.team),
        subtitle: Some(format!(
            "{} · {} matchup read",
            input.matchup.game_date,
            vintage_label(input.matchup.vintage)
        )),
        context: CardContextView {
            view: input.view,
            evidence_at,
            evidence_label,
            builder_version: PLAYER_LINE_MATCHUP_CARD_VERSION.to_owned(),
            methodology_versions: methods,
            joins: CardIdentityJoinsView {
                scenario_comparison_key: Some(match edge.as_ref() {
                    Some(edge) => format!("{}:{}", input.matchup.fingerprint, edge.fingerprint),
                    None => input.matchup.fingerprint.clone(),
                }),
                team_ids: vec![
                    input.matchup.away.team.clone(),
                    input.matchup.home.team.clone(),
                ],
                player_ids,
                game_ids: vec![input.matchup.game_id.to_string()],
                ..CardIdentityJoinsView::default()
            },
            simulation: edge.as_ref().map_or_else(
                || CardSimulationContextView {
                    model_id: Some("player-line-matchup".to_owned()),
                    model_version: Some(input.matchup.method.clone()),
                    parameter_fingerprint: Some(source_fingerprint),
                    seed: None,
                    trials: None,
                },
                |edge| CardSimulationContextView {
                    model_id: Some(edge.model_id.clone()),
                    model_version: Some(edge.model_version.clone()),
                    parameter_fingerprint: edge.training_fingerprint.clone(),
                    seed: None,
                    trials: None,
                },
            ),
        },
        theme: nhl_team_card_theme(&team),
        // Every headshot has an initials fallback, so raster support remains optional.
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "matchup".to_owned(),
                literal_label: "Player, line, and opponent matchup".to_owned(),
                display_label: Some("The Matchup".to_owned()),
                order: 1,
                accessible_summary: format!(
                    "{} five-on-five matchup, dressed units, evidence coverage, and special-teams comparison against {}.",
                    input.team_name.trim(),
                    opponent.team
                ),
                sections: matchup_sections(
                    &team,
                    input.team_name.trim(),
                    focus,
                    opponent,
                    input.matchup.game_id,
                    evidence_label,
                    edge.as_ref(),
                ),
            },
            CardPageView {
                id: "insider".to_owned(),
                literal_label: "Matchup method, limitations, and provenance".to_owned(),
                display_label: Some("The Insider".to_owned()),
                order: 2,
                accessible_summary:
                    "How player profiles, line evidence, opponent style, and manager execution produce this sealed matchup read."
                        .to_owned(),
                sections: insider_sections(&input.matchup, focus),
            },
        ],
        assets: player_assets(focus, input.matchup.captured_at),
        provenance,
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| PlayerLineMatchupCardError::Document(error.to_string()))
}

fn matchup_sections(
    team: &str,
    team_name: &str,
    focus: &PlayerLineMatchupTeamView,
    opponent: &PlayerLineMatchupTeamView,
    game_id: u64,
    evidence_label: EvidenceLabel,
    edge: Option<&PredictionEdgeProjection>,
) -> Vec<CardSectionView> {
    let suitability = focus.matchup_suitability;
    let mut sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "matchup-identity".to_owned(),
            eyebrow: Some("IceCast player-line matchup".to_owned()),
            title: team_name.to_owned(),
            subtitle: Some(format!("{} vs {} · game {}", team, opponent.team, game_id)),
            identities: vec![
                CardIdentityView {
                    kind: CardIdentityKind::Team,
                    subject_id: team.to_owned(),
                    label: team_name.to_owned(),
                    asset_id: None,
                },
                CardIdentityView {
                    kind: CardIdentityKind::Team,
                    subject_id: opponent.team.clone(),
                    label: opponent.team.clone(),
                    asset_id: None,
                },
            ],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "matchup-headline".to_owned(),
            title: Some("Five-on-five read".to_owned()),
            metrics: vec![
                score_metric(
                    "five_on_five_score",
                    "Matchup score",
                    focus.five_on_five_matchup_score,
                    evidence_label,
                ),
                optional_score_metric(
                    "matchup_suitability",
                    "Edge suitability",
                    suitability,
                    evidence_label,
                ),
                percentage_metric(
                    "profile_coverage",
                    "Profile coverage",
                    focus.profile_coverage,
                    evidence_label,
                ),
                percentage_metric(
                    "profile_confidence",
                    "Profile confidence",
                    focus.average_profile_confidence,
                    evidence_label,
                ),
            ],
        }),
        CardSectionView::Decision(DecisionSectionView {
            id: "matchup-explanation".to_owned(),
            title: "Why this matchup reads this way".to_owned(),
            recommendation: suitability.map_or_else(
                || "Matchup suitability withheld".to_owned(),
                |value| format!("{team} {value:+.2} suitability"),
            ),
            rationale: vec![
                format!(
                    "Profile offense {:.1}, defense {:.1}, and opponent-style response {:+.1}.",
                    focus.offense_score, focus.defense_score, focus.opponent_style_response
                ),
                format!(
                    "Pair chemistry {:+.2}, trio chemistry {:+.2}, and manager execution {:.1}%.",
                    focus.pair_chemistry_effect,
                    focus.trio_chemistry_effect,
                    focus.manager_execution_confidence * 100.0
                ),
                format!(
                    "Home last-change adjustment {:+.2}; special teams remain separate from the five-on-five feature.",
                    focus.last_change_adjustment
                ),
            ],
            alternatives: Vec::new(),
            action_id: None,
            token: suitability.map_or(SemanticToken::Warning, |value| {
                if value >= 0.0 {
                    SemanticToken::DecisionHighlight
                } else {
                    SemanticToken::Risk
                }
            }),
            evidence_label,
        }),
        lineup_section(focus, evidence_label),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "special-teams-matchup".to_owned(),
            title: Some("Special teams (reported separately)".to_owned()),
            metrics: vec![
                optional_score_metric(
                    "power_play_score",
                    "Power play",
                    focus.special_teams.power_play_score,
                    evidence_label,
                ),
                optional_score_metric(
                    "opponent_penalty_kill_score",
                    "Opponent penalty kill",
                    focus.special_teams.penalty_kill_score,
                    evidence_label,
                ),
                optional_score_metric(
                    "special_teams_suitability",
                    "PP vs PK suitability",
                    focus.special_teams.suitability,
                    evidence_label,
                ),
            ],
        }),
    ];
    if let Some(edge) = edge {
        sections.insert(
            2,
            CardSectionView::MetricStrip(MetricStripSectionView {
                id: "prediction-edge".to_owned(),
                title: Some("Game probability from the sealed prediction edge".to_owned()),
                metrics: vec![
                    percentage_metric(
                        "win_probability",
                        "Win probability",
                        edge.win_probability,
                        EvidenceLabel::Simulated,
                    ),
                    signed_percentage_point_metric(
                        "matchup_probability_delta",
                        "Matchup factor",
                        edge.matchup_probability_delta,
                    ),
                    optional_percentage_metric(
                        "edge_evidence_confidence",
                        "Edge evidence confidence",
                        edge.evidence_confidence,
                    ),
                ],
            }),
        );
    }
    if suitability.is_none() || !focus.warnings.is_empty() {
        sections.push(CardSectionView::StateNotice(StateNoticeSectionView {
            id: "matchup-authority".to_owned(),
            title: if suitability.is_some() {
                "Matchup evidence warnings".to_owned()
            } else {
                "Matchup suitability withheld".to_owned()
            },
            detail: Some(if focus.warnings.is_empty() {
                "The submitted lineup or profile authority is insufficient for a publishable suitability value."
                    .to_owned()
            } else {
                focus.warnings.join(" ")
            }),
            empty_state: None,
            warnings: Vec::new(),
            token: SemanticToken::Warning,
        }));
    }
    sections
}

fn prediction_edge_projection(
    edge: &TeamGamePredictionEdgeView,
    matchup: &PlayerLineMatchupForecastView,
    focus_team: &str,
) -> Result<PredictionEdgeProjection, PlayerLineMatchupCardError> {
    edge.validate()
        .map_err(|error| PlayerLineMatchupCardError::InvalidPredictionEdge(error.to_string()))?;
    let game = edge
        .games
        .iter()
        .find(|game| game.game_id == matchup.game_id)
        .ok_or(PlayerLineMatchupCardError::PredictionEdgeMismatch)?;
    if edge.season != matchup.season
        || edge.vintage != matchup.vintage
        || game.date != matchup.game_date
        || game.away_team != matchup.away.team
        || game.home_team != matchup.home.team
        || game.forecast_at != Some(matchup.forecast_at)
        || game.captured_at != Some(matchup.captured_at)
    {
        return Err(PlayerLineMatchupCardError::PredictionEdgeMismatch);
    }
    if !game.evidence_fingerprints.contains(&matchup.fingerprint) {
        return Err(PlayerLineMatchupCardError::MissingPredictionEdgeJoin);
    }
    let factor = game
        .factors
        .iter()
        .find(|factor| factor.key == "matchup" && factor.available)
        .ok_or(PlayerLineMatchupCardError::MissingPredictionEdgeFactor)?;
    let focus_is_home = focus_team == matchup.home.team;
    Ok(PredictionEdgeProjection {
        fingerprint: edge.fingerprint.clone(),
        generated_at: edge.generated_at,
        win_probability: if focus_is_home {
            game.enhanced_home_win_probability
        } else {
            1.0 - game.enhanced_home_win_probability
        },
        matchup_probability_delta: if focus_is_home {
            factor.home_win_probability_delta
        } else {
            -factor.home_win_probability_delta
        },
        evidence_confidence: game.evidence_confidence,
        model_id: edge.model.model_id.clone(),
        model_version: edge.model.method.clone(),
        training_fingerprint: edge.model.training_fingerprint.clone(),
    })
}

fn lineup_section(
    focus: &PlayerLineMatchupTeamView,
    evidence_label: EvidenceLabel,
) -> CardSectionView {
    let profiles = focus
        .profiles
        .iter()
        .map(|profile| (profile.player_id, profile))
        .collect::<BTreeMap<_, _>>();
    let groups = focus
        .units
        .iter()
        .map(|unit| {
            let (id, label, kind, slot_labels): (&str, String, CardLineupGroupKind, &[&str]) =
                match unit.kind {
                    PlayerLineMatchupUnitKind::ForwardLine => (
                        "forward",
                        format!("Forward line {}", unit.unit),
                        CardLineupGroupKind::ForwardLine,
                        &["LW", "C", "RW"],
                    ),
                    PlayerLineMatchupUnitKind::DefensePair => (
                        "defense",
                        format!("Defense pair {}", unit.unit),
                        CardLineupGroupKind::DefensePair,
                        &["LD", "RD"],
                    ),
                };
            CardLineupGroupView {
                id: format!("{id}-{}", unit.unit),
                label,
                kind,
                slots: unit
                    .player_ids
                    .iter()
                    .zip(slot_labels)
                    .map(|(player_id, slot_label)| {
                        let profile = profiles
                            .get(player_id)
                            .expect("validated matchup unit players have profiles");
                        lineup_slot(slot_label, profile, evidence_label)
                    })
                    .collect(),
            }
        })
        .collect();
    CardSectionView::Lineup(LineupSectionView {
        id: "dressed-units".to_owned(),
        title: format!("{} dressed units", focus.team),
        groups,
    })
}

fn lineup_slot(
    label: &str,
    profile: &PlayerForecastProfileView,
    evidence_label: EvidenceLabel,
) -> CardLineupSlotView {
    CardLineupSlotView {
        id: format!("player-{}-{label}", profile.player_id),
        label: label.to_owned(),
        subject_id: Some(format!("player:{}", profile.player_id)),
        subject_label: Some(profile.display_name.clone()),
        asset_id: profile
            .portrait
            .as_ref()
            .map(|portrait| portrait.asset_id.clone()),
        metrics: vec![
            score_metric(
                "profile_score",
                "Profile",
                profile.reliability_adjusted_score,
                evidence_label,
            ),
            percentage_metric(
                "sample_confidence",
                "Confidence",
                profile.sample_confidence,
                evidence_label,
            ),
        ],
        evidence_label,
    }
}

fn player_assets(
    team: &PlayerLineMatchupTeamView,
    observed_at: DateTime<Utc>,
) -> Vec<CardAssetView> {
    team.profiles
        .iter()
        .filter_map(|profile| {
            let portrait = profile.portrait.as_ref()?;
            let reference = portrait
                .headshot_canonical_url
                .clone()
                .map(CardAssetReference::ExternalUrl);
            Some(CardAssetView {
                id: portrait.asset_id.clone(),
                subject_id: format!("player:{}", profile.player_id),
                kind: CardAssetKind::Headshot,
                state: if reference.is_some() {
                    CardAssetState::Available
                } else {
                    CardAssetState::Missing
                },
                reference,
                source: SourceKind::Roster,
                observed_at: Some(observed_at),
                integrity_sha256: None,
                alt: format!("{} headshot", profile.display_name),
                fallback: CardAssetFallback::Initials(portrait.fallback_initials.clone()),
            })
        })
        .collect()
}

fn insider_sections(
    matchup: &PlayerLineMatchupForecastView,
    focus: &PlayerLineMatchupTeamView,
) -> Vec<CardSectionView> {
    vec![
        CardSectionView::PlayerList(PlayerListSectionView {
            id: "profile-evidence".to_owned(),
            title: format!("{} player profile evidence", focus.team),
            rows: focus
                .profiles
                .iter()
                .map(|profile| CardPlayerRowView {
                    player_id: profile.player_id.to_string(),
                    name: profile.display_name.clone(),
                    role: Some(format!(
                        "{} games · {:.0} EV minutes · {} shifts",
                        profile.games_played,
                        profile.even_strength_minutes,
                        profile.observed_shifts
                    )),
                    asset_id: profile
                        .portrait
                        .as_ref()
                        .map(|portrait| portrait.asset_id.clone()),
                    metrics: vec![
                        score_metric(
                            "profile_score",
                            "Adjusted score",
                            profile.reliability_adjusted_score,
                            EvidenceLabel::Estimated,
                        ),
                        percentage_metric(
                            "component_coverage",
                            "Coverage",
                            profile.component_coverage,
                            EvidenceLabel::Reported,
                        ),
                        percentage_metric(
                            "sample_confidence",
                            "Confidence",
                            profile.sample_confidence,
                            EvidenceLabel::Estimated,
                        ),
                    ],
                    tokens: Vec::new(),
                    evidence_label: EvidenceLabel::Estimated,
                })
                .collect(),
        }),
        CardSectionView::Methodology(MethodologySectionView {
            id: "matchup-methodology".to_owned(),
            title: "How to read The Matchup".to_owned(),
            methods: vec![CardMethodologyItemView {
                key: "profile-line-matchup".to_owned(),
                label: "Dated player and unit matchup".to_owned(),
                version: PLAYER_LINE_MATCHUP_FORECAST_METHOD.to_owned(),
                summary: "IceLines evaluates the submitted dressed units from reliability-adjusted player profiles, separately sealed pair/trio outcomes, opponent style, and bounded manager execution."
                    .to_owned(),
            }],
            limitations: matchup.disclosures.clone(),
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "matchup-sources".to_owned(),
            title: "Sealed matchup source".to_owned(),
            provenance_ids: vec!["sealed-player-line-matchup".to_owned()],
        }),
    ]
}

fn score_metric(
    key: &str,
    label: &str,
    value: f64,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    metric(
        key,
        label,
        MetricValue::Decimal(value),
        MetricUnit::Score,
        ValuePrecision::TwoDecimals,
        format!("{value:.2}"),
        format!("{label}: {value:.2}"),
        evidence_label,
    )
}

fn optional_score_metric(
    key: &str,
    label: &str,
    value: Option<f64>,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    match value {
        Some(value) => score_metric(key, label, value, evidence_label),
        None => metric(
            key,
            label,
            MetricValue::Missing,
            MetricUnit::Score,
            ValuePrecision::TwoDecimals,
            "n/a".to_owned(),
            format!("{label}: unavailable"),
            EvidenceLabel::NoRead,
        ),
    }
}

fn percentage_metric(
    key: &str,
    label: &str,
    value: f64,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let percentage = value * 100.0;
    metric(
        key,
        label,
        MetricValue::Decimal(percentage),
        MetricUnit::Percentage,
        ValuePrecision::PercentOneDecimal,
        format!("{percentage:.1}%"),
        format!("{label}: {percentage:.1} percent"),
        evidence_label,
    )
}

fn optional_percentage_metric(key: &str, label: &str, value: Option<f64>) -> CardMetricView {
    value.map_or_else(
        || {
            metric(
                key,
                label,
                MetricValue::Missing,
                MetricUnit::Percentage,
                ValuePrecision::PercentOneDecimal,
                "n/a".to_owned(),
                format!("{label}: unavailable"),
                EvidenceLabel::NoRead,
            )
        },
        |value| percentage_metric(key, label, value, EvidenceLabel::Simulated),
    )
}

fn signed_percentage_point_metric(key: &str, label: &str, value: f64) -> CardMetricView {
    let percentage_points = value * 100.0;
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Decimal(percentage_points),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::PercentOneDecimal,
            token: Some(if percentage_points >= 0.0 {
                SemanticToken::DecisionHighlight
            } else {
                SemanticToken::Risk
            }),
        },
        display_text: format!("{percentage_points:+.1} pp"),
        accessible_text: format!("{label}: {percentage_points:+.1} percentage points"),
        comparison: Some(CardMetricComparisonView {
            label: "prediction-edge matchup factor contribution".to_owned(),
            baseline: MetricValue::Decimal(0.0),
            delta: MetricValue::Decimal(percentage_points),
        }),
        evidence_label: EvidenceLabel::Simulated,
    }
}

#[allow(clippy::too_many_arguments)]
fn metric(
    key: &str,
    label: &str,
    value: MetricValue,
    unit: MetricUnit,
    precision: ValuePrecision,
    display_text: String,
    accessible_text: String,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value,
            unit,
            precision,
            token: None,
        },
        display_text,
        accessible_text,
        comparison: None,
        evidence_label,
    }
}

fn evidence_label(state: TeamGameEvidenceState) -> EvidenceLabel {
    match state {
        TeamGameEvidenceState::Confirmed => EvidenceLabel::Confirmed,
        TeamGameEvidenceState::Reported => EvidenceLabel::Reported,
        TeamGameEvidenceState::Modeled => EvidenceLabel::Estimated,
        TeamGameEvidenceState::Unavailable => EvidenceLabel::NoRead,
    }
}

fn completeness(state: TeamGameEvidenceState) -> Completeness {
    match state {
        TeamGameEvidenceState::Confirmed => Completeness::Complete,
        TeamGameEvidenceState::Reported | TeamGameEvidenceState::Modeled => Completeness::Partial,
        TeamGameEvidenceState::Unavailable => Completeness::Unavailable,
    }
}

fn vintage_label(vintage: crate::view_model::TeamGameForecastVintage) -> &'static str {
    match vintage {
        crate::view_model::TeamGameForecastVintage::Preseason => "Preseason",
        crate::view_model::TeamGameForecastVintage::GameMorning => "Game morning",
        crate::view_model::TeamGameForecastVintage::PregameConfirmed => "Pregame confirmed",
    }
}

fn strip_sha256(value: &str) -> String {
    value.strip_prefix("sha256:").unwrap_or(value).to_owned()
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;
    use crate::model::{Position, Season};
    use crate::season_stats::SeasonType;
    use crate::view_model::team_lineup::{
        build_team_lineup_projection, LineupAssignmentEvidence, LineupForwardPosition,
        TeamLineupPlayerInput, TeamLineupRequestedSlot,
    };
    use crate::{
        build_player_line_matchup_forecast, build_team_game_forecast,
        build_team_game_prediction_edge, EvidenceLabel, OpponentTacticalStyle,
        PlayerForecastProfileDimensions, PlayerForecastProfileInput,
        PlayerLineMatchupForecastInput, PlayerLineMatchupTeamInput, TeamCeilingLens,
        TeamForecastGameInput, TeamForecastParameters, TeamForecastStrengthInput,
        TeamGamePredictionEvidenceInput, TeamGamePredictionModel, TeamGamePredictionTeamEvidence,
        ViewWindow, PLAYER_FORECAST_PROFILE_SCHEMA,
    };

    fn seal(letter: char) -> String {
        format!("sha256:{}", letter.to_string().repeat(64))
    }

    fn player(
        id: u32,
        team: &str,
        position: Position,
        slot: TeamLineupRequestedSlot,
    ) -> TeamLineupPlayerInput {
        TeamLineupPlayerInput {
            player_id: id,
            display_name: format!("{team} Player {id}"),
            team: team.to_owned(),
            prior_team: None,
            primary_position: position,
            eligible_positions: vec![position],
            headshot_canonical_url: (id % 10 != 1)
                .then(|| format!("https://assets.nhle.com/mugs/nhl/20262027/{team}/{id}.png")),
            games_played: 82,
            lens_scores: TeamCeilingLens::ALL
                .into_iter()
                .map(|lens| (lens, Some(60.0)))
                .collect(),
            score_evidence: EvidenceLabel::Estimated,
            power_play_role_score: Some(60.0),
            penalty_kill_role_score: Some(60.0),
            special_teams_evidence: Some(EvidenceLabel::Estimated),
            requested_slot: Some(slot),
            assignment_evidence: LineupAssignmentEvidence::Scenario,
        }
    }

    fn lineup(team: &str, start: u32) -> crate::TeamLineupProjectionView {
        let mut players = Vec::new();
        let mut id = start;
        for line in 1..=4 {
            for (position, requested) in [
                (Position::LeftWing, LineupForwardPosition::LeftWing),
                (Position::Center, LineupForwardPosition::Center),
                (Position::RightWing, LineupForwardPosition::RightWing),
            ] {
                players.push(player(
                    id,
                    team,
                    position,
                    TeamLineupRequestedSlot::Forward {
                        line,
                        position: requested,
                    },
                ));
                id += 1;
            }
        }
        for pair in 1..=3 {
            for right_side in [false, true] {
                players.push(player(
                    id,
                    team,
                    Position::Defense,
                    TeamLineupRequestedSlot::Defense { pair, right_side },
                ));
                id += 1;
            }
        }
        for starter in [true, false] {
            players.push(player(
                id,
                team,
                Position::Goalie,
                TeamLineupRequestedSlot::Goalie { starter },
            ));
            id += 1;
        }
        build_team_lineup_projection(team, 20262027, players).unwrap()
    }

    fn dimensions(value: f64) -> PlayerForecastProfileDimensions {
        PlayerForecastProfileDimensions {
            scoring_creation: Some(value),
            finishing: Some(value),
            passing_transition: Some(value),
            forecheck_retrieval: Some(value),
            defensive_suppression: Some(value),
            physical_matchup: Some(value),
            discipline_puck_security: Some(value),
            faceoffs: Some(value),
            power_play: Some(value),
            penalty_kill: Some(value),
        }
    }

    fn team_input(
        lineup: crate::TeamLineupProjectionView,
        value: f64,
    ) -> PlayerLineMatchupTeamInput {
        let profiles = lineup
            .forward_lines
            .iter()
            .flat_map(|line| [&line.left_wing, &line.center, &line.right_wing])
            .chain(
                lineup
                    .defense_pairs
                    .iter()
                    .flat_map(|pair| [&pair.left, &pair.right]),
            )
            .flatten()
            .map(|player| PlayerForecastProfileInput {
                schema: PLAYER_FORECAST_PROFILE_SCHEMA.to_owned(),
                player_id: player.player_id,
                team: lineup.team.clone(),
                evidence_cutoff_at: Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap(),
                games_played: 82,
                even_strength_minutes: 984.0,
                observed_shifts: 1476,
                recency: 1.0,
                dimensions: dimensions(value),
                source_fingerprints: vec![seal('a')],
            })
            .collect();
        PlayerLineMatchupTeamInput {
            lineup,
            lineup_state: TeamGameEvidenceState::Reported,
            profiles,
            chemistry: Vec::new(),
            opponent_style: OpponentTacticalStyle::Balanced,
            manager_execution_confidence: 0.5,
            forward_line_shares_pct: None,
            source_fingerprints: vec![seal('b')],
        }
    }

    fn matchup() -> PlayerLineMatchupForecastView {
        build_player_line_matchup_forecast(PlayerLineMatchupForecastInput {
            game_id: 2026020001,
            season: 20262027,
            game_date: NaiveDate::from_ymd_opt(2026, 10, 10).unwrap(),
            vintage: crate::TeamGameForecastVintage::GameMorning,
            forecast_at: Utc.with_ymd_and_hms(2026, 10, 10, 16, 0, 0).unwrap(),
            captured_at: Utc.with_ymd_and_hms(2026, 10, 10, 15, 0, 0).unwrap(),
            away: team_input(lineup("SEA", 101), 50.0),
            home: team_input(lineup("NYR", 1), 60.0),
        })
        .unwrap()
    }

    fn input(matchup: PlayerLineMatchupForecastView) -> PlayerLineMatchupCardInput {
        PlayerLineMatchupCardInput {
            matchup,
            prediction_edge: None,
            focus_team: "NYR".to_owned(),
            team_name: "New York Rangers".to_owned(),
            view: ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular)),
            evidence_at: None,
        }
    }

    fn edge_team(
        team: &PlayerLineMatchupTeamView,
        matchup_fingerprint: &str,
    ) -> TeamGamePredictionTeamEvidence {
        TeamGamePredictionTeamEvidence {
            team: team.team.clone(),
            roster_strength: Some(team.offense_score),
            roster_state: TeamGameEvidenceState::Reported,
            availability_strength: Some(team.defense_score),
            availability_state: TeamGameEvidenceState::Reported,
            lineup_impact: None,
            lineup_impact_state: TeamGameEvidenceState::Unavailable,
            goalie_quality: None,
            goalie_state: TeamGameEvidenceState::Unavailable,
            goalie_player_id: None,
            goalie_form_quality: None,
            goalie_form_appearances: 0,
            goalie_form_state: TeamGameEvidenceState::Unavailable,
            goalie_workload_readiness: None,
            xg_share: None,
            xg_games: 0,
            opponent_adjusted_xg_share: None,
            opponent_adjusted_xg_games: 0,
            special_teams_strength: None,
            special_teams_games: 0,
            matchup_suitability: team.matchup_suitability,
            matchup_state: team.matchup_state,
            source_fingerprints: vec![matchup_fingerprint.to_owned()],
        }
    }

    fn edge(
        matchup: &PlayerLineMatchupForecastView,
        cited_fingerprint: &str,
    ) -> TeamGamePredictionEdgeView {
        let source = build_team_game_forecast(
            matchup.season,
            vec![TeamForecastGameInput {
                game_id: matchup.game_id,
                date: matchup.game_date,
                away_team: matchup.away.team.clone(),
                home_team: matchup.home.team.clone(),
                away_score: None,
                home_score: None,
                final_result: false,
                last_period: None,
            }],
            vec![
                TeamForecastStrengthInput {
                    team: matchup.away.team.clone(),
                    strength: 49.0,
                },
                TeamForecastStrengthInput {
                    team: matchup.home.team.clone(),
                    strength: 53.0,
                },
            ],
            TeamForecastParameters::default(),
            None,
            None,
        )
        .unwrap();
        build_team_game_prediction_edge(
            &source,
            matchup.vintage,
            matchup.forecast_at + chrono::Duration::hours(1),
            TeamGamePredictionModel::evaluation_challenger(),
            vec![TeamGamePredictionEvidenceInput {
                game_id: matchup.game_id,
                forecast_at: matchup.forecast_at,
                captured_at: matchup.captured_at,
                away: edge_team(&matchup.away, cited_fingerprint),
                home: edge_team(&matchup.home, cited_fingerprint),
            }],
        )
        .unwrap()
    }

    #[test]
    fn sealed_matchup_projects_without_recomputing_hockey_values() {
        let matchup = matchup();
        let expected_score = matchup.home.five_on_five_matchup_score;
        let card = build_player_line_matchup_card(input(matchup.clone())).unwrap();

        assert_eq!(card.card_kind, CardKind::PlayerLineMatchup);
        assert_eq!(
            card.context.simulation.parameter_fingerprint,
            Some(strip_sha256(&matchup.fingerprint))
        );
        assert_eq!(card.context.joins.player_ids.len(), 36);
        assert_eq!(card.pages[0].display_label.as_deref(), Some("The Matchup"));
        assert_eq!(card.assets.len(), 18);
        let missing = card
            .assets
            .iter()
            .find(|asset| asset.subject_id == "player:1")
            .expect("missing headshot asset");
        assert_eq!(missing.state, CardAssetState::Missing);
        assert!(missing.reference.is_none());
        assert!(matches!(
            missing.fallback,
            CardAssetFallback::Initials(ref initials) if initials == "NP1"
        ));
        let available = card
            .assets
            .iter()
            .find(|asset| asset.subject_id == "player:2")
            .expect("available headshot asset");
        assert_eq!(available.state, CardAssetState::Available);
        assert!(matches!(
            available.reference,
            Some(CardAssetReference::ExternalUrl(ref url)) if url.ends_with("/2.png")
        ));
        assert!(card
            .pages
            .iter()
            .flat_map(|page| &page.sections)
            .any(|section| matches!(
                section,
                CardSectionView::Lineup(lineup)
                    if lineup.groups.iter().flat_map(|group| &group.slots).any(
                        |slot| slot.asset_id.as_deref() == Some("player:1:headshot")
                    )
            )));
        let headline = match &card.pages[0].sections[1] {
            CardSectionView::MetricStrip(section) => section,
            other => panic!("expected metric strip, got {other:?}"),
        };
        assert_eq!(
            headline.metrics[0].metric.value,
            MetricValue::Decimal(expected_score)
        );
        let json = serde_json::to_string(&card).unwrap();
        assert_eq!(parse_card_document(&json).unwrap(), card);
    }

    #[test]
    fn cited_prediction_edge_projects_probability_without_recomputing_it() {
        let matchup = matchup();
        let edge = edge(&matchup, &matchup.fingerprint);
        let edge_game = &edge.games[0];
        let expected_probability = edge_game.enhanced_home_win_probability;
        let expected_delta = edge_game
            .factors
            .iter()
            .find(|factor| factor.key == "matchup")
            .unwrap()
            .home_win_probability_delta
            * 100.0;
        let expected_model_id = edge.model.model_id.clone();
        let expected_model_version = edge.model.method.clone();
        let expected_training_fingerprint = edge.model.training_fingerprint.clone();
        let edge_generated_at = edge.generated_at;
        let mut input = input(matchup);
        input.prediction_edge = Some(edge);
        input.view.generated_at = Some(edge_generated_at);
        let card = build_player_line_matchup_card(input).unwrap();

        let section = card.pages[0]
            .sections
            .iter()
            .find_map(|section| match section {
                CardSectionView::MetricStrip(section) if section.id == "prediction-edge" => {
                    Some(section)
                }
                _ => None,
            })
            .expect("prediction edge metric strip");
        assert_eq!(
            section.metrics[0].metric.value,
            MetricValue::Decimal(expected_probability * 100.0)
        );
        assert_eq!(
            section.metrics[1].metric.value,
            MetricValue::Decimal(expected_delta)
        );
        assert_eq!(card.provenance.len(), 2);
        assert_eq!(card.context.simulation.model_id, Some(expected_model_id));
        assert_eq!(
            card.context.simulation.model_version,
            Some(expected_model_version)
        );
        assert_eq!(
            card.context.simulation.parameter_fingerprint,
            expected_training_fingerprint
        );
    }

    #[test]
    fn uncited_prediction_edge_is_rejected_before_projection() {
        let matchup = matchup();
        let edge = edge(&matchup, &seal('e'));
        let mut input = input(matchup);
        input.prediction_edge = Some(edge);

        assert_eq!(
            build_player_line_matchup_card(input),
            Err(PlayerLineMatchupCardError::MissingPredictionEdgeJoin)
        );
    }

    #[test]
    fn card_cannot_predate_its_prediction_edge() {
        let matchup = matchup();
        let edge = edge(&matchup, &matchup.fingerprint);
        let mut input = input(matchup.clone());
        input.prediction_edge = Some(edge);
        input.view.generated_at = Some(matchup.forecast_at);

        assert_eq!(
            build_player_line_matchup_card(input),
            Err(PlayerLineMatchupCardError::CardGeneratedBeforePredictionEdge)
        );
    }

    #[test]
    fn away_team_probability_and_matchup_factor_are_inverted() {
        let matchup = matchup();
        let edge = edge(&matchup, &matchup.fingerprint);
        let game = &edge.games[0];
        let expected_probability = (1.0 - game.enhanced_home_win_probability) * 100.0;
        let expected_delta = -game
            .factors
            .iter()
            .find(|factor| factor.key == "matchup")
            .unwrap()
            .home_win_probability_delta
            * 100.0;
        let edge_generated_at = edge.generated_at;
        let mut input = input(matchup);
        input.focus_team = "SEA".to_owned();
        input.team_name = "Seattle Kraken".to_owned();
        input.prediction_edge = Some(edge);
        input.view.generated_at = Some(edge_generated_at);
        let card = build_player_line_matchup_card(input).unwrap();
        let section = card.pages[0]
            .sections
            .iter()
            .find_map(|section| match section {
                CardSectionView::MetricStrip(section) if section.id == "prediction-edge" => {
                    Some(section)
                }
                _ => None,
            })
            .unwrap();

        assert_eq!(
            section.metrics[0].metric.value,
            MetricValue::Decimal(expected_probability)
        );
        assert_eq!(
            section.metrics[1].metric.value,
            MetricValue::Decimal(expected_delta)
        );
    }

    #[test]
    fn invalid_matchup_fingerprint_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.fingerprint = seal('f');

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "player-line matchup fingerprint mismatch"
        ));
    }

    #[test]
    fn mismatched_headshot_identity_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.home.profiles[0]
            .portrait
            .as_mut()
            .expect("generated profile portrait")
            .headshot_canonical_url =
            Some("https://assets.nhle.com/mugs/nhl/20262027/NYR/101.png".to_owned());

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "invalid player-line matchup team scores or unit shape"
        ));
    }

    #[test]
    fn incomplete_unit_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.home.units[0].player_ids.pop();

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "invalid player-line matchup unit/profile shape"
        ));
    }

    #[test]
    fn cross_team_player_id_is_rejected_before_projection() {
        let mut matchup = matchup();
        let duplicate = matchup.home.profiles[0].player_id;
        let replaced = matchup.away.profiles[0].player_id;
        matchup.away.profiles[0].player_id = duplicate;
        if let Some(portrait) = matchup.away.profiles[0].portrait.as_mut() {
            portrait.asset_id = format!("player:{duplicate}:headshot");
            portrait.headshot_canonical_url = Some(format!(
                "https://assets.nhle.com/mugs/nhl/20262027/SEA/{duplicate}.png"
            ));
        }
        for unit in &mut matchup.away.units {
            for player_id in &mut unit.player_ids {
                if *player_id == replaced {
                    *player_id = duplicate;
                }
            }
        }

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "player-line matchup teams cannot share player IDs"
        ));
    }

    #[test]
    fn out_of_range_profile_value_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.home.profiles[0].sample_confidence = 2.0;

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "invalid player-line matchup team scores or unit shape"
        ));
    }

    #[test]
    fn mismatched_special_teams_identity_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.home.special_teams.defending_team = "BOS".to_owned();

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "invalid player-line matchup team scores or unit shape"
        ));
    }

    #[test]
    fn future_profile_evidence_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.home.profiles[0].evidence_cutoff_at =
            matchup.forecast_at + chrono::Duration::hours(1);

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "invalid player-line matchup team scores or unit shape"
        ));
    }

    #[test]
    fn impossible_matchup_evidence_state_is_rejected_before_projection() {
        let mut matchup = matchup();
        matchup.home.matchup_state = TeamGameEvidenceState::Confirmed;

        assert!(matches!(
            build_player_line_matchup_card(input(matchup)),
            Err(PlayerLineMatchupCardError::InvalidMatchup(message))
                if message == "invalid player-line matchup team scores or unit shape"
        ));
    }
}
