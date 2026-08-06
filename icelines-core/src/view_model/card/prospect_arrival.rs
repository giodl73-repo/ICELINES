//! Focused UI-neutral projection of a league prospect-arrival calibration.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use super::*;
use crate::view_model::{
    Completeness, EvidenceLabel, MetricCell, MetricUnit, MetricValue,
    ProspectArrivalLeagueCalibrationView, ProspectArrivalLeagueTeamView, SemanticToken, SourceKind,
    StatKey, ValuePrecision, ViewContext, ViewWarning, WarningKind,
    PROSPECT_ARRIVAL_LEAGUE_CALIBRATION_SCHEMA,
};

pub const PROSPECT_ARRIVAL_CARD_VERSION: &str = "prospect_arrival_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProspectArrivalCardInput {
    /// Complete league artifact. It is fingerprinted before team selection.
    pub arrival: ProspectArrivalLeagueCalibrationView,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProspectArrivalCardError {
    #[error("prospect arrival team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("prospect arrival card requires a team name")]
    MissingTeamName,
    #[error("arrival season {arrival} does not match view season {view}")]
    SeasonMismatch { arrival: u32, view: u32 },
    #[error("unsupported prospect arrival schema: {0}")]
    UnsupportedSchema(String),
    #[error("arrival artifact has no row for team {0}")]
    MissingTeam(String),
    #[error("source-package fingerprint is not SHA-256")]
    InvalidSourceFingerprint,
    #[error("serialize prospect arrival artifact: {0}")]
    Serialize(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn build_prospect_arrival_card(
    input: ProspectArrivalCardInput,
) -> Result<CardDocumentView, ProspectArrivalCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if team.len() != 3 || !team.bytes().all(|byte| byte.is_ascii_alphabetic()) {
        return Err(ProspectArrivalCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(ProspectArrivalCardError::MissingTeamName);
    }
    if input.arrival.schema != PROSPECT_ARRIVAL_LEAGUE_CALIBRATION_SCHEMA {
        return Err(ProspectArrivalCardError::UnsupportedSchema(
            input.arrival.schema,
        ));
    }
    if input.arrival.forecast_season != input.view.window.season.0 {
        return Err(ProspectArrivalCardError::SeasonMismatch {
            arrival: input.arrival.forecast_season,
            view: input.view.window.season.0,
        });
    }
    let team_row = input
        .arrival
        .teams
        .iter()
        .find(|row| row.organization == team)
        .ok_or_else(|| ProspectArrivalCardError::MissingTeam(team.clone()))?;
    if input
        .arrival
        .population_authority
        .as_ref()
        .is_some_and(|authority| !is_sha256(&authority.source_package_fingerprint))
    {
        return Err(ProspectArrivalCardError::InvalidSourceFingerprint);
    }

    let league_fingerprint = json_fingerprint(&input.arrival)?;
    let authority = input.arrival.population_authority.as_ref();
    let authority_complete = authority.is_some_and(|authority| authority.population_complete);
    let evidence_label = if authority_complete {
        EvidenceLabel::Confirmed
    } else {
        EvidenceLabel::UnderReview
    };
    let mut methods = BTreeMap::new();
    methods.insert("prospect_arrival".to_owned(), input.arrival.schema.clone());
    methods.insert(
        "card_projection".to_owned(),
        PROSPECT_ARRIVAL_CARD_VERSION.to_owned(),
    );
    let mut provenance = vec![CardProvenanceView {
        id: "league-arrival-calibration".to_owned(),
        source: SourceKind::Career,
        label: "Sealed league prospect-arrival calibration".to_owned(),
        state: Completeness::Complete,
        observed_at: input.evidence_at,
        fingerprint: Some(league_fingerprint.clone()),
        note: Some(format!(
            "{} targets across {} organizations; focused only after the league artifact was fingerprinted",
            input.arrival.target_skaters, input.arrival.organizations_represented
        )),
    }];
    if let Some(authority) = authority {
        provenance.push(CardProvenanceView {
            id: "prospect-population-authority".to_owned(),
            source: SourceKind::Snapshot,
            label: "Prospect population and current-control authority".to_owned(),
            state: if authority.population_complete {
                Completeness::Complete
            } else {
                Completeness::Partial
            },
            observed_at: input.evidence_at,
            fingerprint: Some(authority.source_package_fingerprint.clone()),
            note: Some(format!(
                "{} of {} supplied skater studies retained; {} control exclusions",
                authority.controlled_studies,
                authority.supplied_studies,
                authority.control_exclusions
            )),
        });
    }
    let warnings = authority_warnings(authority);
    let player_ids = team_row
        .calibrations
        .iter()
        .map(|row| row.player_id.to_string())
        .chain(
            team_row
                .exclusions
                .iter()
                .map(|row| row.player_id.to_string()),
        )
        .collect();

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_owned(),
        card_kind: CardKind::ProspectArrival,
        document_id: format!("prospect-arrival:{}:{}", team, input.arrival.forecast_season),
        fingerprint: String::new(),
        title: format!("{} prospect arrivals", input.team_name.trim()),
        subtitle: Some(format!(
            "{} forecast · {}/{} calibrated",
            input.arrival.forecast_season, team_row.calibrated_skaters, team_row.target_skaters
        )),
        context: CardContextView {
            view: input.view,
            evidence_at: input.evidence_at,
            evidence_label,
            builder_version: PROSPECT_ARRIVAL_CARD_VERSION.to_owned(),
            methodology_versions: methods,
            joins: CardIdentityJoinsView {
                team_ids: vec![team.clone()],
                player_ids,
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("prospect-arrival-calibration".to_owned()),
                model_version: Some(input.arrival.schema.clone()),
                parameter_fingerprint: Some(league_fingerprint),
                seed: None,
                trials: None,
            },
        },
        theme: nhl_team_card_theme(&team),
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "depth-chart".to_owned(),
                literal_label: "Prospect arrival probabilities".to_owned(),
                display_label: Some("The Depth Chart".to_owned()),
                order: 1,
                accessible_summary: format!(
                    "{} calibrated prospect arrivals and exclusions for {}.",
                    team_row.calibrated_skaters,
                    input.team_name.trim()
                ),
                sections: depth_chart_sections(&team, input.team_name.trim(), team_row),
            },
            CardPageView {
                id: "insider".to_owned(),
                literal_label: "Population authority and calibration methodology".to_owned(),
                display_label: Some("The Insider".to_owned()),
                order: 2,
                accessible_summary:
                    "Current-control coverage, excluded players, historical calibration method, and limitations."
                        .to_owned(),
                sections: insider_sections(&input.arrival, team_row, authority),
            },
        ],
        assets: Vec::new(),
        provenance,
        warnings,
        empty_state: None,
    }
    .seal()
    .map_err(|error| ProspectArrivalCardError::Document(error.to_string()))
}

