//! Core-owned fantasy draft card projected from `FantasyDraftBoardView`.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    FantasyDraftBoardView, MetricUnit, MetricValue, SemanticToken, SourceKind, StatKey,
    ValuePrecision, ViewContext, ViewWarning, WarningKind,
};

pub const FANTASY_DRAFT_CARD_BUILDER_VERSION: &str = "fantasy_draft_card.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyDraftCardInput {
    pub league_id: String,
    pub league_name: String,
    pub fantasy_team_id: String,
    pub fantasy_team_name: String,
    pub roster_snapshot_id: Option<String>,
    pub calendar_fingerprint: Option<String>,
    pub board: FantasyDraftBoardView,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FantasyDraftCardError {
    #[error("fantasy draft card requires {0}")]
    MissingText(&'static str),
    #[error("fantasy draft card requires at least one available recommendation")]
    EmptyBoard,
    #[error("serialize fantasy draft board: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_fantasy_draft_card(
    input: FantasyDraftCardInput,
) -> Result<CardDocumentView, FantasyDraftCardError> {
    for (field, value) in [
        ("league ID", input.league_id.as_str()),
        ("league name", input.league_name.as_str()),
        ("fantasy team ID", input.fantasy_team_id.as_str()),
        ("fantasy team name", input.fantasy_team_name.as_str()),
        ("scoring scheme", input.board.scoring_scheme.as_str()),
        ("scoring season", input.board.scoring_season.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FantasyDraftCardError::MissingText(field));
        }
    }
    let top = input
        .board
        .rows
        .first()
        .ok_or(FantasyDraftCardError::EmptyBoard)?;
    let board_fingerprint = json_fingerprint(&input.board)?;
    let warnings = draft_warnings(&input.board);
    let player_ids = draft_player_ids(&input.board);
    let completeness = input.view.completeness;
    let open_slot_text = if input.board.open_slots.is_empty() {
        "Bench / best value".to_string()
    } else {
        input
            .board
            .open_slots
            .iter()
            .map(|slot| slot.slot_id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let mut methods = BTreeMap::new();
    methods.insert("draft_ranking".to_string(), input.board.schema.clone());
    methods.insert(
        "taken_import".to_string(),
        input.board.taken_import.schema.clone(),
    );
    if let Some(import) = &input.board.eligibility_import {
        methods.insert("eligibility_import".to_string(), import.schema.clone());
    }

    let mut board_sections = vec![
        identity_section(&input),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "draft-state".to_string(),
            title: Some("Draft state".to_string()),
            metrics: vec![
                integer_metric(
                    "available_players",
                    "Available",
                    input.board.available_players as i64,
                ),
                integer_metric(
                    "excluded_taken_players",
                    "Taken",
                    input.board.excluded_taken_players as i64,
                ),
                integer_metric(
                    "open_starter_slots",
                    "Open starters",
                    input.board.open_slots.len() as i64,
                ),
                text_metric("open_slots", "Priority slots", &open_slot_text),
            ],
        }),
        CardSectionView::Decision(recommendation_section(&input.board)),
        CardSectionView::PlayerList(PlayerListSectionView {
            id: "best-available".to_string(),
            title: "Best available".to_string(),
            rows: input.board.rows.iter().map(candidate_row).collect(),
        }),
    ];
    if !warnings.is_empty() {
        board_sections.push(warning_section("draft-warnings", &warnings));
    }

    let mut insider_sections = vec![
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "top-pick-components".to_string(),
            title: Some(format!("Why {} ranks first", top.player)),
            metrics: component_metrics(top),
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "import-state".to_string(),
            title: Some("Imported draft state".to_string()),
            metrics: import_metrics(&input.board),
        }),
    ];
    if !warnings.is_empty() {
        insider_sections.push(warning_section("draft-insider-warnings", &warnings));
    }
    insider_sections.extend([
        CardSectionView::Methodology(MethodologySectionView {
            id: "fantasy-draft-methodology".to_string(),
            title: "Methodology".to_string(),
            methods: vec![
                CardMethodologyItemView {
                    key: "draft-value".to_string(),
                    label: "Draft value".to_string(),
                    version: input.board.schema.clone(),
                    summary: "League-scored quality is adjusted for starter gaps, replacement scarcity, multi-position flexibility, usable starts, quiet slates, schedule collisions, playoff fit, and risk.".to_string(),
                },
                CardMethodologyItemView {
                    key: "draft-state".to_string(),
                    label: "Availability authority".to_string(),
                    version: input.board.taken_import.schema.clone(),
                    summary: "Pasted taken players and platform eligibility are resolved before IceLines ranks the remaining pool.".to_string(),
                },
            ],
            limitations: vec![
                "Recommendations are advisory and do not submit a fantasy draft pick.".to_string(),
                "Refresh the taken-player list, injuries, roles, and platform eligibility immediately before the pick.".to_string(),
            ],
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "fantasy-draft-sources".to_string(),
            title: "Source authority".to_string(),
            provenance_ids: vec!["fantasy-draft-board".to_string()],
        }),
    ]);

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::FantasyDraft,
        document_id: format!(
            "fantasy-draft:{}:{}:{}",
            stable_id(&input.league_id),
            stable_id(&input.fantasy_team_id),
            stable_id(&input.board.scoring_season)
        ),
        fingerprint: String::new(),
        title: format!("{} — draft board", input.fantasy_team_name.trim()),
        subtitle: Some(format!(
            "{} · {} scoring · {} stats",
            input.league_name.trim(),
            input.board.scoring_scheme.trim(),
            input.board.scoring_season.trim()
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: input.evidence_at,
            evidence_label: EvidenceLabel::Estimated,
            builder_version: FANTASY_DRAFT_CARD_BUILDER_VERSION.to_string(),
            methodology_versions: methods,
            joins: CardIdentityJoinsView {
                league_id: Some(input.league_id),
                roster_snapshot_id: input.roster_snapshot_id,
                calendar_fingerprint: input.calendar_fingerprint,
                scoring_scheme_id: Some(input.board.scoring_scheme.clone()),
                team_ids: vec![input.fantasy_team_id.clone()],
                player_ids,
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView::default(),
        },
        theme: CardThemeView {
            theme_key: "fantasy-draft".to_string(),
            primary: Some("#12355B".to_string()),
            secondary: Some("#DCEAF5".to_string()),
            accent: Some("#F4A261".to_string()),
            surface: Some("#FFFFFF".to_string()),
            text: Some("#102A43".to_string()),
            team_abbreviation: None,
            ascii_identity: input.fantasy_team_name.trim().to_string(),
            minimum_text_contrast_x100: 450,
        },
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "draft-board".to_string(),
                literal_label: "Draft recommendations".to_string(),
                display_label: Some("The Draft Board".to_string()),
                order: 1,
                accessible_summary: format!(
                    "Best available players and roster-fit recommendation for {}.",
                    input.fantasy_team_name.trim()
                ),
                sections: board_sections,
            },
            CardPageView {
                id: "draft-insider".to_string(),
                literal_label: "Draft value evidence".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary:
                    "Top-pick value components, imported draft state, warnings, and methodology."
                        .to_string(),
                sections: insider_sections,
            },
        ],
        assets: Vec::new(),
        provenance: vec![CardProvenanceView {
            id: "fantasy-draft-board".to_string(),
            source: SourceKind::FantasyImport,
            label: "Scored player pool, roster gaps, taken list, eligibility, and schedule fit"
                .to_string(),
            state: completeness,
            observed_at: input.evidence_at,
            fingerprint: Some(board_fingerprint),
            note: Some(format!(
                "{} available players after excluding {} taken players",
                input.board.available_players, input.board.excluded_taken_players
            )),
        }],
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| FantasyDraftCardError::Document(error.to_string()))
}

