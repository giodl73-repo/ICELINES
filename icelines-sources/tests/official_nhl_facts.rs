use chrono::{TimeZone, Utc};
use icelines_core::source_facts::{
    FactAuthority, ParticipationAuthority, ParticipationKind, PlayerOrganizationEvent, SourceFact,
    SourceId,
};
use icelines_sources::nhl::club_publication::{
    NhlArticleAcquiredTableCampAdapter, NhlArticleNamedSectionsCampAdapter,
    NhlArticlePtoCampListAdapter, PublishedPositionGroup,
};
use icelines_sources::nhl::contract_publication::NhlArticleContractSigningAdapter;
use icelines_sources::nhl::player_landing::OfficialNhlDraftAdapter;
use icelines_sources::nhl::roster::OfficialNhlRosterAdapter;
use icelines_sources::{ContentHash, SourceAdapter, SourceInput};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

fn input<'a>(source_id: &str, bytes: &'a [u8]) -> SourceInput<'a> {
    SourceInput::new(
        bytes,
        SourceId::try_new(source_id).unwrap(),
        ContentHash::try_new(format!("{:x}", Sha256::digest(bytes))).unwrap(),
    )
}

#[test]
fn official_landing_emits_draft_history_without_claiming_current_rights() {
    let bytes = include_bytes!("../../icelines-fetch/tests/fixtures/landing/bedard_8484144.json");
    let adapter = OfficialNhlDraftAdapter::new(
        8_484_144,
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap();
    let assertion = adapter
        .parse(input("nhl-player-landing:8484144:2026-07-31", bytes))
        .unwrap()
        .expect("Bedard has official draft details");
    assert_eq!(
        assertion.subject(),
        &icelines_core::source_facts::FactSubject::Player(icelines_core::identity::PlayerId(
            8_484_144
        ))
    );
    assert!(matches!(
        assertion.fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::Drafted {
            by,
            year: 2023,
            round: 1,
            overall: 1,
        }) if by.as_str() == "CHI"
    ));
    let json = serde_json::to_value(&assertion).unwrap();
    assert_eq!(json["authority"], "draft");
    assert_ne!(json["authority"], "legal_control");
}

#[test]
fn official_landing_treats_absent_draft_details_as_authoritative_empty() {
    let bytes = br#"{"playerId":8489999,"firstName":{"default":"Undrafted"}}"#;
    let adapter = OfficialNhlDraftAdapter::new(
        8_489_999,
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap();
    assert!(adapter
        .parse(input("nhl-player-landing:8489999:2026-07-31", bytes))
        .unwrap()
        .is_none());
}

#[test]
fn official_roster_emits_assignments_for_forwards_defense_and_goalies() {
    let bytes = include_bytes!("../../tests/fixtures/api/roster_SEA.json");
    let observed_at = Utc
        .with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
        .single()
        .unwrap();
    let adapter = OfficialNhlRosterAdapter::new("sea", observed_at).unwrap();
    let assertions = adapter
        .parse(input("nhl-roster:SEA:2026-07-31", bytes))
        .unwrap();
    assert_eq!(assertions.len(), 10);
    assert!(assertions.iter().all(|assertion| {
        let json = serde_json::to_value(assertion).unwrap();
        json["authority"] == "assignment"
            && matches!(
                assertion.fact(),
                SourceFact::PlayerOrganization(PlayerOrganizationEvent::Assigned { by, to })
                    if by.as_str() == "SEA" && to.as_str() == "NHL:SEA"
            )
    }));
    assert!(assertions.iter().any(|assertion| {
        assertion.subject()
            == &icelines_core::source_facts::FactSubject::Player(icelines_core::identity::PlayerId(
                8_480_020,
            ))
    }));
}

#[test]
fn official_roster_rejects_cross_group_duplicates() {
    let bytes = br#"{
        "forwards":[{"id":1,"positionCode":"C"}],
        "defensemen":[{"id":1,"positionCode":"D"}],
        "goalies":[]
    }"#;
    let adapter = OfficialNhlRosterAdapter::new(
        "NYR",
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap();
    let error = adapter
        .parse(input("nhl-roster:NYR:2026-07-31", bytes))
        .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::SemanticValidation
    );
}

