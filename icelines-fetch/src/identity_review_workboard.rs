//! Projection of sealed source-package proposals into the UI-neutral review queue.

use icelines_core::source_facts::{
    PlayerOrganizationEvent, SourceFact, SourcePackage, StagedPlayerAssertion,
};
use icelines_core::{
    build_identity_review_workboard, IdentityReviewContextInput, IdentityReviewDraftCoordinates,
    IdentityReviewProposalInput, IdentityReviewWorkboardInput, IdentityReviewWorkboardView,
};
use std::collections::{BTreeMap, BTreeSet};

pub fn build_identity_review_workboard_from_source_package(
    package: &SourcePackage,
) -> Result<IdentityReviewWorkboardView, String> {
    package.validate().map_err(|error| error.to_string())?;
    let decided = package
        .identity_review_decisions
        .iter()
        .map(|decision| decision.proposal_id().as_str())
        .collect::<BTreeSet<_>>();
    let mut contexts = BTreeMap::<&str, Vec<IdentityReviewContextInput>>::new();
    for assertion in &package.staged_player_assertions {
        contexts
            .entry(assertion.proposal_id().as_str())
            .or_default()
            .push(context(assertion));
    }
    let proposals = package
        .identity_proposals
        .iter()
        .filter(|proposal| !decided.contains(proposal.proposal_id().as_str()))
        .map(|proposal| {
            let evidence = proposal.evidence();
            IdentityReviewProposalInput {
                proposal_id: proposal.proposal_id().to_string(),
                displayed_name: proposal.displayed_name().to_owned(),
                birth_date: proposal.birth_date().map(str::to_owned),
                proposed_player_id: proposal.proposed_player_id().map(|id| id.0),
                providers: evidence
                    .iter()
                    .map(|item| item.provider().as_str().to_owned())
                    .collect(),
                evidence_urls: evidence
                    .iter()
                    .map(|item| item.source_url().as_str().to_owned())
                    .collect(),
                evidence: evidence.to_vec(),
                contexts: contexts
                    .remove(proposal.proposal_id().as_str())
                    .unwrap_or_default(),
            }
        })
        .collect();
    build_identity_review_workboard(IdentityReviewWorkboardInput {
        evaluation_season: package.evaluation_season.0,
        source_package_id: package.package_id.to_string(),
        source_package_fingerprint: package.fingerprint.to_string(),
        effective_cutoff: package.effective_cutoff.to_rfc3339(),
        knowledge_cutoff: package.knowledge_cutoff.to_rfc3339(),
        proposals,
    })
}

fn context(assertion: &StagedPlayerAssertion) -> IdentityReviewContextInput {
    match assertion.fact() {
        SourceFact::PlayerOrganization(event) => organization_context(event),
        SourceFact::PlayerParticipation(fact) => IdentityReviewContextInput {
            family: "participation".to_owned(),
            organization: Some(fact.organization.as_str().to_owned()),
            draft: None,
            detail: format!("{:?} {:?}", fact.kind, fact.authority),
        },
        SourceFact::CompatibilityProspectRelationship(fact) => IdentityReviewContextInput {
            family: "compatibility_relationship".to_owned(),
            organization: Some(fact.organization.as_str().to_owned()),
            draft: None,
            detail: format!("{:?}", fact.relationship),
        },
    }
}

