use chrono::{TimeZone, Utc};
use icelines_core::source_facts::{ContentHash, PlayerOrganizationEvent, SourceFact, SourceId};
use icelines_sources::ahl::roster_stats::AhlRosterStatsV1Adapter;
use icelines_sources::compat::ahl_identity_review_v1::AhlIdentityReviewV1Adapter;
use icelines_sources::reconciliation::lower_reviewed_ahl_roster;
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
fn reviewed_ahl_roster_lowers_to_club_presence_without_nhl_control() {
    let roster_bytes = include_bytes!("fixtures/ahl_roster_stats_v1.json");
    let roster = AhlRosterStatsV1Adapter
        .parse(input("ahl-roster:2026-27", roster_bytes))
        .unwrap();
    assert_eq!(roster.identity_proposals.len(), 2);
    assert_eq!(roster.roster_observations.len(), 2);
    assert_eq!(roster.staged_assertions.len(), 2);
    assert!(matches!(
        roster.staged_assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::AffiliateRostered { affiliate, at })
            if affiliate.as_str() == "NYR" && at.as_str() == "AHL:HFD"
    ));
    assert_eq!(
        roster.roster_observations[0]
            .nhl_affiliate
            .as_ref()
            .unwrap()
            .as_str(),
        "NYR"
    );

    let review_bytes = include_bytes!("fixtures/ahl_identity_review_decisions_v1.json");
    let decisions = AhlIdentityReviewV1Adapter::new(
        20_262_027,
        "HFD",
        Utc.with_ymd_and_hms(2026, 7, 31, 13, 5, 0)
            .single()
            .unwrap(),
        "https://icelines.local/reviews/ahl/HFD/20262027",
        &roster,
    )
    .unwrap()
    .parse(input("ahl-review:HFD:2026-27", review_bytes))
    .unwrap();
    assert_eq!(decisions.len(), 2);

    let lowered = lower_reviewed_ahl_roster(&roster, &decisions).unwrap();
    assert_eq!(lowered.assertions.len(), 1);
    assert_eq!(lowered.exclusions.len(), 1);
    assert!(matches!(
        lowered.assertions[0].fact(),
        SourceFact::PlayerOrganization(PlayerOrganizationEvent::AffiliateRostered { affiliate, at })
            if affiliate.as_str() == "NYR" && at.as_str() == "AHL:HFD"
    ));
    let fact = serde_json::to_string(&lowered.assertions[0]).unwrap();
    assert!(fact.contains("\"authority\":\"assignment\""));
    assert!(fact.contains("NYR"));
}

#[test]
fn draft_ahl_review_cannot_be_applied_as_authority() {
    let roster_bytes = include_bytes!("fixtures/ahl_roster_stats_v1.json");
    let roster = AhlRosterStatsV1Adapter
        .parse(input("ahl-roster:2026-27", roster_bytes))
        .unwrap();
    let review_bytes = include_bytes!("fixtures/ahl_identity_review_decisions_v1.json");
    let draft = String::from_utf8(review_bytes.to_vec()).unwrap().replacen(
        "\"draft\": false",
        "\"draft\": true",
        1,
    );
    let error = AhlIdentityReviewV1Adapter::new(
        20_262_027,
        "HFD",
        Utc.with_ymd_and_hms(2026, 7, 31, 13, 5, 0)
            .single()
            .unwrap(),
        "https://icelines.local/reviews/ahl/HFD/20262027",
        &roster,
    )
    .unwrap()
    .parse(input("ahl-review-draft:HFD", draft.as_bytes()))
    .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::SemanticValidation
    );
}
