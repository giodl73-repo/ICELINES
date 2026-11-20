use super::*;
use zones::{Band, Zone};
fn draft(name: &str) -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fixtures/rinkcraft-transition-recovery")
        .join(format!("{name}.draft.json"));
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn build(v: Value) -> Request {
    collection::build(serde_json::from_value(v).unwrap()).unwrap()
}
fn case(name: &str) -> ResultSet {
    run(&build(draft(name))).unwrap()
}
fn audits(r: &ResultSet) -> Vec<&zones::Audit> {
    r.events
        .iter()
        .filter_map(|e| e.territory.as_ref())
        .collect()
}
fn completed(r: &ResultSet) -> Option<Side> {
    audits(r)
        .into_iter()
        .find(|a| a.outcome == "attack_established")
        .and_then(|a| a.controlling_side)
}
fn failures(r: &ResultSet) -> Vec<&zones::Audit> {
    audits(r)
        .into_iter()
        .filter(|a| a.outcome == "failed_task_loose")
        .collect()
}
#[test]
fn denied_entry_recovers_in_neutral_and_only_then_turns_over() {
    let r = case("denied-entry-counter");
    assert_eq!(r.schema, "rinkcraft.event-result.0.10");
    assert_eq!(completed(&r), Some(Side::B));
    assert_eq!(failures(&r)[0].band, Band::Neutral);
    assert!(failures(&r)[0].controller.is_none());
    assert_eq!(r.final_state.seconds_remaining, 589);
    assert!(audits(&r)
        .iter()
        .any(|a| a.phase == "turnover" && a.controlling_side == Some(Side::B)));
}
#[test]
fn denied_exit_can_give_opponent_offensive_control_without_crossing() {
    let r = case("denied-exit-counter");
    assert_eq!(completed(&r), Some(Side::B));
    assert_eq!(r.final_state.seconds_remaining, 593);
    assert_eq!(failures(&r)[0].puck.x, -8);
    assert_eq!(audits(&r).last().unwrap().zone_for_b, Zone::Offensive);
    assert!(!audits(&r).iter().any(|a| a.crossing.is_some()));
}
#[test]
fn failed_outlet_receipt_is_retrieved_beyond_own_blue_line() {
    let r = case("failed-outlet-receipt-counter");
    assert_eq!(completed(&r), Some(Side::B));
    assert_eq!(failures(&r)[0].puck.x, -3);
    assert_eq!(r.final_state.seconds_remaining, 590);
}
#[test]
fn failed_entry_receipt_gives_defender_a_defensive_retrieval_and_exit() {
    let r = case("failed-entry-receipt-counter");
    assert_eq!(completed(&r), Some(Side::B));
    assert_eq!(failures(&r)[0].puck.x, 6);
    let t = audits(&r);
    assert!(t
        .iter()
        .any(|a| a.phase == "exit" && a.controlling_side == Some(Side::B)));
    assert_eq!(r.final_state.seconds_remaining, 587);
}
#[test]
fn failed_delivery_recovers_at_departure_not_destination() {
    let r = case("failed-delivery-counter");
    assert_eq!(completed(&r), Some(Side::B));
    assert_eq!(failures(&r)[0].puck.x, 3);
    assert!(!audits(&r)
        .iter()
        .any(|a| a.outcome == "pass_crossing_in_flight"));
    assert_eq!(r.final_state.seconds_remaining, 589);
}
#[test]
fn own_recovery_resets_failed_neutral_progress_and_cannot_skip_to_entry() {
    let r = case("own-neutral-recovery");
    assert_eq!(completed(&r), None);
    assert_eq!(failures(&r).len(), 4);
    assert_eq!(r.final_state.seconds_remaining, 583);
    assert_eq!(audits(&r).last().unwrap().outcome, "transition_limit");
    assert!(!r
        .events
        .iter()
        .filter_map(|e| e.transport_task.as_ref())
        .any(|a| a.task == "entry_carry"));
    assert!(!audits(&r).iter().any(|a| a.phase == "turnover"));
}
#[test]
fn no_reachable_recovery_stays_loose_and_charges_only_window() {
    let r = case("no-arrival");
    assert_eq!(completed(&r), None);
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert_eq!(r.final_state.seconds_remaining, 585);
    assert_eq!(
        r.events
            .iter()
            .filter(|e| e.spatial_collection.is_some())
            .count(),
        1
    );
}
#[test]
fn unknown_context_is_not_a_failed_task_and_never_admits_recovery() {
    let r = case("unknown-context");
    assert!(r.final_state.unresolved.is_some());
    assert!(failures(&r).is_empty());
    assert_eq!(
        r.final_state.puck,
        Puck::Controlled {
            player: "p0".into()
        }
    );
    assert_eq!(
        r.events.iter().filter(|e| e.access_task.is_some()).count(),
        1
    );
}
#[test]
fn unknown_receipt_remains_in_flight_at_destination_without_recovery() {
    let mut d = draft("failed-entry-receipt-counter");
    d["episode"]["steps"][2]["action"]["plan"]["sides"][0]["entry"]["movement"]["receipt"] =
        json!({"kind":"unknown","reason":"Receipt pressure not assessed"});
    let r = run(&build(d)).unwrap();
    assert!(r.final_state.unresolved.is_some());
    assert_eq!(r.final_state.puck, Puck::Flight);
    assert!(failures(&r).is_empty());
    assert_eq!(audits(&r).last().unwrap().puck.x, 6);
}
#[test]
fn exhausted_budget_does_not_admit_a_retrieval_or_owner() {
    let mut d = draft("denied-entry-counter");
    d["episode"]["steps"][2]["seconds"] = json!(5);
    let r = run(&build(d)).unwrap();
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert_eq!(completed(&r), None);
    assert_eq!(r.final_state.seconds_remaining, 593);
    assert_eq!(
        r.events.iter().filter(|e| e.access_task.is_some()).count(),
        1
    );
}
#[test]
fn failed_tasks_with_no_policy_keep_the_frozen_loose_stop_behavior() {
    let mut d = draft("denied-entry-counter");
    d["episode"]["steps"][2]["action"]["plan"]["sides"][0]["entry"]["movement"]
        .as_object_mut()
        .unwrap()
        .remove("on_failure");
    d["episode"]["steps"][2]["action"]["plan"]["scenes"]
        .as_array_mut()
        .unwrap()
        .pop();
    let r = run(&build(d)).unwrap();
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert_eq!(r.final_state.seconds_remaining, 593);
    assert!(failures(&r).is_empty());
}
#[test]
fn opt_in_and_all_failure_scene_references_are_validated_before_play() {
    for kind in 0..5 {
        let mut d = draft("unknown-context");
        let p = &mut d["episode"]["steps"][2]["action"]["plan"];
        match kind {
            0 => p["sides"][0]["entry"]["movement"]["on_failure"]["scene"] = json!("absent"),
            1 => p["sides"][0]["entry"]["movement"]["on_failure"]["basis"] = json!(""),
            2 => {
                p["scenes"][1]["actors"].as_array_mut().unwrap().pop();
            }
            3 => p["scenes"][1]["actors"][0]["offset"]["x"] = json!(20),
            _ => {
                d["schema"] = json!("rinkcraft.event-draft.0.9");
                d["episode"]["schema"] = json!("rinkcraft.event-episode.0.9");
            }
        }
        assert!(
            collection::build(serde_json::from_value(d).unwrap()).is_err(),
            "kind {kind}"
        );
    }
}
#[test]
fn recovery_goalie_first_remains_explicitly_unmapped() {
    let mut d = draft("denied-entry-counter");
    let a = &mut d["episode"]["steps"][2]["action"]["plan"]["scenes"][1]["actors"][11];
    a["offset"] = json!({"x":0,"y":0});
    let r = run(&build(d)).unwrap();
    assert!(r
        .final_state
        .unresolved
        .as_ref()
        .unwrap()
        .contains("goalie reaches first"));
    assert_eq!(completed(&r), None);
}
#[test]
fn all_recovery_cases_replay_with_empty_goal_ledger_and_one_puck() {
    for name in [
        "denied-exit-counter",
        "denied-entry-counter",
        "failed-delivery-counter",
        "failed-outlet-receipt-counter",
        "failed-entry-receipt-counter",
        "own-neutral-recovery",
        "no-arrival",
        "unknown-context",
    ] {
        let q = build(draft(name));
        let a = run(&q).unwrap();
        let b = run(&q).unwrap();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(&b).unwrap()
        );
        assert!(a.goals.is_empty());
        assert_eq!(a.final_state.score, [0, 0]);
    }
}
#[test]
fn old_routes_without_new_policies_keep_identical_events_under_new_protocol() {
    for name in [
        "carry-route",
        "pass-route",
        "dump-lost-counter",
        "regroup",
        "contested-entry",
    ] {
        let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("examples/fixtures/rinkcraft-three-zone")
            .join(format!("{name}.draft.json"));
        let d: collection::Draft =
            serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let q = collection::build(d).unwrap();
        let a = run(&q).unwrap();
        let mut newer = q.clone();
        newer.schema = "rinkcraft.event-episode.0.10".into();
        let b = run(&newer).unwrap();
        assert_eq!(
            serde_json::to_value(a.events).unwrap(),
            serde_json::to_value(b.events).unwrap()
        );
    }
}
#[test]
fn penalty_expiry_during_failure_retrieval_requires_fresh_positions() {
    let mut q = build(draft("denied-entry-counter"));
    let mut step = q.steps[2].clone();
    if let Action::ThreeZoneSequence { plan } = &mut step.action {
        let actors = &mut plan.scenes[1].actors;
        let source = actors[6].clone();
        actors[7].offset = source.offset;
        actors[7].intent = source.intent;
        actors[6].offset = space::Cell { x: 0, y: 12 };
        actors[6].intent = space::Intent::Hold;
    }
    let mut actions = service();
    actions.push(release(false));
    actions.push(flight(json!({"kind":"loose"})));
    q.steps = request(actions).steps;
    q.steps.insert(
        3,
        Step {
            seconds: 111,
            action: Action::Wait,
        },
    );
    q.steps.push(step);
    let r = run(&q).unwrap();
    assert!(r
        .final_state
        .unresolved
        .as_ref()
        .unwrap()
        .contains("new snapshot required"));
    assert_eq!(r.final_state.seconds_remaining, 478);
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert_eq!(completed(&r), None);
    assert!(r.final_state.active.contains(&"p6".into()));
}
#[test]
fn recovery_access_can_be_denied_without_awarding_any_team_control() {
    let mut q = build(draft("denied-entry-counter"));
    if let Action::ThreeZoneSequence { plan } = &mut q.steps[2].action {
        let a = &mut plan.scenes[1].actors[0];
        a.offset = space::Cell { x: -2, y: 0 };
        a.intent = space::Intent::Pursue {
            seconds_per_cell: 1,
            reaction_seconds: 0,
            turn_seconds: 0,
        };
    }
    let mut secured = 0;
    let mut denied = 0;
    for seed in 0..60 {
        q.seed = seed;
        let r = run(&q).unwrap();
        let check = r
            .events
            .iter()
            .filter_map(|e| e.spatial_collection.as_ref())
            .nth(1)
            .unwrap();
        assert_eq!((check.exact_successes, check.combinations), (190, 400));
        assert_eq!(check.collector.player, "p6");
        if check.controlled {
            secured += 1;
            assert_eq!(completed(&r), Some(Side::B));
        } else {
            denied += 1;
            assert_eq!(r.final_state.puck, Puck::Loose);
            assert_eq!(completed(&r), None);
            assert_eq!(r.final_state.seconds_remaining, 591);
        }
    }
    assert!(secured > 0 && denied > 0);
}
