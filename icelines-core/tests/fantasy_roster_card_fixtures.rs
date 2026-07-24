use icelines_core::{parse_card_document, CardKind, CardSectionView};

const DEXTERS_DAWGS: &str =
    include_str!("../../examples/fantasy-roster-card-dexters-dawgs-2026-10-05.json");

#[test]
fn dexters_dawgs_fixture_is_sealed_complete_and_renderer_neutral() {
    let card = parse_card_document(DEXTERS_DAWGS).unwrap();
    assert_eq!(card.card_kind, CardKind::FantasyRoster);
    assert_eq!(card.calculate_fingerprint().unwrap(), card.fingerprint);
    card.validate().unwrap();
    assert_eq!(card.pages.len(), 2);
    assert_eq!(card.context.joins.team_ids, ["dexters-dawgs"]);
    assert_eq!(card.context.joins.player_ids.len(), 20);

    let serialized = serde_json::to_string(&card).unwrap();
    for required in [
        "Nathan MacKinnon",
        "BN4 · BUF · G",
        "IR+2",
        "Pickups remaining",
        "Same day",
        "Best calendar complement: WSH (Class 8)",
        "Class 1: BOS, COL, DET, NYR",
        "deterministic examples, not current claims",
    ] {
        assert!(serialized.contains(required), "missing {required}");
    }

    assert!(card
        .pages
        .iter()
        .flat_map(|page| &page.sections)
        .any(|section| matches!(section, CardSectionView::Decision(_))));
}
