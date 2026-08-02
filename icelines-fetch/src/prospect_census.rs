//! Composition of sealed source packages and prospect-pipeline evidence into
//! the provider-neutral census view.

use icelines_core::identity::PlayerId;
use icelines_core::source_facts::{
    FactAssertion, FactSubject, FreshnessStatus, IdentityReviewAction, OrganizationId,
    PlayerOrganizationEvent, SourceFact, SourceObjectState, SourcePackage,
};
use icelines_core::view_model::prospect_census::{
    build_prospect_census, ProspectCensusCandidateInput, ProspectCensusFreshnessStatus,
    ProspectCensusInput, ProspectCensusLossReason, ProspectCensusOrganizationInput,
    ProspectCensusStage, ProspectCensusView, ProspectPopulationAuthorityStatus,
};
use icelines_core::ProspectProgramBoardView;
use icelines_sources::current_state::{
    reconcile_staged_player_assertions, resolve_player_current_state, IdentityReplayMode,
    ReplayCutoffs, RightsStatus, CURRENT_PLAYER_STATE_POLICY_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{ProspectCareerDiscoveryView, ProspectLeagueDiscoveryView};

pub const PROSPECT_CENSUS_COMPOSER_VERSION: &str = "prospect-census-composer.v1";
pub const PROSPECT_CENSUS_PIPELINE_SCHEMA: &str = "prospect_census_pipeline.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProspectCensusPlayerPipelineEvidence {
    pub player_id: PlayerId,
    pub player_class: String,
    pub position_group: String,
    /// `None` means eligibility cannot be evaluated from available evidence.
    pub prospect_eligible: Option<bool>,
    pub career_evidence_usable: bool,
    pub study_built: bool,
    pub ranked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProspectCensusPipelineEvidence {
    pub schema: String,
    pub eligibility_policy_version: String,
    pub players: Vec<ProspectCensusPlayerPipelineEvidence>,
}

impl ProspectCensusPipelineEvidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != PROSPECT_CENSUS_PIPELINE_SCHEMA
            || self.eligibility_policy_version.trim().is_empty()
        {
            return Err(format!(
                "prospect census pipeline must use {PROSPECT_CENSUS_PIPELINE_SCHEMA} and name its eligibility policy"
            ));
        }
        pipeline_index(&self.players).map(|_| ())
    }
}

