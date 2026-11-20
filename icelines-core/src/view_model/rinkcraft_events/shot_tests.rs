use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
fn q() -> Request {
    collection::build(
        serde_json::from_str(include_str!(
            "../../../examples/fixtures/rinkcraft-shot-contest/neutral.draft.json"
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
fn audit(r: &ResultSet) -> &shot::Audit {
    r.events.iter().find_map(|e| e.shot_task.as_ref()).unwrap()
}
fn boosted(base: &Request, id: &str, index: usize) -> Request {
    let mut q = base.clone();
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
fn forced_goal(q: &mut Request) {
    plan(q).finish = vec![Choice {
        weight: 10000,
        outcome: shot::Finish::Goal,
    }];
}

#[test]
fn all_pairs_map_shooting_and_goalie_tracking_without_domain_substitution() {
    let base = q();
    for (q, counts) in [
        (base.clone(), [190, 20, 190]),
        (boosted(&base, "p0", 3), [247, 17, 136]),
        (boosted(&base, "p11", 0), [136, 17, 247]),
    ] {
        let mut actual = [0; 3];
        for pair in 0..400 {
            let a = shot::resolve(q.profiles.as_ref().unwrap(), "p0", "p11", pair).unwrap();
            assert_eq!(a.capability_ids, ["shooting", "tracking"]);
            assert_eq!(a.exact_counts, counts);
            actual[match a.outcome {
                contest::Outcome::ActorWins => 0,
                contest::Outcome::Stalemate => 1,
                contest::Outcome::OppositionWins => 2,
            }] += 1;
        }
        assert_eq!(actual, counts);
    }
}
#[test]
fn seeded_branches_replay_and_scores_are_only_goal_ledger_entries() {
    let mut seen = BTreeSet::new();
    for seed in 0..500 {
        let mut base = q();
        base.seed = seed;
        let r = run(&base).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&base).unwrap()).unwrap()
        );
        assert_eq!(r.schema, "rinkcraft.event-result.0.5");
        assert_eq!(r.final_state.seconds_remaining, 598);
        assert_eq!(r.final_state.score, [r.goals.len() as u16, 0]);
        assert!(r.goals.len() <= 1);
        for goal in &r.goals {
            assert_eq!(goal.attempt, 1);
            assert_eq!(r.events[(goal.event - 1) as usize].kind, "goal");
        }
        let branch = match audit(&r).outcome {
            contest::Outcome::ActorWins => {
                if !r.goals.is_empty() {
                    "goal"
                } else if r.final_state.puck == Puck::Flight {
                    "post"
                } else {
                    "shot_loose"
                }
            }
            contest::Outcome::Stalemate => {
                assert_eq!(r.final_state.last_touch.as_deref(), Some("p0"));
                "tie_loose"
            }
            contest::Outcome::OppositionWins => {
                assert_eq!(r.final_state.last_touch.as_deref(), Some("p11"));
                if r.final_state.puck == Puck::Dead {
                    "hold"
                } else {
                    "rebound"
                }
            }
        };
        seen.insert(branch);
    }
    assert_eq!(seen.len(), 6);
}
#[test]
fn shooting_and_tracking_upgrades_preserve_dice_and_monotone_contest_wins() {
    let mut wins = [0; 3];
    for seed in 0..200 {
        let mut base = q();
        base.seed = seed;
        let variants = [
            base.clone(),
            boosted(&base, "p0", 3),
            boosted(&base, "p11", 0),
        ];
        let results: Vec<_> = variants.iter().map(|q| run(q).unwrap()).collect();
        for i in 0..3 {
            assert_eq!(audit(&results[0]).dice, audit(&results[i]).dice);
            wins[i] += u16::from(audit(&results[i]).outcome == contest::Outcome::ActorWins);
        }
        if audit(&results[0]).outcome == contest::Outcome::ActorWins {
            assert_eq!(audit(&results[1]).outcome, contest::Outcome::ActorWins);
        }
        if audit(&results[2]).outcome == contest::Outcome::ActorWins {
            assert_eq!(audit(&results[0]).outcome, contest::Outcome::ActorWins);
        }
    }
    assert!(wins[1] > wins[0] && wins[0] > wins[2]);
}
#[test]
fn unmapped_rebounds_capability_cannot_change_authored_containment() {
    let base = q();
    let changed = boosted(&base, "p11", 3);
    for seed in 0..50 {
        let mut a = base.clone();
        a.seed = seed;
        let mut b = changed.clone();
        b.seed = seed;
        assert_eq!(
            serde_json::to_vec(&run(&a).unwrap().events).unwrap(),
            serde_json::to_vec(&run(&b).unwrap().events).unwrap()
        );
    }
}
#[test]
fn all_distributions_validated_even_when_branch_not_reached() {
    for which in 0..4 {
        let mut base = q();
        let p = plan(&mut base);
        match which {
            0 => p.containment.clear(),
            1 => p.finish[0].weight = 4999,
            2 => p.finish[0].weight = 0,
            _ => {
                p.containment = vec![
                    Choice {
                        weight: 5000,
                        outcome: shot::Containment::Hold,
                    },
                    Choice {
                        weight: 5000,
                        outcome: shot::Containment::Hold,
                    },
                ]
            }
        }
        assert!(run(&base).is_err());
    }
}
#[test]
fn wrong_version_profile_branch_and_release_identity_rejected() {
    let mut base = q();
    base.schema = "rinkcraft.event-episode.0.4".into();
    assert!(run(&base).is_err());
    let mut base = q();
    base.profiles.as_mut().unwrap().members.pop();
    assert!(run(&base).is_err());
    let mut base = q();
    base.profiles.as_mut().unwrap().members[1].player = "p7".into();
    assert!(run(&base).is_err());
    let mut base = q();
    let mut extra = base.profiles.as_ref().unwrap().members[0].clone();
    extra.player = "p1".into();
    base.profiles.as_mut().unwrap().members.push(extra);
    plan(&mut base).shooter = "p1".into();
    assert!(run(&base).unwrap_err().contains("deliberate release"));
}
#[test]
fn shot_requires_live_attempt_and_opposing_actual_goalie() {
    let mut base = q();
    base.steps.remove(0);
    assert!(run(&base).unwrap_err().contains("live in-flight"));
    let mut base = q();
    if let Action::Release { shot, .. } = &mut base.steps[0].action {
        *shot = false;
    }
    assert!(run(&base).is_err());
    let mut base = q();
    let mut extra = base.profiles.as_ref().unwrap().members[1].clone();
    extra.player = "p5".into();
    base.profiles.as_mut().unwrap().members.push(extra);
    plan(&mut base).goalie = "p5".into();
    assert!(run(&base).unwrap_err().contains("opposing active"));
}
#[test]
fn post_continues_same_attempt_to_goal_without_second_shot() {
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        plan(&mut base).finish = vec![Choice {
            weight: 10000,
            outcome: shot::Finish::Post,
        }];
        let r = run(&base).unwrap();
        if r.final_state.puck == Puck::Flight {
            base.steps.push(Step {
                seconds: 1,
                action: serde_json::from_value(flight(json!({"kind":"goal"}))).unwrap(),
            });
            let r = run(&base).unwrap();
            assert_eq!(r.goals.len(), 1);
            assert_eq!(r.goals[0].attempt, 1);
            assert_eq!(
                r.events
                    .iter()
                    .filter(|e| e.kind == "shot_released")
                    .count(),
                1
            );
            return;
        }
    }
    panic!("post branch not reached");
}
#[test]
fn tip_preserves_original_shot_identity_and_capability_actor() {
    let d: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-shot-contest/tip-chain.draft.json"
    ))
    .unwrap();
    let q = collection::build(d).unwrap();
    let r = run(&q).unwrap();
    let tip = r
        .events
        .iter()
        .find(|e| e.kind == "player_deflection")
        .unwrap();
    assert_eq!(tip.attempt, Some(1));
    assert_eq!(tip.state.last_touch.as_deref(), Some("p1"));
    assert_eq!(audit(&r).shooter, "p0");
    assert_eq!(r.final_state.last_deliberate.as_deref(), Some("p0"));
}
#[test]
fn rebound_collection_and_new_release_link_parent_attempt() {
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        plan(&mut base).containment = vec![Choice {
            weight: 10000,
            outcome: shot::Containment::Rebound,
        }];
        let r = run(&base).unwrap();
        if audit(&r).outcome != contest::Outcome::OppositionWins {
            continue;
        }
        for action in [collect("p0"), release(true), flight(json!({"kind":"goal"}))] {
            base.steps.push(Step {
                seconds: 1,
                action: serde_json::from_value(action).unwrap(),
            });
        }
        let r = run(&base).unwrap();
        assert_eq!(r.goals[0].attempt, 2);
        let shot = r
            .events
            .iter()
            .filter(|e| e.kind == "shot_released")
            .last()
            .unwrap();
        assert_eq!(shot.parent_attempt, Some(1));
        return;
    }
    panic!("goalie contact not reached");
}
#[test]
fn goal_uses_existing_pending_and_serving_minor_logic() {
    for serving in [false, true] {
        let mut exercised = false;
        for seed in 0..100 {
            let mut base = q();
            base.seed = seed;
            forced_goal(&mut base);
            let end = base.steps.clone();
            base.steps = request(if serving {
                service()
            } else {
                vec![foul(), collect("p0")]
            })
            .steps;
            base.steps.extend(end);
            let r = run(&base).unwrap();
            if r.goals.is_empty() {
                continue;
            }
            assert!(r.final_state.penalty.is_none());
            assert_eq!(r.final_state.active.len(), 12);
            assert!(kinds(&r).contains(&if serving {
                "minor_ended_by_goal"
            } else {
                "pending_minor_washed"
            }));
            exercised = true;
            break;
        }
        assert!(exercised);
    }
}
#[test]
fn goalie_touch_does_not_stop_delayed_call_but_hold_does() {
    for hold in [false, true] {
        let mut exercised = false;
        for seed in 0..100 {
            let mut base = q();
            base.seed = seed;
            plan(&mut base).containment = vec![Choice {
                weight: 10000,
                outcome: if hold {
                    shot::Containment::Hold
                } else {
                    shot::Containment::Rebound
                },
            }];
            let end = base.steps.clone();
            base.steps = request(vec![foul(), collect("p0")]).steps;
            base.steps.extend(end);
            let r = run(&base).unwrap();
            if audit(&r).outcome != contest::Outcome::OppositionWins {
                continue;
            }
            assert_eq!(r.final_state.penalty.as_ref().unwrap().delayed, !hold);
            assert_eq!(r.final_state.active.len(), if hold { 11 } else { 12 });
            assert_eq!(
                r.final_state.puck,
                if hold { Puck::Dead } else { Puck::Loose }
            );
            exercised = true;
            break;
        }
        assert!(exercised);
    }
}
