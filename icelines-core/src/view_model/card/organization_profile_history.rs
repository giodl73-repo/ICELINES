//! UI-neutral focused-team projection of a standing profile-history delta.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::*;
use crate::model::Season;
use crate::season_stats::SeasonType;
use crate::teams::CANONICAL_TEAMS;
use crate::view_model::{
    seal_organization_profile_history_delta, Completeness, EvidenceLabel, MetricCell, MetricUnit,
    MetricValue, OrganizationProfileHistoryChange, OrganizationProfileHistoryDeltaView,
    SemanticToken, SourceKind, StatKey, ValuePrecision, ViewContext, ViewWindow,
    ORGANIZATION_PROFILE_HISTORY_DELTA_SCHEMA,
};

pub const ORGANIZATION_PROFILE_HISTORY_CARD_VERSION: &str = "organization_profile_history_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationProfileHistoryCardInput {
    pub delta: OrganizationProfileHistoryDeltaView,
    pub focus_team: String,
    pub team_name: String,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationProfileHistoryCardError {
    #[error("organization profile history team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("organization profile history card requires a team name")]
    MissingTeamName,
    #[error("unsupported organization profile history delta schema: {0}")]
    UnsupportedSchema(String),
    #[error("organization profile history delta has no row for {0}")]
    MissingTeam(String),
    #[error("organization profile history delta is invalid: {0}")]
    InvalidDelta(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

pub fn project_organization_profile_history_card(
    delta: OrganizationProfileHistoryDeltaView,
    focus_team: &str,
    team_name: Option<&str>,
    evidence_at: Option<DateTime<Utc>>,
) -> Result<CardDocumentView, OrganizationProfileHistoryCardError> {
    let team = focus_team.trim().to_ascii_uppercase();
    let canonical_name = CANONICAL_TEAMS
        .iter()
        .find(|(abbreviation, _)| *abbreviation == team)
        .map(|(_, name)| *name)
        .ok_or_else(|| OrganizationProfileHistoryCardError::InvalidTeam(team.clone()))?;
    build_organization_profile_history_card(OrganizationProfileHistoryCardInput {
        delta,
        focus_team: team,
        team_name: team_name.unwrap_or(canonical_name).to_owned(),
        evidence_at,
    })
}

pub fn build_organization_profile_history_card(
    mut input: OrganizationProfileHistoryCardInput,
) -> Result<CardDocumentView, OrganizationProfileHistoryCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if !CANONICAL_TEAMS
        .iter()
        .any(|(abbreviation, _)| *abbreviation == team)
    {
        return Err(OrganizationProfileHistoryCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(OrganizationProfileHistoryCardError::MissingTeamName);
    }
    if input.delta.schema != ORGANIZATION_PROFILE_HISTORY_DELTA_SCHEMA {
        return Err(OrganizationProfileHistoryCardError::UnsupportedSchema(
            input.delta.schema,
        ));
    }
    input.delta = seal_organization_profile_history_delta(input.delta)
        .map_err(|error| OrganizationProfileHistoryCardError::InvalidDelta(error.to_string()))?;
    let organization = input
        .delta
        .organizations
        .iter()
        .find(|row| row.organization == team)
        .ok_or_else(|| OrganizationProfileHistoryCardError::MissingTeam(team.clone()))?;
    let completeness = if organization.comparable_profiles == input.delta.comparable_profiles
        && organization.comparable_profiles > 0
    {
        Completeness::Complete
    } else if organization.comparable_profiles > 0 {
        Completeness::Partial
    } else {
        Completeness::Unavailable
    };
    let evidence_label = if completeness == Completeness::Complete {
        EvidenceLabel::Confirmed
    } else {
        EvidenceLabel::UnderReview
    };
    let mut view = ViewContext::new(ViewWindow::new(
        Season(input.delta.later.season),
        SeasonType::Regular,
    ));
    view.generated_at = input.evidence_at;
    view.completeness = completeness;

    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert(
        "profile_history_delta".to_owned(),
        ORGANIZATION_PROFILE_HISTORY_DELTA_SCHEMA.to_owned(),
    );
    methodology_versions.insert(
        "card_projection".to_owned(),
        ORGANIZATION_PROFILE_HISTORY_CARD_VERSION.to_owned(),
    );

    let mut sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "window-history-identity".to_owned(),
            eyebrow: Some(format!(
                "{} → {} · {} comparable profiles",
                input.delta.earlier.season,
                input.delta.later.season,
                organization.comparable_profiles
            )),
            title: input.team_name.trim().to_owned(),
            subtitle: Some(format!(
                "{} improving · {} declining · {} unchanged",
                organization.improved_profiles,
                organization.declined_profiles,
                organization.unchanged_profiles
            )),
            identities: vec![CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: team.clone(),
                label: input.team_name.trim().to_owned(),
                asset_id: None,
            }],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "window-history-breadth".to_owned(),
            title: Some("Observed movement breadth".to_owned()),
            metrics: vec![
                count_metric(
                    "history.improved",
                    "Improved",
                    organization.improved_profiles,
                    SemanticToken::Rising,
                    evidence_label,
                ),
                count_metric(
                    "history.declined",
                    "Declined",
                    organization.declined_profiles,
                    SemanticToken::Risk,
                    evidence_label,
                ),
                count_metric(
                    "history.unchanged",
                    "Unchanged",
                    organization.unchanged_profiles,
                    SemanticToken::QuietMetadata,
                    evidence_label,
                ),
            ],
        }),
    ];
    sections.extend(organization.profiles.iter().map(|profile| {
        CardSectionView::ScenarioBridge(ScenarioBridgeSectionView {
            id: format!(
                "window-history-profile-{}",
                profile.profile_key.replace('.', "-")
            ),
            title: profile
                .label
                .clone()
                .unwrap_or_else(|| profile.profile_key.clone()),
            from_label: format!(
                "{} · {}",
                input.delta.earlier.season, input.delta.earlier.as_of
            ),
            to_label: format!("{} · {}", input.delta.later.season, input.delta.later.as_of),
            metrics: vec![profile_metric(profile, evidence_label)],
            evidence_label,
        })
    }));

    let methods = organization
        .profiles
        .iter()
        .map(|profile| CardMethodologyItemView {
            key: profile.profile_key.clone(),
            label: profile
                .label
                .clone()
                .unwrap_or_else(|| profile.profile_key.clone()),
            version: profile.method_version.clone(),
            summary: format!(
                "Raw unit {} · {:?} · directional change {}",
                profile.raw_unit,
                profile.direction,
                profile
                    .directional_delta
                    .map(|value| format!("{value:+.3}"))
                    .unwrap_or_else(|| "not comparable".to_owned())
            ),
        })
        .collect();
    let methodology = vec![
        CardSectionView::Methodology(MethodologySectionView {
            id: "window-history-methodology".to_owned(),
            title: "Methods and limitations".to_owned(),
            methods,
            limitations: input.delta.disclosures.clone(),
        }),
        CardSectionView::Provenance(ProvenanceSectionView {
            id: "window-history-provenance".to_owned(),
            title: "Sealed standing history".to_owned(),
            provenance_ids: vec![
                "profile-history-delta".to_owned(),
                "profile-history".to_owned(),
            ],
        }),
    ];

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_owned(),
        card_kind: CardKind::OrganizationWindow,
        document_id: format!(
            "organization-window-history:{}:{}:{}",
            team, input.delta.earlier.season, input.delta.later.season
        ),
        fingerprint: String::new(),
        title: format!("{} organization history", input.team_name.trim()),
        subtitle: Some(format!(
            "{} profiles improved · {} declined",
            organization.improved_profiles, organization.declined_profiles
        )),
        context: CardContextView {
            view,
            evidence_at: input.evidence_at,
            evidence_label,
            builder_version: ORGANIZATION_PROFILE_HISTORY_CARD_VERSION.to_owned(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                team_ids: vec![team.clone()],
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("organization-profile-history-delta".to_owned()),
                model_version: Some(ORGANIZATION_PROFILE_HISTORY_DELTA_SCHEMA.to_owned()),
                parameter_fingerprint: Some(input.delta.fingerprint.clone()),
                seed: None,
                trials: None,
            },
        },
        theme: nhl_team_card_theme(&team),
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "window-history".to_owned(),
                literal_label: "Observed organization profile movement".to_owned(),
                display_label: Some("The Shift".to_owned()),
                order: 1,
                accessible_summary: format!(
                    "{} observed profile movement from {} to {}.",
                    input.team_name.trim(), input.delta.earlier.season, input.delta.later.season
                ),
                sections,
            },
            CardPageView {
                id: "window-history-insider".to_owned(),
                literal_label: "History methodology and provenance".to_owned(),
                display_label: Some("The Insider".to_owned()),
                order: 2,
                accessible_summary:
                    "Exact profile methods, raw units, direction semantics, limitations, and sealed history identity."
                        .to_owned(),
                sections: methodology,
            },
        ],
        assets: Vec::new(),
        provenance: vec![
            CardProvenanceView {
                id: "profile-history-delta".to_owned(),
                source: SourceKind::Bundle,
                label: format!(
                    "Exact checkpoints {} and {}",
                    input.delta.earlier.as_of, input.delta.later.as_of
                ),
                state: completeness,
                observed_at: input.evidence_at,
                fingerprint: Some(input.delta.fingerprint),
                note: Some("Direction-aware raw profile movement".to_owned()),
            },
            CardProvenanceView {
                id: "profile-history".to_owned(),
                source: SourceKind::Bundle,
                label: input.delta.history_id,
                state: completeness,
                observed_at: input.evidence_at,
                fingerprint: Some(input.delta.history_fingerprint),
                note: Some("Sealed source observation ledger".to_owned()),
            },
        ],
        warnings: Vec::new(),
        empty_state: None,
    }
    .seal()
    .map_err(|error| OrganizationProfileHistoryCardError::Document(error.to_string()))
}

