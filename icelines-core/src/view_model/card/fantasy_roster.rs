//! Core-owned fantasy roster card built from legal lineup and schedule views.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    FantasyActiveSlot, FantasyBenchAssignmentRow, FantasyDailyLineupView, FantasyInjuryPlanView,
    FantasyLineupAssignmentRow, FantasyReserveAssignmentRow, FantasyScheduleView, MetricUnit,
    MetricValue, SemanticToken, SourceKind, StatKey, ValuePrecision, ViewContext, ViewWarning,
    WarningKind,
};

pub const FANTASY_ROSTER_CARD_BUILDER_VERSION: &str = "fantasy_roster_card.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FantasyRosterCardInput {
    pub league_id: String,
    pub league_name: String,
    pub fantasy_team_id: String,
    pub fantasy_team_name: String,
    pub scoring_scheme_id: String,
    pub scoring_scheme_name: String,
    pub roster_snapshot_id: Option<String>,
    pub acquisitions_used_this_week: u8,
    pub injury_plan: FantasyInjuryPlanView,
    pub schedule: Option<FantasyScheduleView>,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FantasyRosterCardError {
    #[error("fantasy roster card requires {0}")]
    MissingText(&'static str),
    #[error("lineup season {lineup} does not match view season {view}")]
    SeasonMismatch { lineup: u32, view: u32 },
    #[error("acquisitions used ({used}) exceed weekly limit ({limit})")]
    AcquisitionLimit { used: u8, limit: u8 },
    #[error("fantasy assistant rules are invalid: {0}")]
    InvalidRules(String),
    #[error("serialize fantasy roster provenance: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_fantasy_roster_card(
    input: FantasyRosterCardInput,
) -> Result<CardDocumentView, FantasyRosterCardError> {
    for (field, value) in [
        ("league ID", input.league_id.as_str()),
        ("league name", input.league_name.as_str()),
        ("fantasy team ID", input.fantasy_team_id.as_str()),
        ("fantasy team name", input.fantasy_team_name.as_str()),
        ("scoring scheme ID", input.scoring_scheme_id.as_str()),
        ("scoring scheme name", input.scoring_scheme_name.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(FantasyRosterCardError::MissingText(field));
        }
    }
    input
        .injury_plan
        .lineup
        .rules
        .validate()
        .map_err(FantasyRosterCardError::InvalidRules)?;
    let season = input.view.window.season.0;
    if input
        .schedule
        .as_ref()
        .is_some_and(|schedule| schedule.season != season)
    {
        return Err(FantasyRosterCardError::SeasonMismatch {
            lineup: input.schedule.as_ref().unwrap().season,
            view: season,
        });
    }
    let rules = &input.injury_plan.lineup.rules;
    if input.acquisitions_used_this_week > rules.weekly_acquisition_limit {
        return Err(FantasyRosterCardError::AcquisitionLimit {
            used: input.acquisitions_used_this_week,
            limit: rules.weekly_acquisition_limit,
        });
    }

    let lineup_fingerprint = json_fingerprint(&input.injury_plan.lineup)?;
    let schedule_fingerprint = input.schedule.as_ref().map(json_fingerprint).transpose()?;
    let warnings = roster_warnings(&input);
    let player_ids = roster_player_ids(&input.injury_plan.lineup);
    let mut methods = BTreeMap::new();
    methods.insert(
        "lineup_assignment".to_string(),
        input.injury_plan.lineup.schema.clone(),
    );
    methods.insert(
        "assistant_rules".to_string(),
        input.injury_plan.lineup.rules.schema.clone(),
    );
    if let Some(schedule) = &input.schedule {
        methods.insert("schedule_classes".to_string(), schedule.schema.clone());
    }

    let mut page_two = vec![
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "roster-rules".to_string(),
            title: Some("League roster and transaction rules".to_string()),
            metrics: rule_metrics(&input),
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "daily-projection".to_string(),
            title: Some("Today's legal lineup".to_string()),
            metrics: daily_metrics(&input.injury_plan.lineup),
        }),
    ];
    if let Some(schedule) = &input.schedule {
        page_two.push(schedule_section(schedule));
    }
    if !warnings.is_empty() {
        page_two.push(CardSectionView::StateNotice(StateNoticeSectionView {
            id: "roster-warnings".to_string(),
            title: "Availability and roster warnings".to_string(),
            detail: Some(
                "Warnings remain attached to the sealed roster decision document.".to_string(),
            ),
            empty_state: None,
            warnings: warnings.clone(),
            token: SemanticToken::Warning,
        }));
    }
    page_two.extend([
        CardSectionView::Methodology(MethodologySectionView {
            id: "fantasy-roster-methodology".to_string(),
            title: "Methodology".to_string(),
            methods: vec![
                CardMethodologyItemView {
                    key: "legal-lineup".to_string(),
                    label: "Daily assignment".to_string(),
                    version: input.injury_plan.lineup.schema.clone(),
                    summary: "Position eligibility, locks, availability, bench, IR, and IR+ are resolved in IceLines core before rendering.".to_string(),
                },
                CardMethodologyItemView {
                    key: "schedule-fit".to_string(),
                    label: "Schedule equivalence classes".to_string(),
                    version: input.schedule.as_ref().map_or("not_loaded", |view| view.schema.as_str()).to_string(),
                    summary: "Exact NHL game-date overlap groups teams so roster construction can spread usable starts across the week.".to_string(),
                },
            ],
            limitations: vec![
                "The card is advisory and does not mutate the fantasy platform.".to_string(),
                "Free-agent and waiver timing reflects persisted league rules; platform state still controls execution.".to_string(),
            ],
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "fantasy-roster-sources".to_string(),
            title: "Source authority".to_string(),
            provenance_ids: if schedule_fingerprint.is_some() {
                vec!["fantasy-lineup".to_string(), "fantasy-schedule".to_string()]
            } else {
                vec!["fantasy-lineup".to_string()]
            },
        }),
    ]);

    let mut provenance = vec![CardProvenanceView {
        id: "fantasy-lineup".to_string(),
        source: SourceKind::FantasyImport,
        label: "Persisted roster, eligibility, status, and assistant rules".to_string(),
        state: input.view.completeness,
        observed_at: input.evidence_at,
        fingerprint: Some(lineup_fingerprint),
        note: Some(format!("Legal lineup for {}", input.injury_plan.date)),
    }];
    if let Some(fingerprint) = schedule_fingerprint.clone() {
        provenance.push(CardProvenanceView {
            id: "fantasy-schedule".to_string(),
            source: SourceKind::Schedule,
            label: "NHL schedule and equivalence classes".to_string(),
            state: Completeness::Complete,
            observed_at: input.evidence_at,
            fingerprint: Some(fingerprint),
            note: Some("Calendar fit only; player quality is scored separately.".to_string()),
        });
    }

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::FantasyRoster,
        document_id: format!(
            "fantasy-roster:{}:{}:{}",
            stable_id(&input.league_id),
            stable_id(&input.fantasy_team_id),
            input.injury_plan.date
        ),
        fingerprint: String::new(),
        title: format!("{} — fantasy roster", input.fantasy_team_name.trim()),
        subtitle: Some(format!(
            "{} · {} · {}",
            input.league_name.trim(),
            input.scoring_scheme_name.trim(),
            input.injury_plan.date
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: input.evidence_at,
            evidence_label: EvidenceLabel::Estimated,
            builder_version: FANTASY_ROSTER_CARD_BUILDER_VERSION.to_string(),
            methodology_versions: methods,
            joins: CardIdentityJoinsView {
                league_id: Some(input.league_id),
                roster_snapshot_id: input.roster_snapshot_id,
                calendar_fingerprint: schedule_fingerprint.clone(),
                scoring_scheme_id: Some(input.scoring_scheme_id),
                team_ids: vec![input.fantasy_team_id.clone()],
                player_ids,
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView::default(),
        },
        theme: CardThemeView {
            theme_key: "fantasy-roster".to_string(),
            primary: Some("#12355B".to_string()),
            secondary: Some("#DCEAF5".to_string()),
            accent: Some("#2A9D8F".to_string()),
            surface: Some("#FFFFFF".to_string()),
            text: Some("#102A43".to_string()),
            team_abbreviation: None,
            ascii_identity: input.fantasy_team_name.trim().to_string(),
            minimum_text_contrast_x100: 450,
        },
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "roster".to_string(),
                literal_label: "Legal daily roster".to_string(),
                display_label: Some("The Lineup".to_string()),
                order: 1,
                accessible_summary: format!(
                    "Legal active, bench, IR, and IR+ assignments for {}.",
                    input.fantasy_team_name.trim()
                ),
                sections: vec![
                    fantasy_identity(
                        &input.fantasy_team_id,
                        &input.fantasy_team_name,
                        &input.league_name,
                    ),
                    CardSectionView::Lineup(roster_lineup_section(&input.injury_plan.lineup)),
                ],
            },
            CardPageView {
                id: "roster-insider".to_string(),
                literal_label: "Roster rules and schedule fit".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary:
                    "League rules, usable starts, move budget, waiver timing, and schedule classes."
                        .to_string(),
                sections: page_two,
            },
        ],
        assets: Vec::new(),
        provenance,
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| FantasyRosterCardError::Document(error.to_string()))
}

