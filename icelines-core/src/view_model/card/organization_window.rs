//! UI-neutral focused-team projection of a sealed organization Window board.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::*;
use crate::teams::CANONICAL_TEAMS;
use crate::view_model::{
    load_organization_window_profile_inventory, validate_organization_window_board, Completeness,
    EvidenceLabel, MetricCell, MetricUnit, MetricValue, OrganizationWindowBoardView,
    OrganizationWindowError, SemanticToken, SourceKind, StatKey, ValuePrecision, ViewContext,
    ViewWindow, WindowRankState, ORGANIZATION_WINDOW_BOARD_SCHEMA,
};

pub const ORGANIZATION_WINDOW_CARD_VERSION: &str = "organization_window_card.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrganizationWindowCardInput {
    pub board: OrganizationWindowBoardView,
    pub focus_team: String,
    pub team_name: String,
    pub view: ViewContext,
    pub evidence_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum OrganizationWindowCardError {
    #[error("organization Window team abbreviation is invalid: {0}")]
    InvalidTeam(String),
    #[error("organization Window card requires a team name")]
    MissingTeamName,
    #[error("organization Window season {board} does not match view season {view}")]
    SeasonMismatch { board: u32, view: u32 },
    #[error("unsupported organization Window schema: {0}")]
    UnsupportedSchema(String),
    #[error("organization Window board has no row for {0}")]
    MissingTeam(String),
    #[error("organization Window board fingerprint is invalid")]
    InvalidFingerprint,
    #[error("organization Window board validation failed: {0}")]
    InvalidBoard(String),
    #[error("card document validation failed: {0}")]
    Document(String),
}

/// Project one canonical team from a sealed league Window board using the
/// shared card context and canonical team identity. Renderers should call this
/// helper instead of carrying their own team-name or completeness logic.
pub fn project_organization_window_card(
    board: OrganizationWindowBoardView,
    focus_team: &str,
    team_name: Option<&str>,
    evidence_at: Option<DateTime<Utc>>,
) -> Result<CardDocumentView, OrganizationWindowCardError> {
    let team = focus_team.trim().to_ascii_uppercase();
    let canonical_name = CANONICAL_TEAMS
        .iter()
        .find(|(abbreviation, _)| *abbreviation == team)
        .map(|(_, name)| *name)
        .ok_or_else(|| OrganizationWindowCardError::InvalidTeam(team.clone()))?;
    let mut view = ViewContext::new(ViewWindow::new(
        crate::model::Season(board.season),
        crate::season_stats::SeasonType::Regular,
    ));
    view.generated_at = evidence_at;
    build_organization_window_card(OrganizationWindowCardInput {
        board,
        focus_team: team,
        team_name: team_name.unwrap_or(canonical_name).to_owned(),
        view,
        evidence_at,
    })
}