fn depth_chart_sections(
    team: &str,
    team_name: &str,
    row: &ProspectArrivalLeagueTeamView,
) -> Vec<CardSectionView> {
    let mut sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "arrival-team".to_owned(),
            eyebrow: Some("Prospect arrival forecast".to_owned()),
            title: team_name.to_owned(),
            subtitle: Some(team.to_owned()),
            identities: vec![CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: team.to_owned(),
                label: team_name.to_owned(),
                asset_id: None,
            }],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "arrival-coverage".to_owned(),
            title: Some("Forecast coverage".to_owned()),
            metrics: vec![
                count_metric("target_skaters", "Targets", row.target_skaters),
                count_metric("calibrated_skaters", "Calibrated", row.calibrated_skaters),
                count_metric("excluded_skaters", "Excluded", row.excluded_skaters),
            ],
        }),
    ];
    if row.calibrations.is_empty() {
        sections.push(CardSectionView::StateNotice(StateNoticeSectionView {
            id: "no-calibrated-arrivals".to_owned(),
            title: "No calibrated skater arrivals".to_owned(),
            detail: Some(
                "Review the exclusion ledger and population authority before interpreting this as an empty prospect pool."
                    .to_owned(),
            ),
            empty_state: None,
            warnings: Vec::new(),
            token: SemanticToken::Info,
        }));
    } else {
        let mut calibrations = row.calibrations.iter().collect::<Vec<_>>();
        calibrations.sort_by(|left, right| {
            right
                .horizon_adjusted_arrival_probability
                .unwrap_or(right.calibrated_arrival_probability)
                .total_cmp(
                    &left
                        .horizon_adjusted_arrival_probability
                        .unwrap_or(left.calibrated_arrival_probability),
                )
                .then_with(|| left.player_id.cmp(&right.player_id))
        });
        sections.push(CardSectionView::PlayerList(PlayerListSectionView {
            id: "calibrated-arrivals".to_owned(),
            title: "Calibrated arrival outlook".to_owned(),
            rows: calibrations
                .into_iter()
                .map(|calibration| {
                    let arrival = calibration
                        .horizon_adjusted_arrival_probability
                        .unwrap_or(calibration.calibrated_arrival_probability);
                    let mut metrics = vec![
                        probability_metric("arrival_probability", "Arrival", arrival),
                        decimal_metric(
                            "observed_signal",
                            "Signal",
                            calibration.observed_signal_score,
                        ),
                    ];
                    if let Some(established) = calibration
                        .horizon_adjusted_established_probability
                        .or(calibration.calibrated_established_probability)
                    {
                        metrics.insert(
                            1,
                            probability_metric(
                                "establishment_probability",
                                "Established role",
                                established,
                            ),
                        );
                    }
                    CardPlayerRowView {
                        player_id: calibration.player_id.to_string(),
                        name: calibration.player.clone(),
                        role: Some(calibration.position_group.clone()),
                        asset_id: None,
                        metrics,
                        tokens: vec![SemanticToken::Rising],
                        evidence_label: EvidenceLabel::Simulated,
                    }
                })
                .collect(),
        }));
    }
    sections
}

