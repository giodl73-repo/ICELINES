//! UI-neutral trade card projected from the core fantasy trade evaluation.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    FantasyTradeEvaluationView, FantasyTradePlayerEvaluation, MetricUnit, MetricValue,
    SemanticToken, SourceKind, StatKey, ValuePrecision, ViewContext, ViewWarning, WarningKind,
};

pub const FANTASY_TRADE_CARD_BUILDER_VERSION: &str = "fantasy_trade_card.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyTradeCardInput {
    pub league_id: String,
    pub scoring_scheme_id: String,
    pub sending_team_id: String,
    pub receiving_team_id: String,
    pub offer_id: Option<String>,
    pub evaluated_at: DateTime<Utc>,
    pub evaluation: FantasyTradeEvaluationView,
    pub view: ViewContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FantasyTradeCardError {
    #[error("fantasy trade card requires {0}")]
    MissingText(&'static str),
    #[error("invalid fantasy trade evaluation: {0}")]
    Evaluation(String),
    #[error("serialize fantasy trade evaluation: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_fantasy_trade_card(
    input: FantasyTradeCardInput,
) -> Result<CardDocumentView, FantasyTradeCardError> {
    for (field, value) in [
        ("league ID", input.league_id.as_str()),
        ("scoring scheme ID", input.scoring_scheme_id.as_str()),
        ("sending team ID", input.sending_team_id.as_str()),
        ("receiving team ID", input.receiving_team_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FantasyTradeCardError::MissingText(field));
        }
    }
    input
        .evaluation
        .validate()
        .map_err(FantasyTradeCardError::Evaluation)?;
    let evaluation_fingerprint = json_fingerprint(&input.evaluation)?;
    let warnings = trade_warnings(&input.evaluation);
    let completeness = input.view.completeness;
    let evidence_label = if input.evaluation.sending_team_result.legal
        && input.evaluation.receiving_team_result.legal
    {
        EvidenceLabel::Estimated
    } else {
        EvidenceLabel::UnderReview
    };
    let mut methods = BTreeMap::new();
    methods.insert(
        "trade_evaluation".to_string(),
        input.evaluation.schema.clone(),
    );

    let mut trade_sections = vec![
        CardSectionView::IdentityHeader(identity_section(&input)),
        CardSectionView::Decision(decision_section(&input.evaluation)),
        CardSectionView::MetricStrip(package_metrics(&input.evaluation)),
        CardSectionView::PlayerList(package_players(
            "sending-package",
            &format!("{} sends", input.evaluation.sending_team),
            &input.evaluation.sends,
        )),
        CardSectionView::PlayerList(package_players(
            "receiving-package",
            &format!("{} sends", input.evaluation.receiving_team),
            &input.evaluation.receives,
        )),
    ];
    if !warnings.is_empty() {
        trade_sections.push(warning_section("trade-board-warnings", &warnings));
    }

    let mut insider_sections = vec![
        CardSectionView::PlayerList(team_results(&input.evaluation)),
        CardSectionView::Methodology(MethodologySectionView {
            id: "fantasy-trade-methodology".to_string(),
            title: "Methodology".to_string(),
            methods: vec![CardMethodologyItemView {
                key: "trade-evaluation".to_string(),
                label: "Trade evaluation".to_string(),
                version: input.evaluation.schema.clone(),
                summary: "League scoring, remaining schedule value, roster capacity, and active-slot coverage are evaluated for both teams before a recommendation is issued.".to_string(),
            }],
            limitations: vec![
                "The card is advisory and does not execute, accept, reject, or save an offer.".to_string(),
                "Injury, role, schedule, and playoff context should be refreshed before either manager accepts.".to_string(),
            ],
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "fantasy-trade-source".to_string(),
            title: "Source authority".to_string(),
            provenance_ids: vec!["fantasy-trade-evaluation".to_string()],
        }),
    ];
    if !warnings.is_empty() {
        insider_sections.insert(1, warning_section("trade-insider-warnings", &warnings));
    }

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::FantasyTrade,
        document_id: format!(
            "fantasy-trade:{}:{}:{}",
            stable_id(&input.league_id),
            stable_id(&input.sending_team_id),
            stable_id(&input.receiving_team_id)
        ),
        fingerprint: String::new(),
        title: format!(
            "{} ↔ {} trade board",
            input.evaluation.sending_team, input.evaluation.receiving_team
        ),
        subtitle: Some(format!(
            "{} · {} · evaluated {}",
            input.evaluation.league,
            input.evaluation.scoring_scheme,
            input.evaluated_at.to_rfc3339()
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: Some(input.evaluated_at),
            evidence_label,
            builder_version: FANTASY_TRADE_CARD_BUILDER_VERSION.to_string(),
            methodology_versions: methods,
            joins: CardIdentityJoinsView {
                league_id: Some(input.league_id),
                scoring_scheme_id: Some(input.scoring_scheme_id),
                scenario_id: input.offer_id.clone(),
                team_ids: vec![input.sending_team_id, input.receiving_team_id],
                player_ids: trade_player_ids(&input.evaluation),
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView::default(),
        },
        theme: CardThemeView {
            theme_key: "fantasy-trade".to_string(),
            primary: Some("#17324D".to_string()),
            secondary: Some("#E7EEF5".to_string()),
            accent: Some("#E09F3E".to_string()),
            surface: Some("#FFFFFF".to_string()),
            text: Some("#102A43".to_string()),
            team_abbreviation: None,
            ascii_identity: "Trade Board".to_string(),
            minimum_text_contrast_x100: 450,
        },
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "trade-board".to_string(),
                literal_label: "Trade recommendation and packages".to_string(),
                display_label: Some("The Trade Board".to_string()),
                order: 1,
                accessible_summary: "Recommendation, fairness gap, legality, and players exchanged.".to_string(),
                sections: trade_sections,
            },
            CardPageView {
                id: "trade-insider".to_string(),
                literal_label: "Trade impact and evidence".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary: "Before-and-after value, schedule, roster coverage, warnings, and methodology for both teams.".to_string(),
                sections: insider_sections,
            },
        ],
        assets: Vec::new(),
        provenance: vec![CardProvenanceView {
            id: "fantasy-trade-evaluation".to_string(),
            source: SourceKind::FantasyImport,
            label: "League roster, scoring, eligibility, and remaining-schedule trade evaluation".to_string(),
            state: completeness,
            observed_at: Some(input.evaluated_at),
            fingerprint: Some(evaluation_fingerprint),
            note: input.offer_id.map(|offer| format!("Pending offer {offer}")),
        }],
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| FantasyTradeCardError::Document(error.to_string()))
}

