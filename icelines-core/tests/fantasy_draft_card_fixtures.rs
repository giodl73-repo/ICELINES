use icelines_core::{parse_card_document, CardKind, CardSectionView};

const DRAFT_CARD: &str = include_str!("../../examples/fantasy-draft-card-sample-squad-pick-7.json");

#[test]
fn sample_squad_draft_fixture_is_sealed_and_preserves_the_pick_evidence() {
    let card = parse_card_document(DRAFT_CARD).unwrap();
    assert_eq!(card.card_kind, CardKind::FantasyDraft);
    assert_eq!(card.calculate_fingerprint().unwrap(), card.fingerprint);
    assert_eq!(card.pages.len(), 2);
    assert_eq!(card.context.joins.team_ids, ["sample-multicategory"]);
    assert_eq!(card.context.joins.player_ids.len(), 6);

    let serialized = serde_json::to_string(&card).unwrap();
    for expected in [
        "Draft Sample Player 003",
        "Fallback: Sample Player 004",
        "LW/RW",
        "Priority slots",
        "Schedule diversity",
        "Taken matched",
        "deterministic fixture inputs, not current claims",
    ] {
        assert!(serialized.contains(expected), "missing {expected}");
    }
    assert!(card
        .pages
        .iter()
        .flat_map(|page| &page.sections)
        .any(|section| matches!(section, CardSectionView::Decision(_))));
}