fn insider_sections(
    view: &ProspectArrivalLeagueCalibrationView,
    row: &ProspectArrivalLeagueTeamView,
    authority: Option<&crate::view_model::ProspectArrivalLeaguePopulationAuthorityView>,
) -> Vec<CardSectionView> {
    let (title, detail, token) = match authority {
        Some(authority) if authority.population_complete => (
            "Complete population authority",
            format!(
                "{} of {} supplied skater studies passed current-control gating; {} were excluded.",
                authority.controlled_studies,
                authority.supplied_studies,
                authority.control_exclusions
            ),
            SemanticToken::SourceComplete,
        ),
        Some(authority) => (
            "Incomplete population authority",
            format!(
                "{} of {} supplied skater studies passed current-control gating; this remains an auditable partial population.",
                authority.controlled_studies, authority.supplied_studies
            ),
            SemanticToken::SourcePartial,
        ),
        None => (
            "Coverage draft",
            "No sealed population authority was supplied. Camp-derived targets do not establish a complete controlled prospect pool."
                .to_owned(),
            SemanticToken::SourceUnavailable,
        ),
    };
    let mut sections = vec![CardSectionView::StateNotice(StateNoticeSectionView {
        id: "population-authority".to_owned(),
        title: title.to_owned(),
        detail: Some(detail),
        empty_state: None,
        warnings: authority_warnings(authority),
        token,
    })];
    if !row.exclusions.is_empty() {
        sections.push(CardSectionView::PlayerList(PlayerListSectionView {
            id: "arrival-exclusions".to_owned(),
            title: "Exclusion ledger".to_owned(),
            rows: row
                .exclusions
                .iter()
                .map(|exclusion| CardPlayerRowView {
                    player_id: exclusion.player_id.to_string(),
                    name: exclusion.player.clone(),
                    role: Some("Not calibrated".to_owned()),
                    asset_id: None,
                    metrics: vec![text_metric("exclusion_reason", "Reason", &exclusion.reason)],
                    tokens: vec![SemanticToken::Warning],
                    evidence_label: EvidenceLabel::UnderReview,
                })
                .collect(),
        }));
    }
    sections.push(CardSectionView::Methodology(MethodologySectionView {
        id: "arrival-methodology".to_owned(),
        title: "How to read the forecast".to_owned(),
        methods: vec![CardMethodologyItemView {
            key: "frozen-neighbor-calibration".to_owned(),
            label: "Frozen same-position neighbor calibration".to_owned(),
            version: view.schema.clone(),
            summary: "Every eligible skater uses the same historical conversion cohort, shrinkage prior, distance gate, and forecast-horizon adjustment."
                .to_owned(),
        }],
        limitations: view.disclosures.clone(),
    }));
    let mut provenance_ids = vec!["league-arrival-calibration".to_owned()];
    if authority.is_some() {
        provenance_ids.push("prospect-population-authority".to_owned());
    }
    sections.push(CardSectionView::Provenance(ProvenanceSectionView {
        id: "arrival-sources".to_owned(),
        title: "Source authority".to_owned(),
        provenance_ids,
    }));
    sections
}

