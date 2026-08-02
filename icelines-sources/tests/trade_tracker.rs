use chrono::{TimeZone, Utc};
use icelines_core::source_facts::{ContentHash, OrganizationId, SourceId};
use icelines_sources::fragment::SourcePackageFragment;
use icelines_sources::nhl::trade_tracker::NhlTradeTrackerAdapter;
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

fn registry() -> BTreeMap<String, OrganizationId> {
    [
        ("New York Rangers", "NYR"),
        ("Vancouver Canucks", "VAN"),
        ("Utah Mammoth", "UTA"),
        ("Boston Bruins", "BOS"),
    ]
    .into_iter()
    .map(|(name, id)| (name.to_owned(), OrganizationId::try_new(id).unwrap()))
    .collect()
}

#[test]
fn official_trade_tracker_preserves_both_player_sides_and_ignores_picks() {
    let bytes = include_bytes!("fixtures/nhl_trade_tracker_acquire_v1.html");
    let output = NhlTradeTrackerAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 27, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/topic/trade-coverage/2026-27-nhl-trades",
        registry(),
    )
    .unwrap()
    .parse(input("nhl-trade-tracker:2026-27", bytes))
    .unwrap();

    assert_eq!(output.transfers.len(), 6);
    assert_eq!(output.staged_assertions.len(), 6);
    assert_eq!(output.identity_proposals.len(), 6);
    let names = output
        .identity_proposals
        .iter()
        .map(|proposal| proposal.displayed_name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "Marcus Pettersson",
            "Vincent Trocheck",
            "Sean Durzi",
            "Cole Beaudoin",
            "Joonas Korpisalo",
            "Kalle Vaisanen",
        ]
    );
    let beaudoin = output
        .transfers
        .iter()
        .zip(&output.identity_proposals)
        .find(|(_, proposal)| proposal.displayed_name() == "Cole Beaudoin")
        .map(|(transfer, _)| transfer)
        .unwrap();
    assert_eq!(beaudoin.from.as_str(), "UTA");
    assert_eq!(beaudoin.to.as_str(), "NYR");
    assert_eq!(
        beaudoin.occurred_at.starts_at.to_rfc3339(),
        "2026-07-01T00:00:00+00:00"
    );
    assert_eq!(output.ignored_assets.len(), 2);
    assert!(output
        .ignored_assets
        .iter()
        .all(|asset| asset.description.to_ascii_lowercase().contains("pick")));
    let fragment = SourcePackageFragment::from(&output);
    assert_eq!(fragment.identity_proposals.len(), 6);
    assert_eq!(fragment.staged_player_assertions.len(), 6);
    assert_eq!(fragment.exclusions.len(), 2);
    assert!(fragment
        .exclusions
        .iter()
        .all(|row| row.reason_code == "non_player_trade_asset"));
}

#[test]
fn unknown_organization_fails_the_source_instead_of_dropping_a_trade_leg() {
    let bytes = br#"<script type="application/ld+json">{"articleBody":"JULY 1: Seattle Kraken acquire forward Example Player from the Unknown Club for a draft pick."}</script>"#;
    let error = NhlTradeTrackerAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 27, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/trades",
        registry(),
    )
    .unwrap()
    .parse(input("unknown-club", bytes))
    .unwrap_err();
    assert_eq!(
        error.category,
        icelines_sources::AdapterErrorCategory::MalformedRecord
    );
}

#[test]
fn winter_trade_uses_the_second_calendar_year_of_the_season() {
    let bytes = br#"<script type="application/ld+json">{"articleBody":"MARCH 5: New York Rangers acquire forward Example Player from the Boston Bruins for a draft pick."}</script>"#;
    let output = NhlTradeTrackerAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2027, 3, 5, 20, 0, 0).single().unwrap(),
        "https://www.nhl.com/news/trades",
        registry(),
    )
    .unwrap()
    .parse(input("deadline-trade", bytes))
    .unwrap();
    assert_eq!(
        output.transfers[0].occurred_at.starts_at.to_rfc3339(),
        "2027-03-05T00:00:00+00:00"
    );
}

#[test]
fn concatenated_markdown_rows_from_live_article_are_split() {
    let bytes = br#"<script type="application/ld+json">{"articleBody":"*Intro copy.***JULY 2:** New York Rangers acquire forward First Player from the Boston Bruins for a draft pick. | **[Story](https://example.test/one)****JULY 1:** Boston Bruins acquire defenseman Second Player from the New York Rangers for a draft pick. | **[Story](https://example.test/two)**"}</script>"#;
    let output = NhlTradeTrackerAdapter::new(
        2026,
        Utc.with_ymd_and_hms(2026, 7, 27, 12, 0, 0)
            .single()
            .unwrap(),
        "https://www.nhl.com/news/trades",
        registry(),
    )
    .unwrap()
    .parse(input("concatenated-markdown", bytes))
    .unwrap();

    assert_eq!(output.transfers.len(), 2);
    assert_eq!(
        output.identity_proposals[0].displayed_name(),
        "First Player"
    );
    assert_eq!(
        output.identity_proposals[1].displayed_name(),
        "Second Player"
    );
}
