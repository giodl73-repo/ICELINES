//! Core-owned morning card projected from `FantasyMorningBriefingView`.

use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    FantasyGoaliePlanAction, FantasyMorningActionKind, FantasyMorningBriefingView, MetricUnit,
    MetricValue, SemanticToken, SourceKind, StatKey, ValuePrecision, ViewContext, ViewWarning,
    WarningKind,
};

pub const FANTASY_MORNING_CARD_BUILDER_VERSION: &str = "fantasy_morning_card.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyMorningCardInput {
    pub league_id: String,
    pub league_name: String,
    pub fantasy_team_id: String,
    pub fantasy_team_name: String,
    pub scoring_scheme_id: String,
    pub scoring_scheme_name: String,
    pub roster_snapshot_id: Option<String>,
    pub briefing: FantasyMorningBriefingView,
    pub view: ViewContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FantasyMorningCardError {
    #[error("fantasy morning card requires {0}")]
    MissingText(&'static str),
    #[error("serialize fantasy morning briefing: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_fantasy_morning_card(
    input: FantasyMorningCardInput,
) -> Result<CardDocumentView, FantasyMorningCardError> {
    for (field, value) in [
        ("league ID", input.league_id.as_str()),
        ("league name", input.league_name.as_str()),
        ("fantasy team ID", input.fantasy_team_id.as_str()),
        ("fantasy team name", input.fantasy_team_name.as_str()),
        ("scoring scheme ID", input.scoring_scheme_id.as_str()),
        ("scoring scheme name", input.scoring_scheme_name.as_str()),
        ("briefing schema", input.briefing.schema.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FantasyMorningCardError::MissingText(field));
        }
    }

    let briefing_fingerprint = json_fingerprint(&input.briefing)?;
    let warnings = morning_warnings(&input.briefing);
    let player_ids = morning_player_ids(&input.briefing);
    let completeness = input.view.completeness;
    let mut methods = BTreeMap::new();
    methods.insert(
        "morning_briefing".to_string(),
        input.briefing.schema.clone(),
    );
    methods.insert(
        "daily_lineup".to_string(),
        input.briefing.injury_plan.lineup.schema.clone(),
    );
    methods.insert(
        "weekly_budget".to_string(),
        input.briefing.budget.schema.clone(),
    );
    if let Some(goalie) = &input.briefing.goalie_plan {
        methods.insert("goalie_plan".to_string(), goalie.schema.clone());
    }
    if let Some(pickups) = &input.briefing.pickup_plan {
        methods.insert("pickup_plan".to_string(), pickups.schema.clone());
    }

    let mut morning_sections = vec![
        identity_section(&input),
        CardSectionView::Decision(action_section(&input.briefing)),
        CardSectionView::Lineup(super::fantasy_roster::roster_lineup_section(
            &input.briefing.injury_plan.lineup,
        )),
    ];
    if !warnings.is_empty() {
        morning_sections.push(warning_section("morning-warnings", &warnings));
    }

    let mut insider_sections = vec![
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "weekly-budget".to_string(),
            title: Some("Pickup budget".to_string()),
            metrics: budget_metrics(&input.briefing),
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "goalie-checkpoints".to_string(),
            title: Some("Goalie checkpoints".to_string()),
            metrics: goalie_checkpoint_metrics(&input.briefing),
        }),
    ];
    let timeline = checkpoint_timeline(&input.briefing);
    if !timeline.items.is_empty() {
        insider_sections.push(CardSectionView::Timeline(timeline));
    }
    for section in [
        goalie_rows(&input.briefing),
        pickup_rows(&input.briefing),
        status_rows(&input.briefing),
    ]
    .into_iter()
    .flatten()
    {
        insider_sections.push(CardSectionView::PlayerList(section));
    }
    if !warnings.is_empty() {
        insider_sections.push(warning_section("morning-insider-warnings", &warnings));
    }
    insider_sections.extend([
        CardSectionView::Methodology(MethodologySectionView {
            id: "fantasy-morning-methodology".to_string(),
            title: "Methodology".to_string(),
            methods: vec![
                CardMethodologyItemView {
                    key: "morning-actions".to_string(),
                    label: "Morning actions".to_string(),
                    version: input.briefing.schema.clone(),
                    summary: "Legal lineup, injury freshness, goalie evidence and locks, pickup budget, weekly moves, and sleepers are resolved before actions are prioritized.".to_string(),
                },
                CardMethodologyItemView {
                    key: "material-change".to_string(),
                    label: "Material change detection".to_string(),
                    version: "material_fingerprint.v1".to_string(),
                    summary: "Decision-bearing state is fingerprinted separately from generation time and warning prose.".to_string(),
                },
            ],
            limitations: vec![
                "The morning card is advisory and does not mutate the fantasy platform.".to_string(),
                "Refresh injuries and goalie starts again near each player's lock time.".to_string(),
            ],
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "fantasy-morning-sources".to_string(),
            title: "Source authority".to_string(),
            provenance_ids: vec!["fantasy-morning-briefing".to_string()],
        }),
    ]);

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::FantasyMorning,
        document_id: format!(
            "fantasy-morning:{}:{}:{}",
            stable_id(&input.league_id),
            stable_id(&input.fantasy_team_id),
            input.briefing.date
        ),
        fingerprint: String::new(),
        title: format!("{} — morning skate", input.fantasy_team_name.trim()),
        subtitle: Some(format!(
            "{} · {} · evaluated {}",
            input.league_name.trim(),
            input.scoring_scheme_name.trim(),
            input.briefing.evaluated_at.to_rfc3339()
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: Some(input.briefing.evaluated_at),
            evidence_label: EvidenceLabel::Estimated,
            builder_version: FANTASY_MORNING_CARD_BUILDER_VERSION.to_string(),
            methodology_versions: methods,
            joins: CardIdentityJoinsView {
                league_id: Some(input.league_id),
                roster_snapshot_id: input.roster_snapshot_id,
                scoring_scheme_id: Some(input.scoring_scheme_id),
                team_ids: vec![input.fantasy_team_id.clone()],
                player_ids,
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView::default(),
        },
        theme: CardThemeView {
            theme_key: "fantasy-morning".to_string(),
            primary: Some("#12355B".to_string()),
            secondary: Some("#DCEAF5".to_string()),
            accent: Some("#E9C46A".to_string()),
            surface: Some("#FFFFFF".to_string()),
            text: Some("#102A43".to_string()),
            team_abbreviation: None,
            ascii_identity: input.fantasy_team_name.trim().to_string(),
            minimum_text_contrast_x100: 450,
        },
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "morning-skate".to_string(),
                literal_label: "Morning lineup and actions".to_string(),
                display_label: Some("The Morning Skate".to_string()),
                order: 1,
                accessible_summary: format!(
                    "Prioritized actions and legal lineup for {} on {}.",
                    input.fantasy_team_name.trim(),
                    input.briefing.date
                ),
                sections: morning_sections,
            },
            CardPageView {
                id: "morning-insider".to_string(),
                literal_label: "Morning evidence and timing".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary: "Pickup budget, goalie evidence and locks, injury freshness, move candidates, warnings, and methodology.".to_string(),
                sections: insider_sections,
            },
        ],
        assets: Vec::new(),
        provenance: vec![CardProvenanceView {
            id: "fantasy-morning-briefing".to_string(),
            source: SourceKind::FantasyImport,
            label: "Legal lineup, availability, goalie, acquisition, and sleeper decision state"
                .to_string(),
            state: completeness,
            observed_at: Some(input.briefing.evaluated_at),
            fingerprint: Some(briefing_fingerprint),
            note: Some(format!(
                "Material decision fingerprint {}",
                input.briefing.material_fingerprint
            )),
        }],
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| FantasyMorningCardError::Document(error.to_string()))
}

