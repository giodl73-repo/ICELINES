use chrono::{TimeZone, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    AdapterVersion, ClubRef, ContentHash, ContractKind, DecisionId, EffectivePrecision,
    EffectiveTime, FactAssertion, FactAuthority, FactId, FactSubject, IdentityReviewAction,
    IdentityReviewDecision, OrganizationId, ParticipationAuthority, ParticipationKind,
    PlayerOrganizationEvent, PlayerParticipationFact, ProposalId, ProviderId,
    ProviderIdentityProposal, ProviderPersonLocator, SourceEvidence, SourceFact, SourceId,
    SourceUrl, StagedAssertionId, StagedPlayerAssertion,
};
use icelines_sources::current_state::{
    reconcile_staged_player_assertions, resolve_player_current_state, AssignmentStatus,
    IdentityReplayMode, ReplayCutoffs, RightsStatus, CURRENT_PLAYER_STATE_POLICY_VERSION,
};

fn at(day: u32) -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, day, 12, 0, 0)
        .single()
        .unwrap()
}

fn evidence(captured_at: chrono::DateTime<Utc>) -> SourceEvidence {
    SourceEvidence::new(
        SourceId::try_new(format!("fixture:{}", captured_at.timestamp())).unwrap(),
        SourceUrl::try_new("https://example.test/source").unwrap(),
        ProviderId::try_new("fixture").unwrap(),
        captured_at,
        ContentHash::try_new("0".repeat(64)).unwrap(),
        AdapterVersion::try_new("v1").unwrap(),
    )
}

fn fact(
    id: &str,
    occurred_at: chrono::DateTime<Utc>,
    captured_at: chrono::DateTime<Utc>,
    authority: FactAuthority,
    value: SourceFact,
) -> FactAssertion<SourceFact> {
    FactAssertion::new(
        FactId::try_new(id).unwrap(),
        format!("fixture:{id}"),
        FactSubject::Player(PlayerId(42)),
        EffectiveTime::new(occurred_at, None, EffectivePrecision::Instant).unwrap(),
        authority,
        value,
        vec![evidence(captured_at)],
    )
    .unwrap()
}

fn cutoffs(effective_day: u32, knowledge_day: u32) -> ReplayCutoffs {
    ReplayCutoffs {
        effective_cutoff: at(effective_day),
        knowledge_cutoff: at(knowledge_day),
        identity_mode: IdentityReplayMode::AsKnown,
    }
}

#[test]
fn draft_and_camp_are_visible_but_never_claim_current_rights() {
    let facts = vec![
        fact(
            "draft",
            at(1),
            at(1),
            FactAuthority::Draft,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
                by: OrganizationId::try_new("NYR").unwrap(),
                year: 2026,
                round: 1,
                overall: 5,
            }),
        ),
        fact(
            "camp",
            at(2),
            at(2),
            FactAuthority::Attendance,
            SourceFact::PlayerParticipation(PlayerParticipationFact {
                organization: OrganizationId::try_new("NYR").unwrap(),
                season: Season(20_262_027),
                kind: ParticipationKind::DevelopmentCamp,
                authority: ParticipationAuthority::ControlledPlayer,
            }),
        ),
    ];
    let state = resolve_player_current_state(PlayerId(42), &facts, cutoffs(10, 10));

    assert_eq!(state.policy_version, CURRENT_PLAYER_STATE_POLICY_VERSION);
    assert_eq!(state.rights.status, RightsStatus::Unknown);
    assert_eq!(state.assignment.status, AssignmentStatus::Unknown);
    assert_eq!(state.participation_only.len(), 1);
    assert_eq!(state.input_fact_ids.len(), 2);
}

#[test]
fn contract_transfer_and_assignment_resolve_independently() {
    let facts = vec![
        fact(
            "contract",
            at(1),
            at(1),
            FactAuthority::Contract,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                with: OrganizationId::try_new("UTA").unwrap(),
                contract_kind: ContractKind::EntryLevel,
            }),
        ),
        fact(
            "transfer",
            at(2),
            at(2),
            FactAuthority::LegalControl,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::RightsTransferred {
                from: OrganizationId::try_new("UTA").unwrap(),
                to: OrganizationId::try_new("NYR").unwrap(),
            }),
        ),
        fact(
            "assignment",
            at(3),
            at(3),
            FactAuthority::Assignment,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Assigned {
                by: OrganizationId::try_new("NYR").unwrap(),
                to: ClubRef::try_new("Hartford Wolf Pack").unwrap(),
            }),
        ),
    ];
    let state = resolve_player_current_state(PlayerId(42), &facts, cutoffs(10, 10));

    assert_eq!(state.rights.status, RightsStatus::Transferred);
    assert_eq!(state.rights.organization.unwrap().as_str(), "NYR");
    assert_eq!(state.assignment.status, AssignmentStatus::Assigned);
    assert_eq!(
        state.assignment.club.unwrap().as_str(),
        "Hartford Wolf Pack"
    );
}

