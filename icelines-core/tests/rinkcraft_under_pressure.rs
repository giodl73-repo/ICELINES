use icelines_core::view_model::{
    rinkcraft_goalie_shots::{Bucket, Context, Data},
    rinkcraft_stat_scores::{self as scores, Player},
    rinkcraft_under_pressure as pressure,
};
use std::collections::BTreeMap;
fn player(group: &str) -> scores::Profile {
    scores::build(
        20262027,
        "all strengths",
        &[Player {
            player_id: 1,
            group: group.into(),
            games: 20,
            minutes: 400.,
            metrics: BTreeMap::new(),
        }],
    )
    .unwrap()
    .remove(0)
}
fn faceoffs() -> pressure::Faceoffs {
    pressure::Faceoffs {
        season: 20262027,
        player_id: 1,
        taken: 100,
        won: 60,
        late_taken: 4,
        late_won: 1,
        late_states: None,
        coverage_games: 200,
        captured_at: "2026-11-24T00:00:00Z".into(),
        source_sha256: "a".repeat(64),
    }
}
#[test]
fn contextual_actions_are_counts_not_opportunity_rates_and_survive_rebuild() {
    let mut p = player("Defense");
    let s = pressure::ActionSample {
        season: p.season,
        player_id: p.player_id,
        counts: pressure::ActionCounts {
            blocks: 25,
            takeaways: 5,
            late_lead_blocks: 3,
            late_lead_takeaways: 0,
        },
        roster_games: 20,
        coverage_games: 200,
        captured_at: faceoffs().captured_at,
        source_sha256: faceoffs().source_sha256,
    };
    let r = pressure::build_with_actions(&p, None, Some(s.clone()));
    pressure::validate(&p, &r).unwrap();
    assert!(r.entries.is_empty());
    assert_eq!(r.actions[0].result, "3 blocks · 0 takeaways");
    assert!(!r.actions[0].result.contains('%'));
    p.under_pressure = Some(r.clone());
    assert_eq!(pressure::rebuild(&p), r);
    assert!(pressure::build(&p, None).actions.is_empty());
    let mut bad = s.clone();
    bad.counts.late_lead_blocks = 26;
    assert!(pressure::validate(&p, &pressure::build_with_actions(&p, None, Some(bad))).is_err());
    let mut bad = s.clone();
    bad.roster_games = 201;
    assert!(pressure::validate(&p, &pressure::build_with_actions(&p, None, Some(bad))).is_err());
    assert!(pressure::validate(
        &player("Goalies"),
        &pressure::build_with_actions(&player("Goalies"), None, Some(s))
    )
    .is_err());
    let old: pressure::Record =
        serde_json::from_value(serde_json::to_value(pressure::build(&p, None)).unwrap()).unwrap();
    assert!(old.actions.is_empty());
    assert!(old.action_sample.is_none());
}
#[test]
fn score_state_partitions_reconcile_and_do_not_invent_missing_splits() {
    let p = player("Forwards");
    let mut f = faceoffs();
    f.late_states = Some(BTreeMap::from([
        ("tied".into(), pressure::Draws { taken: 2, won: 1 }),
        ("leading".into(), pressure::Draws { taken: 1, won: 0 }),
        ("trailing".into(), pressure::Draws { taken: 1, won: 0 }),
    ]));
    let r = pressure::build(&p, Some(f.clone()));
    pressure::validate(&p, &r).unwrap();
    let b = &r.entries[1].breakdown;
    assert_eq!(b.iter().map(|b| b.opportunities).sum::<u32>(), 4);
    assert_eq!(b.iter().map(|b| b.successes).sum::<u32>(), 1);
    assert_eq!(b[0].result, "1/2 won · 50.0%");
    assert_eq!(b[1].name, "Leading by one");
    f.late_states
        .as_mut()
        .unwrap()
        .get_mut("tied")
        .unwrap()
        .taken = 5;
    assert!(pressure::validate(&p, &pressure::build(&p, Some(f))).is_err());
    let legacy = pressure::build(&p, Some(faceoffs()));
    assert!(legacy.entries[1].breakdown.is_empty());
    pressure::validate(&p, &legacy).unwrap();
}
#[test]
fn observed_empty_state_is_not_a_zero_percent_result() {
    let p = player("Forwards");
    let mut f = faceoffs();
    f.late_states = Some(BTreeMap::from([(
        "tied".into(),
        pressure::Draws { taken: 4, won: 1 },
    )]));
    let r = pressure::build(&p, Some(f));
    pressure::validate(&p, &r).unwrap();
    assert_eq!(r.entries[1].breakdown[1].opportunities, 0);
    assert_eq!(
        r.entries[1].breakdown[1].result,
        "No observed opportunities"
    );
}
#[test]
fn skater_challenges_keep_opportunities_windows_and_qualifications_separate() {
    let p = player("Forwards");
    let before = p.clone();
    let r = pressure::build(&p, Some(faceoffs()));
    pressure::validate(&p, &r).unwrap();
    assert_eq!(p, before);
    assert_eq!(r.entries[0].result, "60/100 won · 60.0%");
    assert_eq!(r.entries[1].result, "1/4 won · 25.0%");
    assert!(r.entries[1].definition.contains("Third-period"));
    assert!(r.entries[0].coverage.contains("200 finalized games"));
    assert_eq!(r.games, 20);
    let mut bad = r.clone();
    bad.faceoffs.as_mut().unwrap().season = 20252026;
    assert!(pressure::validate(&p, &bad).is_err());
    let mut bad = r.clone();
    bad.faceoffs.as_mut().unwrap().player_id = 2;
    assert!(pressure::validate(&p, &bad).is_err());
    let mut bad = r.clone();
    bad.entries[0].successes = 99;
    assert!(pressure::validate(&p, &bad).is_err());
    let mut bad = r.clone();
    bad.games = 19;
    assert!(pressure::validate(&p, &bad).is_err());
}
#[test]
fn no_draws_and_missing_tracking_are_gaps_not_zero_ability() {
    let p = player("Defense");
    let mut f = faceoffs();
    f.taken = 0;
    f.won = 0;
    f.late_taken = 0;
    f.late_won = 0;
    let r = pressure::build(&p, Some(f));
    pressure::validate(&p, &r).unwrap();
    assert!(r.entries.is_empty());
    assert!(r.gaps.iter().any(|g| g.contains("no observed draw")));
    assert!(r.gaps.iter().any(|g| g.contains("Entry denial")));
    let r = pressure::build(&p, None);
    assert!(r.gaps[0].contains("not supplied"));
    let mut f = faceoffs();
    f.won = 100;
    assert!(pressure::validate(&p, &pressure::build(&p, Some(f))).is_err());
}
#[test]
fn goalie_saves_exclude_misses_and_keep_empty_samples_unknown() {
    let mut p = player("Goalies");
    let before = p.scores.clone();
    p.shot_context = Some(Context::new(
        Data {
            attempts: 100,
            generated: 4,
            expected: 5.,
            buckets: BTreeMap::from([
                (
                    "fast".into(),
                    Bucket {
                        attempts: 3,
                        on_goal: 2,
                        goals: 1,
                        xg: 0.6,
                    },
                ),
                (
                    "rebound".into(),
                    Bucket {
                        attempts: 6,
                        on_goal: 4,
                        goals: 1,
                        xg: 1.2,
                    },
                ),
            ]),
        },
        "https://peter-tanner.com/moneypuck/downloads/shots_2026.zip".into(),
        "2026-11-24T00:00:00Z".into(),
        "a".repeat(64),
    ));
    let mut ps = vec![p];
    pressure::attach(&mut ps, 20262027);
    let r = ps[0].under_pressure.as_ref().unwrap();
    pressure::validate(&ps[0], r).unwrap();
    assert_eq!(r.entries[0].result, "1/2 saved · 50.0%");
    assert_eq!(r.entries[1].opportunities, 0);
    assert_eq!(r.entries[1].result, "No observed opportunities");
    assert_eq!(r.entries[2].result, "3/4 saved · 75.0%");
    assert_eq!(ps[0].scores, before);
    let mut wrong = ps[0].clone();
    wrong.season = 20252026;
    let withheld = pressure::build(&wrong, None);
    assert!(withheld.entries.is_empty());
    let old = ps[0].clone();
    pressure::attach(&mut ps, 20252026);
    assert_eq!(old, ps[0]);
}