fn identity_section(input: &FantasyDraftCardInput) -> CardSectionView {
    CardSectionView::IdentityHeader(IdentityHeaderSectionView {
        id: "fantasy-draft-identity".to_string(),
        eyebrow: Some(input.league_name.trim().to_string()),
        title: input.fantasy_team_name.trim().to_string(),
        subtitle: Some("On the clock".to_string()),
        identities: vec![CardIdentityView {
            kind: CardIdentityKind::Team,
            subject_id: format!("fantasy-team:{}", stable_id(&input.fantasy_team_id)),
            label: input.fantasy_team_name.trim().to_string(),
            asset_id: None,
        }],
    })
}

fn recommendation_section(board: &FantasyDraftBoardView) -> DecisionSectionView {
    let top = &board.rows[0];
    let mut alternatives = board
        .position_leaders
        .iter()
        .map(|leader| CardDecisionAlternativeView {
            id: format!("position-{}", stable_id(leader.slot_kind.label())),
            label: format!("{}: {}", leader.slot_kind.label(), leader.player),
            detail: Some(format!("Draft value {:.1}", leader.draft_value)),
        })
        .collect::<Vec<_>>();
    if let Some(fallback) = &board.fallback_pick {
        alternatives.push(CardDecisionAlternativeView {
            id: "fallback-pick".to_string(),
            label: format!("Fallback: {}", fallback.player),
            detail: Some(format!("Draft value {:.1}", fallback.draft_value)),
        });
    }
    DecisionSectionView {
        id: "next-pick".to_string(),
        title: "The pick".to_string(),
        recommendation: format!("Draft {} — {:.1} draft value", top.player, top.draft_value),
        rationale: top.reasons.clone(),
        alternatives,
        action_id: None,
        token: SemanticToken::DecisionHighlight,
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn candidate_row(row: &crate::view_model::FantasyDraftCandidateRow) -> CardPlayerRowView {
    let positions = row
        .platform_positions
        .iter()
        .map(|position| position.abbreviation())
        .collect::<Vec<_>>()
        .join("/");
    let slot = row.best_open_slot.map_or_else(
        || "best value".to_string(),
        |slot| format!("fills {}", slot.label()),
    );
    CardPlayerRowView {
        player_id: row.player_key.clone(),
        name: row.player.clone(),
        role: Some(format!(
            "#{} · {} · {} · {}",
            row.rank, row.nhl_team, positions, slot
        )),
        asset_id: None,
        metrics: vec![
            decimal_metric(
                "draft_value",
                "Draft value",
                row.draft_value,
                MetricUnit::Score,
            ),
            decimal_metric(
                "league_scored_quality",
                "League quality",
                row.components.league_scored_quality,
                MetricUnit::Score,
            ),
            decimal_metric(
                "incremental_usable_starts",
                "Usable-start value",
                row.components.incremental_usable_starts,
                MetricUnit::Score,
            ),
        ],
        tokens: if row.rank == 1 {
            vec![SemanticToken::DecisionHighlight]
        } else {
            vec![SemanticToken::SupportingEvidence]
        },
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn component_metrics(row: &crate::view_model::FantasyDraftCandidateRow) -> Vec<CardMetricView> {
    let c = &row.components;
    [
        ("quality", "League quality", c.league_scored_quality),
        ("starter_gap", "Starter gap", c.starter_gap_value),
        ("scarcity", "Scarcity", c.positional_scarcity),
        (
            "flexibility",
            "Position flexibility",
            c.multi_position_flexibility,
        ),
        (
            "usable_starts",
            "Usable starts",
            c.incremental_usable_starts,
        ),
        ("quiet_slate", "Quiet slates", c.quiet_slate_value),
        (
            "schedule_diversity",
            "Schedule diversity",
            c.schedule_diversity,
        ),
        ("playoff_fit", "Playoff fit", c.playoff_fit_value),
        ("collision_cost", "Collision cost", -c.collision_cost),
        ("risk_penalty", "Risk penalty", -c.risk_penalty),
    ]
    .into_iter()
    .map(|(key, label, value)| decimal_metric(key, label, value, MetricUnit::Score))
    .collect()
}

fn import_metrics(board: &FantasyDraftBoardView) -> Vec<CardMetricView> {
    let mut metrics = vec![
        integer_metric(
            "taken_matched",
            "Taken matched",
            board.taken_import.matched as i64,
        ),
        integer_metric(
            "taken_ambiguous",
            "Taken ambiguous",
            board.taken_import.ambiguous as i64,
        ),
        integer_metric(
            "taken_unresolved",
            "Taken unresolved",
            board.taken_import.unresolved as i64,
        ),
    ];
    if let Some(import) = &board.eligibility_import {
        metrics.extend([
            integer_metric(
                "eligibility_imported",
                "Eligibility imported",
                import.imported as i64,
            ),
            integer_metric(
                "eligibility_unresolved",
                "Eligibility unresolved",
                import.unresolved as i64,
            ),
        ]);
    } else {
        metrics.push(text_metric(
            "eligibility_state",
            "Eligibility",
            "Canonical NHL positions",
        ));
    }
    metrics
}

fn warning_section(id: &str, warnings: &[ViewWarning]) -> CardSectionView {
    CardSectionView::StateNotice(StateNoticeSectionView {
        id: id.to_string(),
        title: "Draft evidence warnings".to_string(),
        detail: Some("Resolve changing draft-room state before acting.".to_string()),
        empty_state: None,
        warnings: warnings.to_vec(),
        token: SemanticToken::Warning,
    })
}

fn draft_warnings(board: &FantasyDraftBoardView) -> Vec<ViewWarning> {
    let mut warnings = board
        .warnings
        .iter()
        .map(|message| ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::FantasyImport),
            message: message.clone(),
            recovery: Vec::new(),
        })
        .collect::<Vec<_>>();
    if board.taken_import.ambiguous > 0 || board.taken_import.unresolved > 0 {
        warnings.push(ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::FantasyImport),
            message: format!(
                "Taken list still has {} ambiguous and {} unresolved row(s); refresh it before the pick.",
                board.taken_import.ambiguous, board.taken_import.unresolved
            ),
            recovery: Vec::new(),
        });
    }
    warnings.sort_by(|a, b| a.message.cmp(&b.message));
    warnings.dedup_by(|a, b| a.message == b.message);
    warnings
}