fn identity_section(input: &FantasyMorningCardInput) -> CardSectionView {
    CardSectionView::IdentityHeader(IdentityHeaderSectionView {
        id: "fantasy-morning-identity".to_string(),
        eyebrow: Some(input.league_name.trim().to_string()),
        title: input.fantasy_team_name.trim().to_string(),
        subtitle: Some(format!("{} morning briefing", input.briefing.date)),
        identities: vec![CardIdentityView {
            kind: CardIdentityKind::Team,
            subject_id: format!("fantasy-team:{}", stable_id(&input.fantasy_team_id)),
            label: input.fantasy_team_name.trim().to_string(),
            asset_id: None,
        }],
    })
}

fn action_section(briefing: &FantasyMorningBriefingView) -> DecisionSectionView {
    let recommendation = briefing
        .actions
        .first()
        .map(|row| row.message.clone())
        .unwrap_or_else(|| "No lineup or transaction action is currently recommended.".to_string());
    let rationale = briefing
        .actions
        .first()
        .map(|row| {
            vec![format!(
                "Priority {} · {}{}",
                row.priority,
                action_kind_label(row.kind),
                if row.conditional {
                    " · conditional"
                } else {
                    ""
                }
            )]
        })
        .unwrap_or_else(|| {
            vec!["The current legal lineup and evidence require no change.".to_string()]
        });
    DecisionSectionView {
        id: "morning-action-queue".to_string(),
        title: "Do this first".to_string(),
        recommendation,
        rationale,
        alternatives: briefing
            .actions
            .iter()
            .skip(1)
            .enumerate()
            .map(|(index, row)| CardDecisionAlternativeView {
                id: format!("action-{}-{}", row.priority, index + 2),
                label: format!("{} · {}", row.priority, action_kind_label(row.kind)),
                detail: Some(format!(
                    "{}{}",
                    row.message,
                    if row.conditional {
                        " (conditional)"
                    } else {
                        ""
                    }
                )),
            })
            .collect(),
        action_id: None,
        token: SemanticToken::PrimaryAction,
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn budget_metrics(briefing: &FantasyMorningBriefingView) -> Vec<CardMetricView> {
    let budget = &briefing.budget;
    vec![
        integer_metric(
            "acquisition_limit",
            "Weekly limit",
            budget.acquisition_limit as i64,
        ),
        integer_metric("acquisitions_used", "Used", budget.acquisitions_used as i64),
        integer_metric(
            "acquisitions_remaining",
            "Remaining",
            budget.acquisitions_remaining as i64,
        ),
        integer_metric(
            "proactive_remaining",
            "Safe proactive adds",
            budget.proactive_acquisitions_remaining as i64,
        ),
        integer_metric(
            "injury_reserve",
            "Injury reserve",
            budget.injury_reserve as i64,
        ),
        text_metric(
            "can_add",
            "Can add",
            if budget.can_add { "Yes" } else { "No" },
        ),
    ]
}

fn goalie_checkpoint_metrics(briefing: &FantasyMorningBriefingView) -> Vec<CardMetricView> {
    vec![
        integer_metric(
            "goalie_refreshes_due",
            "Refreshes due now",
            briefing.goalie_refreshes_due_now as i64,
        ),
        integer_metric(
            "goalie_safety_due",
            "Safety checks due now",
            briefing.goalie_safety_checks_due_now as i64,
        ),
        text_metric(
            "next_goalie_refresh",
            "Next refresh",
            &time_text(briefing.next_goalie_refresh_utc),
        ),
        text_metric(
            "next_goalie_safety",
            "Next safety check",
            &time_text(briefing.next_goalie_safety_check_utc),
        ),
        text_metric(
            "next_goalie_lock",
            "Next lock",
            &time_text(briefing.next_goalie_lock_utc),
        ),
    ]
}

fn checkpoint_timeline(briefing: &FantasyMorningBriefingView) -> TimelineSectionView {
    let mut items = [
        (
            "goalie-refresh",
            "Refresh goalie evidence",
            briefing.next_goalie_refresh_utc,
        ),
        (
            "goalie-safety",
            "Final goalie safety check",
            briefing.next_goalie_safety_check_utc,
        ),
        (
            "goalie-lock",
            "Next goalie lineup lock",
            briefing.next_goalie_lock_utc,
        ),
    ]
    .into_iter()
    .filter_map(|(id, label, effective_at)| {
        effective_at.map(|effective_at| CardTimelineItemView {
            id: id.to_string(),
            effective_at,
            observed_at: Some(briefing.evaluated_at),
            label: label.to_string(),
            detail: None,
            evidence_label: EvidenceLabel::Estimated,
            token: SemanticToken::Info,
        })
    })
    .collect::<Vec<_>>();
    items.sort_by_key(|item| item.effective_at);
    TimelineSectionView {
        id: "goalie-deadlines".to_string(),
        title: "Today's goalie checkpoints".to_string(),
        items,
    }
}

fn goalie_rows(briefing: &FantasyMorningBriefingView) -> Option<PlayerListSectionView> {
    let rows = briefing
        .goalie_plan
        .as_ref()?
        .rows
        .iter()
        .filter(|row| row.date == briefing.date)
        .map(|row| CardPlayerRowView {
            player_id: row.player_key.clone(),
            name: row.player.clone(),
            role: Some(format!(
                "{} vs {} · {}",
                row.nhl_team,
                row.opponent,
                goalie_action_label(row.action)
            )),
            asset_id: None,
            metrics: vec![
                percentage_metric(
                    "start_probability",
                    "Start probability",
                    row.start_probability,
                ),
                decimal_metric(
                    "projected_points",
                    "Projected points",
                    row.projected_points,
                    MetricUnit::Points,
                ),
                text_metric(
                    "start_evidence",
                    "Evidence",
                    &format!(
                        "{:?} / {:?}",
                        row.evidence.effective_state, row.evidence.freshness
                    ),
                ),
                text_metric("lineup_lock", "Lock", &time_text(row.game_start_utc)),
            ],
            tokens: vec![if row.conditional {
                SemanticToken::Risk
            } else {
                SemanticToken::SupportingEvidence
            }],
            evidence_label: if row.evidence.requires_refresh {
                EvidenceLabel::UnderReview
            } else {
                EvidenceLabel::Confirmed
            },
        })
        .collect::<Vec<_>>();
    (!rows.is_empty()).then_some(PlayerListSectionView {
        id: "goalie-start-evidence".to_string(),
        title: "Goalie start evidence".to_string(),
        rows,
    })
}

fn pickup_rows(briefing: &FantasyMorningBriefingView) -> Option<PlayerListSectionView> {
    let rows = briefing
        .pickup_plan
        .as_ref()?
        .rows
        .iter()
        .map(|row| CardPlayerRowView {
            player_id: row.add_player_key.clone(),
            name: row.add_player.clone(),
            role: Some(format!("Add for {} · rank {}", row.drop_player, row.rank)),
            asset_id: None,
            metrics: vec![
                decimal_metric(
                    "usable_starts_delta",
                    "Usable starts",
                    row.incremental_usable_starts,
                    MetricUnit::Games,
                ),
                decimal_metric(
                    "projected_value_delta",
                    "Projected value",
                    row.projected_value_delta,
                    MetricUnit::Score,
                ),
            ],
            tokens: vec![SemanticToken::Stream],
            evidence_label: EvidenceLabel::Estimated,
        })
        .collect::<Vec<_>>();
    (!rows.is_empty()).then_some(PlayerListSectionView {
        id: "pickup-candidates".to_string(),
        title: "Best available weekly moves".to_string(),
        rows,
    })
}

fn status_rows(briefing: &FantasyMorningBriefingView) -> Option<PlayerListSectionView> {
    let rows = briefing
        .injury_plan
        .statuses
        .iter()
        .filter(|row| row.requires_pregame_refresh)
        .map(|row| CardPlayerRowView {
            player_id: row.player_key.clone(),
            name: row.player_key.clone(),
            role: row.source.clone(),
            asset_id: None,
            metrics: vec![
                text_metric(
                    "reported_status",
                    "Reported",
                    &format!("{:?}", row.reported_status),
                ),
                text_metric(
                    "effective_status",
                    "Effective",
                    &format!("{:?}", row.effective_status),
                ),
                text_metric("freshness", "Freshness", &format!("{:?}", row.freshness)),
                text_metric(
                    "age_minutes",
                    "Age",
                    &row.age_minutes
                        .map(|age| format!("{age} min"))
                        .unwrap_or_else(|| "Unknown".to_string()),
                ),
            ],
            tokens: vec![SemanticToken::Risk],
            evidence_label: EvidenceLabel::UnderReview,
        })
        .collect::<Vec<_>>();
    (!rows.is_empty()).then_some(PlayerListSectionView {
        id: "injury-refreshes".to_string(),
        title: "Availability refreshes".to_string(),
        rows,
    })
}

fn morning_warnings(briefing: &FantasyMorningBriefingView) -> Vec<ViewWarning> {
    let mut messages = briefing.warnings.clone();
    messages.extend(briefing.injury_plan.warnings.iter().cloned());
    if let Some(plan) = &briefing.goalie_plan {
        messages.extend(plan.warnings.iter().cloned());
    }
    if let Some(plan) = &briefing.pickup_plan {
        messages.extend(plan.warnings.iter().cloned());
    }
    messages.sort();
    messages.dedup();
    messages
        .into_iter()
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
        title: "Morning evidence warnings".to_string(),
        detail: Some("Late news can change lineup and transaction recommendations.".to_string()),
        empty_state: None,
        warnings: warnings.to_vec(),
        token: SemanticToken::Warning,
    })
}

