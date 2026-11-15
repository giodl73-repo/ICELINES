use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
fn q() -> Request {
    collection::build(
        serde_json::from_str(include_str!(
            "../../../examples/fixtures/rinkcraft-goalie-containment/neutral.draft.json"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn plan(q: &mut Request) -> &mut shot::Plan {
    if let Action::ResolveShot { plan } = &mut q.steps.last_mut().unwrap().action {
        plan
    } else {
        panic!()
    }
}
fn audit(r: &ResultSet) -> Option<&rebound::Audit> {
    r.events.iter().find_map(|e| e.rebound_task.as_ref())
}
fn boost(base: &Request, index: usize) -> Request {
    let mut q = base.clone();
    let p = q.profiles.as_mut().unwrap();
    let member = p.members.iter_mut().find(|m| m.player == "p11").unwrap();
    let mut scores = [10; 6];
    scores[index] = 16;
    member.character = game::build(
        &p.character_rules,
        game::BuildRequest {
            template_id: member.character.template_id.clone(),
            name: member.character.name.clone(),
            position: "G".into(),
            scores: game::ScoreMethod::Manual { scores },
            inspiration: None,
        },
    )
    .unwrap();
    q
}
#[test]
fn all_twenty_rolls_hold_equality_and_rebounds_upgrade_exactly() {
    let base = q();
    let high = boost(&base, 3);
    let c = rebound::Context::HoldEligible {
        target: 13,
        basis: "Authored contact".into(),
    };
    let mut counts = [0; 2];
    for draw in 0..20 {
        for (index, q) in [&base, &high].iter().enumerate() {
            let a = rebound::resolve(q.profiles.as_ref().unwrap(), "p11", &c, draw).unwrap();
            assert_eq!(a.capability_id, "rebounds");
            assert_eq!(a.exact_holds, if index == 0 { 8 } else { 11 });
            assert_eq!(a.combinations, 20);
            assert_eq!(a.held, a.total >= 13);
            counts[index] += u16::from(a.held);
        }
    }
    assert_eq!(counts, [8, 11]);
}
#[test]
fn containment_only_runs_after_goalie_contact_and_scores_reconcile() {
    let mut reached = 0;
    let mut skipped = 0;
    for seed in 0..300 {
        let mut base = q();
        base.seed = seed;
        let r = run(&base).unwrap();
        let shot = r.events.iter().find_map(|e| e.shot_task.as_ref()).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&base).unwrap()).unwrap()
        );
        assert_eq!(r.schema, "rinkcraft.event-result.0.6");
        assert_eq!(r.final_state.seconds_remaining, 598);
        assert_eq!(r.final_state.score, [r.goals.len() as u16, 0]);
        if shot.outcome == contest::Outcome::OppositionWins {
            let a = audit(&r).unwrap();
            assert_eq!(
                r.final_state.puck,
                if a.held { Puck::Dead } else { Puck::Loose }
            );
            let contact = r
                .events
                .iter()
                .position(|e| e.kind == "goalie_contact")
                .unwrap();
            let check = r
                .events
                .iter()
                .position(|e| e.rebound_task.is_some())
                .unwrap();
            assert_eq!(check, contact + 1);
            reached += 1;
        } else {
            assert!(audit(&r).is_none());
            skipped += 1;
        }
    }
    assert!(reached > 0 && skipped > 0);
}
#[test]
fn rebound_upgrade_preserves_tracking_contest_and_improves_hold_with_same_die() {
    let mut changed = 0;
    for seed in 0..200 {
        let mut base = q();
        base.seed = seed;
        let high = boost(&base, 3);
        let (a, b) = (run(&base).unwrap(), run(&high).unwrap());
        let sa = a.events.iter().find_map(|e| e.shot_task.as_ref()).unwrap();
        let sb = b.events.iter().find_map(|e| e.shot_task.as_ref()).unwrap();
        assert_eq!(
            serde_json::to_vec(sa).unwrap(),
            serde_json::to_vec(sb).unwrap()
        );
        assert_eq!(a.goals.len(), b.goals.len());
        if let (Some(a), Some(b)) = (audit(&a), audit(&b)) {
            assert_eq!(a.die, b.die);
            if a.held {
                assert!(b.held);
            }
            if !a.held && b.held {
                changed += 1;
            }
        }
    }
    assert!(changed > 0);
}
#[test]
fn tracking_changes_opportunity_not_conditional_rebounds_terms() {
    let base = q();
    let high = boost(&base, 0);
    let c = rebound::Context::HoldEligible {
        target: 13,
        basis: "Authored".into(),
    };
    for draw in 0..20 {
        assert_eq!(
            serde_json::to_vec(
                &rebound::resolve(base.profiles.as_ref().unwrap(), "p11", &c, draw).unwrap()
            )
            .unwrap(),
            serde_json::to_vec(
                &rebound::resolve(high.profiles.as_ref().unwrap(), "p11", &c, draw).unwrap()
            )
            .unwrap()
        );
    }
}
#[test]
fn unmapped_recovery_cannot_change_any_event() {
    let base = q();
    let high = boost(&base, 4);
    for seed in 0..30 {
        let mut a = base.clone();
        a.seed = seed;
        let mut b = high.clone();
        b.seed = seed;
        assert_eq!(
            serde_json::to_vec(&run(&a).unwrap().events).unwrap(),
            serde_json::to_vec(&run(&b).unwrap().events).unwrap()
        );
    }
}
#[test]
fn unknown_context_stops_only_if_contact_reached_without_whistle_or_dice() {
    let mut hit = 0;
    let mut miss = 0;
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        plan(&mut base).rebound_context = Some(rebound::Context::Unknown {
            reason: "Not established".into(),
        });
        let r = run(&base).unwrap();
        let contact = kinds(&r).contains(&"goalie_contact");
        assert!(audit(&r).is_none());
        if contact {
            assert!(r.final_state.unresolved.is_some());
            assert_eq!(r.final_state.puck, Puck::Loose);
            assert!(!kinds(&r).contains(&"whistle"));
            hit += 1;
        } else {
            assert!(r.final_state.unresolved.is_none());
            miss += 1;
        }
    }
    assert!(hit > 0 && miss > 0);
}
#[test]
fn opt_in_and_exclusive_policy_are_required_even_for_unreached_branch() {
    let mut base = q();
    base.schema = "rinkcraft.event-episode.0.5".into();
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).rebound_context = None;
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).containment = vec![Choice {
        weight: 10000,
        outcome: shot::Containment::Hold,
    }];
    assert!(run(&base).is_err());
    for target in [0, 41] {
        let mut base = q();
        plan(&mut base).rebound_context = Some(rebound::Context::HoldEligible {
            target,
            basis: "Authored".into(),
        });
        assert!(run(&base).is_err());
    }
    let mut base = q();
    plan(&mut base).rebound_context = Some(rebound::Context::HoldEligible {
        target: 13,
        basis: "".into(),
    });
    assert!(run(&base).is_err());
}
#[test]
fn direct_scaling_bounds_and_goalie_domain_are_explicit() {
    let mut base = q();
    base.profiles.as_mut().unwrap().score_rule = contest::ScoreRule::Direct;
    let p = base.profiles.as_ref().unwrap();
    let c = rebound::Context::HoldEligible {
        target: 20,
        basis: "Direct scale".into(),
    };
    let a = rebound::resolve(p, "p11", &c, 9).unwrap();
    assert_eq!(a.modifier, 10);
    assert!(a.held);
    assert_eq!(a.exact_holds, 11);
    assert!(rebound::resolve(p, "p0", &c, 9).is_err());
    assert!(rebound::resolve(p, "p11", &c, 20).is_err());
}
#[test]
fn hold_uses_delayed_minor_whistle_while_rebound_keeps_call_live() {
    let mut held = false;
    let mut rebounded = false;
    for seed in 0..200 {
        let mut base = q();
        base.seed = seed;
        let steps = base.steps.clone();
        base.steps = request(vec![foul(), collect("p0")]).steps;
        base.steps.extend(steps);
        let r = run(&base).unwrap();
        if let Some(a) = audit(&r) {
            assert_eq!(r.final_state.penalty.as_ref().unwrap().delayed, !a.held);
            assert_eq!(r.final_state.active.len(), if a.held { 11 } else { 12 });
            if a.held {
                held = true;
            } else {
                rebounded = true;
            }
        }
        if held && rebounded {
            break;
        }
    }
    assert!(held && rebounded);
}
