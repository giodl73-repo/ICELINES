//! Versioned player-state reconciliation over canonical source facts.

use crate::reconciliation::ReviewedPublicationLowering;
use chrono::{DateTime, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::source_facts::{
    ClubRef, FactAssertion, FactId, FactSubject, IdentityReviewAction, IdentityReviewDecision,
    OrganizationId, PlayerOrganizationEvent, PlayerParticipationFact, ProposalId,
    ProviderIdentityProposal, SourceContractError, SourceDisclosure, SourceDisclosureCode,
    SourceExclusion, SourceFact, StagedPlayerAssertion,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const CURRENT_PLAYER_STATE_POLICY_VERSION: &str = "current-player-state.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityReplayMode {
    AsKnown,
    ReconstructedIdentity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayCutoffs {
    pub effective_cutoff: DateTime<Utc>,
    pub knowledge_cutoff: DateTime<Utc>,
    pub identity_mode: IdentityReplayMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightsStatus {
    Supported,
    Expired,
    Transferred,
    Unknown,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RightsResolution {
    pub status: RightsStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub organization: Option<OrganizationId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transferred_from: Option<OrganizationId>,
    pub fact_ids: Vec<FactId>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssignmentStatus {
    Assigned,
    Unknown,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssignmentResolution {
    pub status: AssignmentStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub club: Option<ClubRef>,
    pub fact_ids: Vec<FactId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParticipationObservation {
    pub fact_id: FactId,
    pub occurred_at: DateTime<Utc>,
    pub fact: PlayerParticipationFact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerCurrentState {
    pub policy_version: String,
    pub player_id: PlayerId,
    pub effective_cutoff: DateTime<Utc>,
    pub knowledge_cutoff: DateTime<Utc>,
    pub input_fact_ids: Vec<FactId>,
    pub rights: RightsResolution,
    pub assignment: AssignmentResolution,
    pub participation_only: Vec<ParticipationObservation>,
    pub disclosures: Vec<SourceDisclosure>,
}

/// Lowers reviewed staged facts at explicit replay cutoffs.
///
/// Reconstructed-identity mode may use a later identity decision, but the
/// resulting assertion retains only the staged hockey fact and its evidence.
pub fn reconcile_staged_player_assertions(
    proposals: &[ProviderIdentityProposal],
    staged: &[StagedPlayerAssertion],
    decisions: &[IdentityReviewDecision],
    cutoffs: ReplayCutoffs,
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let proposal_index = unique_proposals(proposals)?;
    let decision_index = unique_decisions(decisions)?;
    let mut output = ReviewedPublicationLowering {
        assertions: Vec::new(),
        exclusions: Vec::new(),
        disclosures: Vec::new(),
    };
    let mut unresolved = 0usize;
    let mut historical = 0usize;

    for assertion in staged {
        if assertion.occurred_at().starts_at > cutoffs.effective_cutoff
            || assertion
                .evidence()
                .iter()
                .any(|evidence| evidence.captured_at() > cutoffs.knowledge_cutoff)
        {
            historical += 1;
            continue;
        }
        let proposal = proposal_index.get(assertion.proposal_id()).ok_or_else(|| {
            SourceContractError::UnknownStagedProposal {
                assertion_id: assertion.assertion_id().to_string(),
                proposal_id: assertion.proposal_id().to_string(),
            }
        })?;
        let Some(decision) = decision_index.get(assertion.proposal_id()) else {
            unresolved += 1;
            continue;
        };
        if cutoffs.identity_mode == IdentityReplayMode::AsKnown
            && decision.reviewed_at() > cutoffs.knowledge_cutoff
        {
            unresolved += 1;
            continue;
        }
        if decision.action() == IdentityReviewAction::Reject {
            output.exclusions.push(SourceExclusion {
                exclusion_id: format!("rejected-staged:{}", assertion.assertion_id()),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "Staged identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: assertion
                    .evidence()
                    .iter()
                    .map(|evidence| evidence.source_id().clone())
                    .collect(),
            });
            continue;
        }
        let player_id = decision
            .canonical_player_id()
            .ok_or(SourceContractError::MissingDecisionPlayer)?;
        output.assertions.push(FactAssertion::new(
            FactId::try_new(format!("reconciled:{}", assertion.assertion_id()))?,
            assertion.semantic_key(),
            FactSubject::Player(player_id),
            assertion.occurred_at().clone(),
            assertion.authority(),
            assertion.fact().clone(),
            assertion.evidence().to_vec(),
        )?);
    }

    if unresolved > 0 {
        output.disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::UnresolvedIdentity,
            scope: "staged_player_assertions".to_owned(),
            message: format!(
                "{unresolved} staged assertion(s) lack an identity decision available at the replay knowledge cutoff."
            ),
        });
    }
    if historical > 0 {
        output.disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::HistoricalCutoff,
            scope: "staged_player_assertions".to_owned(),
            message: format!(
                "{historical} staged assertion(s) were excluded by effective or knowledge cutoff."
            ),
        });
    }
    Ok(output)
}

pub fn resolve_player_current_state(
    player_id: PlayerId,
    assertions: &[FactAssertion<SourceFact>],
    cutoffs: ReplayCutoffs,
) -> PlayerCurrentState {
    let mut disclosures = Vec::new();
    let mut cutoff_count = 0usize;
    let mut eligible = assertions
        .iter()
        .filter(|assertion| assertion.subject() == &FactSubject::Player(player_id))
        .filter(|assertion| {
            let allowed = assertion.occurred_at().starts_at <= cutoffs.effective_cutoff
                && assertion
                    .evidence()
                    .iter()
                    .all(|evidence| evidence.captured_at() <= cutoffs.knowledge_cutoff);
            if !allowed {
                cutoff_count += 1;
            }
            allowed
        })
        .collect::<Vec<_>>();

    let corrected = eligible
        .iter()
        .flat_map(|assertion| {
            assertion
                .supersedes()
                .iter()
                .chain(assertion.retracts().iter())
        })
        .collect::<BTreeSet<_>>();
    eligible.retain(|assertion| !corrected.contains(assertion.fact_id()));
    eligible.sort_by(|left, right| {
        left.occurred_at()
            .starts_at
            .cmp(&right.occurred_at().starts_at)
            .then_with(|| left.fact_id().cmp(right.fact_id()))
    });

    if cutoff_count > 0 {
        disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::HistoricalCutoff,
            scope: format!("player:{}", player_id.0),
            message: format!(
                "{cutoff_count} canonical fact(s) were excluded by effective or knowledge cutoff."
            ),
        });
    }

    let rights = resolve_rights(&eligible);
    if rights.status == RightsStatus::Conflicted {
        disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::ConflictingControl,
            scope: format!("player:{}", player_id.0),
            message: rights.reason.clone(),
        });
    }
    if rights.status == RightsStatus::Unknown {
        disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::RightsPolicy,
            scope: format!("player:{}", player_id.0),
            message: rights.reason.clone(),
        });
    }
    let assignment = resolve_assignment(&eligible);
    let participation_only = eligible
        .iter()
        .filter_map(|assertion| match assertion.fact() {
            SourceFact::PlayerParticipation(fact) => Some(ParticipationObservation {
                fact_id: assertion.fact_id().clone(),
                occurred_at: assertion.occurred_at().starts_at,
                fact: fact.clone(),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    if !participation_only.is_empty() {
        disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::ParticipationOnly,
            scope: format!("player:{}", player_id.0),
            message: "Participation observations are reported separately and do not establish current control or assignment.".to_owned(),
        });
    }

    PlayerCurrentState {
        policy_version: CURRENT_PLAYER_STATE_POLICY_VERSION.to_owned(),
        player_id,
        effective_cutoff: cutoffs.effective_cutoff,
        knowledge_cutoff: cutoffs.knowledge_cutoff,
        input_fact_ids: eligible
            .iter()
            .map(|assertion| assertion.fact_id().clone())
            .collect(),
        rights,
        assignment,
        participation_only,
        disclosures,
    }
}

fn resolve_rights(assertions: &[&FactAssertion<SourceFact>]) -> RightsResolution {
    let control = assertions
        .iter()
        .filter_map(|assertion| match assertion.fact() {
            SourceFact::PlayerOrganization(
                event @ (PlayerOrganizationEvent::ContractSigned { .. }
                | PlayerOrganizationEvent::RightsTransferred { .. }
                | PlayerOrganizationEvent::RightsExpired { .. }
                | PlayerOrganizationEvent::Released { .. }),
            ) => Some((*assertion, event)),
            _ => None,
        })
        .collect::<Vec<_>>();
    if control.is_empty() {
        return RightsResolution {
            status: RightsStatus::Unknown,
            organization: None,
            transferred_from: None,
            fact_ids: Vec::new(),
            reason: "No contract, legal-control transfer, release, or expiry fact supports current rights; draft and participation facts are insufficient.".to_owned(),
        };
    }

    let mut group_start = 0usize;
    while group_start < control.len() {
        let occurred_at = control[group_start].0.occurred_at().starts_at;
        let mut group_end = group_start + 1;
        while group_end < control.len()
            && control[group_end].0.occurred_at().starts_at == occurred_at
        {
            group_end += 1;
        }
        let outcomes = control[group_start..group_end]
            .iter()
            .map(|(_, event)| match event {
                PlayerOrganizationEvent::ContractSigned { with, .. } => {
                    format!("controlled:{}", with.as_str())
                }
                PlayerOrganizationEvent::RightsTransferred { to, .. } => {
                    format!("controlled:{}", to.as_str())
                }
                PlayerOrganizationEvent::RightsExpired { .. }
                | PlayerOrganizationEvent::Released { .. } => "uncontrolled".to_owned(),
                _ => unreachable!("control events were filtered above"),
            })
            .collect::<BTreeSet<_>>();
        if outcomes.len() > 1 {
            return RightsResolution {
                status: RightsStatus::Conflicted,
                organization: None,
                transferred_from: None,
                fact_ids: control[group_start..group_end]
                    .iter()
                    .map(|(assertion, _)| assertion.fact_id().clone())
                    .collect(),
                reason: format!(
                    "Control facts at {} imply different current outcomes.",
                    occurred_at.to_rfc3339()
                ),
            };
        }
        group_start = group_end;
    }

    let mut current: Option<OrganizationId> = None;
    let mut resolution = None;
    for (assertion, event) in control {
        let next = match event {
            PlayerOrganizationEvent::ContractSigned { with, .. } => RightsResolution {
                status: RightsStatus::Supported,
                organization: Some(with.clone()),
                transferred_from: None,
                fact_ids: vec![assertion.fact_id().clone()],
                reason: format!(
                    "Latest control evidence is a contract with {}.",
                    with.as_str()
                ),
            },
            PlayerOrganizationEvent::RightsTransferred { from, to } => {
                if current
                    .as_ref()
                    .is_some_and(|organization| organization != from)
                {
                    return conflict_rights(assertion, format!(
                        "Rights transfer says {} -> {}, but the preceding supported controller was {}.",
                        from.as_str(), to.as_str(), current.as_ref().expect("checked").as_str()
                    ));
                }
                RightsResolution {
                    status: RightsStatus::Transferred,
                    organization: Some(to.clone()),
                    transferred_from: Some(from.clone()),
                    fact_ids: vec![assertion.fact_id().clone()],
                    reason: format!(
                        "Latest control evidence transfers rights from {} to {}.",
                        from.as_str(),
                        to.as_str()
                    ),
                }
            }
            PlayerOrganizationEvent::RightsExpired { organization }
            | PlayerOrganizationEvent::Released { by: organization } => {
                if current
                    .as_ref()
                    .is_some_and(|controller| controller != organization)
                {
                    return conflict_rights(assertion, format!(
                        "Release or expiry names {}, but the preceding supported controller was {}.",
                        organization.as_str(), current.as_ref().expect("checked").as_str()
                    ));
                }
                RightsResolution {
                    status: RightsStatus::Expired,
                    organization: Some(organization.clone()),
                    transferred_from: None,
                    fact_ids: vec![assertion.fact_id().clone()],
                    reason: format!(
                        "Latest control evidence releases or expires rights held by {}.",
                        organization.as_str()
                    ),
                }
            }
            _ => unreachable!("control events were filtered above"),
        };
        current = match next.status {
            RightsStatus::Supported | RightsStatus::Transferred => next.organization.clone(),
            RightsStatus::Expired => None,
            RightsStatus::Unknown | RightsStatus::Conflicted => current,
        };
        resolution = Some(next);
    }
    resolution.expect("non-empty control fact list produces a resolution")
}

fn conflict_rights(assertion: &FactAssertion<SourceFact>, reason: String) -> RightsResolution {
    RightsResolution {
        status: RightsStatus::Conflicted,
        organization: None,
        transferred_from: None,
        fact_ids: vec![assertion.fact_id().clone()],
        reason,
    }
}

fn resolve_assignment(assertions: &[&FactAssertion<SourceFact>]) -> AssignmentResolution {
    let assignments = assertions
        .iter()
        .filter_map(|assertion| match assertion.fact() {
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Assigned { to, .. })
            | SourceFact::PlayerOrganization(PlayerOrganizationEvent::Rostered { at: to })
            | SourceFact::PlayerOrganization(PlayerOrganizationEvent::AffiliateRostered {
                at: to,
                ..
            })
            | SourceFact::PlayerOrganization(PlayerOrganizationEvent::Recalled { to, .. })
            | SourceFact::PlayerOrganization(PlayerOrganizationEvent::Loaned { to, .. }) => {
                Some((*assertion, to))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    let Some((latest, club)) = assignments.last() else {
        return AssignmentResolution {
            status: AssignmentStatus::Unknown,
            club: None,
            fact_ids: Vec::new(),
            reason: "No current assignment or roster fact is available.".to_owned(),
        };
    };
    let same_time = assignments
        .iter()
        .rev()
        .take_while(|(assertion, _)| {
            assertion.occurred_at().starts_at == latest.occurred_at().starts_at
        })
        .collect::<Vec<_>>();
    if same_time
        .iter()
        .any(|(_, candidate)| candidate.as_str() != club.as_str())
    {
        return AssignmentResolution {
            status: AssignmentStatus::Conflicted,
            club: None,
            fact_ids: same_time
                .iter()
                .map(|(assertion, _)| assertion.fact_id().clone())
                .collect(),
            reason: "Multiple latest assignment facts name different clubs.".to_owned(),
        };
    }
    AssignmentResolution {
        status: AssignmentStatus::Assigned,
        club: Some((*club).clone()),
        fact_ids: same_time
            .iter()
            .map(|(assertion, _)| assertion.fact_id().clone())
            .collect(),
        reason: format!(
            "Latest assignment evidence places the player at {}.",
            club.as_str()
        ),
    }
}

fn unique_proposals(
    proposals: &[ProviderIdentityProposal],
) -> Result<BTreeMap<ProposalId, &ProviderIdentityProposal>, SourceContractError> {
    let mut index = BTreeMap::new();
    for proposal in proposals {
        if index
            .insert(proposal.proposal_id().clone(), proposal)
            .is_some()
        {
            return Err(SourceContractError::DuplicateId {
                kind: "identity_proposal",
                id: proposal.proposal_id().to_string(),
            });
        }
    }
    Ok(index)
}

fn unique_decisions(
    decisions: &[IdentityReviewDecision],
) -> Result<BTreeMap<ProposalId, &IdentityReviewDecision>, SourceContractError> {
    let mut index = BTreeMap::new();
    for decision in decisions {
        if index
            .insert(decision.proposal_id().clone(), decision)
            .is_some()
        {
            return Err(SourceContractError::DuplicateId {
                kind: "identity_decision_for_proposal",
                id: decision.proposal_id().to_string(),
            });
        }
    }
    Ok(index)
}