fn fantasy_identity(team_id: &str, team_name: &str, league_name: &str) -> CardSectionView {
    CardSectionView::IdentityHeader(IdentityHeaderSectionView {
        id: "fantasy-team-identity".to_string(),
        eyebrow: Some(league_name.trim().to_string()),
        title: team_name.trim().to_string(),
        subtitle: Some("Daily legal roster".to_string()),
        identities: vec![CardIdentityView {
            kind: CardIdentityKind::Team,
            subject_id: format!("fantasy-team:{}", stable_id(team_id)),
            label: team_name.trim().to_string(),
            asset_id: None,
        }],
    })
}

pub(super) fn roster_lineup_section(lineup: &FantasyDailyLineupView) -> LineupSectionView {
    let mut active = lineup.active.iter().map(active_slot).collect::<Vec<_>>();
    active.extend(lineup.missing_active_slots.iter().map(open_active_slot));
    active.sort_by(|a, b| a.id.cmp(&b.id));
    let mut groups = vec![CardLineupGroupView {
        id: "active-slots".to_string(),
        label: "Active".to_string(),
        kind: CardLineupGroupKind::ActiveSlots,
        slots: active,
    }];
    groups.push(bench_group(lineup));
    groups.push(reserve_group(
        "injured-reserve",
        "IR",
        lineup.rules.ir_slots as usize,
        &lineup.injured_reserve,
    ));
    groups.push(reserve_group(
        "injured-reserve-plus",
        "IR+",
        lineup.rules.ir_plus_slots as usize,
        &lineup.injured_reserve_plus,
    ));
    if !lineup.overflow.is_empty() {
        groups.push(name_only_group(
            "overflow",
            "Overflow",
            CardLineupGroupKind::Extras,
            "OVER",
            lineup.overflow.len(),
            &lineup.overflow,
        ));
    }
    LineupSectionView {
        id: "fantasy-daily-lineup".to_string(),
        title: "Active slots, bench, and injury reserve".to_string(),
        groups,
    }
}

