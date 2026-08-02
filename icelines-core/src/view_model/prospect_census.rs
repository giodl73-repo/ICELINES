//! UI-neutral prospect population and ranking coverage funnel.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROSPECT_CENSUS_SCHEMA: &str = "prospect_census.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectCensusStage {
    Discovered,
    CanonicalIdentity,
    ControlledRelationship,
    ProspectEligible,
    CareerEvidenceUsable,
    StudyBuilt,
    Ranked,
}

impl ProspectCensusStage {
    pub const ALL: [Self; 7] = [
        Self::Discovered,
        Self::CanonicalIdentity,
        Self::ControlledRelationship,
        Self::ProspectEligible,
        Self::CareerEvidenceUsable,
        Self::StudyBuilt,
        Self::Ranked,
    ];

    fn next(self) -> Option<Self> {
        Self::ALL
            .iter()
            .position(|stage| *stage == self)
            .and_then(|index| Self::ALL.get(index + 1))
            .copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectCensusLossReason {
    UnresolvedIdentity,
    ConflictingControl,
    UnsupportedControl,
    MissingEligibilityEvidence,
    ProspectIneligible,
    MissingCareerEvidence,
    StudyBuildFailed,
    RankingWithheld,
    MissingSourceFamily,
    ExcludedByPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectPopulationAuthorityStatus {
    Complete,
    Incomplete,
    Conflicted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectCensusPublicationStatus {
    Published,
    PopulationIncomplete,
    DepthIncomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectScorePublicationStatus {
    Published,
    ScoreWithheld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectRankPublicationStatus {
    Published,
    RankWithheld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProspectCensusFreshnessStatus {
    Fresh,
    Stale,
    Mixed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusCandidateInput {
    pub candidate_key: String,
    pub organization: String,
    pub discovery_source_family: String,
    pub player_class: String,
    pub position_group: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u32>,
    pub reached_stage: ProspectCensusStage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loss_reason: Option<ProspectCensusLossReason>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub loss_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusOrganizationInput {
    pub organization: String,
    pub population_authority_status: ProspectPopulationAuthorityStatus,
    pub requested_ranking_depth: usize,
    #[serde(default)]
    pub authority_disclosures: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusCounts {
    pub discovered: usize,
    pub canonical_identity: usize,
    pub controlled_relationship: usize,
    pub prospect_eligible: usize,
    pub career_evidence_usable: usize,
    pub study_built: usize,
    pub ranked: usize,
}

impl ProspectCensusCounts {
    fn observe(&mut self, reached: ProspectCensusStage) {
        self.discovered += 1;
        if reached >= ProspectCensusStage::CanonicalIdentity {
            self.canonical_identity += 1;
        }
        if reached >= ProspectCensusStage::ControlledRelationship {
            self.controlled_relationship += 1;
        }
        if reached >= ProspectCensusStage::ProspectEligible {
            self.prospect_eligible += 1;
        }
        if reached >= ProspectCensusStage::CareerEvidenceUsable {
            self.career_evidence_usable += 1;
        }
        if reached >= ProspectCensusStage::StudyBuilt {
            self.study_built += 1;
        }
        if reached >= ProspectCensusStage::Ranked {
            self.ranked += 1;
        }
    }

    fn add(&mut self, other: &Self) {
        self.discovered += other.discovered;
        self.canonical_identity += other.canonical_identity;
        self.controlled_relationship += other.controlled_relationship;
        self.prospect_eligible += other.prospect_eligible;
        self.career_evidence_usable += other.career_evidence_usable;
        self.study_built += other.study_built;
        self.ranked += other.ranked;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusLossRow {
    pub candidate_key: String,
    pub organization: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_id: Option<u32>,
    pub reached_stage: ProspectCensusStage,
    pub blocked_stage: ProspectCensusStage,
    pub reason: ProspectCensusLossReason,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusDimensionRow {
    /// `None` denotes the league aggregate for the remaining dimensions.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    pub source_family: String,
    pub player_class: String,
    pub position_group: String,
    pub counts: ProspectCensusCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusOrganizationView {
    pub organization: String,
    pub population_authority_status: ProspectPopulationAuthorityStatus,
    pub ranking_depth_complete: bool,
    pub requested_ranking_depth: usize,
    pub ranking_depth_shortfall: usize,
    pub publication_status: ProspectCensusPublicationStatus,
    pub score_status: ProspectScorePublicationStatus,
    pub rank_status: ProspectRankPublicationStatus,
    pub counts: ProspectCensusCounts,
    pub losses: Vec<ProspectCensusLossRow>,
    pub disclosures: Vec<String>,
    pub remediation: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusView {
    pub schema: String,
    pub evaluation_season: u32,
    pub effective_cutoff: String,
    pub knowledge_cutoff: String,
    pub freshness_status: ProspectCensusFreshnessStatus,
    pub source_package_fingerprint: String,
    pub reconciliation_policy_version: String,
    pub eligibility_policy_version: String,
    pub organizations: Vec<ProspectCensusOrganizationView>,
    pub league_counts: ProspectCensusCounts,
    pub dimensions: Vec<ProspectCensusDimensionRow>,
    pub losses: Vec<ProspectCensusLossRow>,
    pub disclosures: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusInput {
    pub evaluation_season: u32,
    pub effective_cutoff: String,
    pub knowledge_cutoff: String,
    pub freshness_status: ProspectCensusFreshnessStatus,
    pub source_package_fingerprint: String,
    pub reconciliation_policy_version: String,
    pub eligibility_policy_version: String,
    pub organizations: Vec<ProspectCensusOrganizationInput>,
    pub candidates: Vec<ProspectCensusCandidateInput>,
}

pub fn build_prospect_census(input: ProspectCensusInput) -> Result<ProspectCensusView, String> {
    validate_input(&input)?;
    let candidates_by_organization = input.candidates.iter().fold(
        BTreeMap::<&str, Vec<&ProspectCensusCandidateInput>>::new(),
        |mut grouped, candidate| {
            grouped
                .entry(candidate.organization.as_str())
                .or_default()
                .push(candidate);
            grouped
        },
    );
    let mut organizations = Vec::with_capacity(input.organizations.len());
    let mut league_counts = ProspectCensusCounts::default();
    let mut all_losses = Vec::new();

    for organization in &input.organizations {
        let candidates = candidates_by_organization
            .get(organization.organization.as_str())
            .cloned()
            .unwrap_or_default();
        let mut counts = ProspectCensusCounts::default();
        let mut losses = Vec::new();
        for candidate in candidates {
            counts.observe(candidate.reached_stage);
            if let Some(blocked_stage) = candidate.reached_stage.next() {
                losses.push(ProspectCensusLossRow {
                    candidate_key: candidate.candidate_key.clone(),
                    organization: candidate.organization.clone(),
                    player_id: candidate.player_id,
                    reached_stage: candidate.reached_stage,
                    blocked_stage,
                    reason: candidate
                        .loss_reason
                        .expect("validated non-ranked candidate loss"),
                    message: candidate
                        .loss_message
                        .clone()
                        .expect("validated non-ranked candidate loss message"),
                });
            }
        }
        losses.sort_by(|left, right| left.candidate_key.cmp(&right.candidate_key));
        league_counts.add(&counts);
        all_losses.extend(losses.iter().cloned());
        let ranking_depth_shortfall = organization
            .requested_ranking_depth
            .saturating_sub(counts.ranked);
        let ranking_depth_complete = ranking_depth_shortfall == 0;
        let publication_status = match organization.population_authority_status {
            ProspectPopulationAuthorityStatus::Complete if ranking_depth_complete => {
                ProspectCensusPublicationStatus::Published
            }
            ProspectPopulationAuthorityStatus::Complete => {
                ProspectCensusPublicationStatus::DepthIncomplete
            }
            ProspectPopulationAuthorityStatus::Incomplete
            | ProspectPopulationAuthorityStatus::Conflicted => {
                ProspectCensusPublicationStatus::PopulationIncomplete
            }
        };
        let publish = publication_status == ProspectCensusPublicationStatus::Published;
        let mut disclosures = organization.authority_disclosures.clone();
        let mut remediation = Vec::new();
        if !publish {
            disclosures.push(match publication_status {
                ProspectCensusPublicationStatus::PopulationIncomplete => {
                    remediation.push(
                        "Acquire, validate, or explicitly mark every requested source-family object; then resolve or exclude every control conflict."
                            .to_owned(),
                    );
                    "Program score and rank are withheld until the enumerated population source matrix is complete and conflict-free.".to_owned()
                }
                ProspectCensusPublicationStatus::DepthIncomplete => format!(
                    "Program score and rank are withheld because {} additional ranked prospect(s) are required.",
                    ranking_depth_shortfall
                ),
                ProspectCensusPublicationStatus::Published => unreachable!(),
            });
            if publication_status == ProspectCensusPublicationStatus::DepthIncomplete {
                remediation.push(format!(
                    "Resolve typed candidate losses and build {} additional eligible ranked study or studies without changing requested depth.",
                    ranking_depth_shortfall
                ));
            }
        }
        organizations.push(ProspectCensusOrganizationView {
            organization: organization.organization.clone(),
            population_authority_status: organization.population_authority_status,
            ranking_depth_complete,
            requested_ranking_depth: organization.requested_ranking_depth,
            ranking_depth_shortfall,
            publication_status,
            score_status: if publish {
                ProspectScorePublicationStatus::Published
            } else {
                ProspectScorePublicationStatus::ScoreWithheld
            },
            rank_status: if publish {
                ProspectRankPublicationStatus::Published
            } else {
                ProspectRankPublicationStatus::RankWithheld
            },
            counts,
            losses,
            disclosures,
            remediation,
        });
    }
    organizations.sort_by(|left, right| left.organization.cmp(&right.organization));
    all_losses.sort_by(|left, right| {
        left.organization
            .cmp(&right.organization)
            .then_with(|| left.candidate_key.cmp(&right.candidate_key))
    });

    Ok(ProspectCensusView {
        schema: PROSPECT_CENSUS_SCHEMA.to_owned(),
        evaluation_season: input.evaluation_season,
        effective_cutoff: input.effective_cutoff,
        knowledge_cutoff: input.knowledge_cutoff,
        freshness_status: input.freshness_status,
        source_package_fingerprint: input.source_package_fingerprint,
        reconciliation_policy_version: input.reconciliation_policy_version,
        eligibility_policy_version: input.eligibility_policy_version,
        organizations,
        league_counts,
        dimensions: build_dimensions(&input.candidates),
        losses: all_losses,
        disclosures: vec![
            "Population authority and requested ranking depth are independent gates; neither is inferred from the other.".to_owned(),
            "Each candidate contributes monotonically through the last evidenced stage. Every stopped candidate has a typed loss row; missing candidates are represented by population-authority disclosures, never invented rows.".to_owned(),
            "Program scores and ordinal ranks are publication outputs, not census inputs, and remain withheld unless both gates pass.".to_owned(),
        ],
    })
}

pub fn require_publishable_prospect_census(view: &ProspectCensusView) -> Result<(), String> {
    let blocked = view
        .organizations
        .iter()
        .filter(|organization| {
            organization.publication_status != ProspectCensusPublicationStatus::Published
        })
        .map(|organization| organization.organization.as_str())
        .collect::<Vec<_>>();
    if blocked.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "prospect census publication refused for organization(s): {}",
            blocked.join(", ")
        ))
    }
}

fn validate_input(input: &ProspectCensusInput) -> Result<(), String> {
    if input.evaluation_season == 0 {
        return Err("prospect census evaluation_season must be non-zero".to_owned());
    }
    for (field, value) in [
        ("effective_cutoff", input.effective_cutoff.as_str()),
        ("knowledge_cutoff", input.knowledge_cutoff.as_str()),
        (
            "source_package_fingerprint",
            input.source_package_fingerprint.as_str(),
        ),
        (
            "reconciliation_policy_version",
            input.reconciliation_policy_version.as_str(),
        ),
        (
            "eligibility_policy_version",
            input.eligibility_policy_version.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(format!("prospect census {field} must not be empty"));
        }
    }
    if chrono::DateTime::parse_from_rfc3339(&input.effective_cutoff).is_err()
        || chrono::DateTime::parse_from_rfc3339(&input.knowledge_cutoff).is_err()
        || input.source_package_fingerprint.len() != 64
        || !input
            .source_package_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(
            "prospect census cutoffs must be RFC 3339 and package fingerprint must be canonical SHA-256 hex"
                .to_owned(),
        );
    }
    let mut organizations = BTreeSet::new();
    for organization in &input.organizations {
        if organization.organization.trim().is_empty()
            || organization.requested_ranking_depth == 0
            || !organizations.insert(organization.organization.as_str())
        {
            return Err(
                "prospect census organizations must be unique, named, and request non-zero depth"
                    .to_owned(),
            );
        }
    }
    if organizations.is_empty() {
        return Err("prospect census requires at least one organization".to_owned());
    }
    let mut candidate_keys = BTreeSet::new();
    for candidate in &input.candidates {
        if candidate.candidate_key.trim().is_empty()
            || candidate.discovery_source_family.trim().is_empty()
            || candidate.player_class.trim().is_empty()
            || candidate.position_group.trim().is_empty()
            || !organizations.contains(candidate.organization.as_str())
            || !candidate_keys.insert(candidate.candidate_key.as_str())
            || candidate.player_id == Some(0)
            || (candidate.reached_stage >= ProspectCensusStage::CanonicalIdentity
                && candidate.player_id.is_none())
        {
            return Err("invalid or duplicate prospect census candidate".to_owned());
        }
        let stopped = candidate.reached_stage != ProspectCensusStage::Ranked;
        let has_reason = candidate.loss_reason.is_some();
        let has_message = candidate
            .loss_message
            .as_deref()
            .is_some_and(|message| !message.trim().is_empty());
        if (stopped && !(has_reason && has_message)) || (!stopped && (has_reason || has_message)) {
            return Err("every stopped candidate requires exactly one typed loss and ranked candidates require none".to_owned());
        }
    }
    Ok(())
}

fn build_dimensions(
    candidates: &[ProspectCensusCandidateInput],
) -> Vec<ProspectCensusDimensionRow> {
    let mut organization_rows =
        BTreeMap::<(String, String, String, String), ProspectCensusCounts>::new();
    let mut league_rows = BTreeMap::<(String, String, String), ProspectCensusCounts>::new();
    for candidate in candidates {
        organization_rows
            .entry((
                candidate.organization.clone(),
                candidate.discovery_source_family.clone(),
                candidate.player_class.clone(),
                candidate.position_group.clone(),
            ))
            .or_default()
            .observe(candidate.reached_stage);
        league_rows
            .entry((
                candidate.discovery_source_family.clone(),
                candidate.player_class.clone(),
                candidate.position_group.clone(),
            ))
            .or_default()
            .observe(candidate.reached_stage);
    }
    organization_rows
        .into_iter()
        .map(
            |((organization, source_family, player_class, position_group), counts)| {
                ProspectCensusDimensionRow {
                    organization: Some(organization),
                    source_family,
                    player_class,
                    position_group,
                    counts,
                }
            },
        )
        .chain(league_rows.into_iter().map(
            |((source_family, player_class, position_group), counts)| ProspectCensusDimensionRow {
                organization: None,
                source_family,
                player_class,
                position_group,
                counts,
            },
        ))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn organization(
        name: &str,
        authority: ProspectPopulationAuthorityStatus,
    ) -> ProspectCensusOrganizationInput {
        ProspectCensusOrganizationInput {
            organization: name.to_owned(),
            population_authority_status: authority,
            requested_ranking_depth: 2,
            authority_disclosures: Vec::new(),
        }
    }

    fn candidate(
        key: &str,
        organization: &str,
        player_id: Option<u32>,
        reached_stage: ProspectCensusStage,
        reason: Option<ProspectCensusLossReason>,
    ) -> ProspectCensusCandidateInput {
        ProspectCensusCandidateInput {
            candidate_key: key.to_owned(),
            organization: organization.to_owned(),
            discovery_source_family: "nhl_draft".to_owned(),
            player_class: "prospect".to_owned(),
            position_group: if key.contains("goalie") {
                "goalie"
            } else {
                "skater"
            }
            .to_owned(),
            player_id,
            reached_stage,
            loss_reason: reason,
            loss_message: reason.map(|reason| format!("Stopped because {reason:?}.")),
        }
    }

    fn input() -> ProspectCensusInput {
        ProspectCensusInput {
            evaluation_season: 20_262_027,
            effective_cutoff: "2026-07-31T12:00:00Z".to_owned(),
            knowledge_cutoff: "2026-07-31T12:00:00Z".to_owned(),
            freshness_status: ProspectCensusFreshnessStatus::Fresh,
            source_package_fingerprint: "a".repeat(64),
            reconciliation_policy_version: "current-player-state.v1".to_owned(),
            eligibility_policy_version: "prospect-eligibility.v1".to_owned(),
            organizations: vec![
                organization("NYR", ProspectPopulationAuthorityStatus::Complete),
                organization("SEA", ProspectPopulationAuthorityStatus::Incomplete),
            ],
            candidates: vec![
                candidate("nyr-1", "NYR", Some(1), ProspectCensusStage::Ranked, None),
                candidate("nyr-2", "NYR", Some(2), ProspectCensusStage::Ranked, None),
                candidate(
                    "sea-unresolved",
                    "SEA",
                    None,
                    ProspectCensusStage::Discovered,
                    Some(ProspectCensusLossReason::UnresolvedIdentity),
                ),
                candidate(
                    "sea-goalie",
                    "SEA",
                    Some(3),
                    ProspectCensusStage::CareerEvidenceUsable,
                    Some(ProspectCensusLossReason::StudyBuildFailed),
                ),
            ],
        }
    }

    #[test]
    fn authority_and_depth_are_independent_publication_gates() {
        let view = build_prospect_census(input()).unwrap();
        let nyr = view
            .organizations
            .iter()
            .find(|row| row.organization == "NYR")
            .unwrap();
        let sea = view
            .organizations
            .iter()
            .find(|row| row.organization == "SEA")
            .unwrap();

        assert_eq!(
            nyr.publication_status,
            ProspectCensusPublicationStatus::Published
        );
        assert_eq!(nyr.score_status, ProspectScorePublicationStatus::Published);
        assert!(nyr.ranking_depth_complete);
        assert_eq!(
            sea.publication_status,
            ProspectCensusPublicationStatus::PopulationIncomplete
        );
        assert_eq!(sea.rank_status, ProspectRankPublicationStatus::RankWithheld);
        assert!(!sea.ranking_depth_complete);
        assert!(require_publishable_prospect_census(&view).is_err());
    }

    #[test]
    fn every_candidate_loss_is_typed_and_stage_counts_reconcile() {
        let view = build_prospect_census(input()).unwrap();
        assert_eq!(view.league_counts.discovered, 4);
        assert_eq!(view.league_counts.canonical_identity, 3);
        assert_eq!(view.league_counts.ranked, 2);
        assert_eq!(view.losses.len(), 2);
        assert_eq!(
            view.losses
                .iter()
                .find(|row| row.candidate_key == "sea-unresolved")
                .unwrap()
                .blocked_stage,
            ProspectCensusStage::CanonicalIdentity
        );
        let organization_sum = view
            .organizations
            .iter()
            .map(|row| row.counts.discovered)
            .sum::<usize>();
        assert_eq!(organization_sum, view.league_counts.discovered);
        assert!(view
            .dimensions
            .iter()
            .any(|row| row.position_group == "goalie"));
    }

    #[test]
    fn stopped_candidate_without_reason_is_rejected() {
        let mut invalid = input();
        invalid.candidates[2].loss_reason = None;
        invalid.candidates[2].loss_message = None;
        assert!(build_prospect_census(invalid).is_err());
    }
}