#[test]
fn conflicting_control_chain_fails_closed() {
    let facts = vec![
        fact(
            "contract",
            at(1),
            at(1),
            FactAuthority::Contract,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                with: OrganizationId::try_new("SEA").unwrap(),
                contract_kind: ContractKind::StandardPlayer,
            }),
        ),
        fact(
            "bad-transfer",
            at(2),
            at(2),
            FactAuthority::LegalControl,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::RightsTransferred {
                from: OrganizationId::try_new("BOS").unwrap(),
                to: OrganizationId::try_new("NYR").unwrap(),
            }),
        ),
    ];
    let state = resolve_player_current_state(PlayerId(42), &facts, cutoffs(10, 10));

    assert_eq!(state.rights.status, RightsStatus::Conflicted);
    assert!(state.disclosures.iter().any(
        |row| row.code == icelines_core::source_facts::SourceDisclosureCode::ConflictingControl
    ));
}

#[test]
fn effective_and_knowledge_cutoffs_filter_different_dimensions() {
    let facts = vec![
        fact(
            "known-contract",
            at(1),
            at(1),
            FactAuthority::Contract,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
                with: OrganizationId::try_new("SEA").unwrap(),
                contract_kind: ContractKind::EntryLevel,
            }),
        ),
        fact(
            "known-late",
            at(9),
            at(2),
            FactAuthority::LegalControl,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Released {
                by: OrganizationId::try_new("SEA").unwrap(),
            }),
        ),
        fact(
            "learned-late",
            at(2),
            at(9),
            FactAuthority::LegalControl,
            SourceFact::PlayerOrganization(PlayerOrganizationEvent::Released {
                by: OrganizationId::try_new("SEA").unwrap(),
            }),
        ),
    ];
    let state = resolve_player_current_state(PlayerId(42), &facts, cutoffs(5, 5));

    assert_eq!(state.rights.status, RightsStatus::Supported);
    assert_eq!(
        state.input_fact_ids,
        vec![FactId::try_new("known-contract").unwrap()]
    );
    assert!(state.disclosures.iter().any(|row| {
        row.code == icelines_core::source_facts::SourceDisclosureCode::HistoricalCutoff
    }));
}

#[test]
fn reconstructed_identity_uses_later_review_without_later_hockey_evidence() {
    let old_evidence = evidence(at(1));
    let proposal_id = ProposalId::try_new("proposal:old-row").unwrap();
    let proposal = ProviderIdentityProposal::new(
        proposal_id.clone(),
        ProviderPersonLocator::SourceRow {
            source_id: old_evidence.source_id().clone(),
            row_key: "row:1".to_owned(),
        },
        "Example Player",
        None,
        None,
        vec![old_evidence.clone()],
    )
    .unwrap();
    let staged = StagedPlayerAssertion::new(
        StagedAssertionId::try_new("staged:old-row").unwrap(),
        "player:proposal:old-row:contract",
        proposal_id.clone(),
        EffectiveTime::new(at(1), None, EffectivePrecision::Day).unwrap(),
        FactAuthority::Contract,
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned {
            with: OrganizationId::try_new("SEA").unwrap(),
            contract_kind: ContractKind::EntryLevel,
        }),
        vec![old_evidence.clone()],
    )
    .unwrap();
    let later_identity_evidence = evidence(at(9));
    let decision = IdentityReviewDecision::new(
        DecisionId::try_new("review:old-row").unwrap(),
        proposal_id,
        IdentityReviewAction::SetIdentity,
        Some(PlayerId(42)),
        "fixture-reviewer",
        at(9),
        "Later evidence resolves only the historical row identity.",
        vec![later_identity_evidence],
    )
    .unwrap();

    let as_known = reconcile_staged_player_assertions(
        std::slice::from_ref(&proposal),
        std::slice::from_ref(&staged),
        std::slice::from_ref(&decision),
        cutoffs(5, 5),
    )
    .unwrap();
    assert!(as_known.assertions.is_empty());

    let reconstructed = reconcile_staged_player_assertions(
        &[proposal],
        &[staged],
        &[decision],
        ReplayCutoffs {
            identity_mode: IdentityReplayMode::ReconstructedIdentity,
            ..cutoffs(5, 5)
        },
    )
    .unwrap();
    assert_eq!(reconstructed.assertions.len(), 1);
    assert_eq!(reconstructed.assertions[0].evidence(), &[old_evidence]);
}