fn active_slot(row: &FantasyLineupAssignmentRow) -> CardLineupSlotView {
    let positions = row
        .platform_positions
        .iter()
        .map(|position| position.abbreviation())
        .collect::<Vec<_>>()
        .join("/");
    CardLineupSlotView {
        id: format!("active:{}", stable_id(&row.slot_id)),
        label: format!("{} · {} · {}", row.slot_id, row.nhl_team, positions),
        subject_id: Some(format!("player:{}", stable_id(&row.player_key))),
        subject_label: Some(row.player.clone()),
        asset_id: None,
        metrics: vec![decimal_metric(
            "projected_value",
            "Projected value",
            row.projected_value,
            MetricUnit::PerGame,
            EvidenceLabel::Estimated,
        )],
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn open_active_slot(slot: &FantasyActiveSlot) -> CardLineupSlotView {
    CardLineupSlotView {
        id: format!("active:{}", stable_id(&slot.slot_id)),
        label: slot.slot_id.clone(),
        subject_id: None,
        subject_label: None,
        asset_id: None,
        metrics: Vec::new(),
        evidence_label: EvidenceLabel::NoRead,
    }
}

fn name_only_group(
    id: &str,
    label: &str,
    kind: CardLineupGroupKind,
    slot_prefix: &str,
    capacity: usize,
    names: &[String],
) -> CardLineupGroupView {
    let count = capacity.max(names.len()).max(1);
    CardLineupGroupView {
        id: id.to_string(),
        label: label.to_string(),
        kind,
        slots: (0..count)
            .map(|index| CardLineupSlotView {
                id: format!("{id}:{}", index + 1),
                label: format!("{slot_prefix}{}", index + 1),
                subject_id: names
                    .get(index)
                    .map(|name| format!("{id}:{}:{}", index + 1, stable_id(name))),
                subject_label: names.get(index).cloned(),
                asset_id: None,
                metrics: Vec::new(),
                evidence_label: if names.get(index).is_some() {
                    EvidenceLabel::Confirmed
                } else {
                    EvidenceLabel::NoRead
                },
            })
            .collect(),
    }
}

fn bench_group(lineup: &FantasyDailyLineupView) -> CardLineupGroupView {
    if lineup.bench_assignments.is_empty() {
        return name_only_group(
            "bench",
            "Bench",
            CardLineupGroupKind::Bench,
            "BN",
            lineup.rules.bench_slots as usize,
            &lineup.bench,
        );
    }
    let count = (lineup.rules.bench_slots as usize)
        .max(lineup.bench_assignments.len())
        .max(1);
    CardLineupGroupView {
        id: "bench".to_string(),
        label: "Bench".to_string(),
        kind: CardLineupGroupKind::Bench,
        slots: (0..count)
            .map(|index| bench_slot(index, lineup.bench_assignments.get(index)))
            .collect(),
    }
}

fn bench_slot(index: usize, row: Option<&FantasyBenchAssignmentRow>) -> CardLineupSlotView {
    let label = row.map_or_else(
        || format!("BN{}", index + 1),
        |row| {
            let positions = row
                .platform_positions
                .iter()
                .map(|position| position.abbreviation())
                .collect::<Vec<_>>()
                .join("/");
            format!("{} · {} · {}", row.bench_slot, row.nhl_team, positions)
        },
    );
    CardLineupSlotView {
        id: format!("bench:{}", index + 1),
        label,
        subject_id: row.map(|row| format!("player:{}", stable_id(&row.player_key))),
        subject_label: row.map(|row| row.player.clone()),
        asset_id: None,
        metrics: row
            .map(|row| {
                vec![decimal_metric(
                    "projected_value",
                    "Projected value",
                    row.projected_value,
                    MetricUnit::PerGame,
                    EvidenceLabel::Estimated,
                )]
            })
            .unwrap_or_default(),
        evidence_label: if row.is_some() {
            EvidenceLabel::Estimated
        } else {
            EvidenceLabel::NoRead
        },
    }
}

fn reserve_group(
    id: &str,
    label: &str,
    capacity: usize,
    rows: &[FantasyReserveAssignmentRow],
) -> CardLineupGroupView {
    let count = capacity.max(rows.len()).max(1);
    CardLineupGroupView {
        id: id.to_string(),
        label: label.to_string(),
        kind: CardLineupGroupKind::InjuredReserve,
        slots: (0..count)
            .map(|index| {
                let row = rows.get(index);
                CardLineupSlotView {
                    id: format!("{id}:{}", index + 1),
                    label: row.map_or_else(
                        || format!("{label}{}", index + 1),
                        |row| row.reserve_slot.clone(),
                    ),
                    subject_id: row.map(|row| format!("player:{}", stable_id(&row.player_key))),
                    subject_label: row.map(|row| row.player.clone()),
                    asset_id: None,
                    metrics: Vec::new(),
                    evidence_label: if row.is_some() {
                        EvidenceLabel::Reported
                    } else {
                        EvidenceLabel::NoRead
                    },
                }
            })
            .collect(),
    }
}

fn rule_metrics(input: &FantasyRosterCardInput) -> Vec<CardMetricView> {
    let rules = &input.injury_plan.lineup.rules;
    let remaining = rules.weekly_acquisition_limit - input.acquisitions_used_this_week;
    vec![
        integer_metric(
            "active_slots",
            "Active slots",
            rules.active_slot_count() as i64,
            MetricUnit::Count,
        ),
        integer_metric(
            "bench_slots",
            "Bench",
            rules.bench_slots.into(),
            MetricUnit::Count,
        ),
        integer_metric("ir_slots", "IR", rules.ir_slots.into(), MetricUnit::Count),
        integer_metric(
            "ir_plus_slots",
            "IR+",
            rules.ir_plus_slots.into(),
            MetricUnit::Count,
        ),
        integer_metric(
            "weekly_acquisition_limit",
            "Weekly pickups",
            rules.weekly_acquisition_limit.into(),
            MetricUnit::Count,
        ),
        integer_metric(
            "acquisitions_remaining",
            "Pickups remaining",
            remaining.into(),
            MetricUnit::Count,
        ),
        integer_metric(
            "waiver_days",
            "Dropped-player waivers",
            rules.waiver_days.into(),
            MetricUnit::Count,
        ),
        text_metric(
            "free_agent_activation",
            "Free-agent activation",
            if rules.free_agent_same_day {
                "Same day"
            } else {
                "Next day"
            },
        ),
    ]
}

fn daily_metrics(lineup: &FantasyDailyLineupView) -> Vec<CardMetricView> {
    vec![
        integer_metric(
            "usable_starts",
            "Usable starts",
            lineup.usable_starts as i64,
            MetricUnit::Count,
        ),
        decimal_metric(
            "projected_active_value",
            "Projected active value",
            lineup.projected_active_value,
            MetricUnit::Points,
            EvidenceLabel::Estimated,
        ),
        integer_metric(
            "missing_active_slots",
            "Open active slots",
            lineup.missing_active_slots.len() as i64,
            MetricUnit::Count,
        ),
        integer_metric(
            "overflow_players",
            "Roster overflow",
            lineup.overflow.len() as i64,
            MetricUnit::Count,
        ),
    ]
}

fn schedule_section(schedule: &FantasyScheduleView) -> CardSectionView {
    let roster = schedule.roster.as_ref();
    let recommendation = roster
        .and_then(|roster| roster.best_complements.first())
        .map_or_else(
            || "Spread additions across schedule classes".to_string(),
            |row| {
                format!(
                    "Best calendar complement: {} (Class {})",
                    row.team, row.equivalence_class
                )
            },
        );
    let mut rationale = vec![format!(
        "{} schedule classes across {} regular-season games.",
        schedule.equivalence_classes.len(),
        schedule.game_count
    )];
    if let Some(roster) = roster {
        rationale.push(format!(
            "Current roster utilization: {:.1}% across {} distinct dates.",
            roster.utilization_pct, roster.distinct_active_dates
        ));
    }
    CardSectionView::Decision(DecisionSectionView {
        id: "schedule-equivalence".to_string(),
        title: "Schedule spread".to_string(),
        recommendation,
        rationale,
        alternatives: schedule
            .equivalence_classes
            .iter()
            .map(|class| CardDecisionAlternativeView {
                id: format!("schedule-class-{}", class.class_id),
                label: format!("Class {}: {}", class.class_id, class.teams.join(", ")),
                detail: Some(format!(
                    "{:.1}% average within-class overlap",
                    class.average_within_overlap_pct
                )),
            })
            .collect(),
        action_id: None,
        token: SemanticToken::ScheduleEdge,
        evidence_label: EvidenceLabel::Confirmed,
    })
}

fn roster_warnings(input: &FantasyRosterCardInput) -> Vec<ViewWarning> {
    let mut warnings = input
        .injury_plan
        .warnings
        .iter()
        .chain(input.injury_plan.lineup.warnings.iter())
        .map(|message| ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::FantasyImport),
            message: message.clone(),
            recovery: Vec::new(),
        })
        .collect::<Vec<_>>();
    if input.schedule.is_none() {
        warnings.push(ViewWarning {
            kind: WarningKind::MissingSource,
            source: Some(SourceKind::Schedule),
            message: "Schedule equivalence classes are not loaded for this roster card."
                .to_string(),
            recovery: Vec::new(),
        });
    }
    warnings.sort_by(|a, b| a.message.cmp(&b.message));
    warnings.dedup_by(|a, b| a.message == b.message);
    warnings
}

