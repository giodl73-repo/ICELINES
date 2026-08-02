//! Explicit lowering of staged publication rows after identity review.

use crate::ahl::roster_stats::AhlRosterStatsOutput;
use crate::nhl::club_publication::ClubPublicationOutput;
use crate::nhl::contract_publication::ContractPublicationOutput;
use crate::nhl::draft_picks::DraftPicksOutput;
use crate::nhl::termination_publication::TerminationPublicationOutput;
use crate::nhl::trade_tracker::TradeTrackerOutput;
use icelines_core::source_facts::{
    EffectivePrecision, EffectiveTime, FactAssertion, FactAuthority, FactId, FactSubject,
    IdentityReviewAction, IdentityReviewDecision, PlayerOrganizationEvent, PlayerParticipationFact,
    ProposalId, SourceContractError, SourceDisclosure, SourceDisclosureCode, SourceExclusion,
    SourceFact,
};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct ReviewedPublicationLowering {
    pub assertions: Vec<FactAssertion<SourceFact>>,
    pub exclusions: Vec<SourceExclusion>,
    pub disclosures: Vec<SourceDisclosure>,
}

pub fn lower_reviewed_camp_publication(
    publication: &ClubPublicationOutput,
    decisions: &[IdentityReviewDecision],
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let decisions = decision_index(decisions)?;
    let proposals = publication
        .identity_proposals
        .iter()
        .map(|proposal| (proposal.proposal_id(), proposal))
        .collect::<BTreeMap<_, _>>();
    let mut output = ReviewedPublicationLowering {
        assertions: Vec::new(),
        exclusions: Vec::new(),
        disclosures: Vec::new(),
    };
    let mut unresolved = 0usize;
    for participant in &publication.participants {
        let proposal = proposals.get(&participant.proposal_id).ok_or_else(|| {
            SourceContractError::UnknownDecisionProposal {
                decision_id: "staged_participation".to_owned(),
                proposal_id: participant.proposal_id.to_string(),
            }
        })?;
        let Some(decision) = decisions.get(&participant.proposal_id) else {
            unresolved += 1;
            continue;
        };
        if decision.action() == IdentityReviewAction::Reject {
            output.exclusions.push(SourceExclusion {
                exclusion_id: format!("rejected-camp:{}", participant.proposal_id),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "Camp publication identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: proposal
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
        let starts_at = proposal
            .evidence()
            .iter()
            .map(|evidence| evidence.captured_at())
            .min()
            .ok_or(SourceContractError::EmptyEvidence)?;
        output.assertions.push(FactAssertion::new(
            FactId::try_new(format!("reviewed-camp:{}", participant.proposal_id))?,
            format!(
                "player:{}:participation:{}:{}:{:?}",
                player_id.0,
                participant.organization.as_str(),
                participant.season.0,
                participant.kind
            ),
            FactSubject::Player(player_id),
            EffectiveTime::new(starts_at, None, EffectivePrecision::Day)?,
            FactAuthority::Attendance,
            SourceFact::PlayerParticipation(PlayerParticipationFact {
                organization: participant.organization.clone(),
                season: participant.season,
                kind: participant.kind,
                authority: participant.authority,
            }),
            proposal.evidence().to_vec(),
        )?);
    }
    if unresolved > 0 {
        output.disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::UnresolvedIdentity,
            scope: "club_camp_publication".to_owned(),
            message: format!(
                "{unresolved} staged camp participant(s) remain outside canonical facts pending identity review."
            ),
        });
    }
    Ok(output)
}

pub fn lower_reviewed_contract_publication(
    publication: &ContractPublicationOutput,
    decisions: &[IdentityReviewDecision],
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let decisions = decision_index(decisions)?;
    let proposal = &publication.identity_proposal;
    let Some(decision) = decisions.get(&publication.signing.proposal_id) else {
        return Ok(ReviewedPublicationLowering {
            assertions: Vec::new(),
            exclusions: Vec::new(),
            disclosures: vec![SourceDisclosure {
                code: SourceDisclosureCode::UnresolvedIdentity,
                scope: "contract_publication".to_owned(),
                message: "The staged contract signing remains outside canonical facts pending identity review.".to_owned(),
            }],
        });
    };
    if decision.action() == IdentityReviewAction::Reject {
        return Ok(ReviewedPublicationLowering {
            assertions: Vec::new(),
            exclusions: vec![SourceExclusion {
                exclusion_id: format!("rejected-contract:{}", publication.signing.proposal_id),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "Contract publication identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: proposal
                    .evidence()
                    .iter()
                    .map(|evidence| evidence.source_id().clone())
                    .collect(),
            }],
            disclosures: Vec::new(),
        });
    }
    let player_id = decision
        .canonical_player_id()
        .ok_or(SourceContractError::MissingDecisionPlayer)?;
    let assertion = FactAssertion::new(
        FactId::try_new(format!(
            "reviewed-contract:{}",
            publication.signing.proposal_id
        ))?,
        format!("player:{}:contract_signing", player_id.0),
        FactSubject::Player(player_id),
        publication.signing.occurred_at.clone(),
        FactAuthority::Contract,
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
            with: publication.signing.organization.clone(),
            contract_kind: publication.signing.contract_kind.clone(),
        }),
        proposal.evidence().to_vec(),
    )?;
    Ok(ReviewedPublicationLowering {
        assertions: vec![assertion],
        exclusions: Vec::new(),
        disclosures: Vec::new(),
    })
}