fn morning_player_ids(briefing: &FantasyMorningBriefingView) -> Vec<String> {
    let lineup = &briefing.injury_plan.lineup;
    lineup
        .active
        .iter()
        .map(|row| row.player_key.clone())
        .chain(
            lineup
                .bench_assignments
                .iter()
                .map(|row| row.player_key.clone()),
        )
        .chain(
            lineup
                .injured_reserve
                .iter()
                .map(|row| row.player_key.clone()),
        )
        .chain(
            lineup
                .injured_reserve_plus
                .iter()
                .map(|row| row.player_key.clone()),
        )
        .chain(
            briefing
                .actions
                .iter()
                .filter_map(|row| row.player_key.clone()),
        )
        .chain(
            briefing
                .goalie_plan
                .iter()
                .flat_map(|plan| plan.rows.iter().map(|row| row.player_key.clone())),
        )
        .chain(briefing.pickup_plan.iter().flat_map(|plan| {
            plan.rows
                .iter()
                .flat_map(|row| [row.add_player_key.clone(), row.drop_player_key.clone()])
        }))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn action_kind_label(kind: FantasyMorningActionKind) -> &'static str {
    match kind {
        FantasyMorningActionKind::MoveToIr => "Move to IR",
        FantasyMorningActionKind::MoveToIrPlus => "Move to IR+",
        FantasyMorningActionKind::RefreshStatus => "Refresh injury",
        FantasyMorningActionKind::Start => "Start skater",
        FantasyMorningActionKind::RefreshGoalie => "Refresh goalie",
        FantasyMorningActionKind::StartGoalie => "Start goalie",
        FantasyMorningActionKind::BenchGoalie => "Bench goalie",
        FantasyMorningActionKind::GoalieLocked => "Goalie locked",
        FantasyMorningActionKind::GoalieStreamReview => "Goalie stream",
        FantasyMorningActionKind::GoalieFallback => "Goalie fallback",
        FantasyMorningActionKind::PickupReview => "Pickup review",
        FantasyMorningActionKind::SleeperWatch => "Sleeper watch",
    }
}

