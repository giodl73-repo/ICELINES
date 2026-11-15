use super::*;
use crate::view_model::{rinkcraft_contest as contest, rinkcraft_game as game};
fn q() -> Request {
    collection::build(
        serde_json::from_str(include_str!(
            "../../../examples/fixtures/rinkcraft-rebound-followup/neutral.draft.json"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn plan(q: &mut Request) -> &mut followup::Plan {
    if let Action::ReboundFollowup { plan } = &mut q.steps.last_mut().unwrap().action {
        plan
    } else {
        panic!()
    }
}
fn recovery(r: &ResultSet) -> Option<&followup::Audit> {
    r.events.iter().find_map(|e| e.recovery_task.as_ref())
}
fn boost(base: &Request, id: &str, index: usize) -> Request {
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
#[test]
fn exact_recovery_pairs_use_recovery_against_skating_and_ties_stay_unknown() {
    let base = q();
    for (q, expected) in [
        (base.clone(), [190, 20, 190]),
        (boost(&base, "p11", 4), [247, 17, 136]),
        (boost(&base, "p1", 0), [136, 17, 247]),
    ] {
        let mut counts = [0; 3];
        for pair in 0..400 {
            let a = followup::resolve(q.profiles.as_ref().unwrap(), "p11", "p1", pair, -2).unwrap();
            assert_eq!(a.capability_ids, ["recovery", "skating"]);
            assert_eq!(a.exact_counts, expected);
            let index = match a.outcome {
                contest::Outcome::ActorWins => {
                    assert_eq!(a.next_goalie_adjustment, Some(0));
                    0
                }
                contest::Outcome::Stalemate => {
                    assert!(a.next_goalie_adjustment.is_none());
                    1
                }
                contest::Outcome::OppositionWins => {
                    assert_eq!(a.next_goalie_adjustment, Some(-2));
                    2
                }
            };
            counts[index] += 1;
        }
        assert_eq!(counts, expected);
    }
}
#[test]
fn full_chain_replay_admission_clocks_goals_and_parent_ids() {
    let mut skipped = 0;
    let mut denied = 0;
    let mut unknown = 0;
    let mut reset = 0;
    let mut late = 0;
    let mut goals = 0;
    for seed in 0..700 {
        let mut base = q();
        base.seed = seed;
        let r = run(&base).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&base).unwrap()).unwrap()
        );
        assert_eq!(r.schema, "rinkcraft.event-result.0.7");
        assert_eq!(r.final_state.score, [r.goals.len() as u16, 0]);
        assert!(r.goals.len() <= 1);
        let second: Vec<_> = r
            .events
            .iter()
            .filter(|e| e.kind == "shot_released" && e.step == 2)
            .collect();
        if kinds(&r).contains(&"followup_skipped") {
            assert_eq!(r.final_state.seconds_remaining, 598);
            assert!(recovery(&r).is_none());
            skipped += 1;
        } else if kinds(&r).contains(&"followup_collection_denied") {
            assert_eq!(r.final_state.seconds_remaining, 597);
            assert_eq!(r.final_state.puck, Puck::Loose);
            assert!(second.is_empty());
            denied += 1;
        } else if let Some(a) = recovery(&r) {
            if a.next_goalie_adjustment.is_none() {
                assert!(r.final_state.unresolved.is_some());
                assert_eq!(
                    r.final_state.puck,
                    Puck::Controlled {
                        player: "p1".into()
                    }
                );
                assert_eq!(r.final_state.seconds_remaining, 597);
                assert!(second.is_empty());
                unknown += 1;
            } else {
                assert_eq!(r.final_state.seconds_remaining, 595);
                assert_eq!(second.len(), 1);
                assert_eq!(
                    (second[0].attempt, second[0].parent_attempt),
                    (Some(2), Some(1))
                );
                let shot = r
                    .events
                    .iter()
                    .filter_map(|e| e.shot_task.as_ref())
                    .last()
                    .unwrap();
                assert_eq!(shot.shooter, "p1");
                assert_eq!(shot.goalie_adjustment, a.next_goalie_adjustment);
                if a.readiness == "reset" {
                    reset += 1;
                } else {
                    late += 1;
                }
                if !r.goals.is_empty() {
                    assert_eq!(r.goals[0].attempt, 2);
                    goals += 1;
                }
            }
        }
        for goal in &r.goals {
            assert_eq!(r.events[(goal.event - 1) as usize].kind, "goal");
        }
    }
    assert!(skipped > 0 && denied > 0 && unknown > 0 && reset > 0 && late > 0 && goals > 0);
}
#[test]
fn recovery_improvement_preserves_initial_shot_collection_and_race_dice() {
    let mut improved = 0;
    for seed in 0..250 {
        let mut base = q();
        base.seed = seed;
        let high = boost(&base, "p11", 4);
        let (a, b) = (run(&base).unwrap(), run(&high).unwrap());
        if let (Some(a), Some(b)) = (recovery(&a), recovery(&b)) {
            assert_eq!(a.dice, b.dice);
            if a.readiness == "reset" {
                assert_eq!(b.readiness, "reset");
            }
            if a.readiness == "late" && b.readiness == "reset" {
                improved += 1;
            }
        }
        let initial = |r: &ResultSet| {
            serde_json::to_vec(
                &r.events
                    .iter()
                    .filter(|e| e.step < 2 || e.kind == "followup_collection")
                    .collect::<Vec<_>>(),
            )
            .unwrap()
        };
        assert_eq!(initial(&a), initial(&b));
    }
    assert!(improved > 0);
}
#[test]
fn adjustment_changes_only_next_contest_terms_not_tracking_grade() {
    let base = q();
    let p = base.profiles.as_ref().unwrap();
    for pair in 0..400 {
        let normal = shot::resolve_adjusted(p, "p1", "p11", pair, Some(0)).unwrap();
        let late = shot::resolve_adjusted(p, "p1", "p11", pair, Some(-2)).unwrap();
        assert_eq!(normal.scores, late.scores);
        assert_eq!(normal.dice, late.dice);
        assert_eq!(late.totals[1], normal.totals[1] - 2);
        assert_eq!(late.exact_counts, [229, 18, 153]);
    }
    assert!(shot::resolve_adjusted(p, "p1", "p11", 0, Some(1)).is_err());
}
#[test]
fn policy_and_identity_guards_validate_even_when_initial_shot_would_end_play() {
    let mut base = q();
    base.schema = "rinkcraft.event-episode.0.6".into();
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).late_tracking_adjustment = 0;
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).shot.shooter = "p0".into();
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).access_basis.clear();
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).release_seconds = 0;
    assert!(run(&base).is_err());
    let mut base = q();
    base.steps.remove(1);
    assert!(run(&base).is_err());
    let mut base = q();
    plan(&mut base).pressure = "p0".into();
    assert!(run(&base).is_err());
}
#[test]
fn unmapped_movement_does_not_supply_a_hidden_readiness_bonus() {
    let base = q();
    let high = boost(&base, "p11", 2);
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
fn synthetic_release_period_boundary_never_invents_second_shot() {
    let mut exercised = false;
    for seed in 0..300 {
        let mut base = q();
        base.seed = seed;
        base.seconds_remaining = 4;
        let r = run(&base).unwrap();
        if recovery(&r).is_some_and(|a| a.next_goalie_adjustment.is_some()) {
            assert!(r.final_state.unresolved.is_some());
            assert!(!r
                .events
                .iter()
                .any(|e| e.kind == "shot_released" && e.step == 2));
            exercised = true;
            break;
        }
    }
    assert!(exercised);
}