#[test]
fn roster_authority_is_assignment_not_contract_or_legal_control() {
    let bytes = br#"{
        "forwards":[],
        "defensemen":[],
        "goalies":[{"id":2,"positionCode":"G"}]
    }"#;
    let adapter = OfficialNhlRosterAdapter::new(
        "SEA",
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
    )
    .unwrap();
    let assertions = adapter
        .parse(input("nhl-roster:SEA:2026-07-31", bytes))
        .unwrap();
    let json = serde_json::to_value(&assertions[0]).unwrap();
    assert_eq!(json["authority"], serde_json::json!("assignment"));
    assert_ne!(json["authority"], serde_json::json!("contract"));
    assert_ne!(json["authority"], serde_json::json!("legal_control"));
    let _ = FactAuthority::Assignment;
}

#[test]
fn official_club_article_creates_identity_proposals_before_participation_facts() {
    let bytes = include_bytes!("fixtures/nhl_club_article_named_sections_v1.html");
    let adapter = NhlArticleNamedSectionsCampAdapter::new(
        "VGK",
        icelines_core::model::Season(20_262_027),
        ParticipationKind::DevelopmentCamp,
        Utc.with_ymd_and_hms(2026, 6, 28, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/goldenknights/news/topic/transactions/golden-knights-announce-roster-schedule-for-2026-development-camp",
    )
    .unwrap();
    let output = adapter
        .parse(input("nhl-club-article:VGK:2026-development-camp", bytes))
        .unwrap();
    assert_eq!(output.identity_proposals.len(), 30);
    assert_eq!(output.participants.len(), 30);
    assert_eq!(output.staged_assertions.len(), 30);
    assert_eq!(
        output
            .participants
            .iter()
            .filter(|participant| participant.position_group == PublishedPositionGroup::Goalie)
            .count(),
        4
    );
    assert!(output
        .participants
        .iter()
        .all(|participant| participant.authority == ParticipationAuthority::Unknown));
    assert!(output.identity_proposals.iter().all(|proposal| {
        let value = serde_json::to_value(proposal).unwrap();
        value["proposed_player_id"].is_null() && value["locator"]["locator"] == "source_row"
    }));
}

#[test]
fn official_club_article_fails_closed_when_declared_count_does_not_match() {
    let bytes = br#"<script type="application/ld+json">{"articleBody":"**Forwards (2):** One Player\n**Defensemen (0):** \n**Goaltenders (0):** "}</script>"#;
    let adapter = NhlArticleNamedSectionsCampAdapter::new(
        "SEA",
        icelines_core::model::Season(20_262_027),
        ParticipationKind::DevelopmentCamp,
        Utc.with_ymd_and_hms(2026, 6, 28, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/kraken/news/development-camp",
    )
    .unwrap();
    let error = adapter
        .parse(input("nhl-club-article:SEA:bad-count", bytes))
        .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::MalformedRecord
    );
}

#[test]
fn acquired_table_preserves_invite_contract_and_draft_authority() {
    let bytes = include_bytes!("fixtures/nhl_club_article_acquired_table_v1.html");
    let adapter = NhlArticleAcquiredTableCampAdapter::new(
        "CBJ",
        icelines_core::model::Season(20_262_027),
        ParticipationKind::DevelopmentCamp,
        Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).single().unwrap(),
        "https://www.nhl.com/bluejackets/news/blue-jackets-prospects-development-camp-roster-2026",
        "Blue Jackets 2026 Development Camp roster",
    )
    .unwrap();
    let output = adapter
        .parse(input("nhl-club-article:CBJ:2026-development-camp", bytes))
        .unwrap();

    assert_eq!(output.participants.len(), 4);
    assert_eq!(output.staged_assertions.len(), 4);
    assert_eq!(
        output.participants[0].authority,
        ParticipationAuthority::Unknown,
        "draft history alone must not imply current rights"
    );
    assert_eq!(
        output.participants[1].authority,
        ParticipationAuthority::FreeAgentInvite
    );
    assert_eq!(
        output.participants[2].authority,
        ParticipationAuthority::ControlledPlayer
    );
    assert_eq!(
        output.participants[3].authority,
        ParticipationAuthority::FreeAgentInvite
    );
    assert_eq!(
        output.participants[3].position_group,
        PublishedPositionGroup::Goalie
    );
    assert_eq!(
        output.identity_proposals[0].displayed_name(),
        "Tommi Männistö"
    );
    assert!(matches!(
        output.staged_assertions[1].fact(),
        SourceFact::PlayerParticipation(fact)
            if fact.authority == ParticipationAuthority::FreeAgentInvite
    ));
    assert!(matches!(
        output.staged_assertions[2].fact(),
        SourceFact::PlayerParticipation(fact)
            if fact.authority == ParticipationAuthority::ControlledPlayer
    ));
}