fn count_metric(
    key: &str,
    label: &str,
    value: usize,
    token: SemanticToken,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Integer(value as i64),
            unit: MetricUnit::Count,
            precision: ValuePrecision::Integer,
            token: Some(token),
        },
        display_text: value.to_string(),
        accessible_text: format!("{label} profiles {value}"),
        comparison: None,
        evidence_label,
    }
}

fn profile_metric(
    profile: &crate::view_model::OrganizationProfileHistoryProfileDeltaView,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let label = match profile.change {
        OrganizationProfileHistoryChange::Improved => "Improved",
        OrganizationProfileHistoryChange::Declined => "Declined",
        OrganizationProfileHistoryChange::Unchanged => "Unchanged",
        OrganizationProfileHistoryChange::NotComparable => "Not comparable",
    };
    let token = match profile.change {
        OrganizationProfileHistoryChange::Improved => SemanticToken::Rising,
        OrganizationProfileHistoryChange::Declined => SemanticToken::Risk,
        OrganizationProfileHistoryChange::Unchanged => SemanticToken::QuietMetadata,
        OrganizationProfileHistoryChange::NotComparable => SemanticToken::Warning,
    };
    let display = profile
        .earlier_raw_value
        .zip(profile.later_raw_value)
        .map(|(earlier, later)| format!("{earlier:.3} → {later:.3} · {label}"))
        .unwrap_or_else(|| label.to_owned());
    CardMetricView {
        metric: MetricCell {
            key: StatKey(format!("history.profile.{}.delta", profile.profile_key)),
            label: format!("Change ({})", profile.raw_unit),
            value: profile
                .directional_delta
                .map_or(MetricValue::Missing, MetricValue::Decimal),
            unit: MetricUnit::None,
            precision: ValuePrecision::ThreeDecimals,
            token: Some(token),
        },
        display_text: display.clone(),
        accessible_text: format!(
            "{} {}; favorable-direction delta {} {}",
            profile.label.as_deref().unwrap_or(&profile.profile_key),
            display,
            profile
                .directional_delta
                .map(|value| format!("{value:+.3}"))
                .unwrap_or_else(|| "unavailable".to_owned()),
            profile.raw_unit
        ),
        comparison: profile
            .earlier_raw_value
            .zip(profile.raw_delta)
            .map(|(baseline, delta)| CardMetricComparisonView {
                label: "raw observed change".to_owned(),
                baseline: MetricValue::Decimal(baseline),
                delta: MetricValue::Decimal(delta),
            }),
        evidence_label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn delta() -> OrganizationProfileHistoryDeltaView {
        serde_json::from_str(include_str!(
            "../../../../examples/organization-profile-history-delta-2024-25-to-2025-26.json"
        ))
        .unwrap()
    }

    #[test]
    fn rangers_and_kraken_history_cards_are_sealed_and_team_specific() {
        let nyr = project_organization_profile_history_card(delta(), "NYR", None, None).unwrap();
        let sea = project_organization_profile_history_card(delta(), "SEA", None, None).unwrap();
        assert_eq!(nyr.pages[0].display_label.as_deref(), Some("The Shift"));
        assert_eq!(nyr.context.joins.team_ids, ["NYR"]);
        assert_eq!(sea.context.joins.team_ids, ["SEA"]);
        assert_ne!(nyr.fingerprint, sea.fingerprint);
        assert!(nyr
            .subtitle
            .as_deref()
            .unwrap()
            .contains("1 profiles improved"));
        assert!(sea
            .subtitle
            .as_deref()
            .unwrap()
            .contains("4 profiles improved"));
    }

    #[test]
    fn checked_in_history_cards_round_trip_through_the_shared_document() {
        for json in [
            include_str!(
                "../../../../examples/organization-profile-history-card-nyr-2024-25-to-2025-26.json"
            ),
            include_str!(
                "../../../../examples/organization-profile-history-card-sea-2024-25-to-2025-26.json"
            ),
        ] {
            let card = parse_card_document(json).unwrap();
            assert_eq!(card.card_kind, CardKind::OrganizationWindow);
            assert_eq!(card.pages.len(), 2);
            assert_eq!(
                card.context.builder_version,
                ORGANIZATION_PROFILE_HISTORY_CARD_VERSION
            );
        }
    }
}
