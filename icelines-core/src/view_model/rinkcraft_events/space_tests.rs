use super::*;
fn q() -> Request {
    collection::build(
        serde_json::from_str(include_str!(
            "../../../examples/fixtures/rinkcraft-spatial-access/early-attacker.draft.json"
        ))
        .unwrap(),
    )
    .unwrap()
}
fn plan(q: &mut Request) -> &mut space::Plan {
    if let Action::LoosePuckAccess { plan } = &mut q.steps[2].action {
        plan
    } else {
        panic!()
    }
}
fn actor<'a>(p: &'a mut space::Plan, id: &str) -> &'a mut space::Located {
    p.actors.iter_mut().find(|a| a.player == id).unwrap()
}
fn access(r: &ResultSet) -> Option<&space::Audit> {
    r.events.iter().find_map(|e| e.access_task.as_ref())
}
fn check(r: &ResultSet) -> Option<&space::CollectionAudit> {
    r.events.iter().find_map(|e| e.spatial_collection.as_ref())
}
#[test]
fn geometry_turn_and_hold_intents_determine_arrival_without_ability_conversion() {
    let mut base = q();
    let p = plan(&mut base);
    let a = space::arrivals(p);
    assert_eq!(
        a.iter().find(|a| a.player == "p0").unwrap().seconds,
        Some(2)
    );
    assert!(a
        .iter()
        .find(|a| a.player == "p1")
        .unwrap()
        .seconds
        .is_none());
    actor(p, "p0").facing = space::Facing::West;
    assert_eq!(
        space::arrivals(p)
            .iter()
            .find(|a| a.player == "p0")
            .unwrap()
            .seconds,
        Some(4)
    );
    p.reach_cells = 1;
    assert_eq!(
        space::arrivals(p)
            .iter()
            .find(|a| a.player == "p0")
            .unwrap()
            .seconds,
        Some(3)
    );
}
#[test]
fn earliest_access_is_not_automatic_collection() {
    let mut won = 0;
    let mut lost = 0;
    for seed in 0..100 {
        let mut base = q();
        base.seed = seed;
        let r = run(&base).unwrap();
        let a = access(&r).unwrap();
        let c = check(&r).unwrap();
        assert_eq!(a.winner.as_deref(), Some("p0"));
        assert!(a.access_rolls.is_empty());
        assert!(c.pressure.is_empty());
        assert_eq!((c.exact_successes, c.combinations), (16, 20));
        assert_eq!(r.final_state.seconds_remaining, 595);
        assert_eq!(r.final_state.score, [0, 0]);
        if c.controlled {
            assert_eq!(
                r.final_state.puck,
                Puck::Controlled {
                    player: "p0".into()
                }
            );
            won += 1;
        } else {
            assert_eq!(r.final_state.puck, Puck::Loose);
            lost += 1;
        }
    }
    assert!(won > 0 && lost > 0);
}
#[test]
fn pressure_comes_from_all_opponents_reachable_by_collection_end() {
    let draft: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-spatial-access/two-pressure.draft.json"
    ))
    .unwrap();
    let r = run(&collection::build(draft).unwrap()).unwrap();
    let c = check(&r).unwrap();
    assert_eq!(
        c.pressure
            .iter()
            .map(|r| r.player.as_str())
            .collect::<Vec<_>>(),
        ["p6", "p7"]
    );
    assert_eq!((c.exact_successes, c.combinations), (2470, 8000));
    assert_eq!(space::exact_collection(0, &[0], 5), (190, 400));
    assert_eq!(space::exact_collection(0, &[0, 0, 0], 5), (36100, 160000));
}
#[test]
fn tied_multi_player_access_has_one_winner_or_explicit_unknown() {
    let draft: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-spatial-access/tied-access.draft.json"
    ))
    .unwrap();
    let base = collection::build(draft).unwrap();
    let mut seen = BTreeSet::new();
    for seed in 0..200 {
        let mut q = base.clone();
        q.seed = seed;
        let r = run(&q).unwrap();
        let a = access(&r).unwrap();
        assert_eq!(a.earliest, ["p0", "p6"]);
        assert_eq!(a.access_rolls.len(), 2);
        if let Some(winner) = &a.winner {
            assert_eq!(check(&r).unwrap().collector.player, *winner);
            seen.insert(winner.clone());
        } else {
            assert!(r.final_state.unresolved.is_some());
            assert_eq!(r.final_state.puck, Puck::Loose);
            assert!(check(&r).is_none());
            seen.insert("tie".into());
        }
    }
    assert_eq!(seen.len(), 3);
}
#[test]
fn roster_order_cannot_choose_access_dice_or_collection_winner() {
    let mut base = q();
    let mut alternate = base.clone();
    actor(plan(&mut base), "p6").cell = space::Cell { x: 4, y: 0 };
    actor(plan(&mut alternate), "p6").cell = space::Cell { x: 4, y: 0 };
    actor(plan(&mut base), "p6").facing = space::Facing::West;
    actor(plan(&mut alternate), "p6").facing = space::Facing::West;
    plan(&mut alternate).actors.reverse();
    alternate.players.reverse();
    alternate.profiles.as_mut().unwrap().members.reverse();
    assert_eq!(
        serde_json::to_vec(&run(&base).unwrap().events).unwrap(),
        serde_json::to_vec(&run(&alternate).unwrap().events).unwrap()
    );
}
#[test]
fn all_actual_active_actors_and_fresh_snapshot_are_required() {
    let mut base = q();
    plan(&mut base).snapshot_seconds_remaining = 597;
    assert!(run(&base).unwrap_err().contains("stale"));
    let mut base = q();
    plan(&mut base).actors.pop();
    assert!(run(&base).unwrap_err().contains("actual active pool"));
    let mut base = q();
    let duplicate = plan(&mut base).actors[0].clone();
    plan(&mut base).actors[1] = duplicate;
    assert!(run(&base).is_err());
    let mut base = q();
    base.schema = "rinkcraft.event-episode.0.7".into();
    assert!(run(&base).is_err());
}
#[test]
fn no_reachable_arrival_uses_window_without_inventing_collection() {
    let draft: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-spatial-access/no-arrival.draft.json"
    ))
    .unwrap();
    let r = run(&collection::build(draft).unwrap()).unwrap();
    assert!(access(&r).unwrap().winner.is_none());
    assert_eq!(r.final_state.seconds_remaining, 588);
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert!(check(&r).is_none());
}
#[test]
fn a_first_goalie_is_preserved_as_an_unmapped_opportunity() {
    let draft: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-spatial-access/goalie-first.draft.json"
    ))
    .unwrap();
    let r = run(&collection::build(draft).unwrap()).unwrap();
    assert_eq!(access(&r).unwrap().earliest, ["p11"]);
    assert!(r
        .final_state
        .unresolved
        .as_deref()
        .unwrap()
        .contains("goalie reaches first"));
    assert!(check(&r).is_none());
}
#[test]
fn penalty_expiry_requires_a_new_snapshot_before_new_actor_can_be_used() {
    let mut base = q();
    let mut spatial = base.steps[2].clone();
    if let Action::LoosePuckAccess { plan } = &mut spatial.action {
        plan.snapshot_seconds_remaining = 480;
        plan.actors.retain(|a| a.player != "p6");
    }
    base.steps = request(service()).steps;
    base.steps.push(Step {
        seconds: 116,
        action: Action::Wait,
    });
    base.steps.push(Step {
        seconds: 1,
        action: serde_json::from_value(release(false)).unwrap(),
    });
    base.steps.push(Step {
        seconds: 1,
        action: serde_json::from_value(flight(json!({"kind":"loose"}))).unwrap(),
    });
    base.steps.push(spatial);
    let r = run(&base).unwrap();
    assert!(r
        .final_state
        .unresolved
        .unwrap()
        .contains("active pool changed"));
    assert_eq!(r.final_state.seconds_remaining, 478);
    assert!(r.final_state.active.contains(&"p6".into()));
    assert!(check(&run(&base).unwrap()).is_none());
}
#[test]
fn bad_rates_grid_cells_missing_profiles_and_puck_state_are_rejected() {
    let mut base = q();
    plan(&mut base).puck.x = 21;
    assert!(run(&base).is_err());
    let mut base = q();
    actor(plan(&mut base), "p0").intent = space::Intent::Pursue {
        seconds_per_cell: 0,
        reaction_seconds: 0,
        turn_seconds: 0,
    };
    assert!(run(&base).is_err());
    let mut base = q();
    base.profiles
        .as_mut()
        .unwrap()
        .members
        .retain(|m| m.player != "p6");
    assert!(run(&base).is_err());
    let mut base = q();
    base.steps[1].action =
        serde_json::from_value(flight(json!({"kind":"receive","player":"p0"}))).unwrap();
    assert!(run(&base).unwrap_err().contains("loose puck"));
}
#[test]
fn all_cases_replay_with_causal_ids_and_no_extra_puck_or_goal() {
    for seed in 0..30 {
        let mut base = q();
        base.seed = seed;
        let r = run(&base).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&base).unwrap()).unwrap()
        );
        for (index, e) in r.events.iter().enumerate() {
            assert_eq!(e.id, index as u32 + 1);
            assert_eq!(e.cause, if index == 0 { None } else { Some(index as u32) });
            if let Puck::Controlled { player } = &e.state.puck {
                assert!(e.state.active.contains(player));
            }
        }
        assert!(r.goals.is_empty());
        assert_eq!(r.final_state.score, [0, 0]);
    }
}
#[test]
fn arrival_at_deadline_is_recorded_without_claiming_time_to_collect() {
    let mut base = q();
    let p = plan(&mut base);
    actor(p, "p0").cell.x = -8;
    actor(p, "p6").cell.x = -10;
    let r = run(&base).unwrap();
    let a = access(&r).unwrap();
    assert_eq!(a.earliest_seconds, Some(10));
    assert_eq!(a.earliest, ["p0"]);
    assert_eq!(a.status, "arrival_without_collection_window");
    assert!(a.access_rolls.is_empty());
    assert!(check(&r).is_none());
    assert_eq!(r.final_state.seconds_remaining, 588);
}
#[test]
fn three_arrival_candidates_never_create_multiple_controllers() {
    let mut base = q();
    let p = plan(&mut base);
    actor(p, "p6").cell = space::Cell { x: 4, y: 0 };
    actor(p, "p6").facing = space::Facing::West;
    actor(p, "p7").cell = space::Cell { x: 2, y: -2 };
    actor(p, "p7").facing = space::Facing::North;
    actor(p, "p7").intent = space::Intent::Pursue {
        seconds_per_cell: 1,
        reaction_seconds: 0,
        turn_seconds: 2,
    };
    let mut seen = BTreeSet::new();
    for seed in 0..150 {
        let mut q = base.clone();
        q.seed = seed;
        let r = run(&q).unwrap();
        let a = access(&r).unwrap();
        assert_eq!(a.earliest.len(), 3);
        assert_eq!(a.access_rolls.len(), 3);
        if let Some(id) = &a.winner {
            seen.insert(id.clone());
            assert_eq!(check(&r).unwrap().collector.player, *id);
        } else {
            seen.insert("tie".into());
            assert!(r.final_state.unresolved.is_some());
        }
    }
    assert_eq!(seen.len(), 4);
}
#[test]
fn defending_access_alone_does_not_stop_a_delayed_call_but_collection_does() {
    let d: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-spatial-access/early-defender.draft.json"
    ))
    .unwrap();
    let mut base = collection::build(d).unwrap();
    let mut spatial = base.steps[2].clone();
    if let Action::LoosePuckAccess { plan } = &mut spatial.action {
        plan.snapshot_seconds_remaining = 599;
    }
    base.steps = request(vec![foul()]).steps;
    base.steps.push(spatial);
    let mut collected = false;
    let mut denied = false;
    for seed in 0..100 {
        let mut q = base.clone();
        q.seed = seed;
        let r = run(&q).unwrap();
        assert_eq!(access(&r).unwrap().winner.as_deref(), Some("p6"));
        if check(&r).unwrap().controlled {
            assert_eq!(r.final_state.puck, Puck::Dead);
            assert!(!r.final_state.penalty.as_ref().unwrap().delayed);
            assert_eq!(r.final_state.active.len(), 11);
            collected = true;
        } else {
            assert_eq!(r.final_state.puck, Puck::Loose);
            assert!(r.final_state.penalty.as_ref().unwrap().delayed);
            denied = true;
        }
    }
    assert!(collected && denied);
}
#[test]
fn newer_spatial_protocol_composes_with_existing_followup_actions() {
    let draft: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-rebound-followup/handling.draft.json"
    ))
    .unwrap();
    let old = collection::build(draft).unwrap();
    let mut new = old.clone();
    new.schema = "rinkcraft.event-episode.0.8".into();
    assert_eq!(
        serde_json::to_vec(&run(&old).unwrap().events).unwrap(),
        serde_json::to_vec(&run(&new).unwrap().events).unwrap()
    );
}
#[test]
fn skating_grade_does_not_change_declared_arrival_rates() {
    use crate::view_model::rinkcraft_game as game;
    let base = q();
    let mut high = base.clone();
    let p = high.profiles.as_mut().unwrap();
    let m = p.members.iter_mut().find(|m| m.player == "p0").unwrap();
    m.character = game::build(
        &p.character_rules,
        game::BuildRequest {
            template_id: m.character.template_id.clone(),
            name: m.character.name.clone(),
            position: m.character.position.clone(),
            scores: game::ScoreMethod::Manual {
                scores: [16, 10, 10, 10, 10, 10],
            },
            inspiration: None,
        },
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&run(&base).unwrap().events).unwrap(),
        serde_json::to_vec(&run(&high).unwrap().events).unwrap()
    );
}
