use chrono::{TimeZone, Utc};
use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::source_facts::{
    ContentHash, DecisionId, IdentityReviewAction, IdentityReviewDecision, ParticipationKind,
    PlayerOrganizationEvent, SourceFact, SourceId,
};
use icelines_sources::nhl::club_publication::NhlArticleNamedSectionsCampAdapter;
use icelines_sources::nhl::contract_publication::NhlArticleContractSigningAdapter;
use icelines_sources::nhl::termination_publication::NhlArticleContractTerminationAdapter;
use icelines_sources::nhl::trade_tracker::NhlTradeTrackerAdapter;
use icelines_sources::reconciliation::{
    lower_reviewed_camp_publication, lower_reviewed_contract_publication,
    lower_reviewed_termination_publication, lower_reviewed_trade_tracker,
};
use icelines_sources::{SourceAdapter, SourceInput};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn input<'a>(source_id: &str, bytes: &'a [u8]) -> SourceInput<'a> {
    SourceInput::new(
        bytes,
        SourceId::try_new(source_id).unwrap(),
        ContentHash::try_new(format!("{:x}", Sha256::digest(bytes))).unwrap(),
    )
}

fn decision(
    proposal: &icelines_core::source_facts::ProviderIdentityProposal,
    player_id: u32,
) -> IdentityReviewDecision {
    IdentityReviewDecision::new(
        DecisionId::try_new(format!("review:{}", proposal.proposal_id())).unwrap(),
        proposal.proposal_id().clone(),
        IdentityReviewAction::SetIdentity,
        Some(PlayerId(player_id)),
        "fixture-reviewer",
        Utc.with_ymd_and_hms(2026, 7, 31, 13, 0, 0)
            .single()
            .unwrap(),
        "Frozen fixture confirms the canonical identity.",
        proposal.evidence().to_vec(),
    )
    .unwrap()
}

#[test]
fn camp_rows_enter_canonical_facts_only_after_review() {
    let bytes = include_bytes!("fixtures/nhl_club_article_named_sections_v1.html");
    let publication = NhlArticleNamedSectionsCampAdapter::new(
        "VGK",
        Season(20_262_027),
        ParticipationKind::DevelopmentCamp,
        Utc.with_ymd_and_hms(2026, 6, 28, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/goldenknights/news/example",
    )
    .unwrap()
    .parse(input("vgk-camp", bytes))
    .unwrap();

    let pending = lower_reviewed_camp_publication(&publication, &[]).unwrap();
    assert!(pending.assertions.is_empty());
    assert_eq!(pending.disclosures.len(), 1);

    let reviewed = lower_reviewed_camp_publication(
        &publication,
        &[decision(&publication.identity_proposals[0], 8_489_001)],
    )
    .unwrap();
    assert_eq!(reviewed.assertions.len(), 1);
    assert_eq!(reviewed.disclosures.len(), 1);
    assert!(matches!(
        reviewed.assertions[0].fact(),
        SourceFact::PlayerParticipation(participation)
            if participation.organization.as_str() == "VGK"
    ));
}

#[test]
fn reviewed_contract_row_becomes_contract_authority_not_legal_control() {
    let bytes = include_bytes!("fixtures/nhl_contract_article_signs_v1.html");
    let publication = NhlArticleContractSigningAdapter::new(
        "UTA",
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/logan-cooley-signs-8-year-contract-with-utah-mammoth",
    )
    .unwrap()
    .parse(input("cooley-contract", bytes))
    .unwrap();
    let reviewed = lower_reviewed_contract_publication(
        &publication,
        &[decision(&publication.identity_proposal, 8_483_431)],
    )
    .unwrap();
    assert_eq!(reviewed.assertions.len(), 1);
    assert!(matches!(
        reviewed.assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned { with, .. })
            if with.as_str() == "UTA"
    ));
    let value = serde_json::to_value(&reviewed.assertions[0]).unwrap();
    assert_eq!(value["authority"], "contract");
}