pub fn build_organization_window_card(
    input: OrganizationWindowCardInput,
) -> Result<CardDocumentView, OrganizationWindowCardError> {
    let team = input.focus_team.trim().to_ascii_uppercase();
    if !CANONICAL_TEAMS
        .iter()
        .any(|(abbreviation, _)| *abbreviation == team)
    {
        return Err(OrganizationWindowCardError::InvalidTeam(team));
    }
    if input.team_name.trim().is_empty() {
        return Err(OrganizationWindowCardError::MissingTeamName);
    }
    if input.board.schema != ORGANIZATION_WINDOW_BOARD_SCHEMA {
        return Err(OrganizationWindowCardError::UnsupportedSchema(
            input.board.schema,
        ));
    }
    if input.board.season != input.view.window.season.0 {
        return Err(OrganizationWindowCardError::SeasonMismatch {
            board: input.board.season,
            view: input.view.window.season.0,
        });
    }
    let inventory = load_organization_window_profile_inventory()
        .map_err(|error| OrganizationWindowCardError::InvalidBoard(error.to_string()))?;
    validate_organization_window_board(&input.board, &inventory).map_err(|error| match error {
        OrganizationWindowError::BoardFingerprintMismatch
        | OrganizationWindowError::ManifestFingerprintMismatch => {
            OrganizationWindowCardError::InvalidFingerprint
        }
        error => OrganizationWindowCardError::InvalidBoard(error.to_string()),
    })?;
    let organization = input
        .board
        .organization(&team)
        .ok_or_else(|| OrganizationWindowCardError::MissingTeam(team.clone()))?;
    let evidence_label = if organization.overall.rank_status.state == WindowRankState::Ranked {
        EvidenceLabel::Estimated
    } else {
        EvidenceLabel::UnderReview
    };
    let completeness = if organization.overall.coverage >= 0.999 {
        Completeness::Complete
    } else if organization.overall.coverage > 0.0 {
        Completeness::Partial
    } else {
        Completeness::Unavailable
    };
    let mut view = input.view.clone();
    view.completeness = completeness;
    let mut methodology_versions = BTreeMap::new();
    methodology_versions.insert(
        "organization_window".to_owned(),
        input.board.manifest.manifest_version.clone(),
    );
    methodology_versions.insert(
        "classification".to_owned(),
        input.board.manifest.classification_method.clone(),
    );
    methodology_versions.insert(
        "card_projection".to_owned(),
        ORGANIZATION_WINDOW_CARD_VERSION.to_owned(),
    );

    let mut dimension_sections = Vec::new();
    for dimension in &organization.dimensions {
        dimension_sections.push(CardSectionView::MetricStrip(MetricStripSectionView {
            id: format!("dimension-{}", dimension.key),
            title: Some(dimension.label.clone()),
            metrics: vec![
                optional_metric(
                    &format!("window.dimension.{}.score", dimension.key),
                    "Score",
                    dimension.score,
                    MetricUnit::Score,
                    evidence_label,
                ),
                percent_metric(
                    &format!("window.dimension.{}.confidence", dimension.key),
                    "Confidence",
                    dimension.confidence,
                    evidence_label,
                ),
                percent_metric(
                    &format!("window.dimension.{}.coverage", dimension.key),
                    "Coverage",
                    dimension.coverage,
                    evidence_label,
                ),
            ],
        }));
    }
    let rank_text = organization
        .overall
        .rank
        .map(|rank| format!("#{rank} of {}", input.board.organizations.len()))
        .unwrap_or_else(|| "NR".to_owned());
    let classification_text = organization
        .overall
        .published_classification()
        .map(|classification| format!("{classification:?}"))
        .unwrap_or_else(|| "Under review".to_owned());
    let mut first_sections = vec![
        CardSectionView::IdentityHeader(IdentityHeaderSectionView {
            id: "window-identity".to_owned(),
            eyebrow: Some(format!(
                "{} · {} · {}",
                input.board.as_of,
                input.board.manifest.label,
                input
                    .board
                    .manifest
                    .fingerprint
                    .chars()
                    .take(8)
                    .collect::<String>()
            )),
            title: input.team_name.trim().to_owned(),
            subtitle: Some(format!("{classification_text} · {rank_text}")),
            identities: vec![CardIdentityView {
                kind: CardIdentityKind::Team,
                subject_id: team.clone(),
                label: input.team_name.trim().to_owned(),
                asset_id: None,
            }],
        }),
        CardSectionView::MetricStrip(MetricStripSectionView {
            id: "window-overall".to_owned(),
            title: Some("Organization Window".to_owned()),
            metrics: vec![
                optional_metric(
                    "window.overall.score",
                    "Score",
                    organization.overall.score,
                    MetricUnit::Score,
                    evidence_label,
                ),
                text_metric(
                    "window.overall.rank",
                    "League rank",
                    &rank_text,
                    evidence_label,
                ),
                percent_metric(
                    "window.overall.confidence",
                    "Confidence",
                    organization.overall.confidence,
                    evidence_label,
                ),
                percent_metric(
                    "window.overall.coverage",
                    "Coverage",
                    organization.overall.coverage,
                    evidence_label,
                ),
            ],
        }),
    ];
    first_sections.extend(dimension_sections);
    if !organization.blockers.is_empty() || !organization.overall.rank_status.reasons.is_empty() {
        first_sections.push(CardSectionView::StateNotice(StateNoticeSectionView {
            id: "window-rank-state".to_owned(),
            title: "Rank withheld".to_owned(),
            detail: Some(
                organization
                    .overall
                    .rank_status
                    .reasons
                    .iter()
                    .chain(&organization.blockers)
                    .cloned()
                    .collect::<Vec<_>>()
                    .join("; "),
            ),
            empty_state: None,
            warnings: Vec::new(),
            token: SemanticToken::Warning,
        }));
    }

    let profile_methods = organization
        .dimensions
        .iter()
        .flat_map(|dimension| {
            dimension
                .profiles
                .iter()
                .map(move |profile| CardMethodologyItemView {
                    key: profile.profile_key.clone(),
                    label: format!("{} · {}", dimension.label, profile.raw_unit),
                    version: profile.method_version.clone(),
                    summary: format!(
                        "Raw {} · normalized {} · confidence {:.0}% · coverage {:.0}%",
                        profile
                            .raw_value
                            .map(|value| format!("{value:.2}"))
                            .unwrap_or_else(|| "missing".to_owned()),
                        profile
                            .normalized_score
                            .map(|value| format!("{value:.1}"))
                            .unwrap_or_else(|| "NR".to_owned()),
                        profile.confidence * 100.0,
                        profile.coverage * 100.0,
                    ),
                })
        })
        .collect::<Vec<_>>();
    let limitations = organization
        .dimensions
        .iter()
        .flat_map(|dimension| &dimension.profiles)
        .flat_map(|profile| profile.limitations.iter().cloned())
        .chain(input.board.disclosures.iter().cloned())
        .collect::<Vec<_>>();

    CardDocumentView {
        schema: CARD_DOCUMENT_SCHEMA.to_owned(),
        card_kind: CardKind::OrganizationWindow,
        document_id: format!(
            "organization-window:{}:{}:{}:{}",
            team,
            input.board.season,
            input.board.as_of.format("%Y%m%d"),
            input.board.manifest.manifest_id
        ),
        fingerprint: String::new(),
        title: format!("{} organization Window", input.team_name.trim()),
        subtitle: Some(format!("{classification_text} · {rank_text} · {:.1}/100", organization.overall.score.unwrap_or(0.0))),
        context: CardContextView {
            view,
            evidence_at: input.evidence_at,
            evidence_label,
            builder_version: ORGANIZATION_WINDOW_CARD_VERSION.to_owned(),
            methodology_versions,
            joins: CardIdentityJoinsView {
                team_ids: vec![team.clone()],
                ..CardIdentityJoinsView::default()
            },
            simulation: CardSimulationContextView {
                model_id: Some("organization-window".to_owned()),
                model_version: Some(input.board.manifest.manifest_version.clone()),
                parameter_fingerprint: Some(input.board.manifest.fingerprint.clone()),
                seed: None,
                trials: None,
            },
        },
        theme: nhl_team_card_theme(&team),
        required_capabilities: Vec::new(),
        pages: vec![
            CardPageView {
                id: "window".to_owned(),
                literal_label: "Organization health panes".to_owned(),
                display_label: Some("The Window".to_owned()),
                order: 1,
                accessible_summary: format!("{} organization score, rank state, confidence, coverage, and dimension scores.", input.team_name.trim()),
                sections: first_sections,
            },
            CardPageView {
                id: "insider".to_owned(),
                literal_label: "Window evidence and methodology".to_owned(),
                display_label: Some("The Insider".to_owned()),
                order: 2,
                accessible_summary: "Raw inputs, normalized profile scores, method versions, limitations, and sealed source identity.".to_owned(),
                sections: vec![
                    CardSectionView::Methodology(MethodologySectionView {
                        id: "window-methodology".to_owned(),
                        title: "Profiles and evidence".to_owned(),
                        methods: profile_methods,
                        limitations,
                    }),
                    CardSectionView::Provenance(ProvenanceSectionView {
                        id: "window-provenance".to_owned(),
                        title: "Sealed board".to_owned(),
                        provenance_ids: vec!["window-board".to_owned()],
                    }),
                ],
            },
        ],
        assets: Vec::new(),
        provenance: vec![CardProvenanceView {
            id: "window-board".to_owned(),
            source: SourceKind::Bundle,
            label: format!("{} · {}", input.board.manifest.label, input.board.as_of),
            state: completeness,
            observed_at: input.evidence_at,
            fingerprint: Some(input.board.fingerprint),
            note: Some(format!("Complete cohort retained: {} organizations", input.board.expected_organizations.len())),
        }],
        warnings: Vec::new(),
        empty_state: None,
    }
    .seal()
    .map_err(|error| OrganizationWindowCardError::Document(error.to_string()))
}

