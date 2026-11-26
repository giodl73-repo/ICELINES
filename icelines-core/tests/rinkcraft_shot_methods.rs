use icelines_core::view_model::{
    rinkcraft_shot_methods as methods, rinkcraft_stat_scores as scores,
};
use std::collections::BTreeMap;
fn player() -> scores::Profile {
    scores::build(
        20262027,
        "all strengths",
        &[scores::Player {
            player_id: 1,
            group: "Forwards".into(),
            games: 4,
            minutes: 40.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0)
}
fn sample() -> methods::Sample {
    methods::Sample {
        player_id: 1,
        season: 20262027,
        roster_games: 7,
        coverage_games: 100,
        by_type: BTreeMap::from([
            (
                "wrist".into(),
                methods::Counts {
                    attempts: 5,
                    on_goal: 4,
                    goals: 1,
                },
            ),
            (
                "snap".into(),
                methods::Counts {
                    attempts: 2,
                    on_goal: 1,
                    goals: 0,
                },
            ),
            (
                "unknown".into(),
                methods::Counts {
                    attempts: 3,
                    on_goal: 2,
                    goals: 1,
                },
            ),
        ]),
        captured_at: "2026-11-24T00:00:00Z".into(),
        source_sha256: "a".repeat(64),
    }
}
#[test]
fn typed_shares_and_goal_counts_keep_their_own_sample_denominator() {
    let mut p = player();
    let scores = p.scores.clone();
    p.shot_method_sample = Some(sample());
    let r = methods::build(&p).unwrap();
    assert_eq!(r.typed_attempts, 7);
    assert_eq!(r.total_attempts, 10);
    assert_eq!(r.primary[0].share, "71.4% of typed attempts");
    assert_eq!(r.primary[0].result, "Attempts 5 · Goals 1");
    assert_eq!(r.primary[0].outcomes, "On goal 4 · Misses 1");
    assert_eq!(p.scores, scores);
    methods::validate(&p, &r).unwrap();
}
#[test]
fn missing_and_empty_or_unknown_only_samples_do_not_invent_methods() {
    let mut p = player();
    assert!(methods::build(&p).is_none());
    let mut s = sample();
    s.by_type.clear();
    p.shot_method_sample = Some(s.clone());
    let r = methods::build(&p).unwrap();
    assert_eq!(r.total_attempts, 0);
    assert!(!r.coverage.contains("0/0"));
    assert!(r.primary.is_empty());
    s.by_type.insert(
        "unknown".into(),
        methods::Counts {
            attempts: 2,
            on_goal: 1,
            goals: 0,
        },
    );
    p.shot_method_sample = Some(s);
    let r = methods::build(&p).unwrap();
    assert_eq!(r.total_attempts, 2);
    assert_eq!(r.typed_attempts, 0);
    assert!(r.primary.is_empty());
}
#[test]
fn rare_provider_labels_stay_distinct_and_the_display_cap_preserves_additional_types() {
    let mut p = player();
    let mut s = sample();
    s.by_type.clear();
    for id in [
        "wrist",
        "snap",
        "slap",
        "backhand",
        "tip-in",
        "deflected",
        "wrap-around",
        "bat",
        "poke",
        "between-legs",
        "cradle",
    ] {
        s.by_type.insert(
            id.into(),
            methods::Counts {
                attempts: 1,
                on_goal: 1,
                goals: 0,
            },
        );
    }
    p.shot_method_sample = Some(s);
    let r = methods::build(&p).unwrap();
    assert_eq!(r.primary.len(), 3);
    assert_eq!(r.additional.len(), 8);
    assert_eq!(r.typed_attempts, 11);
    assert!(r
        .additional
        .iter()
        .chain(&r.primary)
        .any(|m| m.name == "Cradle"));
}
#[test]
fn invalid_counts_stale_windows_goalies_and_forged_text_are_rejected() {
    let mut p = player();
    for edit in 0..5 {
        let mut s = sample();
        match edit {
            0 => s.player_id = 2,
            1 => s.season = 20252026,
            2 => s.by_type.get_mut("wrist").unwrap().goals = 5,
            3 => s
                .by_type
                .insert(
                    "one-timer".into(),
                    methods::Counts {
                        attempts: 1,
                        on_goal: 1,
                        goals: 0,
                    },
                )
                .map(|_| ())
                .unwrap_or(()),
            _ => s.roster_games = 101,
        };
        assert!(methods::validate_sample(&p, &s).is_err());
    }
    p.shot_method_sample = Some(sample());
    let mut r = methods::build(&p).unwrap();
    r.primary[0].share = "100% mastery".into();
    assert!(methods::validate(&p, &r).is_err());
    p.group = "Goalies".into();
    assert!(methods::build(&p).is_none());
}
#[test]
fn optional_projection_roundtrips_without_reclassifying_a_player() {
    let mut p = player();
    assert!(serde_json::to_value(&p)
        .unwrap()
        .get("shot_method_sample")
        .is_none());
    p.shot_method_sample = Some(sample());
    p.shot_methods = methods::build(&p);
    let c = scores::Catalog {
        schema: "rinkcraft.stat-attribute-catalog.v1".into(),
        profiles: vec![p],
    };
    let decoded = scores::Catalog::from_json(&serde_json::to_string(&c).unwrap()).unwrap();
    assert_eq!(decoded.profiles[0].shot_methods, c.profiles[0].shot_methods);
}
