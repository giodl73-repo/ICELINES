use icelines_core::{parse_card_document, CardKind, CardSectionView, WarningKind};

const NYR_CARD: &str = include_str!("../../examples/prospect-arrival-card-alp-2026-27.json");
const SEA_CARD: &str = include_str!("../../examples/prospect-arrival-card-brv-2026-27.json");

#[test]
fn nyr_and_sea_arrival_cards_share_the_sealed_league_artifact() {
    let nyr = parse_card_document(NYR_CARD).unwrap();
    let sea = parse_card_document(SEA_CARD).unwrap();

    assert_eq!(nyr.card_kind, CardKind::ProspectArrival);
    assert_eq!(sea.card_kind, CardKind::ProspectArrival);
    assert_eq!(nyr.pages.len(), 2);
    assert_eq!(sea.pages.len(), 2);
    assert_eq!(
        nyr.context.simulation.parameter_fingerprint,
        sea.context.simulation.parameter_fingerprint
    );
    assert_eq!(nyr.provenance[0].fingerprint, sea.provenance[0].fingerprint);
    assert_ne!(nyr.fingerprint, sea.fingerprint);
    assert_eq!(nyr.warnings[0].kind, WarningKind::MissingSource);
    assert_eq!(sea.warnings[0].kind, WarningKind::MissingSource);
    assert!(nyr.pages[0].sections.iter().any(
        |section| matches!(section, CardSectionView::PlayerList(list) if list.id == "calibrated-arrivals")
    ));
    assert!(sea.pages[1].sections.iter().any(
        |section| matches!(section, CardSectionView::PlayerList(list) if list.id == "arrival-exclusions")
    ));
}