fn optional_metric(
    key: &str,
    label: &str,
    value: Option<f64>,
    unit: MetricUnit,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: value.map_or(MetricValue::Missing, MetricValue::Decimal),
            unit,
            precision: ValuePrecision::OneDecimal,
            token: None,
        },
        display_text: value
            .map(|value| format!("{value:.1}"))
            .unwrap_or_else(|| "NR".to_owned()),
        accessible_text: value
            .map(|value| format!("{label} {value:.1}"))
            .unwrap_or_else(|| format!("{label} not ranked")),
        comparison: None,
        evidence_label,
    }
}

fn percent_metric(
    key: &str,
    label: &str,
    value: f64,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    let percent = value * 100.0;
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Decimal(percent),
            unit: MetricUnit::Percentage,
            precision: ValuePrecision::PercentOneDecimal,
            token: None,
        },
        display_text: format!("{percent:.0}%"),
        accessible_text: format!("{label} {percent:.0} percent"),
        comparison: None,
        evidence_label,
    }
}

fn text_metric(
    key: &str,
    label: &str,
    value: &str,
    evidence_label: EvidenceLabel,
) -> CardMetricView {
    CardMetricView {
        metric: MetricCell {
            key: StatKey(key.to_owned()),
            label: label.to_owned(),
            value: MetricValue::Text(value.to_owned()),
            unit: MetricUnit::Ranking,
            precision: ValuePrecision::Raw,
            token: None,
        },
        display_text: value.to_owned(),
        accessible_text: format!("{label} {value}"),
        comparison: None,
        evidence_label,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;

    fn board() -> OrganizationWindowBoardView {
        serde_json::from_str(include_str!(
            "../../../../examples/organization-window-board-partial-2026-07-28.json"
        ))
        .expect("sealed organization Window board fixture")
    }

    #[test]
    fn all_32_window_cards_project_from_one_sealed_board() {
        let board = board();
        let mut fingerprints = BTreeSet::new();
        for (team, name) in CANONICAL_TEAMS {
            let card = project_organization_window_card(board.clone(), team, None, None)
                .expect("canonical focused-team card");
            assert_eq!(card.context.joins.team_ids, [*team]);
            assert_eq!(card.title, format!("{name} organization Window"));
            assert_eq!(card.context.view.completeness, Completeness::Partial);
            assert!(card
                .subtitle
                .as_deref()
                .is_some_and(|subtitle| { subtitle.starts_with("Under review · NR · ") }));
            assert!(fingerprints.insert(card.fingerprint));
        }
        assert_eq!(fingerprints.len(), CANONICAL_TEAMS.len());
    }

    #[test]
    fn current_partial_board_matches_its_14_of_16_source_audit() {
        let board = board();
        let audit: crate::view_model::OrganizationWindowSourceCoverageView =
            serde_json::from_str(include_str!(
                "../../../../examples/organization-window-source-audit-partial-2026-07-28.json"
            ))
            .expect("sealed partial source audit fixture");

        assert_eq!(audit.package_fingerprint.len(), 64);
        assert_eq!(audit.board_fingerprint, board.fingerprint);
        assert_eq!(audit.expected_organizations, CANONICAL_TEAMS.len());
        assert_eq!(audit.complete_required_profiles, 14);
        assert_eq!(audit.required_profiles, 16);
        assert_eq!(audit.rank_eligible_organizations, 0);
        assert!(!audit.production_ranked);

        let incomplete_required = audit
            .profiles
            .iter()
            .filter(|profile| profile.required && !profile.complete)
            .map(|profile| profile.profile_key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(
            incomplete_required,
            ["development.organization_depth", "development.recall_depth"]
        );
        assert!(board.organizations.iter().all(|organization| {
            organization.overall.rank.is_none()
                && organization.blockers.len() == incomplete_required.len()
                && organization
                    .blockers
                    .iter()
                    .all(|blocker| incomplete_required.iter().any(|key| blocker.contains(key)))
        }));
    }

    #[test]
    fn withheld_rank_does_not_publish_a_team_classification() {
        let card = project_organization_window_card(board(), "NYR", None, None).unwrap();
        let identity = card.pages[0]
            .sections
            .iter()
            .find_map(|section| match section {
                CardSectionView::IdentityHeader(identity) => Some(identity),
                _ => None,
            })
            .expect("identity section");
        assert_eq!(identity.subtitle.as_deref(), Some("Under review · NR"));
        assert!(!card.subtitle.as_deref().unwrap().contains("Rebuilding"));
    }
}