#[test]
fn two_decisions_for_one_proposal_fail_closed() {
    let bytes = include_bytes!("fixtures/nhl_contract_article_signs_v1.html");
    let publication = NhlArticleContractSigningAdapter::new(
        "UTA",
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/example",
    )
    .unwrap()
    .parse(input("duplicate-review", bytes))
    .unwrap();
    let first = decision(&publication.identity_proposal, 8_483_431);
    let second = IdentityReviewDecision::new(
        DecisionId::try_new("review:duplicate").unwrap(),
        publication.identity_proposal.proposal_id().clone(),
        IdentityReviewAction::SetIdentity,
        Some(PlayerId(8_483_432)),
        "second-reviewer",
        Utc.with_ymd_and_hms(2026, 7, 31, 14, 0, 0)
            .single()
            .unwrap(),
        "Conflicting identity decision used to verify fail-closed behavior.",
        publication.identity_proposal.evidence().to_vec(),
    )
    .unwrap();
    assert!(lower_reviewed_contract_publication(&publication, &[first, second]).is_err());
}

#[test]
fn reviewed_beaudoin_trade_leg_becomes_a_rights_transfer() {
    let bytes = include_bytes!("fixtures/nhl_trade_tracker_acquire_v1.html");
    let organizations = [
        ("New York Rangers", "NYR"),
        ("Vancouver Canucks", "VAN"),
        ("Utah Mammoth", "UTA"),
        ("Boston Bruins", "BOS"),
    ]
    .into_iter()
    .map(|(name, id)| {
        (
            name.to_owned(),
            icelines_core::source_facts::OrganizationId::try_new(id).unwrap(),
        )
    })
    .collect::<BTreeMap<_, _>>();
    let publication = NhlTradeTrackerAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 27, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/topic/trade-coverage/2026-27-nhl-trades",
        organizations,
    )
    .unwrap()
    .parse(input("trade-review", bytes))
    .unwrap();
    let beaudoin = publication
        .identity_proposals
        .iter()
        .find(|proposal| proposal.displayed_name() == "Cole Beaudoin")
        .unwrap();
    let lowered =
        lower_reviewed_trade_tracker(&publication, &[decision(beaudoin, 8_484_771)]).unwrap();
    assert_eq!(lowered.assertions.len(), 1);
    assert_eq!(lowered.exclusions.len(), 2);
    assert_eq!(lowered.disclosures.len(), 1);
    assert!(matches!(
        lowered.assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::RightsTransferred { from, to })
            if from.as_str() == "UTA" && to.as_str() == "NYR"
    ));
    let value = serde_json::to_value(&lowered.assertions[0]).unwrap();
    assert_eq!(value["authority"], "legal_control");
}

#[test]
fn only_a_completed_reviewed_termination_becomes_a_release() {
    let bytes = include_bytes!("fixtures/nhl_contract_termination_completed_v1.html");
    let publication = NhlArticleContractTerminationAdapter::new(
        "CAR",
        "Carolina Hurricanes",
        Utc.with_ymd_and_hms(2024, 7, 18, 16, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/evgeny-kuznetsov-placed-on-waivers-by-carolina",
    )
    .unwrap()
    .parse(input("kuznetsov-termination", bytes))
    .unwrap();
    assert_eq!(
        publication.identity_proposal.displayed_name(),
        "Evgeny Kuznetsov"
    );
    assert!(matches!(
        publication.staged_assertion.fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Released { by })
            if by.as_str() == "CAR"
    ));
    let pending = lower_reviewed_termination_publication(&publication, &[]).unwrap();
    assert!(pending.assertions.is_empty());
    let lowered = lower_reviewed_termination_publication(
        &publication,
        &[decision(&publication.identity_proposal, 8_478_425)],
    )
    .unwrap();
    assert!(matches!(
        lowered.assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Released { by })
            if by.as_str() == "CAR"
    ));
}

#[test]
fn intended_future_termination_is_not_parsed_as_completed_release() {
    let bytes = br#"<script type="application/ld+json">{"headline":"Player to have contract terminated","datePublished":"2025-01-29","articleBody":"Example Player will be placed on unconditional waivers with the purpose of terminating his contract."}</script>"#;
    let error = NhlArticleContractTerminationAdapter::new(
        "STL",
        "St. Louis Blues",
        Utc.with_ymd_and_hms(2025, 1, 29, 16, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/example",
    )
    .unwrap()
    .parse(input("future-termination", bytes))
    .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::UnsupportedLayout
    );
}