#[test]
fn acquired_table_fails_closed_on_an_unreviewed_acquisition_label() {
    let bytes = br#"<h2>Camp roster</h2><table><tr><td>Pos</td><td>No</td><td>Name</td><td>Team</td><td>Acquired</td></tr><tr><td>F</td><td>1</td><td>Player One</td><td>Club</td><td>Future considerations</td></tr></table>"#;
    let adapter = NhlArticleAcquiredTableCampAdapter::new(
        "SEA",
        icelines_core::model::Season(20_262_027),
        ParticipationKind::DevelopmentCamp,
        Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).single().unwrap(),
        "https://www.nhl.com/kraken/news/camp-roster",
        "Camp roster",
    )
    .unwrap();
    let error = adapter
        .parse(input("nhl-club-article:SEA:bad-acquired", bytes))
        .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::MalformedRecord
    );
}

#[test]
fn official_pto_list_emits_tryouts_including_a_goalie() {
    let bytes = include_bytes!("fixtures/nhl_pto_camp_list_v1.html");
    let organizations = [
        ("Pittsburgh Penguins", "PIT"),
        ("Chicago Blackhawks", "CHI"),
        ("Toronto Maple Leafs", "TOR"),
    ]
    .into_iter()
    .map(|(name, id)| {
        (
            name.to_owned(),
            icelines_core::source_facts::OrganizationId::try_new(id).unwrap(),
        )
    })
    .collect::<BTreeMap<_, _>>();
    let output = NhlArticlePtoCampListAdapter::new(
        icelines_core::model::Season(20_252_026),
        Utc.with_ymd_and_hms(2025, 10, 7, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/topic/offseason/players-signed-to-ptos-for-2025-2026-nhl-training-camps",
        organizations,
    )
    .unwrap()
    .parse(input("nhl-pto-camp-list:2025-26", bytes))
    .unwrap();

    assert_eq!(output.participants.len(), 3);
    assert_eq!(output.staged_assertions.len(), 3);
    assert!(output
        .participants
        .iter()
        .all(|participant| participant.authority == ParticipationAuthority::Tryout));
    assert_eq!(
        output.participants[2].position_group,
        PublishedPositionGroup::Goalie
    );
    assert!(matches!(
        output.staged_assertions[2].fact(),
        SourceFact::PlayerParticipation(fact)
            if fact.authority == ParticipationAuthority::Tryout
                && fact.organization.as_str() == "TOR"
    ));
}

#[test]
fn official_contract_article_stages_signing_until_identity_review() {
    let bytes = include_bytes!("fixtures/nhl_contract_article_signs_v1.html");
    let adapter = NhlArticleContractSigningAdapter::new(
        "UTA",
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/logan-cooley-signs-8-year-contract-with-utah-mammoth",
    )
    .unwrap();
    let output = adapter
        .parse(input("nhl-contract-article:cooley:2025-10-29", bytes))
        .unwrap();
    let proposal = serde_json::to_value(&output.identity_proposal).unwrap();
    assert_eq!(proposal["displayed_name"], "Logan Cooley");
    assert!(proposal["proposed_player_id"].is_null());
    assert_eq!(output.signing.organization.as_str(), "UTA");
    assert_eq!(
        output.signing.contract_kind,
        icelines_core::source_facts::ContractKind::Unknown
    );
    assert_eq!(
        output.signing.occurred_at.starts_at.to_rfc3339(),
        "2025-10-29T00:00:00+00:00"
    );
    assert!(matches!(
        output.staged_assertion.fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::ContractSigned { with, .. })
            if with.as_str() == "UTA"
    ));
}

#[test]
fn official_contract_article_rejects_an_unreviewed_headline_layout() {
    let bytes = br#"<script type="application/ld+json">{"headline":"Ducks match a contract offer","datePublished":"2026-07-09"}</script>"#;
    let adapter = NhlArticleContractSigningAdapter::new(
        "ANA",
        Utc.with_ymd_and_hms(2026, 7, 31, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/example",
    )
    .unwrap();
    let error = adapter
        .parse(input("nhl-contract-article:unsupported", bytes))
        .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::UnsupportedLayout
    );
}
