use icelines_core::view_model::{
    rinkcraft_achievements::{AchievementProfile, Condition, Feat},
    rinkcraft_task_repertoire::select,
};
use std::collections::BTreeMap;
fn feat(id: &str, keys: &[&str], tier: &str) -> Feat {
    Feat {
        id: id.into(),
        name: id.into(),
        state: "earned".into(),
        placements: vec!["task_skills".into()],
        note: None,
        tier: Some(tier.into()),
        matched: keys.len(),
        required: keys.len(),
        conditions: keys
            .iter()
            .map(|k| Condition {
                raw_thresholds: BTreeMap::new(),
                key: (*k).into(),
                label: (*k).into(),
                value: Some(1.),
                unit: "PerGame".into(),
                numerator: Some(20.),
                denominator: Some(20.),
                percentile: Some(95.),
                peers: 100,
                reference_sha256: "synthetic".into(),
                qualifies: Some(true),
            })
            .collect(),
    }
}
fn profile() -> AchievementProfile {
    AchievementProfile {
        task_repertoire: None,
        player_id: 1,
        season: 20262027,
        season_type: "regular".into(),
        group: "Forwards".into(),
        games: 20,
        minimum_games: 10,
        rules_sha256: "rules".into(),
        source_sha256: "input".into(),
        source_urls: vec![],
        earned: 6,
        notes: vec![],
        feats: vec![
            feat("shooting-scorer", &["shots", "goals"], "Gold"),
            feat("dual-creator", &["shots", "assists"], "Gold"),
            feat("goal-output", &["goals"], "Gold"),
            feat("assist-output", &["assists"], "Gold"),
            feat("penalty-magnet", &["drawn"], "Silver"),
            feat("pp-helper", &["pp-assists"], "Bronze"),
        ],
    }
}
#[test]
fn a_gold_specialty_is_not_buried_by_a_weaker_compound_task() {
    let mut p = profile();
    p.feats = vec![
        feat("disrupting-scorer", &["takeaways", "points"], "Bronze"),
        feat("puck-thief", &["takeaways"], "Gold"),
        feat("goal-output", &["goals"], "Silver"),
    ];
    let r = select(&p);
    assert_eq!(r.schema, "rinkcraft.task-repertoire.v2");
    assert_eq!(
        r.skills
            .iter()
            .map(|s| s.feat_id.as_str())
            .collect::<Vec<_>>(),
        vec!["puck-thief", "goal-output"]
    );
    assert_eq!(r.highlights[0].name, "Takeaways");
    assert_eq!(r.highlights[0].tier, "Gold");
}
#[test]
fn identity_highlights_include_role_feats_without_creating_tasks_or_padding() {
    let mut p = profile();
    let mut pp = feat("pp-striker", &["pp-goals"], "Gold");
    pp.placements = vec!["role_coverage".into()];
    let mut flag = feat("penalty-load", &["taken"], "Gold");
    flag.placements = vec!["risk_flags".into()];
    p.feats = vec![pp, flag];
    let mut sh = feat("sh-points", &["sh-points"], "Gold");
    sh.placements = vec!["role_coverage".into()];
    p.feats.push(sh);
    let before = p.clone();
    let r = select(&p);
    assert!(r.skills.is_empty());
    assert_eq!(r.highlights.len(), 1);
    assert_eq!(r.highlights[0].name, "PP goals");
    assert_eq!(p, before);
    p.feats[0].state = "not_earned".into();
    assert!(select(&p).highlights.is_empty());
}
#[test]
fn identity_does_not_repeat_metric_keys_and_legacy_repertoires_remain_readable() {
    let mut p = profile();
    p.feats = vec![
        feat("shot-shield", &["hits", "blocks"], "Silver"),
        feat("shot-blocker", &["blocks"], "Silver"),
    ];
    let r = select(&p);
    assert_eq!(r.highlights.len(), 1);
    let mut old = serde_json::to_value(r).unwrap();
    old.as_object_mut().unwrap().remove("highlights");
    old["schema"] = "rinkcraft.task-repertoire.v1".into();
    let old: icelines_core::view_model::rinkcraft_task_repertoire::Repertoire =
        serde_json::from_value(old).unwrap();
    assert!(old.highlights.is_empty());
}
#[test]
fn selects_complementary_tasks_without_changing_qualifications() {
    let p = profile();
    let before = p.clone();
    let r = select(&p);
    assert_eq!(before, p);
    let ids = r
        .skills
        .iter()
        .map(|s| s.feat_id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        vec![
            "dual-creator",
            "shooting-scorer",
            "penalty-magnet",
            "pp-helper"
        ]
    );
    assert_eq!(r.skills[0].name, "Shoot & assist");
    assert_eq!(r.season, p.season);
    assert_eq!(r.games, p.games);
    let mut reversed = p.clone();
    reversed.feats.reverse();
    assert_eq!(r, select(&reversed));
}
#[test]
fn excludes_locked_cautions_and_unknown_tasks_and_never_pads() {
    let mut p = profile();
    for f in &mut p.feats {
        f.state = "not_earned".into();
    }
    p.feats.push(feat("future-task", &["hits"], "Gold"));
    let mut caution = feat("hit-parade", &["hits"], "Gold");
    caution.placements = vec!["risk_flags".into()];
    p.feats.push(caution);
    assert!(select(&p).skills.is_empty());
    p.feats.push(feat("penalty-magnet", &["drawn"], "Bronze"));
    assert_eq!(select(&p).skills.len(), 1);
}
#[test]
fn caps_at_five_and_preserves_goalie_and_season_identity() {
    let mut p = profile();
    p.feats.extend([
        feat("shot-shield", &["hits", "blocks"], "Silver"),
        feat("puck-thief", &["takeaways"], "Silver"),
    ]);
    let r = select(&p);
    assert_eq!(r.skills.len(), 5);
    let families = r
        .skills
        .iter()
        .map(|s| &s.family)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(families.len(), 5);
    p.group = "Goalies".into();
    p.season = 20252026;
    p.feats = vec![feat(
        "high-save-workload",
        &["shots-faced", "save-fraction"],
        "Silver",
    )];
    let r = select(&p);
    assert_eq!(r.group, "Goalies");
    assert_eq!(r.season, 20252026);
    assert_eq!(r.skills[0].name, "Busy saves");
}
