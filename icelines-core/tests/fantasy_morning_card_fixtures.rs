use icelines_core::{parse_card_document, CardKind, CardSectionView};

const SAMPLE_SQUAD: &str =
    include_str!("../../examples/fantasy-morning-card-sample-squad-2026-10-08.json");

#[test]
fn sample_squad_morning_fixture_is_sealed_and_renderer_neutral() {
    let card = parse_card_document(SAMPLE_SQUAD).unwrap();
    assert_eq!(card.card_kind, CardKind::FantasyMorning);
    assert_eq!(card.calculate_fingerprint().unwrap(), card.fingerprint);
    card.validate().unwrap();
    assert_eq!(card.pages.len(), 2);
    assert_eq!(card.pages[0].id, "morning-skate");
    assert_eq!(card.pages[1].id, "morning-insider");

    let serialized = serde_json::to_string(&card).unwrap();
    for required in [
        "Sample Player 029",
        "Sample Player 006",
        "Goalie start evidence",
        "Safe proactive adds",
        "Refresh availability for sample-player-024 before lineup lock",
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
