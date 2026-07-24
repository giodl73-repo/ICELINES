use icelines_core::{parse_card_document, CardKind, CardSectionView};

const DEXTERS_DAWGS: &str =
    include_str!("../../examples/fantasy-morning-card-dexters-dawgs-2026-10-08.json");

#[test]
fn dexters_dawgs_morning_fixture_is_sealed_and_renderer_neutral() {
    let card = parse_card_document(DEXTERS_DAWGS).unwrap();
    assert_eq!(card.card_kind, CardKind::FantasyMorning);
    assert_eq!(card.calculate_fingerprint().unwrap(), card.fingerprint);
    card.validate().unwrap();
    assert_eq!(card.pages.len(), 2);
    assert_eq!(card.pages[0].id, "morning-skate");
    assert_eq!(card.pages[1].id, "morning-insider");

    let serialized = serde_json::to_string(&card).unwrap();
    for required in [
        "Darren Raddysh",
        "Igor Shesterkin",
        "Goalie start evidence",
        "Safe proactive adds",
        "Refresh availability for justin-brazeau before lineup lock",
        "deterministic fixture evidence",
    ] {
        assert!(serialized.contains(required), "missing {required}");
    }
    assert!(card
        .pages
        .iter()
        .flat_map(|page| &page.sections)
        .any(|section| matches!(section, CardSectionView::Timeline(_))));
}
