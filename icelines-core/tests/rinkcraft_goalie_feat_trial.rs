use icelines_core::view_model::{
    rinkcraft_achievements::{self as feats, Input, PlayerInput},
    rinkcraft_goalie_feat_trial as trial,
};
use std::collections::BTreeMap;
fn input() -> Input {
    Input {
        season: 20262027,
        source_sha256: "a".repeat(64),
        source_urls: vec![],
        players: (1..=40)
            .map(|id| PlayerInput {
                player_id: id,
                name: format!("Example {id}"),
                group: "Goalies".into(),
                games: 20,
                counts: BTreeMap::from([
                    ("starts".into(), 20.),
                    ("shutouts".into(), 0.),
                    ("shots_against".into(), 100. + 10. * id as f64),
                    ("saves".into(), 100. + 8. * id as f64),
                ]),
            })
            .collect(),
    }
}
#[test]
fn separate_candidates_recognize_distinct_patterns_without_changing_active_feats() {
    let input = input();
    let before = serde_json::to_string(&feats::default_rules()).unwrap();
    let r = trial::evaluate(&input).unwrap();
    assert!(r.trial_recognized > r.baseline_recognized);
    assert!(r.baseline_feats_unchanged);
    assert_eq!(
        before,
        serde_json::to_string(&feats::default_rules()).unwrap()
    );
    assert_eq!(feats::default_rules().len(), 34);
    assert!(r.profiles.iter().any(|p| p
        .feats
        .iter()
        .any(|f| f.id == "goalie-save-percentage" && f.state == "earned")
        && p.feats
            .iter()
            .any(|f| f.id == "high-save-workload" && f.state != "earned")));
    assert!(r
        .candidate_counts
        .iter()
        .all(|c| c.earned == c.bronze + c.silver + c.gold));
}
#[test]
fn eligibility_missingness_and_group_scope_remain_separate() {
    let mut i = input();
    i.players[0].games = 9;
    i.players[0].counts.insert("starts".into(), 9.);
    i.players[1].counts.remove("saves");
    let r = trial::evaluate(&i).unwrap();
    assert!(r.profiles[0].feats.iter().all(|f| f.state == "ineligible"));
    assert_eq!(
        r.profiles[1]
            .feats
            .iter()
            .find(|f| f.id == "goalie-save-percentage")
            .unwrap()
            .state,
        "unavailable"
    );
    i.players[0].group = "Forwards".into();
    assert!(trial::evaluate(&i).is_err());
}
#[test]
fn additional_goal_accounting_does_not_rewrite_reported_save_counts() {
    let mut i = input();
    i.players[0].counts.insert("goals_against".into(), 3.);
    let r = trial::evaluate(&i).unwrap();
    assert_eq!(r.accounting_differences.len(), 1);
    let c = &r.profiles[0]
        .feats
        .iter()
        .find(|f| f.id == "goalie-save-percentage")
        .unwrap()
        .conditions[0];
    assert_eq!(c.numerator, Some(108.));
    assert_eq!(c.denominator, Some(110.));
    assert!(r.profiles[0]
        .notes
        .iter()
        .any(|n| n.contains("Additional NHL accounting")));
}
