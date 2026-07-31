//! UI-neutral card projection of a sealed IceCast forecast movement artifact.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    Completeness, EvidenceLabel, MetricCell, MetricUnit, MetricValue, SourceKind, StatKey,
    TeamSeasonForecastMovementRow, TeamSeasonForecastMovementView, ValuePrecision, ViewContext,
    TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA,
};

pub const FORECAST_MOVEMENT_CARD_VERSION: &str = "forecast_movement_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ForecastMovementCardInput {
    pub movement: TeamSeasonForecastMovementView,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ForecastMovementCardError {
    #[error("forecast movement team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("forecast movement card requires a team name")]
    MissingTeamName,
    #[error("movement season {movement} does not match view season {view}")]
    SeasonMismatch { movement: u32, view: u32 },
    #[error("unsupported forecast movement schema: {0}")]
    UnsupportedSchema(String),
    #[error("movement has no row for team {0}")]
    MissingMovementTeam(String),
    #[error("movement source fingerprint is invalid: {0}")]
    InvalidSourceFingerprint(&'static str),
    #[error("serialize forecast movement: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_forecast_movement_card(
    input: ForecastMovementCardInput,
) -> Result<CardDocumentView, ForecastMovementCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(ForecastMovementCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(ForecastMovementCardError::MissingTeamName);
    }
    if input.movement.season != input.view.window.season.0 {
        return Err(ForecastMovementCardError::SeasonMismatch {
            movement: input.movement.season,
            view: input.view.window.season.0,
        });
    }
    if input.movement.schema != TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA {
        return Err(ForecastMovementCardError::UnsupportedSchema(
            input.movement.schema,
        ));
    }
    validate_source_fingerprint("earlier", &input.movement.earlier_fingerprint)?;
    validate_source_fingerprint("later", &input.movement.later_fingerprint)?;
    let row = input
        .movement
        .teams
        .iter()
        .find(|row| row.team == team)
        .ok_or_else(|| ForecastMovementCardError::MissingMovementTeam(team.clone()))?;
    let movement_fingerprint = json_fingerprint(&input.movement)?;
    let earlier_label = input
        .movement
        .earlier_label
        .clone()
        .unwrap_or_else(|| cutoff_label(input.movement.earlier_as_of_date, "Earlier run"));
    let later_label = input
        .movement
        .later_label
        .clone()
        .unwrap_or_else(|| cutoff_label(input.movement.later_as_of_date, "Later run"));
    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert(
        "forecast_movement".to_string(),
        input.movement.schema.clone(),
    );
    methodology_versions.insert(
        "card_projection".to_string(),
        FORECAST_MOVEMENT_CARD_VERSION.to_string(),
    );

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_string(),
        card_kind: CardKind::ForecastMovement,
        document_id: format!(
            "forecast-movement:{}:{}:{}:{}",
            team,
            input.movement.season,
            stable_date(input.movement.earlier_as_of_date, "earlier"),
            stable_date(input.movement.later_as_of_date, "later")
        ),
        fingerprint: String::new(),
        title: format!("{} outlook movement", input.team_name.trim()),
        subtitle: Some(format!(
            "{} → {} · {} trials · seed {}",
            earlier_label, later_label, input.movement.trials, input.movement.seed
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: input.evidence_at,
            evidence_label: EvidenceLabel::Simulated,
            builder_version: FORECAST_MOVEMENT_CARD_VERSION.to_string(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                scenario_comparison_key: Some(format!(
                    "{}:{}",
                    input.movement.earlier_fingerprint, input.movement.later_fingerprint
                )),
                team_ids: vec![team.clone()],
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("icecast-movement".to_string()),
                model_version: Some(input.movement.schema.clone()),
                parameter_fingerprint: Some(movement_fingerprint),
                seed: Some(input.movement.seed),
                trials: Some(u64::from(input.movement.trials)),
            },
        },
        theme: nhl_team_card_theme(&team),
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "shift".to_string(),
                literal_label: "Forecast movement between sealed checkpoints".to_string(),
                display_label: Some("The Shift".to_string()),
                order: 1,
                accessible_summary: format!(
                    "{} projected points, playoff odds, Cup odds, observed standings, and remaining-season movement.",
                    input.team_name.trim()
                ),
                sections: shift_sections(
                    &team,
                    input.team_name.trim(),
                    &earlier_label,
                    &later_label,
                    row,
                ),
            },
            CardPageView {
                id: "insider".to_string(),
                literal_label: "Movement methodology and source authority".to_string(),
                display_label: Some("The Insider".to_string()),
                order: 2,
                accessible_summary:
                    "How the two checkpoint runs were compared and which complete artifacts support the deltas."
                        .to_string(),
                sections: vec![
                    CardSectionView::Methodology(MethodologySectionView {
                        id: "movement-methodology".to_string(),
                        title: "How to read the movement".to_string(),
                        methods: vec![CardMethodologyItemView {
                            key: "sealed-checkpoint-delta".to_string(),
                            label: "Sealed checkpoint delta".to_string(),
                            version: input.movement.schema.clone(),
                            summary: "Every value is later minus earlier. Both complete league artifacts are fingerprinted before this team is selected.".to_string(),
                        }],
                        limitations: input.movement.disclosures.clone(),
                    }),
                    CardSectionView::Provenance(ProvenanceSectionView {
                        id: "movement-sources".to_string(),
                        title: "Source authority".to_string(),
                        provenance_ids: vec!["earlier-run".to_string(), "later-run".to_string()],
                    }),
                ],
            },
        ],
        assets: Vec::new(),
        provenance: vec![
            CardProvenanceView {
                id: "earlier-run".to_string(),
                source: SourceKind::Schedule,
                label: "Earlier sealed IceCast league run".to_string(),
                state: Completeness::Complete,
                observed_at: None,
                fingerprint: Some(input.movement.earlier_fingerprint),
                note: Some(earlier_label),
            },
            CardProvenanceView {
                id: "later-run".to_string(),
                source: SourceKind::Schedule,
                label: "Later sealed IceCast league run".to_string(),
                state: Completeness::Complete,
                observed_at: input.evidence_at,
                fingerprint: Some(input.movement.later_fingerprint),
                note: Some(later_label),
            },
        ],
        warnings: Vec::new(),
        empty_state: None,
    }
    .seal()
    .map_err(|error| ForecastMovementCardError::Document(error.to_string()))
}