fn goalie_action_label(action: FantasyGoaliePlanAction) -> &'static str {
    match action {
        FantasyGoaliePlanAction::Start => "start",
        FantasyGoaliePlanAction::Bench => "bench",
        FantasyGoaliePlanAction::Wait => "wait",
        FantasyGoaliePlanAction::Refresh => "refresh",
        FantasyGoaliePlanAction::Locked => "locked",
    }
}

fn time_text(time: Option<DateTime<Utc>>) -> String {
    time.map(|time| time.to_rfc3339())
        .unwrap_or_else(|| "Not scheduled".to_string())
}

fn integer_metric(key: &str, label: &str, value: i64) -> CardMetricView {
    metric(
        key,
        label,
        MetricValue::Integer(value),
        value.to_string(),
        MetricUnit::Count,
        ValuePrecision::Integer,
        EvidenceLabel::Confirmed,
    )
}
fn decimal_metric(key: &str, label: &str, value: f64, unit: MetricUnit) -> CardMetricView {
    metric(
        key,
        label,
        MetricValue::Decimal(value),
        format!("{value:.2}"),
        unit,
        ValuePrecision::TwoDecimals,
        EvidenceLabel::Estimated,
    )
}
fn percentage_metric(key: &str, label: &str, value: f64) -> CardMetricView {
    let value = value * 100.0;
    metric(
        key,
        label,
        MetricValue::Decimal(value),
        format!("{value:.0}%"),
        MetricUnit::Percentage,
        ValuePrecision::PercentOneDecimal,
        EvidenceLabel::Estimated,
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
        EvidenceLabel::Confirmed,
    )
}
fn metric(
    key: &str,
    label: &str,
    value: MetricValue,
    display_text: String,
    unit: MetricUnit,
    precision: ValuePrecision,
    evidence_label: EvidenceLabel,
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
        evidence_label,
    }
}

