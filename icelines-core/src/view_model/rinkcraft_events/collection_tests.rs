use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
fn rules() -> game::GameRules {
    game::GameRules {
        schema: game::RULES_SCHEMA.into(),
        version: "event-collection-test.0.1".into(),
        title: "Neutral event test".into(),
        standard_array: [17, 16, 15, 13, 11, 9],
        templates: vec![game::GameTemplate {
            id: "skater".into(),
            branch: game::Branch::Skater,
            class_id: "retriever".into(),
            class_name: "Retriever".into(),
            specialty: "Loose pucks".into(),
            description: "Fictional event test".into(),
            priority: [0, 1, 2, 3, 4, 5],
            task_skills: vec![],
            methods: vec![],
            tendencies: vec![],
            playbook_options: vec![],
        }],
    }
}
fn character(r: &game::GameRules, scores: [u8; 6]) -> game::GameCharacter {
    game::build(
        r,
        game::BuildRequest {
            template_id: "skater".into(),
            name: "Fictional skater".into(),
            position: "F".into(),
            scores: game::ScoreMethod::Manual { scores },
            inspiration: None,
        },
    )
    .unwrap()
}
fn profiled(handling: u8, defending: u8) -> Request {
    let mut q = request(vec![
        release(false),
        flight(json!({"kind":"loose"})),
        json!({"kind":"collect_under_pressure","collector":"p0","pressure":"p6"}),
    ]);
    q.schema = "rinkcraft.event-episode.0.2".into();
    let r = rules();
    let mut a = [10; 6];
    a[1] = handling;
    let mut b = [10; 6];
    b[5] = defending;
    q.profiles = Some(collection::Profiles {
        schema: "rinkcraft.event-profiles.0.1".into(),
        character_rules: r.clone(),
        score_rule: contest::ScoreRule::HalfOffset,
        members: vec![
            collection::Binding {
                player: "p0".into(),
                character: character(&r, a),
            },
            collection::Binding {
                player: "p6".into(),
                character: character(&r, b),
            },
        ],
    });
    q
}
fn audit(r: &ResultSet) -> &collection::Audit {
    r.events.iter().find_map(|e| e.task.as_ref()).unwrap()
}