fn integer_metric(key: &str, label: &str, value: i64, unit: MetricUnit) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Integer(value),
            unit,
            precision: ValuePrecision::Integer,
            token: None,
        },
        display_text: value.to_string(),
        accessible_text: format!("{label} {value}"),
        comparison: None,
        evidence_label: EvidenceLabel::Confirmed,
    }
}

fn decimal_metric(
    key: &str,
    label: &str,
    value: f64,
    unit: MetricUnit,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(value),
            unit,
            precision: ValuePrecision::TwoDecimals,
            token: None,
        },
        display_text: format!("{value:.2}"),
        accessible_text: format!("{label} {value:.2}"),
        comparison: None,
        evidence_label,
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

fn roster_player_ids(lineup: &FantasyDailyLineupView) -> Vec<String> {
    let mut ids = lineup
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
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn stable_id(value: &str) -> String {
    let id = value
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
        .collect::<String>();
    id.split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, FantasyRosterCardError> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| FantasyRosterCardError::Serialize(error.to_string()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[cfg(test)]
mod tests {
    use chrono::{NaiveDate, TimeZone};

    use super::*;
    use crate::view_model::{
        build_fantasy_daily_lineup, FantasyAssistantRules, FantasyLineupPlayerInput,
        FantasyPlayerAvailabilityStatus, FantasyScheduleClassRow, ViewWindow,
        FANTASY_INJURY_PLAN_SCHEMA, FANTASY_SCHEDULE_SCHEMA,
    };
    use crate::{
        model::{Position, Season},
        season_stats::SeasonType,
    };

    fn input() -> FantasyRosterCardInput {
        let rules = FantasyAssistantRules::configured_2026();
        let positions = [
            Position::Center,
            Position::Center,
            Position::Center,
            Position::Center,
            Position::LeftWing,
            Position::LeftWing,
            Position::LeftWing,
            Position::RightWing,
            Position::RightWing,
            Position::RightWing,
            Position::Defense,
            Position::Defense,
            Position::Defense,
            Position::Defense,
            Position::Goalie,
            Position::Goalie,
        ];
        let mut players = positions
            .iter()
            .enumerate()
            .map(|(index, position)| FantasyLineupPlayerInput {
                player_key: format!("player-{index}"),
                display_name: format!("Player {index}"),
                nhl_team: if index % 2 == 0 {
                    "NYR".to_string()
                } else {
                    "SEA".to_string()
                },
                platform_positions: vec![*position],
                projected_value: 20.0 - index as f64,
                has_game: true,
                status: FantasyPlayerAvailabilityStatus::Healthy,
                locked_slot: None,
                locked: false,
            })
            .collect::<Vec<_>>();
        for (index, status) in [
            FantasyPlayerAvailabilityStatus::InjuredReserve,
            FantasyPlayerAvailabilityStatus::LongTermInjuredReserve,
            FantasyPlayerAvailabilityStatus::DayToDay,
            FantasyPlayerAvailabilityStatus::Out,
        ]
        .into_iter()
        .enumerate()
        {
            players.push(FantasyLineupPlayerInput {
                player_key: format!("injured-{index}"),
                display_name: format!("Injured Player {index}"),
                nhl_team: "NYR".to_string(),
                platform_positions: vec![Position::Center],
                projected_value: 3.0,
                has_game: true,
                status,
                locked_slot: None,
                locked: false,
            });
        }
        let lineup = build_fantasy_daily_lineup(rules, players).unwrap();
        let timestamp = Utc.with_ymd_and_hms(2026, 10, 5, 14, 0, 0).unwrap();
        let mut view = ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular));
        view.generated_at = Some(timestamp);
        FantasyRosterCardInput {
            league_id: "league-1".to_string(),
            league_name: "Ice League".to_string(),
            fantasy_team_id: "sample-multicategory".to_string(),
            fantasy_team_name: "Sample Multicategory".to_string(),
            scoring_scheme_id: "league-scoring-v1".to_string(),
            scoring_scheme_name: "League scoring".to_string(),
            roster_snapshot_id: Some("roster-2026-10-05".to_string()),
            acquisitions_used_this_week: 2,
            injury_plan: FantasyInjuryPlanView {
                schema: FANTASY_INJURY_PLAN_SCHEMA.to_string(),
                date: NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
                lineup,
                statuses: Vec::new(),
                warnings: vec!["Refresh two day-to-day statuses before puck drop.".to_string()],
            },
            schedule: Some(FantasyScheduleView {
                schema: FANTASY_SCHEDULE_SCHEMA.to_string(),
                season: 20262027,
                game_count: 1344,
                season_start: NaiveDate::from_ymd_opt(2026, 10, 5).unwrap(),
                season_end: NaiveDate::from_ymd_opt(2027, 4, 18).unwrap(),
                off_night_max_games: 4,
                daily_slates: Vec::new(),
                weeks: Vec::new(),
                teams: Vec::new(),
                equivalence_classes: vec![
                    FantasyScheduleClassRow {
                        class_id: 1,
                        teams: vec!["NYR".to_string()],
                        average_within_overlap_pct: 100.0,
                    },
                    FantasyScheduleClassRow {
                        class_id: 2,
                        teams: vec!["SEA".to_string()],
                        average_within_overlap_pct: 100.0,
                    },
                ],
                roster: None,
                disclosures: Vec::new(),
            }),
            view,
            evidence_at: Some(timestamp),
        }
    }

    #[test]
    fn fantasy_roster_card_preserves_rules_slots_and_schedule_classes() {
        let card = build_fantasy_roster_card(input()).unwrap();
        assert_eq!(card.card_kind, CardKind::FantasyRoster);
        assert_eq!(card.pages.len(), 2);
        let json = serde_json::to_string(&card).unwrap();
        for expected in [
            "Sample Multicategory",
            "C1",
            "BN4",
            "IR2",
            "IR+2",
            "Same day",
            "Pickups remaining",
            "Class 1: NYR",
            "Class 2: SEA",
        ] {
            assert!(json.contains(expected), "missing {expected}");
        }
        card.validate().unwrap();
    }

    #[test]
    fn fantasy_roster_card_rejects_move_count_over_limit() {
        let mut invalid = input();
        invalid.acquisitions_used_this_week = 5;
        assert_eq!(
            build_fantasy_roster_card(invalid),
            Err(FantasyRosterCardError::AcquisitionLimit { used: 5, limit: 4 })
        );
    }

    #[test]
    fn legacy_rules_default_same_day_free_agents() {
        let mut value = serde_json::to_value(FantasyAssistantRules::configured_2026()).unwrap();
        value.as_object_mut().unwrap().remove("free_agent_same_day");
        let restored: FantasyAssistantRules = serde_json::from_value(value).unwrap();
        assert!(restored.free_agent_same_day);
    }
}
