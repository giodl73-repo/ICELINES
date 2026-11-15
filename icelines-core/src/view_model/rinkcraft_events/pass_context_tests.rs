use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
fn q() -> Request {
    collection::build(
        serde_json::from_str(include_str!(
            "../../../examples/fixtures/rinkcraft-pass-context/open-open.draft.json"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn plan(q: &mut Request) -> &mut pass_context::Plan {
    if let Action::PassWithContext { plan } = &mut q.steps[0].action {
        plan
    } else {
        panic!()
    }
}
fn audits(r: &ResultSet) -> Vec<&pass_context::Audit> {
    r.events
        .iter()
        .filter_map(|e| e.context_task.as_ref())
        .collect()
}
fn open() -> pass_context::Context {
    pass_context::Context::Open {
        target: 5,
        basis: "Authored test context".into(),
    }
}
#[test]
fn unopposed_all_dice_and_exact_target_equality() {
    let base = q();
    let p = base.profiles.as_ref().unwrap();
    let mut successes = 0;
    for draw in 0..20 {
        let a = pass_context::resolve(p, "p0", &open(), passing::Task::Delivery, draw).unwrap();
        if let pass_context::Audit::Open {
            die,
            total,
            target,
            exact_successes,
            combinations,
            ..
        } = &a
        {
            assert_eq!(*die, draw as u8 + 1);
            assert_eq!(*total, i16::from(*die));
            assert_eq!(*target, 5);
            assert_eq!((*exact_successes, *combinations), (16, 20));
            assert_eq!(a.success(), *die >= 5);
        } else {
            panic!()
        }
        successes += u16::from(a.success());
    }
    assert_eq!(successes, 16);
    assert!(contest::ScoreRule::HalfOffset.term(0).is_err());
    assert_eq!(contest::ScoreRule::HalfOffset.term(1).unwrap(), -5);
}
#[test]
fn score_rule_direct_and_high_skill_remain_explicit() {
    let mut base = q();
    let p = base.profiles.as_mut().unwrap();
    p.score_rule = contest::ScoreRule::Direct;
    let a = pass_context::resolve(
        p,
        "p0",
        &pass_context::Context::Open {
            target: 20,
            basis: "Different scale".into(),
        },
        passing::Task::Delivery,
        9,
    )
    .unwrap();
    assert!(a.success());
    assert!(pass_context::resolve(p, "p0", &open(), passing::Task::Delivery, 20).is_err());
}
#[test]
fn open_checks_have_no_phantom_opponent_or_binding() {
    let base = q();
    assert_eq!(base.profiles.as_ref().unwrap().members.len(), 2);
    for seed in 0..100 {
        let mut base = base.clone();
        base.seed = seed;
        let r = run(&base).unwrap();
        assert!(audits(&r)
            .iter()
            .all(|a| matches!(a, pass_context::Audit::Open { .. })));
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&base).unwrap()).unwrap()
        );
        assert!(r.goals.is_empty());
        assert_eq!(r.final_state.score, [0, 0]);
    }
}
#[test]
fn contextual_defended_rolls_match_old_stage_math() {
    let d: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-pass-context/defended-defended.draft.json"
    ))
    .unwrap();
    let base = collection::build(d).unwrap();
    let p = base.profiles.as_ref().unwrap();
    for pair in 0..400 {
        let a = pass_context::resolve(
            p,
            "p0",
            &pass_context::Context::Defended {
                defender: "p6".into(),
            },
            passing::Task::Delivery,
            pair,
        )
        .unwrap();
        let b = passing::resolve(p, "p0", "p6", passing::Task::Delivery, pair).unwrap();
        if let pass_context::Audit::Defended { audit } = a {
            assert_eq!(
                serde_json::to_vec(&audit).unwrap(),
                serde_json::to_vec(&b).unwrap()
            );
        } else {
            panic!()
        }
    }
}
#[test]
fn unknown_lane_preserves_controller_without_release_or_dice() {
    let mut base = q();
    plan(&mut base).lane = pass_context::Context::Unknown {
        reason: "No evidence".into(),
    };
    let r = run(&base).unwrap();
    assert!(r.final_state.unresolved.unwrap().contains("lane context"));
    assert_eq!(
        r.final_state.puck,
        Puck::Controlled {
            player: "p0".into()
        }
    );
    assert_eq!(r.final_state.seconds_remaining, 599);
    assert!(audits(&run(&base).unwrap()).is_empty());
    assert!(!kinds(&run(&base).unwrap()).contains(&"pass_released"));
}
#[test]
fn unknown_receipt_only_stops_if_reached() {
    let mut reached = 0;
    let mut missed = 0;
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        plan(&mut base).receipt = pass_context::Context::Unknown {
            reason: "Not known".into(),
        };
        let r = run(&base).unwrap();
        assert_eq!(audits(&r).len(), 1);
        assert_eq!(r.final_state.seconds_remaining, 599);
        if audits(&r)[0].success() {
            assert!(r.final_state.unresolved.is_some());
            assert_eq!(r.final_state.puck, Puck::Flight);
            reached += 1;
        } else {
            assert!(r.final_state.unresolved.is_none());
            assert_eq!(r.final_state.puck, Puck::Loose);
            missed += 1;
        }
    }
    assert!(reached > 0 && missed > 0);
}
#[test]
fn two_open_stage_clocks_and_ownership_follow_actual_checks() {
    let mut kinds_seen = BTreeSet::new();
    for seed in 0..200 {
        let mut base = q();
        base.seed = seed;
        let r = run(&base).unwrap();
        let a = audits(&r);
        let delivery = a[0].success();
        assert_eq!(a.len(), if delivery { 2 } else { 1 });
        assert_eq!(
            r.final_state.seconds_remaining,
            600 - 1 - u16::from(delivery)
        );
        let won = delivery && a[1].success();
        assert_eq!(
            r.final_state.puck,
            if won {
                Puck::Controlled {
                    player: "p1".into(),
                }
            } else {
                Puck::Loose
            }
        );
        assert_eq!(
            r.final_state.last_touch.as_deref(),
            Some(if delivery { "p1" } else { "p0" })
        );
        kinds_seen.insert(if won {
            0
        } else if delivery {
            1
        } else {
            2
        });
    }
    assert_eq!(kinds_seen.len(), 3);
}
#[test]
fn improved_open_passing_raises_exact_chance_without_changing_rolls() {
    let low = q();
    let mut high = low.clone();
    let p = high.profiles.as_mut().unwrap();
    let m = &mut p.members[0];
    let mut scores = [10; 6];
    scores[2] = 16;
    m.character = game::build(
        &p.character_rules,
        game::BuildRequest {
            template_id: m.character.template_id.clone(),
            name: m.character.name.clone(),
            position: m.character.position.clone(),
            scores: game::ScoreMethod::Manual { scores },
            inspiration: None,
        },
    )
    .unwrap();
    for draw in 0..20 {
        let a = pass_context::resolve(
            low.profiles.as_ref().unwrap(),
            "p0",
            &open(),
            passing::Task::Delivery,
            draw,
        )
        .unwrap();
        let b = pass_context::resolve(
            high.profiles.as_ref().unwrap(),
            "p0",
            &open(),
            passing::Task::Delivery,
            draw,
        )
        .unwrap();
        if a.success() {
            assert!(b.success());
        }
        if let pass_context::Audit::Open {
            exact_successes, ..
        } = b
        {
            assert_eq!(exact_successes, 19);
        } else {
            panic!()
        }
    }
}
#[test]
fn invalid_context_schema_target_basis_and_sides_rejected() {
    let mut base = q();
    base.schema = "rinkcraft.event-episode.0.3".into();
    assert!(run(&base).is_err());
    for target in [0, 41] {
        let mut base = q();
        plan(&mut base).lane = pass_context::Context::Open {
            target,
            basis: "Authored".into(),
        };
        assert!(run(&base).is_err());
    }
    let mut base = q();
    plan(&mut base).lane = pass_context::Context::Open {
        target: 5,
        basis: "".into(),
    };
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).lane = pass_context::Context::Defended {
        defender: "p1".into(),
    };
    assert!(run(&base).is_err());
    let mut base = q();
    base.profiles = None;
    assert!(run(&base).is_err());
}
#[test]
fn open_receipt_period_boundary_stops_before_contact() {
    let mut exercised = false;
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        base.seconds_remaining = 2;
        let r = run(&base).unwrap();
        if r.final_state.unresolved.is_some() {
            assert_eq!(audits(&r).len(), 1);
            assert!(!kinds(&r).contains(&"receiver_contact"));
            exercised = true;
            break;
        }
    }
    assert!(exercised);
}