fn shift_sections(
    team: &str,
    team_name: &str,
    earlier_label: &str,
    later_label: &str,
    row: &TeamSeasonForecastMovementRow,
) -> Vec<CardSectionView> {
    let mut sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "movement-team".to_string(),
            eyebrow: Some("IceCast forecast movement".to_string()),
            title: team_name.to_string(),
            subtitle: Some(format!("{earlier_label} → {later_label}")),
            identities: vec![CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: team.to_string(),
                label: team_name.to_string(),
                asset_id: None,
            }],
        }),
        CardSectionView::ScenarioBridge(ScenarioBridgeSectionView {
            id: "forecast-shift".to_string(),
            title: "What changed".to_string(),
            from_label: earlier_label.to_string(),
            to_label: later_label.to_string(),
            metrics: vec![
                signed_metric(
                    "projected_points_delta",
                    "Projected points",
                    row.average_points_delta,
                    MetricUnit::Points,
                ),
                signed_metric(
                    "playoff_probability_delta",
                    "Playoff odds",
                    row.playoff_probability_delta * 100.0,
                    MetricUnit::Percentage,
                ),
                signed_metric(
                    "cup_probability_delta",
                    "Stanley Cup odds",
                    row.stanley_cup_probability_delta * 100.0,
                    MetricUnit::Percentage,
                ),
                signed_metric(
                    "longest_streak_delta",
                    "Longest win streak",
                    row.average_longest_win_streak_delta,
                    MetricUnit::Games,
                ),
            ],
            evidence_label: EvidenceLabel::Simulated,
        }),
    ];
    let checkpoint_metrics = [
        row.completed_games_delta.map(|value| {
            signed_metric(
                "completed_games_delta",
                "New games",
                value as f64,
                MetricUnit::Games,
            )
        }),
        row.observed_standings_points_delta.map(|value| {
            signed_metric(
                "observed_points_delta",
                "Observed standings points",
                value as f64,
                MetricUnit::Points,
            )
        }),
        row.expected_remaining_points_delta.map(|value| {
            signed_metric(
                "remaining_points_delta",
                "Expected remaining points",
                value,
                MetricUnit::Points,
            )
        }),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    if !checkpoint_metrics.is_empty() {
        sections.push(CardSectionView::MetricStrip(MetricStripSectionView {
            id: "checkpoint-shift".to_string(),
            title: Some("Actual checkpoint movement".to_string()),
            metrics: checkpoint_metrics,
        }));
    }
    sections
}

