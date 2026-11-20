use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
fn q() -> Request {
    collection::build(
        serde_json::from_str(include_str!(
            "../../../examples/fixtures/rinkcraft-events-passing/neutral.draft.json"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn boosted(q: &Request, id: &str, index: usize) -> Request {
    let mut q = q.clone();
    let p = q.profiles.as_mut().unwrap();
    let m = p.members.iter_mut().find(|m| m.player == id).unwrap();
    let mut scores = [10; 6];
    scores[index] = 16;
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
    q
}
fn audits(r: &ResultSet) -> Vec<&passing::Audit> {
    r.events
        .iter()
        .filter_map(|e| e.pass_task.as_ref())
        .collect()
}

#[test]
fn delivery_and_receipt_exhaust_exact_pairs_and_correct_capabilities() {
    let base = q();
    for (task, a, b, expected, ids) in [
        (
            passing::Task::Delivery,
            "p0",
            "p6",
            [190, 20, 190],
            ["passing", "reading"],
        ),
        (
            passing::Task::Receipt,
            "p1",
            "p7",
            [190, 20, 190],
            ["handling", "defending"],
        ),
    ] {
        let mut counts = [0; 3];
        for pair in 0..400 {
            let audit =
                passing::resolve(base.profiles.as_ref().unwrap(), a, b, task, pair).unwrap();
            assert_eq!(audit.exact_counts, expected);
            assert_eq!(audit.capability_ids, ids);
            counts[match audit.outcome {
                contest::Outcome::ActorWins => 0,
                contest::Outcome::Stalemate => 1,
                contest::Outcome::OppositionWins => 2,
            }] += 1;
        }
        assert_eq!(counts, expected);
    }
    let high = boosted(&base, "p0", 2);
    assert_eq!(
        passing::resolve(
            high.profiles.as_ref().unwrap(),
            "p0",
            "p6",
            passing::Task::Delivery,
            0
        )
        .unwrap()
        .exact_counts,
        [247, 17, 136]
    );
}
#[test]
fn branches_conserve_control_clock_rng_and_attempts() {
    let mut seen = BTreeSet::new();
    for seed in 0..400 {
        let mut q = q();
        q.seed = seed;
        let r = run(&q).unwrap();
        let a = audits(&r);
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&q).unwrap()).unwrap()
        );
        assert_eq!(r.schema, "rinkcraft.event-result.0.3");
        if a[0].outcome != contest::Outcome::ActorWins {
            assert_eq!(a.len(), 1);
            assert_eq!(r.final_state.seconds_remaining, 599);
            assert_eq!(r.final_state.last_touch.as_deref(), Some("p0"));
            assert_eq!(r.final_state.puck, Puck::Loose);
            seen.insert(format!("delivery:{:?}", a[0].outcome));
        } else {
            assert_eq!(a.len(), 2);
            assert_eq!(r.final_state.seconds_remaining, 598);
            assert_eq!(r.final_state.last_touch.as_deref(), Some("p1"));
            assert_eq!(
                r.final_state.puck,
                if a[1].outcome == contest::Outcome::ActorWins {
                    Puck::Controlled {
                        player: "p1".into(),
                    }
                } else {
                    Puck::Loose
                }
            );
            seen.insert(format!("receipt:{:?}", a[1].outcome));
        }
        assert!(r.goals.is_empty());
        assert_eq!(r.final_state.score, [0, 0]);
        assert!(r.final_state.attempt.is_none());
        assert_eq!(r.final_state.last_deliberate.as_deref(), Some("p0"));
    }
    assert_eq!(seen.len(), 5);
}
#[test]
fn receiving_upgrade_preserves_delivery_and_dice_while_improving_control() {
    let mut flips = 0;
    for seed in 0..200 {
        let mut low = q();
        low.seed = seed;
        let high = boosted(&low, "p1", 1);
        let (a, b) = (run(&low).unwrap(), run(&high).unwrap());
        assert_eq!(
            serde_json::to_vec(&a.events[..4]).unwrap(),
            serde_json::to_vec(&b.events[..4]).unwrap()
        );
        let (aa, bb) = (audits(&a), audits(&b));
        if aa.len() == 2 {
            assert_eq!(aa[1].dice, bb[1].dice);
            if aa[1].outcome == contest::Outcome::ActorWins {
                assert_eq!(bb[1].outcome, contest::Outcome::ActorWins);
            }
            if a.final_state.puck == Puck::Loose
                && matches!(b.final_state.puck, Puck::Controlled { .. })
            {
                flips += 1;
            }
        }
    }
    assert!(flips > 0);
}
#[test]
fn pass_upgrade_only_changes_delivery_terms_and_opportunity_count() {
    let mut opened = 0;
    for seed in 0..200 {
        let mut low = q();
        low.seed = seed;
        let high = boosted(&low, "p0", 2);
        let (a, b) = (run(&low).unwrap(), run(&high).unwrap());
        let (aa, bb) = (audits(&a), audits(&b));
        assert_eq!(aa[0].dice, bb[0].dice);
        if aa.len() == 2 {
            assert_eq!(bb.len(), 2);
            assert_eq!(
                serde_json::to_vec(aa[1]).unwrap(),
                serde_json::to_vec(bb[1]).unwrap()
            );
        }
        if aa.len() == 1 && bb.len() == 2 {
            opened += 1;
        }
    }
    assert!(opened > 0);
}
#[test]
fn unused_skating_and_class_labels_cannot_change_pass() {
    let low = q();
    let high = boosted(&low, "p0", 0);
    assert_eq!(
        serde_json::to_vec(&run(&low).unwrap().events).unwrap(),
        serde_json::to_vec(&run(&high).unwrap().events).unwrap()
    );
}
#[test]
fn schema_profile_actor_and_duration_guards() {
    let mut base = q();
    base.schema = "rinkcraft.event-episode.0.2".into();
    assert!(run(&base).unwrap_err().contains("0.3"));
    let mut base = q();
    base.profiles = None;
    assert!(run(&base).is_err());
    let mut base = q();
    base.controller = "p1".into();
    assert!(run(&base).unwrap_err().contains("actual puck controller"));
    let mut base = q();
    if let Action::PassUnderPressure {
        receive_seconds, ..
    } = &mut base.steps[0].action
    {
        *receive_seconds = 0;
    }
    assert!(run(&base).is_err());
    let mut base = q();
    if let Action::PassUnderPressure { receiver, .. } = &mut base.steps[0].action {
        *receiver = "p0".into();
    }
    assert!(run(&base).is_err());
    let mut base = q();
    base.profiles.as_mut().unwrap().members.pop();
    assert!(run(&base).unwrap_err().contains("missing verified"));
}
#[test]
fn same_side_defender_and_goalie_substitution_rejected() {
    let mut base = q();
    base.players[6].side = Side::A;
    base.players[2].side = Side::B;
    assert!(run(&base).unwrap_err().contains("opposing defenders"));
    let mut base = q();
    base.profiles.as_mut().unwrap().members[0].player = "p5".into();
    assert!(run(&base).is_err());
}
#[test]
fn serving_defender_cannot_contest_a_pass() {
    let mut base = q();
    let pass = base.steps[0].clone();
    base.steps = request(service()).steps;
    base.steps.push(pass);
    assert!(run(&base).unwrap_err().contains("inactive player p6"));
}
#[test]
fn receipt_period_boundary_stops_before_contact_or_control() {
    let mut exercised = false;
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        base.seconds_remaining = 2;
        let r = run(&base).unwrap();
        if r.final_state.unresolved.is_some() {
            assert_eq!(audits(&r).len(), 1);
            assert_eq!(r.final_state.puck, Puck::Flight);
            assert!(!kinds(&r).contains(&"receiver_contact"));
            exercised = true;
            break;
        }
    }
    assert!(exercised);
}
#[test]
fn receiving_stage_accounts_for_live_penalty_expiry_exactly() {
    let mut exercised = false;
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        let mut pass = base.steps[0].clone();
        if let Action::PassUnderPressure { lane_defender, .. } = &mut pass.action {
            *lane_defender = "p7".into();
        }
        base.steps = request(service()).steps;
        base.steps.push(Step {
            seconds: 118,
            action: Action::Wait,
        });
        base.steps.push(pass);
        let r = run(&base).unwrap();
        if audits(&r).len() == 2 {
            let expiry = r.events.iter().find(|e| e.kind == "minor_expired").unwrap();
            assert_eq!(expiry.state.seconds_remaining, 478);
            assert_eq!(r.final_state.seconds_remaining, 478);
            assert!(r.final_state.penalty.is_none());
            exercised = true;
            break;
        }
    }
    assert!(exercised);
}
