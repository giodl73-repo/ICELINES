use icelines_core::{parse_card_document, CardKind, CardSectionView};

const TRADE: &str =
    include_str!("../../examples/fantasy-trade-card-dexters-dawgs-fox-rantanen.json");

#[test]
fn dexters_dawgs_trade_fixture_is_sealed_and_renderer_neutral() {
    let card = parse_card_document(TRADE).unwrap();
    assert_eq!(card.card_kind, CardKind::FantasyTrade);
    assert_eq!(card.calculate_fingerprint().unwrap(), card.fingerprint);
    card.validate().unwrap();
    assert_eq!(card.pages.len(), 2);
    assert_eq!(card.pages[0].id, "trade-board");
    assert_eq!(card.pages[1].id, "trade-insider");
    assert_eq!(
        card.context.joins.player_ids,
        ["adam-fox", "mikko-rantanen"]
    );

    let serialized = serde_json::to_string(&card).unwrap();
    for required in [
        "Adam Fox",
        "Mikko Rantanen",
        "Package value gap",
        "Before and after",
        "Roster 16/16 · Legal",
        "deterministic fixture inputs, not current trade advice",
    ] {
        assert!(serialized.contains(required), "missing {required}");
    }
    assert!(card
        .pages
        .iter()
        .flat_map(|page| &page.sections)
        .any(|section| matches!(section, CardSectionView::Decision(_))));
}