fn signed_metric(key: &str, label: &str, value: f64, unit: MetricUnit) -> CardMetricView {
    let suffix = if unit == MetricUnit::Percentage {
        " pp"
    } else {
        ""
    };
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_string()),
            label: label.to_string(),
            value: MetricValue::Decimal(value),
            unit,
            precision: ValuePrecision::TwoDecimals,
            token: None,
        },
        display_text: format!("{value:+.2}{suffix}"),
        accessible_text: format!("{label} changed by {value:+.2}{suffix}"),
        comparison: Some(CardMetricComparisonView {
            label: "later minus earlier".to_string(),
            baseline: MetricValue::Decimal(0.0),
            delta: MetricValue::Decimal(value),
        }),
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn validate_source_fingerprint(
    label: &'static str,
    fingerprint: &str,
) -> Result<(), ForecastMovementCardError> {
    if fingerprint.len() != 64
        || !fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ForecastMovementCardError::InvalidSourceFingerprint(label));
    }
    Ok(())
}

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, ForecastMovementCardError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| ForecastMovementCardError::Serialize(error.to_string()))
}

fn cutoff_label(date: Option<chrono::NaiveDate>, fallback: &str) -> String {
    date.map_or_else(|| fallback.to_string(), |date| date.to_string())
}

fn stable_date(date: Option<chrono::NaiveDate>, fallback: &str) -> String {
    date.map_or_else(
        || fallback.to_string(),
        |date| date.format("%Y%m%d").to_string(),
    )
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;

    use super::*;
    use crate::{model::Season, season_stats::SeasonType, ViewWindow};

    fn movement() -> TeamSeasonForecastMovementView {
        TeamSeasonForecastMovementView {
            schema: TEAM_SEASON_FORECAST_MOVEMENT_SCHEMA.to_string(),
            season: 20262027,
            trials: 1_000,
            seed: 27,
            earlier_label: None,
            later_label: None,
            earlier_as_of_date: Some(NaiveDate::from_ymd_opt(2027, 1, 15).unwrap()),
            later_as_of_date: Some(NaiveDate::from_ymd_opt(2027, 2, 15).unwrap()),
            earlier_fingerprint: "a".repeat(64),
            later_fingerprint: "b".repeat(64),
            teams: vec![TeamSeasonForecastMovementRow {
                team: "NYR".to_string(),
                average_points_delta: 2.5,
                playoff_probability_delta: 0.07,
                stanley_cup_probability_delta: 0.012,
                average_longest_win_streak_delta: 0.4,
                completed_games_delta: Some(14),
                observed_standings_points_delta: Some(19),
                expected_remaining_points_delta: Some(-16.5),
            }],
            disclosures: vec!["Same-season comparison only.".to_string()],
        }
    }

    fn input() -> ForecastMovementCardInput {
        ForecastMovementCardInput {
            movement: movement(),
            focus_team: "nyr".to_string(),
            team_name: "New York Rangers".to_string(),
            view: ViewContext::new(ViewWindow::new(Season(20262027), SeasonType::Regular)),
            evidence_at: None,
        }
    }

    #[test]
    fn movement_card_preserves_both_sealed_runs_and_typed_deltas() {
        let card = build_forecast_movement_card(input()).unwrap();
        let earlier_fingerprint = "a".repeat(64);
        let later_fingerprint = "b".repeat(64);
        assert_eq!(card.card_kind, CardKind::ForecastMovement);
        assert_eq!(card.pages[0].display_label.as_deref(), Some("The Shift"));
        assert_eq!(
            card.provenance[0].fingerprint.as_deref(),
            Some(earlier_fingerprint.as_str())
        );
        assert_eq!(
            card.provenance[1].fingerprint.as_deref(),
            Some(later_fingerprint.as_str())
        );
        let CardSectionView::ScenarioBridge(bridge) = &card.pages[0].sections[1] else {
            panic!("expected movement bridge");
        };
        assert_eq!(bridge.metrics.len(), 4);
        assert_eq!(bridge.metrics[1].display_text, "+7.00 pp");
        card.validate().unwrap();
    }

    #[test]
    fn movement_card_rejects_unsealed_source_fingerprint() {
        let mut input = input();
        input.movement.later_fingerprint = "not-sealed".to_string();
        assert_eq!(
            build_forecast_movement_card(input),
            Err(ForecastMovementCardError::InvalidSourceFingerprint("later"))
        );
    }

    #[test]
    fn movement_card_uses_explicit_scenario_labels_without_renderer_math() {
        let mut input = input();
        input.movement.earlier_label = Some("July baseline".to_owned());
        input.movement.later_label = Some("Preseason edge v1".to_owned());
        let card = build_forecast_movement_card(input).unwrap();
        assert_eq!(
            card.subtitle.as_deref(),
            Some("July baseline → Preseason edge v1 · 1000 trials · seed 27")
        );
        let CardSectionView::ScenarioBridge(bridge) = &card.pages[0].sections[1] else {
            panic!("expected movement bridge");
        };
        assert_eq!(bridge.from_label, "July baseline");
        assert_eq!(bridge.to_label, "Preseason edge v1");
    }
}