fn stable_id(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, FantasyMorningCardError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| FantasyMorningCardError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;
    use crate::{
        model::{Position, Season},
        season_stats::SeasonType,
        view_model::{
            build_fantasy_daily_lineup, build_fantasy_morning_briefing, build_fantasy_week_budget,
            FantasyAssistantRules, FantasyInjuryPlanView, FantasyLineupPlayerInput,
            FantasyPlayerAvailabilityStatus, ViewWindow, FANTASY_INJURY_PLAN_SCHEMA,
        },
    };

    #[test]
    fn morning_card_seals_prioritized_actions_and_legal_lineup() {
        let evaluated_at = Utc.with_ymd_and_hms(2026, 10, 8, 14, 0, 0).unwrap();
        let rules = FantasyAssistantRules::configured_2026();
        let lineup = build_fantasy_daily_lineup(
            rules.clone(),
            vec![FantasyLineupPlayerInput {
                player_key: "sample-player-002".to_string(),
                display_name: "Sample Player 002".to_string(),
                nhl_team: "COL".to_string(),
                platform_positions: vec![Position::Center],
                projected_value: 8.4,
                has_game: true,
                status: FantasyPlayerAvailabilityStatus::Healthy,
                locked_slot: None,
                locked: false,
            }],
        )
        .unwrap();
        let injury_plan = FantasyInjuryPlanView {
            schema: FANTASY_INJURY_PLAN_SCHEMA.to_string(),
            date: NaiveDate::from_ymd_opt(2026, 10, 8).unwrap(),
            lineup,
            statuses: Vec::new(),
            warnings: vec!["Deterministic test assumptions only.".to_string()],
        };
        let budget = build_fantasy_week_budget(
            evaluated_at,
            &rules.timezone,
            rules.weekly_acquisition_limit,
            &[],
        )
        .unwrap();
        let briefing = build_fantasy_morning_briefing(
            evaluated_at,
            evaluated_at,
            rules.timezone,
            injury_plan,
            None,
            budget,
            None,
            None,
        );
        let card = build_fantasy_morning_card(FantasyMorningCardInput {
            league_id: "league-1".to_string(),
            league_name: "Sample League".to_string(),
            fantasy_team_id: "sample-multicategory".to_string(),
            fantasy_team_name: "Sample Multicategory".to_string(),
            scoring_scheme_id: "league-points".to_string(),
            scoring_scheme_name: "League points".to_string(),
            roster_snapshot_id: Some("roster-2026-10-08".to_string()),
            briefing,
            view: ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular)),
        })
        .unwrap();

        assert_eq!(card.card_kind, CardKind::FantasyMorning);
        assert_eq!(card.pages.len(), 2);
        assert_eq!(card.pages[0].id, "morning-skate");
        assert!(card.pages[0]
            .sections
            .iter()
            .any(|section| matches!(section, CardSectionView::Decision(_))));
        assert!(card.pages[0]
            .sections
            .iter()
            .any(|section| matches!(section, CardSectionView::Lineup(_))));
        assert!(!card.fingerprint.is_empty());
    }
}