#[test]
fn all_400_pairs_match_existing_contest_and_exact_counts() {
    for (handling, defending, expected) in [
        (10, 10, [190, 20, 190]),
        (16, 10, [247, 17, 136]),
        (10, 16, [136, 17, 247]),
    ] {
        let q = profiled(handling, defending);
        let p = q.profiles.as_ref().unwrap();
        let mut counts = [0; 3];
        for pair in 0..400 {
            let a = collection::resolve(p, "p0", "p6", pair).unwrap();
            assert_eq!(a.exact_counts, expected);
            assert_eq!(a.capability_ids, ["handling", "defending"]);
            assert_eq!(a.dice, [(pair / 20 + 1) as u8, (pair % 20 + 1) as u8]);
            counts[match a.outcome {
                contest::Outcome::ActorWins => 0,
                contest::Outcome::Stalemate => 1,
                contest::Outcome::OppositionWins => 2,
            }] += 1;
        }
        assert_eq!(counts, expected);
    }
}
#[test]
fn task_results_replay_and_disruption_never_grants_opponent_control() {
    let q = profiled(10, 10);
    let mut seen = BTreeSet::new();
    for seed in 0..100 {
        let mut q = q.clone();
        q.seed = seed;
        let r = run(&q).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&q).unwrap()).unwrap()
        );
        assert_eq!(r.schema, "rinkcraft.event-result.0.2");
        let a = audit(&r);
        seen.insert(format!("{:?}", a.outcome));
        assert_eq!(r.final_state.seconds_remaining, 597);
        assert_eq!(r.final_state.score, [0, 0]);
        if a.outcome == contest::Outcome::ActorWins {
            assert_eq!(
                r.final_state.puck,
                Puck::Controlled {
                    player: "p0".into()
                }
            );
        } else {
            assert_eq!(r.final_state.puck, Puck::Loose);
        }
    }
    assert_eq!(seen.len(), 3);
}
#[test]
fn matched_seed_improvements_change_only_task_terms_and_monotone_control() {
    for seed in 0..100 {
        let mut low = profiled(10, 10);
        low.seed = seed;
        let mut high = profiled(16, 10);
        high.seed = seed;
        let mut pressured = profiled(10, 16);
        pressured.seed = seed;
        let (a, b, c) = (
            run(&low).unwrap(),
            run(&high).unwrap(),
            run(&pressured).unwrap(),
        );
        assert_eq!(audit(&a).dice, audit(&b).dice);
        assert_eq!(audit(&a).dice, audit(&c).dice);
        if audit(&a).outcome == contest::Outcome::ActorWins {
            assert_eq!(audit(&b).outcome, contest::Outcome::ActorWins);
        }
        if audit(&c).outcome == contest::Outcome::ActorWins {
            assert_eq!(audit(&a).outcome, contest::Outcome::ActorWins);
        }
        assert_eq!(
            serde_json::to_vec(&a.events[..4]).unwrap(),
            serde_json::to_vec(&b.events[..4]).unwrap()
        );
    }
}
#[test]
fn unused_capabilities_and_class_labels_do_not_grant_hidden_bonuses() {
    let q = profiled(10, 10);
    let mut changed = q.clone();
    let p = changed.profiles.as_mut().unwrap();
    let mut scores = [20; 6];
    scores[1] = 10;
    p.character_rules.templates[0].class_name = "Wizard of pucks".into();
    p.members[0].character = character(&p.character_rules, scores);
    p.members[1].character = character(&p.character_rules, [10; 6]);
    let a = run(&q).unwrap();
    let b = run(&changed).unwrap();
    assert_eq!(
        serde_json::to_vec(&a.events).unwrap(),
        serde_json::to_vec(&b.events).unwrap()
    );
}
#[test]
fn tampered_unused_and_missing_profiles_are_rejected() {
    let mut q = profiled(10, 10);
    q.profiles.as_mut().unwrap().members[1]
        .character
        .capabilities[5]
        .score = 20;
    assert!(run(&q).unwrap_err().contains("provenance"));
    let mut q = profiled(10, 10);
    q.profiles.as_mut().unwrap().members.pop();
    assert!(run(&q).unwrap_err().contains("missing verified"));
    let mut q = profiled(10, 10);
    let member = q.profiles.as_ref().unwrap().members[0].clone();
    q.profiles.as_mut().unwrap().members.push(member);
    assert!(run(&q).is_err());
    let mut q = profiled(10, 10);
    q.profiles.as_mut().unwrap().members[1].player = "bench".into();
    assert!(run(&q).is_err());
}
#[test]
fn schema_opt_in_is_required_and_legacy_bytes_stay_lean() {
    let mut q = profiled(10, 10);
    q.schema = "rinkcraft.event-episode.0.1".into();
    assert!(run(&q).is_err());
    q.profiles = None;
    assert!(run(&q).is_err());
    q.schema = "rinkcraft.event-episode.0.2".into();
    assert!(run(&q).is_err());
    let old = request(vec![release(true), flight(json!({"kind":"goal"}))]);
    let value = serde_json::to_value(run(&old).unwrap()).unwrap();
    assert!(value["request"].get("profiles").is_none());
    assert!(value["events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e.get("task").is_none()));
}
#[test]
fn same_side_controller_and_goalie_collection_are_rejected() {
    let mut q = profiled(10, 10);
    q.players[6].side = Side::A;
    q.players[1].side = Side::B;
    assert!(run(&q).unwrap_err().contains("opposing active skaters"));
    let mut q = profiled(10, 10);
    q.steps.remove(1);
    assert!(run(&q).unwrap_err().contains("loose puck"));
    let mut q = profiled(10, 10);
    q.profiles.as_mut().unwrap().members[1].player = "p11".into();
    assert!(run(&q).is_err());
}
#[test]
fn serving_minor_excludes_task_pressure_even_with_verified_profile() {
    let mut q = profiled(10, 10);
    let mut steps = service();
    steps.extend([
        release(false),
        flight(json!({"kind":"loose"})),
        json!({"kind":"collect_under_pressure","collector":"p0","pressure":"p6"}),
    ]);
    q.steps = request(steps).steps;
    assert!(run(&q).unwrap_err().contains("inactive player p6"));
}
#[test]
fn task_control_triggers_pending_minor_whistle() {
    let mut q = profiled(10, 10);
    q.steps = request(vec![
        foul(),
        json!({"kind":"collect_under_pressure","collector":"p6","pressure":"p0"}),
    ])
    .steps;
    for seed in 0..100 {
        q.seed = seed;
        let r = run(&q).unwrap();
        if audit(&r).outcome == contest::Outcome::ActorWins {
            assert_eq!(r.final_state.puck, Puck::Dead);
            assert_eq!(r.final_state.active.len(), 11);
            assert!(!r.final_state.penalty.unwrap().delayed);
            return;
        }
    }
    panic!("no actor win exercised");
}
#[test]
fn draft_builds_verified_characters_and_rejects_profile_injection() {
    let mut q = profiled(16, 10);
    let p = q.profiles.take().unwrap();
    let d = collection::Draft {
        schema: "rinkcraft.event-draft.0.2".into(),
        episode: q,
        character_rules: p.character_rules,
        score_rule: p.score_rule,
        members: p
            .members
            .into_iter()
            .map(|m| collection::DraftBinding {
                player: m.player,
                build: game::BuildRequest {
                    template_id: m.character.template_id,
                    name: m.character.name,
                    position: m.character.position,
                    scores: m.character.score_method,
                    inspiration: None,
                },
            })
            .collect(),
    };
    let built = collection::build(d.clone()).unwrap();
    assert_eq!(audit(&run(&built).unwrap()).scores, [16, 10]);
    let mut d = d;
    d.episode.profiles = built.profiles;
    assert!(collection::build(d).is_err());
}
#[test]
fn direct_rule_is_explicit_and_bad_pair_index_is_rejected() {
    let mut q = profiled(16, 10);
    q.profiles.as_mut().unwrap().score_rule = contest::ScoreRule::Direct;
    let r = run(&q).unwrap();
    assert_eq!(audit(&r).modifiers, [16, 10]);
    assert!(collection::resolve(q.profiles.as_ref().unwrap(), "p0", "p6", 400).is_err());
}
