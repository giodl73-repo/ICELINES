//! Fetch-owned assembly of pure normalized source fragments.

use chrono::{DateTime, Utc};
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterVersion, ContentHash, IdentityReviewDecision, PackageId, PolicyVersion, SourceConflict,
    SourceContractError, SourceCoverageBucket, SourceDisclosure, SourceInputRecord, SourcePackage,
    SourceRunManifest,
};
use icelines_sources::fragment::SourcePackageFragment;

#[derive(Debug, Clone)]
pub struct SourcePackageBuildInput {
    pub package_id: PackageId,
    pub evaluation_season: Season,
    pub effective_cutoff: DateTime<Utc>,
    pub knowledge_cutoff: DateTime<Utc>,
    pub adapter_registry_version: AdapterVersion,
    pub reconciliation_policy_version: PolicyVersion,
    pub review_registry_fingerprint: ContentHash,
    pub run_manifest: SourceRunManifest,
    pub inputs: Vec<SourceInputRecord>,
    pub identity_review_decisions: Vec<IdentityReviewDecision>,
    pub conflicts: Vec<SourceConflict>,
    pub coverage: Vec<SourceCoverageBucket>,
    pub disclosures: Vec<SourceDisclosure>,
}

pub fn build_source_package(
    input: SourcePackageBuildInput,
    fragments: impl IntoIterator<Item = SourcePackageFragment>,
) -> Result<SourcePackage, SourceContractError> {
    let fragment = fragments
        .into_iter()
        .fold(SourcePackageFragment::default(), |combined, fragment| {
            combined.combine(fragment)
        });
    SourcePackage::seal(
        input.package_id,
        input.evaluation_season,
        input.effective_cutoff,
        input.knowledge_cutoff,
        input.adapter_registry_version,
        input.reconciliation_policy_version,
        input.review_registry_fingerprint,
        input.run_manifest,
        input.inputs,
        fragment.fact_assertions,
        fragment.identity_proposals,
        input.identity_review_decisions,
        input.conflicts,
        fragment.exclusions,
        input.coverage,
        input.disclosures,
    )
    .and_then(|package| package.with_staged_player_assertions(fragment.staged_player_assertions))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use icelines_core::source_facts::{
        EffectivePrecision, EffectiveTime, FactAuthority, OrganizationId, ParticipationAuthority,
        ParticipationKind, PlayerParticipationFact, ProposalId, ProviderId,
        ProviderIdentityProposal, ProviderPersonLocator, SourceEvidence, SourceFact, SourceId,
        SourceObjectOutcome, SourceObjectState, SourceUrl, StagedAssertionId,
        StagedPlayerAssertion,
    };

    fn hash(character: char) -> ContentHash {
        ContentHash::try_new(character.to_string().repeat(64)).unwrap()
    }

    fn at(hour: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 31, hour, 0, 0)
            .single()
            .unwrap()
    }

    #[test]
    fn assembler_seals_fragments_without_losing_staged_facts() {
        let evidence = SourceEvidence::new(
            SourceId::try_new("camp").unwrap(),
            SourceUrl::try_new("https://example.com/camp").unwrap(),
            ProviderId::try_new("fixture").unwrap(),
            at(10),
            hash('a'),
            AdapterVersion::try_new("v1").unwrap(),
        );
        let proposal_id = ProposalId::try_new("camp-player-1").unwrap();
        let fragment = SourcePackageFragment {
            identity_proposals: vec![ProviderIdentityProposal::new(
                proposal_id.clone(),
                ProviderPersonLocator::SourceRow {
                    source_id: SourceId::try_new("camp").unwrap(),
                    row_key: "row-1".to_owned(),
                },
                "Camp Player",
                None,
                None,
                vec![evidence.clone()],
            )
            .unwrap()],
            staged_player_assertions: vec![StagedPlayerAssertion::new(
                StagedAssertionId::try_new("staged-camp-player-1").unwrap(),
                "proposal:camp-player-1:camp",
                proposal_id,
                EffectiveTime::new(at(9), None, EffectivePrecision::Unknown).unwrap(),
                FactAuthority::Attendance,
                SourceFact::PlayerParticipation(PlayerParticipationFact {
                    organization: OrganizationId::try_new("SEA").unwrap(),
                    season: Season(20_262_027),
                    kind: ParticipationKind::TrainingCamp,
                    authority: ParticipationAuthority::FreeAgentInvite,
                }),
                vec![evidence],
            )
            .unwrap()],
            ..SourcePackageFragment::default()
        };
        let package = build_source_package(
            SourcePackageBuildInput {
                package_id: PackageId::try_new("all-32-2026-27").unwrap(),
                evaluation_season: Season(20_262_027),
                effective_cutoff: at(12),
                knowledge_cutoff: at(12),
                adapter_registry_version: AdapterVersion::try_new("registry.v1").unwrap(),
                reconciliation_policy_version: PolicyVersion::try_new("reconcile.v1").unwrap(),
                review_registry_fingerprint: hash('f'),
                run_manifest: SourceRunManifest {
                    requested_scope: "fixture".to_owned(),
                    source_catalog_version: "catalog.v1".to_owned(),
                    objects: vec![SourceObjectOutcome {
                        object_id: "SEA:camp".to_owned(),
                        source_family: "camp".to_owned(),
                        organization: Some(OrganizationId::try_new("SEA").unwrap()),
                        terminal_pagination: true,
                        state: SourceObjectState::Acquired { records: 1 },
                    }],
                    complete: true,
                },
                inputs: Vec::new(),
                identity_review_decisions: Vec::new(),
                conflicts: Vec::new(),
                coverage: Vec::new(),
                disclosures: Vec::new(),
            },
            [fragment],
        )
        .unwrap();

        package.validate().unwrap();
        assert_eq!(package.identity_proposals.len(), 1);
        assert_eq!(package.staged_player_assertions.len(), 1);
    }
}