fn draft_player_ids(board: &FantasyDraftBoardView) -> Vec<String> {
    let mut ids = board
        .rows
        .iter()
        .map(|row| row.player_key.clone())
        .chain(
            board
                .position_leaders
                .iter()
                .map(|row| row.player_key.clone()),
        )
        .chain(board.fallback_pick.iter().map(|row| row.player_key.clone()))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    ids.sort();
    ids
}

fn integer_metric(key: &str, label: &str, value: i64) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Integer(value),
            unit: MetricUnit::Count,
            precision: ValuePrecision::Integer,
            token: None,
        },
        display_text: value.to_string(),
        accessible_text: format!("{label} {value}"),
        comparison: None,
        evidence_label: EvidenceLabel::Confirmed,
    }
}

fn decimal_metric(key: &str, label: &str, value: f64, unit: MetricUnit) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(value),
            unit,
            precision: ValuePrecision::OneDecimal,
            token: None,
        },
        display_text: format!("{value:.1}"),
        accessible_text: format!("{label} {value:.1}"),
        comparison: None,
        evidence_label: EvidenceLabel::Estimated,
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
        evidence_label: EvidenceLabel::Confirmed,
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

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, FantasyDraftCardError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| FantasyDraftCardError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::model::{Position, Season};
    use crate::season_stats::SeasonType;
    use crate::view_model::{
        build_fantasy_draft_board, import_fantasy_taken_players, FantasyActiveSlot,
        FantasyActiveSlotKind, FantasyDraftCandidateInput, FantasyDraftIdentityInput, ViewWindow,
    };

    fn candidate(
        key: &str,
        player: &str,
        team: &str,
        positions: Vec<Position>,
        quality: f64,
        collision: f64,
    ) -> FantasyDraftCandidateInput {
        FantasyDraftCandidateInput {
            player_key: key.to_string(),
            player: player.to_string(),
            nhl_team: team.to_string(),
            platform_positions: positions,
            league_scored_quality: quality,
            replacement_level: 45.0,
            incremental_usable_starts: 18.0,
            quiet_slate_games: 7.0,
            schedule_collision_rate: collision,
            playoff_incremental_usable_starts: 2.0,
            playoff_usable_value_delta: 4.0,
            risk_penalty: 1.0,
        }
    }

    fn input() -> FantasyDraftCardInput {
        let identities = [
            FantasyDraftIdentityInput {
                player_key: "flex-wing".to_string(),
                display_name: "Flex Wing".to_string(),
                aliases: Vec::new(),
            },
            FantasyDraftIdentityInput {
                player_key: "top-defense".to_string(),
                display_name: "Top Defense".to_string(),
                aliases: Vec::new(),
            },
            FantasyDraftIdentityInput {
                player_key: "goalie-option".to_string(),
                display_name: "Goalie Option".to_string(),
                aliases: Vec::new(),
            },
        ];
        let taken = import_fantasy_taken_players("", &identities).unwrap();
        let mut board = build_fantasy_draft_board(
            "dexters-dawgs",
            "20252026",
            vec![
                FantasyActiveSlot {
                    slot_id: "LW1".to_string(),
                    kind: FantasyActiveSlotKind::LeftWing,
                },
                FantasyActiveSlot {
                    slot_id: "D1".to_string(),
                    kind: FantasyActiveSlotKind::Defense,
                },
                FantasyActiveSlot {
                    slot_id: "G1".to_string(),
                    kind: FantasyActiveSlotKind::Goalie,
                },
            ],
            vec![
                candidate(
                    "flex-wing",
                    "Flex Wing",
                    "WSH",
                    vec![Position::Center, Position::LeftWing],
                    74.0,
                    0.28,
                ),
                candidate(
                    "top-defense",
                    "Top Defense",
                    "SEA",
                    vec![Position::Defense],
                    68.0,
                    0.44,
                ),
                candidate(
                    "goalie-option",
                    "Goalie Option",
                    "NYR",
                    vec![Position::Goalie],
                    66.0,
                    0.51,
                ),
            ],
            taken,
            10,
        )
        .unwrap();
        board
            .warnings
            .push("Refresh injury and role evidence before drafting.".to_string());
        let timestamp = Utc.with_ymd_and_hms(2026, 9, 30, 19, 0, 0).unwrap();
        let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
        view.generated_at = Some(timestamp);
        FantasyDraftCardInput {
            league_id: "dexters-league".to_string(),
            league_name: "Dexter's 2026-27 League".to_string(),
            fantasy_team_id: "dexters-dawgs".to_string(),
            fantasy_team_name: "Dexter's Dawgs".to_string(),
            roster_snapshot_id: Some("draft-pick-7".to_string()),
            calendar_fingerprint: Some("calendar-fixture".to_string()),
            board,
            view,
            evidence_at: Some(timestamp),
        }
    }

    #[test]
    fn fantasy_draft_card_preserves_pick_gaps_flexibility_and_schedule_fit() {
        let card = build_fantasy_draft_card(input()).unwrap();
        assert_eq!(card.card_kind, CardKind::FantasyDraft);
        assert_eq!(card.pages.len(), 2);
        assert_eq!(card.calculate_fingerprint().unwrap(), card.fingerprint);
        let json = serde_json::to_string(&card).unwrap();
        for expected in [
            "The Draft Board",
            "Draft Flex Wing",
            "LW1, D1, G1",
            "C/LW",
            "2-position eligibility",
            "Schedule diversity",
            "Fallback:",
            "Refresh injury and role evidence",
        ] {
            assert!(json.contains(expected), "missing {expected}");
        }
        card.validate().unwrap();
    }

    #[test]
    fn fantasy_draft_card_rejects_an_empty_recommendation_board() {
        let mut input = input();
        input.board.rows.clear();
        assert_eq!(
            build_fantasy_draft_card(input),
            Err(FantasyDraftCardError::EmptyBoard)
        );
    }
}