pub fn lower_reviewed_trade_tracker(
    publication: &TradeTrackerOutput,
    decisions: &[IdentityReviewDecision],
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let decisions = decision_index(decisions)?;
    let proposals = publication
        .identity_proposals
        .iter()
        .map(|proposal| (proposal.proposal_id(), proposal))
        .collect::<BTreeMap<_, _>>();
    let mut output = ReviewedPublicationLowering {
        assertions: Vec::new(),
        exclusions: publication
            .ignored_assets
            .iter()
            .enumerate()
            .map(|(index, asset)| SourceExclusion {
                exclusion_id: format!(
                    "non-player-trade-asset:{}:{}:{}",
                    asset.transaction_row,
                    asset.from.as_str(),
                    index + 1
                ),
                stage: "transaction_asset_classification".to_owned(),
                subject: None,
                reason_code: "non_player_trade_asset".to_owned(),
                message: format!(
                    "Ignored {} -> {} trade asset: {}",
                    asset.from.as_str(),
                    asset.to.as_str(),
                    asset.description
                ),
                source_ids: vec![publication.evidence.source_id().clone()],
            })
            .collect(),
        disclosures: Vec::new(),
    };
    let mut unresolved = 0usize;
    for transfer in &publication.transfers {
        let proposal = proposals.get(&transfer.proposal_id).ok_or_else(|| {
            SourceContractError::UnknownDecisionProposal {
                decision_id: "staged_rights_transfer".to_owned(),
                proposal_id: transfer.proposal_id.to_string(),
            }
        })?;
        let Some(decision) = decisions.get(&transfer.proposal_id) else {
            unresolved += 1;
            continue;
        };
        if decision.action() == IdentityReviewAction::Reject {
            output.exclusions.push(SourceExclusion {
                exclusion_id: format!("rejected-trade:{}", transfer.proposal_id),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "Trade publication identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: proposal
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
            FactId::try_new(format!("reviewed-trade:{}", transfer.proposal_id))?,
            format!("player:{}:rights_transfer", player_id.0),
            FactSubject::Player(player_id),
            transfer.occurred_at.clone(),
            FactAuthority::LegalControl,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::RightsTransferred {
                from: transfer.from.clone(),
                to: transfer.to.clone(),
            }),
            proposal.evidence().to_vec(),
        )?);
    }
    if unresolved > 0 {
        output.disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::UnresolvedIdentity,
            scope: "trade_tracker".to_owned(),
            message: format!(
                "{unresolved} staged trade player leg(s) remain outside canonical facts pending identity review."
            ),
        });
    }
    Ok(output)
}

