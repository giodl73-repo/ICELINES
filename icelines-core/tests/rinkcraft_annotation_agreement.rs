use icelines_core::view_model::rinkcraft_annotation_agreement::{
    compare, Annotation, Key, Outcome as O,
};
fn row(id: &str, outcome: Option<O>) -> Annotation {
    Annotation {
        key: Key {
            source_lineage_id: "synthetic-v1".into(),
            game_id: "synthetic-game".into(),
            period: 1,
            actor_id: 1,
            component_id: "TEST-C".into(),
            source_event_id: id.into(),
        },
        outcome,
    }
}
#[test]
fn perfect_matched_labels_do_not_hide_disagreeing_opportunity_sets() {
    let a = [
        row("s1", Some(O::Success)),
        row("s2", Some(O::Censored)),
        row("a3", Some(O::Success)),
        row("a4", None),
    ];
    let b = [
        row("s1", Some(O::Success)),
        row("s2", Some(O::Censored)),
        row("b3", Some(O::Untested)),
        row("b4", None),
    ];
    let r = compare(&a, &b).unwrap();
    assert_eq!(r.matched, 2);
    assert_eq!(r.only_first.len(), 2);
    assert_eq!(r.only_second.len(), 2);
    assert_eq!(r.detection_overlap, Some(0.5));
    assert_eq!(r.outcome_agreement, Some(1.));
    assert_eq!(r.confusion_matrix[7][7], 1);
}
#[test]
fn empty_and_unlabeled_denominators_are_not_perfect_agreement() {
    let r = compare(&[], &[]).unwrap();
    assert_eq!(r.detection_overlap, None);
    assert_eq!(r.outcome_agreement, None);
    let r = compare(&[row("s", None)], &[row("s", Some(O::Success))]).unwrap();
    assert_eq!(r.matched_without_two_labels, 1);
    assert_eq!(r.matched_both_labeled, 0);
    assert_eq!(r.outcome_agreement, None);
}
#[test]
fn duplicate_and_invalid_keys_fail_without_reconciliation() {
    let a = row("s", None);
    assert!(compare(&[a.clone(), a], &[]).is_err());
    let mut a = row("s", None);
    a.key.actor_id = 0;
    assert!(compare(&[a], &[]).is_err());
}
#[test]
fn source_versions_and_components_never_force_matches() {
    let a = row("s", Some(O::Success));
    let mut b = a.clone();
    b.key.source_lineage_id = "synthetic-v2".into();
    assert_eq!(compare(&[a.clone()], &[b]).unwrap().matched, 0);
    let mut b = a.clone();
    b.key.component_id = "OTHER".into();
    assert_eq!(compare(&[a], &[b]).unwrap().matched, 0);
}
#[test]
fn disagreement_is_a_matrix_cell_not_a_merged_failure_label() {
    let r = compare(
        &[row("s", Some(O::TechnicalFailure))],
        &[row("s", Some(O::ChoiceFailure))],
    )
    .unwrap();
    assert_eq!(r.detection_overlap, Some(1.));
    assert_eq!(r.outcome_agreement, Some(0.));
    assert_eq!(r.confusion_matrix[1][2], 1);
    assert_eq!(r.confusion_matrix.iter().flatten().sum::<usize>(), 1);
}