fn authority_warnings(
    authority: Option<&crate::view_model::ProspectArrivalLeaguePopulationAuthorityView>,
) -> Vec<ViewWarning> {
    match authority {
        Some(authority) if authority.population_complete => Vec::new(),
        Some(_) => vec![ViewWarning {
            kind: WarningKind::PartialSource,
            source: Some(SourceKind::Snapshot),
            message: "Prospect population authority is incomplete; do not publish this as a complete organizational census."
                .to_owned(),
            recovery: Vec::new(),
        }],
        None => vec![ViewWarning {
            kind: WarningKind::MissingSource,
            source: Some(SourceKind::Snapshot),
            message: "No sealed prospect population source package was supplied; this is a coverage draft."
                .to_owned(),
            recovery: Vec::new(),
        }],
    }
}

fn count_metric(key: &str, label: &str, value: usize) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Integer(value as i64),
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

fn probability_metric(key: &str, label: &str, value: f64) -> CardMetricView {
    let percentage = value * 100.0;
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Decimal(percentage),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::PercentOneDecimal,
            token: None,
        },
        display_text: format!("{percentage:.1}%"),
        accessible_text: format!("{label} {percentage:.1} percent"),
        comparison: None,
        evidence_label: EvidenceLabel::Simulated,
    }
}

fn decimal_metric(key: &str, label: &str, value: f64) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Decimal(value),
            unit: MetricUnit::Score,
            precision: ValuePrecision::TwoDecimals,
            token: None,
        },
        display_text: format!("{value:.2}"),
        accessible_text: format!("{label} {value:.2}"),
        comparison: None,
        evidence_label: EvidenceLabel::Estimated,
    }
}

fn text_metric(key: &str, label: &str, value: &str) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Text(value.to_owned()),
            unit: MetricUnit::None,
            precision: ValuePrecision::Raw,
            token: None,
        },
        display_text: value.to_owned(),
        accessible_text: format!("{label}: {value}"),
        comparison: None,
        evidence_label: EvidenceLabel::UnderReview,
    }
}

fn json_fingerprint<T: Serialize>(value: &T) -> Result<String, ProspectArrivalCardError> {
    serde_json::to_vec(value)
        .map(|bytes| format!("{:x}", Sha256::digest(bytes)))
        .map_err(|error| ProspectArrivalCardError::Serialize(error.to_string()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{model::Season, season_stats::SeasonType, ViewWindow};

    fn arrival() -> ProspectArrivalLeagueCalibrationView {
        serde_json::from_str(include_str!(
            "../../../../examples/icecast-prospect-arrival-league-2026-27.json"
        ))
        .unwrap()
    }

    fn input(arrival: ProspectArrivalLeagueCalibrationView) -> ProspectArrivalCardInput {
        ProspectArrivalCardInput {
            arrival,
            focus_team: "NYR".to_owned(),
            team_name: "New York Rangers".to_owned(),
            view: ViewContext::new(ViewWindow::new(Season(20_262_027), SeasonType::Regular)),
            evidence_at: None,
        }
    }

    #[test]
    fn card_retains_league_seal_and_labels_camp_draft_authority() {
        let card = build_prospect_arrival_card(input(arrival())).unwrap();

        assert_eq!(card.card_kind, CardKind::ProspectArrival);
        assert_eq!(
            card.pages[0].display_label.as_deref(),
            Some("The Depth Chart")
        );
        assert_eq!(card.pages[1].display_label.as_deref(), Some("The Insider"));
        assert_eq!(card.context.joins.team_ids, vec!["NYR"]);
        assert_eq!(card.provenance.len(), 1);
        assert_eq!(card.warnings.len(), 1);
        assert_eq!(card.warnings[0].kind, WarningKind::MissingSource);
        card.validate().unwrap();
    }

    #[test]
    fn complete_population_authority_is_fingerprinted_without_warning() {
        let mut arrival = arrival();
        arrival.population_authority = Some(
            crate::view_model::ProspectArrivalLeaguePopulationAuthorityView {
                source_package_fingerprint: "a".repeat(64),
                population_complete: true,
                supplied_studies: arrival.target_skaters,
                controlled_studies: arrival.target_skaters,
                control_exclusions: 0,
            },
        );

        let card = build_prospect_arrival_card(input(arrival)).unwrap();
        assert!(card.warnings.is_empty());
        assert_eq!(card.provenance.len(), 2);
        assert_eq!(card.provenance[1].state, Completeness::Complete);
        assert_eq!(
            card.provenance[1].fingerprint.as_deref(),
            Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        );
    }
}