pub fn lower_reviewed_ahl_roster(
    roster: &AhlRosterStatsOutput,
    decisions: &[IdentityReviewDecision],
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let decisions = decision_index(decisions)?;
    let proposals = roster
        .identity_proposals
        .iter()
        .map(|proposal| (proposal.proposal_id(), proposal))
        .collect::<BTreeMap<_, _>>();
    let mut output = ReviewedPublicationLowering {
        assertions: Vec::new(),
        exclusions: Vec::new(),
        disclosures: vec![SourceDisclosure {
            code: SourceDisclosureCode::ParticipationOnly,
            scope: "ahl_roster".to_owned(),
            message: "An AHL roster observation establishes the player's AHL club, not NHL-affiliate contract or control.".to_owned(),
        }],
    };
    let mut unresolved = 0usize;
    for observation in &roster.roster_observations {
        let proposal = proposals.get(&observation.proposal_id).ok_or_else(|| {
            SourceContractError::UnknownDecisionProposal {
                decision_id: "staged_ahl_roster".to_owned(),
                proposal_id: observation.proposal_id.to_string(),
            }
        })?;
        let Some(decision) = decisions.get(&observation.proposal_id) else {
            unresolved += 1;
            continue;
        };
        if decision.action() == IdentityReviewAction::Reject {
            output.exclusions.push(SourceExclusion {
                exclusion_id: format!("rejected-ahl-roster:{}", observation.proposal_id),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "AHL roster identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: proposal
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
        let event = match &observation.nhl_affiliate {
            Some(affiliate) => PlayerOrganizationEvent::AffiliateRostered {
                affiliate: affiliate.clone(),
                at: observation.ahl_club.clone(),
            },
            None => PlayerOrganizationEvent::Rostered {
                at: observation.ahl_club.clone(),
            },
        };
        output.assertions.push(FactAssertion::new(
            FactId::try_new(format!("reviewed-ahl-roster:{}", observation.proposal_id))?,
            format!(
                "player:{}:rostered:{}",
                player_id.0,
                observation.ahl_club.as_str()
            ),
            FactSubject::Player(player_id),
            EffectiveTime::new(observation.observed_at, None, EffectivePrecision::Instant)?,
            FactAuthority::Assignment,
            SourceFact::PlayerOrganization(event),
            proposal.evidence().to_vec(),
        )?);
    }
    if unresolved > 0 {
        output.disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::UnresolvedIdentity,
            scope: "ahl_roster".to_owned(),
            message: format!(
                "{unresolved} staged AHL roster identity or identities remain outside canonical facts pending review."
            ),
        });
    }
    Ok(output)
}

pub fn lower_reviewed_termination_publication(
    publication: &TerminationPublicationOutput,
    decisions: &[IdentityReviewDecision],
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let decisions = decision_index(decisions)?;
    let proposal = &publication.identity_proposal;
    let Some(decision) = decisions.get(&publication.release.proposal_id) else {
        return Ok(ReviewedPublicationLowering {
            assertions: Vec::new(),
            exclusions: Vec::new(),
            disclosures: vec![SourceDisclosure {
                code: SourceDisclosureCode::UnresolvedIdentity,
                scope: "contract_termination_publication".to_owned(),
                message: "The staged player release remains outside canonical facts pending identity review.".to_owned(),
            }],
        });
    };
    if decision.action() == IdentityReviewAction::Reject {
        return Ok(ReviewedPublicationLowering {
            assertions: Vec::new(),
            exclusions: vec![SourceExclusion {
                exclusion_id: format!("rejected-termination:{}", publication.release.proposal_id),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "Termination publication identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: proposal
                    .evidence()
                    .iter()
                    .map(|evidence| evidence.source_id().clone())
                    .collect(),
            }],
            disclosures: Vec::new(),
        });
    }
    let player_id = decision
        .canonical_player_id()
        .ok_or(SourceContractError::MissingDecisionPlayer)?;
    let assertion = FactAssertion::new(
        FactId::try_new(format!(
            "reviewed-termination:{}",
            publication.release.proposal_id
        ))?,
        format!(
            "player:{}:released:{}",
            player_id.0,
            publication.release.organization.as_str()
        ),
        FactSubject::Player(player_id),
        publication.release.occurred_at.clone(),
        FactAuthority::LegalControl,
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Released {
            by: publication.release.organization.clone(),
        }),
        proposal.evidence().to_vec(),
    )?;
    Ok(ReviewedPublicationLowering {
        assertions: vec![assertion],
        exclusions: Vec::new(),
        disclosures: Vec::new(),
    })
}

pub fn lower_reviewed_draft_picks(
    ledger: &DraftPicksOutput,
    decisions: &[IdentityReviewDecision],
) -> Result<ReviewedPublicationLowering, SourceContractError> {
    let decisions = decision_index(decisions)?;
    let proposals = ledger
        .identity_proposals
        .iter()
        .map(|proposal| (proposal.proposal_id(), proposal))
        .collect::<BTreeMap<_, _>>();
    let mut output = ReviewedPublicationLowering {
        assertions: Vec::new(),
        exclusions: Vec::new(),
        disclosures: Vec::new(),
    };
    let mut unresolved = 0usize;
    for selection in &ledger.selections {
        let proposal = proposals.get(&selection.proposal_id).ok_or_else(|| {
            SourceContractError::UnknownDecisionProposal {
                decision_id: "staged_draft_selection".to_owned(),
                proposal_id: selection.proposal_id.to_string(),
            }
        })?;
        let Some(decision) = decisions.get(&selection.proposal_id) else {
            unresolved += 1;
            continue;
        };
        if decision.action() == IdentityReviewAction::Reject {
            output.exclusions.push(SourceExclusion {
                exclusion_id: format!("rejected-draft:{}", selection.proposal_id),
                stage: "identity_review".to_owned(),
                subject: None,
                reason_code: "identity_rejected".to_owned(),
                message: format!(
                    "Draft ledger identity `{}` was explicitly rejected.",
                    proposal.displayed_name()
                ),
                source_ids: proposal
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
            FactId::try_new(format!("reviewed-draft:{}", selection.proposal_id))?,
            format!("player:{}:draft", player_id.0),
            FactSubject::Player(player_id),
            selection.occurred_at.clone(),
            FactAuthority::Draft,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by: selection.organization.clone(),
                year: selection.year,
                round: selection.round,
                overall: selection.overall,
            }),
            proposal.evidence().to_vec(),
        )?);
    }
    if unresolved > 0 {
        output.disclosures.push(SourceDisclosure {
            code: SourceDisclosureCode::UnresolvedIdentity,
            scope: "nhl_draft_picks".to_owned(),
            message: format!(
                "{unresolved} staged draft selection(s) remain outside canonical facts pending identity review."
            ),
        });
    }
    Ok(output)
}

fn decision_index(
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
