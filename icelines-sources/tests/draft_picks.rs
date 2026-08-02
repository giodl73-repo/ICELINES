use chrono::{TimeZone, Utc};
use icelines_core::source_facts::{
    ContentHash, DecisionId, IdentityReviewAction, IdentityReviewDecision, PlayerOrganizationEvent,
    SourceFact, SourceId,
};
use icelines_sources::nhl::draft_picks::OfficialNhlDraftPicksAdapter;
use icelines_sources::reconciliation::lower_reviewed_draft_picks;
use icelines_sources::{SourceAdapter, SourceInput};
use sha2::{Digest, Sha256};

fn input<'a>(source_id: &str, bytes: &'a [u8]) -> SourceInput<'a> {
    SourceInput::new(
        bytes,
        SourceId::try_new(source_id).unwrap(),
        ContentHash::try_new(format!("{:x}", Sha256::digest(bytes))).unwrap(),
    )
}

#[test]
fn multi_year_ledger_stages_skaters_and_goalies_for_identity_review() {
    let bytes = include_bytes!("fixtures/nhl_draft_picks_all_v1.json");
    let ledger = OfficialNhlDraftPicksAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap()
    .parse(input("nhl-draft-picks:2026", bytes))
    .unwrap();
    assert_eq!(ledger.selections.len(), 3);
    assert_eq!(ledger.staged_assertions.len(), 3);
    assert_eq!(
        ledger.identity_proposals[0].displayed_name(),
        "Alberts Smits"
    );
    assert!(ledger
        .selections
        .iter()
        .any(|selection| selection.position_code == "G"));
    assert!(ledger
        .identity_proposals
        .iter()
        .all(|proposal| proposal.proposed_player_id().is_none()));
    assert!(matches!(
        ledger.staged_assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
            by,
            overall: 5,
            ..
        }) if by.as_str() == "NYR"
    ));
}

#[test]
fn reviewed_smits_selection_becomes_historical_draft_fact_only() {
    let bytes = include_bytes!("fixtures/nhl_draft_picks_all_v1.json");
    let ledger = OfficialNhlDraftPicksAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap()
    .parse(input("nhl-draft-picks:2026", bytes))
    .unwrap();
    let proposal = &ledger.identity_proposals[0];
    let decision = IdentityReviewDecision::new(
        DecisionId::try_new("review:alberts-smits-2026").unwrap(),
        proposal.proposal_id().clone(),
        IdentityReviewAction::SetIdentity,
        Some(icelines_core::identity::PlayerId(8_489_005)),
        "fixture-reviewer",
        Utc.with_ymd_and_hms(2026, 7, 31, 13, 0, 0)
            .single()
            .unwrap(),
        "Official identity evidence resolves the draft ledger row.",
        proposal.evidence().to_vec(),
    )
    .unwrap();
    let lowered = lower_reviewed_draft_picks(&ledger, &[decision]).unwrap();
    assert_eq!(lowered.assertions.len(), 1);
    assert_eq!(lowered.disclosures.len(), 1);
    assert!(matches!(
        lowered.assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
            by,
            year: 2026,
            round: 1,
            overall: 5,
        }) if by.as_str() == "NYR"
    ));
    let value = serde_json::to_value(&lowered.assertions[0]).unwrap();
    assert_eq!(value["authority"], "draft");
}

#[test]
fn live_draft_state_cannot_claim_a_terminal_all_picks_ledger() {
    let bytes = include_bytes!("fixtures/nhl_draft_picks_all_v1.json");
    let live = String::from_utf8(bytes.to_vec()).unwrap().replacen(
        "\"state\": \"over\"",
        "\"state\": \"live\"",
        1,
    );
    let error = OfficialNhlDraftPicksAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 6, 26, 23, 30, 0)
            .single()
            .unwrap(),
    )
    .unwrap()
    .parse(input("nhl-draft-picks:2026:live", live.as_bytes()))
    .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::SemanticValidation
    );
}

#[test]
fn forfeited_slot_is_recorded_without_becoming_a_player() {
    let bytes = br#"{"broadcastStartTimeUTC":"2026-06-26T23:00:00Z","draftYear":2026,"state":"over","picks":[{"round":2,"pickInRound":31,"overallPick":63,"teamAbbrev":"VGK","lastName":{"default":"Forfeited"}}]}"#;
    let ledger = OfficialNhlDraftPicksAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap()
    .parse(input("nhl-draft-picks:2026", bytes))
    .unwrap();

    assert!(ledger.identity_proposals.is_empty());
    assert!(ledger.selections.is_empty());
    assert_eq!(ledger.forfeited_slots.len(), 1);
    assert_eq!(ledger.forfeited_slots[0].organization.as_str(), "VGK");
    assert_eq!(ledger.forfeited_slots[0].overall, 63);
}