fn identity_section(input: &FantasyTradeCardInput) -> IdentityHeaderSectionView {
    IdentityHeaderSectionView {
        id: "fantasy-trade-identity".to_string(),
        eyebrow: Some(input.evaluation.league.clone()),
        title: format!(
            "{} ↔ {}",
            input.evaluation.sending_team, input.evaluation.receiving_team
        ),
        subtitle: Some(input.evaluation.scoring_scheme.clone()),
        identities: vec![
            CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: input.sending_team_id.clone(),
                label: input.evaluation.sending_team.clone(),
                asset_id: None,
            },
            CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: input.receiving_team_id.clone(),
                label: input.evaluation.receiving_team.clone(),
                asset_id: None,
            },
        ],
    }
}

fn decision_section(evaluation: &FantasyTradeEvaluationView) -> DecisionSectionView {
    DecisionSectionView {
        id: "trade-recommendation".to_string(),
        title: "Trade call".to_string(),
        recommendation: evaluation.recommendation.clone(),
        rationale: vec![
            format!(
                "Package value gap: {:+.1}%",
                evaluation.package_value_gap_percent
            ),
            format!(
                "Roster legality: {} / {}",
                legality(evaluation.sending_team_result.legal),
                legality(evaluation.receiving_team_result.legal)
            ),
        ],
        alternatives: Vec::new(),
        action_id: None,
        token: if evaluation.sending_team_result.legal && evaluation.receiving_team_result.legal {
            SemanticToken::PrimaryAction
        } else {
            SemanticToken::Risk
        },
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn package_metrics(evaluation: &FantasyTradeEvaluationView) -> MetricStripSectionView {
    MetricStripSectionView {
        id: "trade-package-balance".to_string(),
        title: Some("Package balance".to_string()),
        metrics: vec![
            decimal_metric(
                "package_value_gap",
                "Value gap",
                evaluation.package_value_gap,
                MetricUnit::Score,
            ),
            decimal_metric(
                "package_value_gap_percent",
                "Value gap percent",
                evaluation.package_value_gap_percent,
                MetricUnit::Percentage,
            ),
            text_metric(
                "sending_legal",
                &format!("{} legal", evaluation.sending_team),
                legality(evaluation.sending_team_result.legal),
            ),
            text_metric(
                "receiving_legal",
                &format!("{} legal", evaluation.receiving_team),
                legality(evaluation.receiving_team_result.legal),
            ),
        ],
    }
}

fn package_players(
    id: &str,
    title: &str,
    players: &[FantasyTradePlayerEvaluation],
) -> PlayerListSectionView {
    PlayerListSectionView {
        id: id.to_string(),
        title: title.to_string(),
        rows: players
            .iter()
            .map(|player| CardPlayerRowView {
                player_id: player.player_key.clone(),
                name: player.player.clone(),
                role: Some(format!(
                    "{} · {}",
                    player.nhl_team,
                    player
                        .positions
                        .iter()
                        .map(|position| position.abbreviation())
                        .collect::<Vec<_>>()
                        .join("/")
                )),
                asset_id: None,
                metrics: vec![
                    decimal_metric(
                        "projected_remaining_value",
                        "Remaining value",
                        player.projected_remaining_value,
                        MetricUnit::Score,
                    ),
                    decimal_metric(
                        "league_value_per_game",
                        "Value/game",
                        player.league_value_per_game,
                        MetricUnit::Score,
                    ),
                    integer_metric(
                        "remaining_games",
                        "Games left",
                        i64::from(player.remaining_games),
                    ),
                ],
                tokens: vec![SemanticToken::SupportingEvidence],
                evidence_label: EvidenceLabel::Estimated,
            })
            .collect(),
    }
}

fn team_results(evaluation: &FantasyTradeEvaluationView) -> PlayerListSectionView {
    PlayerListSectionView {
        id: "trade-team-impacts".to_string(),
        title: "Before and after".to_string(),
        rows: [
            &evaluation.sending_team_result,
            &evaluation.receiving_team_result,
        ]
        .into_iter()
        .map(|team| CardPlayerRowView {
            player_id: stable_id(&team.team),
            name: team.team.clone(),
            role: Some(format!(
                "Roster {}/{} · {}",
                team.roster_size_after,
                team.standard_capacity,
                legality(team.legal)
            )),
            asset_id: None,
            metrics: vec![
                decimal_metric(
                    "before_value",
                    "Before",
                    team.before_value,
                    MetricUnit::Score,
                ),
                decimal_metric("after_value", "After", team.after_value, MetricUnit::Score),
                decimal_metric(
                    "value_delta",
                    "Value delta",
                    team.value_delta,
                    MetricUnit::Score,
                ),
                integer_metric(
                    "remaining_games_delta",
                    "Games delta",
                    i64::from(team.remaining_games_delta),
                ),
                integer_metric(
                    "missing_slots_before",
                    "Open slots before",
                    team.missing_active_slots_before as i64,
                ),
                integer_metric(
                    "missing_slots_after",
                    "Open slots after",
                    team.missing_active_slots_after as i64,
                ),
            ],
            tokens: vec![if team.legal {
                SemanticToken::SupportingEvidence
            } else {
                SemanticToken::Risk
            }],
            evidence_label: EvidenceLabel::Estimated,
        })
        .collect(),
    }
}

fn trade_player_ids(evaluation: &FantasyTradeEvaluationView) -> Vec<String> {
    evaluation
        .sends
        .iter()
        .chain(&evaluation.receives)
        .map(|player| player.player_key.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn trade_warnings(evaluation: &FantasyTradeEvaluationView) -> Vec<ViewWarning> {
    evaluation
        .warnings
        .iter()
        .cloned()
        .map(|message| ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::FantasyImport),
            message,
            recovery: Vec::new(),
        })
        .collect()
}

fn warning_section(id: &str, warnings: &[ViewWarning]) -> CardSectionView {
    CardSectionView::StateNotice(StateNoticeSectionView {
        id: id.to_string(),
        title: "Trade evidence warnings".to_string(),
        detail: Some(
            "Refresh roster, injury, role, and schedule evidence before accepting.".to_string(),
        ),
        empty_state: None,
        warnings: warnings.to_vec(),
        token: SemanticToken::Warning,
    })
}

fn legality(legal: bool) -> &'static str {
    if legal {
        "Legal"
    } else {
        "Illegal"
    }
}

fn integer_metric(key: &str, label: &str, value: i64) -> CardMetricView {
    metric(
        key,
        label,
        MetricValue::Integer(value),
        value.to_string(),
        MetricUnit::Count,
        ValuePrecision::Integer,
    )
}

fn decimal_metric(key: &str, label: &str, value: f64, unit: MetricUnit) -> CardMetricView {
    metric(
        key,
        label,
        MetricValue::Decimal(value),
        format!("{value:+.2}"),
        unit,
        ValuePrecision::TwoDecimals,
    )
}

fn text_metric(key: &str, label: &str, value: &str) -> CardMetricView {
    metric(
        key,
        label,
        MetricValue::Text(value.to_string()),
        value.to_string(),
        MetricUnit::None,
        ValuePrecision::Raw,
    )
}

fn metric(
    key: &str,
    label: &str,
    value: MetricValue,
    display_text: String,
    unit: MetricUnit,
    precision: ValuePrecision,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value,
            unit,
            precision,
            token: None,
        },
        accessible_text: format!("{label} {display_text}"),
        display_text,
        comparison: None,
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn stable_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, FantasyTradeCardError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| FantasyTradeCardError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::{
        model::{Position, Season},
        season_stats::SeasonType,
        view_model::{FantasyTradeTeamEvaluation, ViewWindow, FANTASY_TRADE_EVALUATION_SCHEMA},
    };

    fn player(
        key: &str,
        name: &str,
        team: &str,
        position: Position,
        value: f64,
    ) -> FantasyTradePlayerEvaluation {
        FantasyTradePlayerEvaluation {
            player_key: key.to_string(),
            player: name.to_string(),
            nhl_team: team.to_string(),
            positions: vec![position],
            league_value: value,
            league_value_per_game: value / 82.0,
            remaining_games: 40,
            projected_remaining_value: value / 82.0 * 40.0,
        }
    }

    fn team(name: &str, before: f64, after: f64) -> FantasyTradeTeamEvaluation {
        FantasyTradeTeamEvaluation {
            team: name.to_string(),
            before_value: before,
            after_value: after,
            value_delta: after - before,
            remaining_games_delta: 0,
            roster_size_after: 16,
            standard_capacity: 16,
            missing_active_slots_before: 0,
            missing_active_slots_after: 0,
            legal: true,
        }
    }

    #[test]
    fn trade_card_seals_packages_legality_and_both_team_deltas() {
        let evaluated_at = Utc.with_ymd_and_hms(2026, 11, 12, 15, 0, 0).unwrap();
        let evaluation = FantasyTradeEvaluationView {
            schema: FANTASY_TRADE_EVALUATION_SCHEMA.to_string(),
            executed: false,
            saved_offer_id: None,
            league: "Dexter's League".to_string(),
            scoring_scheme: "Dexter's Dawgs".to_string(),
            sending_team: "Dexter's Dawgs".to_string(),
            receiving_team: "Blue Line Bandits".to_string(),
            sends: vec![player(
                "adam-fox",
                "Adam Fox",
                "NYR",
                Position::Defense,
                320.0,
            )],
            receives: vec![player(
                "mikko-rantanen",
                "Mikko Rantanen",
                "DAL",
                Position::RightWing,
                330.0,
            )],
            sending_team_result: team("Dexter's Dawgs", 4_000.0, 4_010.0),
            receiving_team_result: team("Blue Line Bandits", 3_900.0, 3_890.0),
            package_value_gap: 4.88,
            package_value_gap_percent: 3.13,
            recommendation:
                "Reasonable offer range; decide on positional need, schedule fit, and injury risk"
                    .to_string(),
            warnings: vec!["Deterministic test inputs, not current trade advice.".to_string()],
        };
        let card = build_fantasy_trade_card(FantasyTradeCardInput {
            league_id: "league-1".to_string(),
            scoring_scheme_id: "dexters-dawgs".to_string(),
            sending_team_id: "dexters-dawgs".to_string(),
            receiving_team_id: "blue-line-bandits".to_string(),
            offer_id: None,
            evaluated_at,
            evaluation,
            view: ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular)),
        })
        .unwrap();

        assert_eq!(card.card_kind, CardKind::FantasyTrade);
        assert_eq!(card.pages.len(), 2);
        assert_eq!(
            card.context.joins.player_ids,
            ["adam-fox", "mikko-rantanen"]
        );
        assert!(card.pages[0]
            .sections
            .iter()
            .any(|section| matches!(section, CardSectionView::Decision(_))));
        assert!(card.pages[1].sections.iter().any(|section| matches!(section, CardSectionView::PlayerList(players) if players.id == "trade-team-impacts")));
        assert!(!card.fingerprint.is_empty());
    }
}