pub fn build_prospect_census_pipeline_from_discoveries(
    league_discoveries: &[ProspectLeagueDiscoveryView],
    career_discoveries: &[ProspectCareerDiscoveryView],
    program: &ProspectProgramBoardView,
    eligibility_policy_version: &str,
) -> Result<ProspectCensusPipelineEvidence, String> {
    if league_discoveries.is_empty() && career_discoveries.is_empty() {
        return Err("prospect census pipeline requires at least one discovery artifact".to_owned());
    }
    if eligibility_policy_version.trim().is_empty() {
        return Err("prospect census pipeline requires an eligibility policy version".to_owned());
    }
    let ranked = program
        .programs
        .iter()
        .flat_map(|organization| organization.top_prospects.iter())
        .map(|player| PlayerId(player.player_id))
        .collect::<BTreeSet<_>>();
    let mut rows = BTreeMap::<PlayerId, ProspectCensusPlayerPipelineEvidence>::new();

    for discovery in league_discoveries {
        for study in &discovery.studies {
            merge_study(
                &mut rows,
                study.player_id,
                "prospect",
                position_group(&study.position),
                ranked.contains(&PlayerId(study.player_id)),
            )?;
        }
        for study in &discovery.goalie_studies {
            merge_study(
                &mut rows,
                study.player_id,
                "prospect",
                "goalie",
                ranked.contains(&PlayerId(study.player_id)),
            )?;
        }
        for exclusion in &discovery.excluded {
            merge_missing_career(&mut rows, exclusion.player_id)?;
        }
    }
    for discovery in career_discoveries {
        for study in &discovery.studies {
            merge_study(
                &mut rows,
                study.player_id,
                "prospect",
                position_group(&study.position),
                ranked.contains(&PlayerId(study.player_id)),
            )?;
        }
        for study in &discovery.goalie_studies {
            merge_study(
                &mut rows,
                study.player_id,
                "prospect",
                "goalie",
                ranked.contains(&PlayerId(study.player_id)),
            )?;
        }
        for exclusion in &discovery.excluded {
            merge_missing_career(&mut rows, exclusion.player_id)?;
        }
    }
    let absent_ranked = ranked
        .iter()
        .filter(|player_id| !rows.contains_key(player_id))
        .map(|player_id| player_id.0)
        .collect::<Vec<_>>();
    if !absent_ranked.is_empty() {
        return Err(format!(
            "program board ranks player(s) absent from supplied discovery artifacts: {}",
            absent_ranked
                .iter()
                .map(u32::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    let pipeline = ProspectCensusPipelineEvidence {
        schema: PROSPECT_CENSUS_PIPELINE_SCHEMA.to_owned(),
        eligibility_policy_version: eligibility_policy_version.to_owned(),
        players: rows.into_values().collect(),
    };
    pipeline.validate()?;
    Ok(pipeline)
}

fn merge_study(
    rows: &mut BTreeMap<PlayerId, ProspectCensusPlayerPipelineEvidence>,
    player_id: u32,
    player_class: &str,
    position_group: &str,
    ranked: bool,
) -> Result<(), String> {
    let row = ProspectCensusPlayerPipelineEvidence {
        player_id: PlayerId(player_id),
        player_class: player_class.to_owned(),
        position_group: position_group.to_owned(),
        prospect_eligible: Some(true),
        career_evidence_usable: true,
        study_built: true,
        ranked,
    };
    if let Some(existing) = rows.get(&row.player_id) {
        if !existing.career_evidence_usable {
            rows.insert(row.player_id, row);
        } else if existing != &row {
            return Err(format!(
                "conflicting discovery pipeline evidence for player {}",
                player_id
            ));
        }
    } else {
        rows.insert(row.player_id, row);
    }
    Ok(())
}

fn merge_missing_career(
    rows: &mut BTreeMap<PlayerId, ProspectCensusPlayerPipelineEvidence>,
    player_id: u32,
) -> Result<(), String> {
    if player_id == 0 {
        return Err("discovery exclusion contains zero player id".to_owned());
    }
    rows.entry(PlayerId(player_id))
        .or_insert_with(|| ProspectCensusPlayerPipelineEvidence {
            player_id: PlayerId(player_id),
            player_class: "prospect".to_owned(),
            position_group: "unknown".to_owned(),
            prospect_eligible: Some(true),
            career_evidence_usable: false,
            study_built: false,
            ranked: false,
        });
    Ok(())
}

fn position_group(position: &str) -> &'static str {
    if position.eq_ignore_ascii_case("G") || position.eq_ignore_ascii_case("goalie") {
        "goalie"
    } else {
        "skater"
    }
}

pub fn build_prospect_census_from_source_package(
    package: &SourcePackage,
    pipeline: &[ProspectCensusPlayerPipelineEvidence],
    requested_ranking_depth: usize,
    eligibility_policy_version: &str,
) -> Result<ProspectCensusView, String> {
    package.validate().map_err(|error| error.to_string())?;
    if requested_ranking_depth == 0 || eligibility_policy_version.trim().is_empty() {
        return Err(
            "census composition requires non-zero depth and an eligibility policy version"
                .to_owned(),
        );
    }
    let pipeline = pipeline_index(pipeline)?;
    let cutoffs = ReplayCutoffs {
        effective_cutoff: package.effective_cutoff,
        knowledge_cutoff: package.knowledge_cutoff,
        identity_mode: IdentityReplayMode::AsKnown,
    };
    let reconciled = reconcile_staged_player_assertions(
        &package.identity_proposals,
        &package.staged_player_assertions,
        &package.identity_review_decisions,
        cutoffs,
    )
    .map_err(|error| error.to_string())?;
    let mut canonical_facts = package.fact_assertions.clone();
    canonical_facts.extend(reconciled.assertions);
    canonical_facts.sort_by(|left, right| left.fact_id().cmp(right.fact_id()));

    let organization_inputs = organization_inputs(package, requested_ranking_depth);
    let organization_scope = organization_inputs
        .iter()
        .map(|row| row.organization.as_str())
        .collect::<BTreeSet<_>>();
    let decisions = package
        .identity_review_decisions
        .iter()
        .map(|decision| (decision.proposal_id(), decision))
        .collect::<BTreeMap<_, _>>();
    let mut candidates = Vec::new();

    for staged in &package.staged_player_assertions {
        let Some(organization) = fact_organization(staged.fact()) else {
            continue;
        };
        if !organization_scope.contains(organization.as_str()) {
            continue;
        }
        let decision = decisions.get(staged.proposal_id());
        if decision.is_some_and(|decision| decision.action() != IdentityReviewAction::Reject) {
            continue;
        }
        let rejected = decision.is_some();
        candidates.push(ProspectCensusCandidateInput {
            candidate_key: format!("proposal:{}", staged.proposal_id()),
            organization: organization.as_str().to_owned(),
            discovery_source_family: source_family(staged.fact()).to_owned(),
            player_class: "unresolved_candidate".to_owned(),
            position_group: "unknown".to_owned(),
            player_id: None,
            reached_stage: ProspectCensusStage::Discovered,
            loss_reason: Some(if rejected {
                ProspectCensusLossReason::ExcludedByPolicy
            } else {
                ProspectCensusLossReason::UnresolvedIdentity
            }),
            loss_message: Some(if rejected {
                "The provider identity was explicitly rejected and cannot enter canonical prospect facts."
                    .to_owned()
            } else {
                "The provider identity has no reviewed canonical player mapping at the knowledge cutoff."
                    .to_owned()
            }),
        });
    }

    let discoveries = resolved_discoveries(&canonical_facts, &organization_scope);
    for (player_id, discovery) in discoveries {
        let state = resolve_player_current_state(player_id, &canonical_facts, cutoffs);
        let (organization, source_family) = state
            .rights
            .organization
            .as_ref()
            .filter(|organization| organization_scope.contains(organization.as_str()))
            .map(|organization| (organization.as_str(), discovery.source_family.as_str()))
            .unwrap_or((
                discovery.organization.as_str(),
                discovery.source_family.as_str(),
            ));
        let pipeline_row = pipeline.get(&player_id);
        let (reached_stage, loss_reason, loss_message) =
            player_stage(&state.rights.status, pipeline_row);
        candidates.push(ProspectCensusCandidateInput {
            candidate_key: format!("player:{}", player_id.0),
            organization: organization.to_owned(),
            discovery_source_family: source_family.to_owned(),
            player_class: pipeline_row
                .map(|row| row.player_class.clone())
                .unwrap_or_else(|| "unknown".to_owned()),
            position_group: pipeline_row
                .map(|row| row.position_group.clone())
                .unwrap_or_else(|| "unknown".to_owned()),
            player_id: Some(player_id.0),
            reached_stage,
            loss_reason,
            loss_message,
        });
    }

    candidates.sort_by(|left, right| left.candidate_key.cmp(&right.candidate_key));
    build_prospect_census(ProspectCensusInput {
        evaluation_season: package.evaluation_season.0,
        effective_cutoff: package.effective_cutoff.to_rfc3339(),
        knowledge_cutoff: package.knowledge_cutoff.to_rfc3339(),
        freshness_status: package_freshness(package),
        source_package_fingerprint: package.fingerprint.as_str().to_owned(),
        reconciliation_policy_version: CURRENT_PLAYER_STATE_POLICY_VERSION.to_owned(),
        eligibility_policy_version: eligibility_policy_version.to_owned(),
        organizations: organization_inputs,
        candidates,
    })
}

fn package_freshness(package: &SourcePackage) -> ProspectCensusFreshnessStatus {
    if package.inputs.is_empty() {
        return ProspectCensusFreshnessStatus::Unknown;
    }
    let has_fresh = package.inputs.iter().any(|input| {
        matches!(
            input.freshness.status,
            FreshnessStatus::Fresh | FreshnessStatus::Static
        )
    });
    let has_stale = package
        .inputs
        .iter()
        .any(|input| input.freshness.status == FreshnessStatus::Stale);
    let has_unknown = package
        .inputs
        .iter()
        .any(|input| input.freshness.status == FreshnessStatus::Unknown);
    match (has_fresh, has_stale, has_unknown) {
        (true, false, false) => ProspectCensusFreshnessStatus::Fresh,
        (false, true, false) => ProspectCensusFreshnessStatus::Stale,
        (false, false, true) => ProspectCensusFreshnessStatus::Unknown,
        _ => ProspectCensusFreshnessStatus::Mixed,
    }
}

struct ResolvedDiscovery {
    organization: OrganizationId,
    source_family: String,
    occurred_at: chrono::DateTime<chrono::Utc>,
}

fn resolved_discoveries(
    facts: &[FactAssertion<SourceFact>],
    organization_scope: &BTreeSet<&str>,
) -> BTreeMap<PlayerId, ResolvedDiscovery> {
    let mut discoveries = BTreeMap::new();
    for assertion in facts {
        let FactSubject::Player(player_id) = assertion.subject() else {
            continue;
        };
        let Some(organization) = fact_organization(assertion.fact()) else {
            continue;
        };
        if !organization_scope.contains(organization.as_str()) {
            continue;
        }
        let candidate = ResolvedDiscovery {
            organization: organization.clone(),
            source_family: source_family(assertion.fact()).to_owned(),
            occurred_at: assertion.occurred_at().starts_at,
        };
        let replace = discoveries
            .get(player_id)
            .is_none_or(|current: &ResolvedDiscovery| candidate.occurred_at > current.occurred_at);
        if replace {
            discoveries.insert(*player_id, candidate);
        }
    }
    discoveries
}

fn player_stage(
    rights: &RightsStatus,
    pipeline: Option<&&ProspectCensusPlayerPipelineEvidence>,
) -> (
    ProspectCensusStage,
    Option<ProspectCensusLossReason>,
    Option<String>,
) {
    match rights {
        RightsStatus::Conflicted => {
            return stopped(
                ProspectCensusStage::CanonicalIdentity,
                ProspectCensusLossReason::ConflictingControl,
                "Current organization control is conflicted.",
            )
        }
        RightsStatus::Unknown | RightsStatus::Expired => {
            return stopped(
                ProspectCensusStage::CanonicalIdentity,
                ProspectCensusLossReason::UnsupportedControl,
                "No current controlled organization relationship is supported at the cutoffs.",
            )
        }
        RightsStatus::Supported | RightsStatus::Transferred => {}
    }
    let Some(pipeline) = pipeline else {
        return stopped(
            ProspectCensusStage::ControlledRelationship,
            ProspectCensusLossReason::MissingEligibilityEvidence,
            "No prospect-eligibility evidence was supplied for the controlled player.",
        );
    };
    match pipeline.prospect_eligible {
        None => stopped(
            ProspectCensusStage::ControlledRelationship,
            ProspectCensusLossReason::MissingEligibilityEvidence,
            "Prospect eligibility is unresolved from the supplied age and workload evidence.",
        ),
        Some(false) => stopped(
            ProspectCensusStage::ControlledRelationship,
            ProspectCensusLossReason::ProspectIneligible,
            "The versioned prospect policy classifies the player as ineligible.",
        ),
        Some(true) if !pipeline.career_evidence_usable => stopped(
            ProspectCensusStage::ProspectEligible,
            ProspectCensusLossReason::MissingCareerEvidence,
            "The eligible player lacks usable career evidence for a development study.",
        ),
        Some(true) if !pipeline.study_built => stopped(
            ProspectCensusStage::CareerEvidenceUsable,
            ProspectCensusLossReason::StudyBuildFailed,
            "Usable career evidence did not produce a valid prospect study.",
        ),
        Some(true) if !pipeline.ranked => stopped(
            ProspectCensusStage::StudyBuilt,
            ProspectCensusLossReason::RankingWithheld,
            "The valid study was not admitted to the requested ranking output.",
        ),
        Some(true) => (ProspectCensusStage::Ranked, None, None),
    }
}

fn stopped(
    reached: ProspectCensusStage,
    reason: ProspectCensusLossReason,
    message: &str,
) -> (
    ProspectCensusStage,
    Option<ProspectCensusLossReason>,
    Option<String>,
) {
    (reached, Some(reason), Some(message.to_owned()))
}

fn pipeline_index(
    pipeline: &[ProspectCensusPlayerPipelineEvidence],
) -> Result<BTreeMap<PlayerId, &ProspectCensusPlayerPipelineEvidence>, String> {
    let mut index = BTreeMap::new();
    for row in pipeline {
        if row.player_id.0 == 0
            || row.player_class.trim().is_empty()
            || row.position_group.trim().is_empty()
            || row.ranked && !row.study_built
            || row.study_built && !row.career_evidence_usable
            || row.career_evidence_usable && row.prospect_eligible != Some(true)
            || index.insert(row.player_id, row).is_some()
        {
            return Err(
                "invalid, non-monotonic, or duplicate prospect pipeline evidence".to_owned(),
            );
        }
    }
    Ok(index)
}

fn organization_inputs(
    package: &SourcePackage,
    requested_ranking_depth: usize,
) -> Vec<ProspectCensusOrganizationInput> {
    let mut objects = BTreeMap::<String, Vec<_>>::new();
    for object in &package.run_manifest.objects {
        if let Some(organization) = &object.organization {
            objects
                .entry(organization.as_str().to_owned())
                .or_default()
                .push(object);
        }
    }
    objects
        .into_iter()
        .map(|(organization, objects)| {
            let conflicted = !package.conflicts.is_empty()
                || objects
                    .iter()
                    .any(|object| matches!(object.state, SourceObjectState::Quarantined { .. }));
            let complete = objects.iter().all(|object| {
                matches!(
                    object.state,
                    SourceObjectState::Acquired { .. } | SourceObjectState::NotApplicable { .. }
                ) && (matches!(object.state, SourceObjectState::NotApplicable { .. })
                    || object.terminal_pagination)
            });
            let status = if conflicted {
                ProspectPopulationAuthorityStatus::Conflicted
            } else if complete {
                ProspectPopulationAuthorityStatus::Complete
            } else {
                ProspectPopulationAuthorityStatus::Incomplete
            };
            let authority_disclosures = objects
                .iter()
                .filter_map(|object| match &object.state {
                    SourceObjectState::Failed { reason }
                    | SourceObjectState::Quarantined { reason }
                    | SourceObjectState::NotApplicable { reason } => Some(format!(
                        "{}: {} ({reason})",
                        object.source_family,
                        state_label(&object.state)
                    )),
                    SourceObjectState::IncompletePagination => {
                        Some(format!("{}: incomplete pagination", object.source_family))
                    }
                    SourceObjectState::Acquired { .. } => None,
                })
                .collect();
            ProspectCensusOrganizationInput {
                organization,
                population_authority_status: status,
                requested_ranking_depth,
                authority_disclosures,
            }
        })
        .collect()
}

fn state_label(state: &SourceObjectState) -> &'static str {
    match state {
        SourceObjectState::Acquired { .. } => "acquired",
        SourceObjectState::NotApplicable { .. } => "not applicable",
        SourceObjectState::Failed { .. } => "failed",
        SourceObjectState::Quarantined { .. } => "quarantined",
        SourceObjectState::IncompletePagination => "incomplete pagination",
    }
}

fn fact_organization(fact: &SourceFact) -> Option<&OrganizationId> {
    match fact {
        SourceFact::PlayerOrganization(event) => match event {
            PlayerOrganizationEvent::Drafted { by, .. }
            | PlayerOrganizationEvent::Released { by }
            | PlayerOrganizationEvent::Assigned { by, .. }
            | PlayerOrganizationEvent::Recalled { by, .. }
            | PlayerOrganizationEvent::Loaned { by, .. } => Some(by),
            PlayerOrganizationEvent::ContractSigned { with, .. } => Some(with),
            PlayerOrganizationEvent::RightsTransferred { to, .. } => Some(to),
            PlayerOrganizationEvent::RightsExpired { organization } => Some(organization),
            PlayerOrganizationEvent::AffiliateRostered { affiliate, .. } => Some(affiliate),
            PlayerOrganizationEvent::Rostered { .. } => None,
        },
        SourceFact::PlayerParticipation(fact) => Some(&fact.organization),
        SourceFact::CompatibilityProspectRelationship(fact) => Some(&fact.organization),
    }
}

fn source_family(fact: &SourceFact) -> &'static str {
    match fact {
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted { .. }) => "nhl_draft",
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned { .. }) => {
            "nhl_contract_publication"
        }
        SourceFact::PlayerOrganization(
            PlayerOrganizationEvent::RightsTransferred { .. }
            | PlayerOrganizationEvent::RightsExpired { .. }
            | PlayerOrganizationEvent::Released { .. },
        ) => "nhl_transaction_publication",
        SourceFact::PlayerOrganization(
            PlayerOrganizationEvent::Rostered { .. }
            | PlayerOrganizationEvent::AffiliateRostered { .. },
        ) => "ahl_current_assignment",
        SourceFact::PlayerOrganization(
            PlayerOrganizationEvent::Assigned { .. }
            | PlayerOrganizationEvent::Recalled { .. }
            | PlayerOrganizationEvent::Loaned { .. },
        ) => "nhl_current_assignment",
        SourceFact::PlayerParticipation(_) => "nhl_club_camp_publication",
        SourceFact::CompatibilityProspectRelationship(_) => "compatibility_overlay",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_package_builder::{build_source_package, SourcePackageBuildInput};
    use chrono::{TimeZone, Utc};
    use icelines_core::model::Season;
    use icelines_core::source_facts::{
        AdapterVersion, ContentHash, ContractKind, EffectivePrecision, EffectiveTime,
        FactAuthority, FactId, PackageId, PolicyVersion, ProposalId, ProviderId,
        ProviderIdentityProposal, ProviderPersonLocator, SourceEvidence, SourceId,
        SourceObjectOutcome, SourceRunManifest, SourceUrl, StagedAssertionId,
        StagedPlayerAssertion,
    };
    use icelines_sources::fragment::SourcePackageFragment;

    fn hash(character: char) -> ContentHash {
        ContentHash::try_new(character.to_string().repeat(64)).unwrap()
    }

    fn at(hour: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 31, hour, 0, 0)
            .single()
            .unwrap()
    }

    fn evidence(source: &str) -> SourceEvidence {
        SourceEvidence::new(
            SourceId::try_new(source).unwrap(),
            SourceUrl::try_new("https://example.test/source").unwrap(),
            ProviderId::try_new("fixture").unwrap(),
            at(9),
            hash('a'),
            AdapterVersion::try_new("v1").unwrap(),
        )
    }

    #[test]
    fn usable_study_supersedes_a_missing_career_row_from_another_adapter() {
        let mut rows = BTreeMap::new();
        merge_missing_career(&mut rows, 42).unwrap();
        merge_study(&mut rows, 42, "prospect", "skater", true).unwrap();

        let row = rows.get(&PlayerId(42)).unwrap();
        assert!(row.career_evidence_usable);
        assert!(row.study_built);
        assert!(row.ranked);
        assert_eq!(row.position_group, "skater");
    }

    #[test]
    fn package_composition_keeps_unresolved_loss_and_withholds_incomplete_authority() {
        let contract_evidence = evidence("contract");
        let contract = FactAssertion::new(
            FactId::try_new("contract:42").unwrap(),
            "player:42:contract",
            FactSubject::Player(PlayerId(42)),
            EffectiveTime::new(at(8), None, EffectivePrecision::Day).unwrap(),
            FactAuthority::Contract,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                with: OrganizationId::try_new("SEA").unwrap(),
                contract_kind: ContractKind::EntryLevel,
            }),
            vec![contract_evidence],
        )
        .unwrap();
        let staged_evidence = evidence("draft");
        let proposal_id = ProposalId::try_new("draft:unresolved").unwrap();
        let proposal = ProviderIdentityProposal::new(
            proposal_id.clone(),
            ProviderPersonLocator::SourceRow {
                source_id: staged_evidence.source_id().clone(),
                row_key: "overall:1".to_owned(),
            },
            "Unresolved Prospect",
            None,
            None,
            vec![staged_evidence.clone()],
        )
        .unwrap();
        let staged = StagedPlayerAssertion::new(
            StagedAssertionId::try_new("staged:draft:unresolved").unwrap(),
            "proposal:draft:unresolved:draft",
            proposal_id,
            EffectiveTime::new(at(7), None, EffectivePrecision::Day).unwrap(),
            FactAuthority::Draft,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by: OrganizationId::try_new("SEA").unwrap(),
                year: 2026,
                round: 1,
                overall: 1,
            }),
            vec![staged_evidence],
        )
        .unwrap();
        let package = build_source_package(
            SourcePackageBuildInput {
                package_id: PackageId::try_new("census-fixture").unwrap(),
                evaluation_season: Season(20_262_027),
                effective_cutoff: at(12),
                knowledge_cutoff: at(12),
                adapter_registry_version: AdapterVersion::try_new("registry.v1").unwrap(),
                reconciliation_policy_version: PolicyVersion::try_new("reconcile.v1").unwrap(),
                review_registry_fingerprint: hash('f'),
                run_manifest: SourceRunManifest {
                    requested_scope: "SEA".to_owned(),
                    source_catalog_version: "catalog.v1".to_owned(),
                    objects: vec![
                        SourceObjectOutcome {
                            object_id: "SEA:draft".to_owned(),
                            source_family: "nhl_draft".to_owned(),
                            organization: Some(OrganizationId::try_new("SEA").unwrap()),
                            terminal_pagination: true,
                            state: SourceObjectState::Acquired { records: 1 },
                        },
                        SourceObjectOutcome {
                            object_id: "SEA:ahl".to_owned(),
                            source_family: "ahl_current_assignment".to_owned(),
                            organization: Some(OrganizationId::try_new("SEA").unwrap()),
                            terminal_pagination: false,
                            state: SourceObjectState::Failed {
                                reason: "missing fixture source".to_owned(),
                            },
                        },
                    ],
                    complete: false,
                },
                inputs: Vec::new(),
                identity_review_decisions: Vec::new(),
                conflicts: Vec::new(),
                coverage: Vec::new(),
                disclosures: Vec::new(),
            },
            [SourcePackageFragment {
                fact_assertions: vec![contract],
                identity_proposals: vec![proposal],
                staged_player_assertions: vec![staged],
                ..SourcePackageFragment::default()
            }],
        )
        .unwrap();

        let view = build_prospect_census_from_source_package(
            &package,
            &[ProspectCensusPlayerPipelineEvidence {
                player_id: PlayerId(42),
                player_class: "prospect".to_owned(),
                position_group: "skater".to_owned(),
                prospect_eligible: Some(true),
                career_evidence_usable: true,
                study_built: true,
                ranked: true,
            }],
            1,
            "prospect-eligibility.v1",
        )
        .unwrap();

        assert_eq!(view.organizations.len(), 1);
        assert_eq!(view.organizations[0].counts.ranked, 1);
        assert_eq!(view.organizations[0].counts.discovered, 2);
        assert_eq!(
            view.organizations[0].population_authority_status,
            ProspectPopulationAuthorityStatus::Incomplete
        );
        assert_eq!(view.losses.len(), 1);
        assert_eq!(
            view.losses[0].reason,
            ProspectCensusLossReason::UnresolvedIdentity
        );
    }
}