fn organization_context(event: &PlayerOrganizationEvent) -> IdentityReviewContextInput {
    match event {
        PlayerOrganizationEvent::Drafted {
            by,
            year,
            round,
            overall,
        } => IdentityReviewContextInput {
            family: "draft".to_owned(),
            organization: Some(by.as_str().to_owned()),
            draft: Some(IdentityReviewDraftCoordinates {
                year: *year,
                round: *round,
                overall: *overall,
            }),
            detail: format!("{year} round {round} overall {overall}"),
        },
        PlayerOrganizationEvent::ContractSigned {
            with,
            contract_kind,
        } => IdentityReviewContextInput {
            family: "contract".to_owned(),
            organization: Some(with.as_str().to_owned()),
            draft: None,
            detail: format!("{:?}", contract_kind),
        },
        PlayerOrganizationEvent::RightsTransferred { from, to } => IdentityReviewContextInput {
            family: "rights_transfer".to_owned(),
            organization: Some(to.as_str().to_owned()),
            draft: None,
            detail: format!("{} to {}", from.as_str(), to.as_str()),
        },
        PlayerOrganizationEvent::RightsExpired { organization } => IdentityReviewContextInput {
            family: "rights_expiry".to_owned(),
            organization: Some(organization.as_str().to_owned()),
            draft: None,
            detail: "rights expired".to_owned(),
        },
        PlayerOrganizationEvent::Assigned { by, to } => IdentityReviewContextInput {
            family: "assignment".to_owned(),
            organization: Some(by.as_str().to_owned()),
            draft: None,
            detail: format!("assigned to {}", to.as_str()),
        },
        PlayerOrganizationEvent::Rostered { at } => IdentityReviewContextInput {
            family: "roster".to_owned(),
            organization: None,
            draft: None,
            detail: format!("rostered at {}", at.as_str()),
        },
        PlayerOrganizationEvent::AffiliateRostered { affiliate, at } => {
            IdentityReviewContextInput {
                family: "affiliate_roster".to_owned(),
                organization: Some(affiliate.as_str().to_owned()),
                draft: None,
                detail: format!("rostered at {}", at.as_str()),
            }
        }
        PlayerOrganizationEvent::Recalled { by, from, to } => IdentityReviewContextInput {
            family: "recall".to_owned(),
            organization: Some(by.as_str().to_owned()),
            draft: None,
            detail: format!("{} to {}", from.as_str(), to.as_str()),
        },
        PlayerOrganizationEvent::Loaned { by, to } => IdentityReviewContextInput {
            family: "loan".to_owned(),
            organization: Some(by.as_str().to_owned()),
            draft: None,
            detail: format!("loaned to {}", to.as_str()),
        },
        PlayerOrganizationEvent::Released { by } => IdentityReviewContextInput {
            family: "release".to_owned(),
            organization: Some(by.as_str().to_owned()),
            draft: None,
            detail: "released".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use icelines_core::identity::PlayerId;
    use icelines_core::model::Season;
    use icelines_core::source_facts::*;

    fn evidence() -> SourceEvidence {
        SourceEvidence::new(
            SourceId::try_new("draft").unwrap(),
            SourceUrl::try_new("https://example.test/draft").unwrap(),
            ProviderId::try_new("official_nhl").unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).single().unwrap(),
            ContentHash::try_new("a".repeat(64)).unwrap(),
            AdapterVersion::try_new("v1").unwrap(),
        )
    }

    #[test]
    fn excludes_decided_proposals_and_preserves_draft_context() {
        let proposal = |id: &str, name: &str| {
            ProviderIdentityProposal::new(
                ProposalId::try_new(id).unwrap(),
                ProviderPersonLocator::SourceRow {
                    source_id: SourceId::try_new("draft").unwrap(),
                    row_key: id.to_owned(),
                },
                name,
                None,
                None,
                vec![evidence()],
            )
            .unwrap()
        };
        let staged = |id: &str, overall| {
            StagedPlayerAssertion::new(
                StagedAssertionId::try_new(format!("staged-{id}")).unwrap(),
                format!("draft-{id}"),
                ProposalId::try_new(id).unwrap(),
                EffectiveTime::new(
                    Utc.with_ymd_and_hms(2026, 6, 26, 0, 0, 0).single().unwrap(),
                    None,
                    EffectivePrecision::Day,
                )
                .unwrap(),
                FactAuthority::Draft,
                SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                    by: OrganizationId::try_new("NYR").unwrap(),
                    year: 2026,
                    round: 1,
                    overall,
                }),
                vec![evidence()],
            )
            .unwrap()
        };
        let mut package = SourcePackage::seal(
            PackageId::try_new("fixture").unwrap(),
            Season(20_262_027),
            Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap(),
            AdapterVersion::try_new("v1").unwrap(),
            PolicyVersion::try_new("v1").unwrap(),
            ContentHash::try_new("b".repeat(64)).unwrap(),
            SourceRunManifest {
                requested_scope: "fixture".to_owned(),
                source_catalog_version: "v1".to_owned(),
                objects: vec![],
                complete: true,
            },
            vec![],
            vec![],
            vec![
                proposal("p1", "Pending Player"),
                proposal("p2", "Done Player"),
            ],
            vec![IdentityReviewDecision::new(
                DecisionId::try_new("d2").unwrap(),
                ProposalId::try_new("p2").unwrap(),
                IdentityReviewAction::SetIdentity,
                Some(PlayerId(8_480_002)),
                "reviewer",
                Utc.with_ymd_and_hms(2026, 7, 2, 0, 0, 0).single().unwrap(),
                "reviewed",
                vec![evidence()],
            )
            .unwrap()],
            vec![],
            vec![],
            vec![],
            vec![],
        )
        .unwrap();
        package = package
            .with_staged_player_assertions(vec![staged("p1", 5), staged("p2", 6)])
            .unwrap();
        let view = build_identity_review_workboard_from_source_package(&package).unwrap();
        assert_eq!(view.unresolved_count, 1);
        assert_eq!(view.rows[0].proposal_id, "p1");
        assert_eq!(
            view.rows[0].contexts[0].organization.as_deref(),
            Some("NYR")
        );
    }
}
