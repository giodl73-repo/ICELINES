use super::*;
use zones::{Band, Zone};
fn draft(name: &str) -> Value {
    let file = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fixtures/rinkcraft-three-zone")
        .join(format!("{name}.draft.json"));
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}
fn build(v: Value) -> Request {
    collection::build(serde_json::from_value(v).unwrap()).unwrap()
}
fn case(name: &str) -> ResultSet {
    run(&build(draft(name))).unwrap()
}
fn territories(r: &ResultSet) -> Vec<&zones::Audit> {
    r.events
        .iter()
        .filter_map(|e| e.territory.as_ref())
        .collect()
}
fn complete(r: &ResultSet) -> Option<Side> {
    territories(r)
        .into_iter()
        .find(|a| a.outcome == "attack_established")
        .and_then(|a| a.controlling_side)
}
#[test]
fn fixed_frame_does_not_follow_control_and_handles_both_directions() {
    for east in [false, true] {
        let f = zones::Frame {
            west_blue_line: -4,
            east_blue_line: 4,
            a_attacks_east: east,
            period: 1,
        };
        for x in -20..=20 {
            let p = space::Cell { x, y: 0 };
            let a = f.zone(p, Side::A);
            let b = f.zone(p, Side::B);
            assert!(matches!(
                (a, b),
                (Zone::Defensive, Zone::Offensive)
                    | (Zone::Offensive, Zone::Defensive)
                    | (Zone::Neutral, Zone::Neutral)
            ));
            assert_eq!(a == Zone::Neutral, (-4..=4).contains(&x));
        }
    }
}
#[test]
fn carry_route_establishes_attack_and_charges_only_reached_costs() {
    let r = case("carry-route");
    assert_eq!(r.schema, "rinkcraft.event-result.0.9");
    assert_eq!(complete(&r), Some(Side::A));
    assert_eq!(r.final_state.seconds_remaining, 593);
    assert!(r.goals.is_empty());
    assert_eq!(r.final_state.score, [0, 0]);
    let tasks: Vec<_> = r
        .events
        .iter()
        .filter_map(|e| e.transport_task.as_ref())
        .collect();
    assert_eq!(
        tasks.iter().map(|a| a.task.as_str()).collect::<Vec<_>>(),
        ["exit_carry", "neutral_carry", "entry_carry"]
    );
    assert!(tasks
        .iter()
        .all(|a| a.combinations == 20 && a.exact_successes == 20));
    assert_eq!(tasks[0].actor.capability_id, "skating");
    assert_eq!(tasks[2].actor.capability_id, "handling");
}
#[test]
fn pass_crossing_is_in_flight_then_receipt_secures_control() {
    let r = case("pass-route");
    assert_eq!(complete(&r), Some(Side::A));
    assert_eq!(r.final_state.seconds_remaining, 591);
    let crossings: Vec<_> = r
        .events
        .iter()
        .filter(|e| {
            e.territory
                .as_ref()
                .is_some_and(|a| a.outcome == "pass_crossing_in_flight")
        })
        .collect();
    assert_eq!(crossings.len(), 2);
    for e in crossings {
        assert_eq!(e.state.puck, Puck::Flight);
        assert!(
            !e.territory
                .as_ref()
                .unwrap()
                .crossing
                .as_ref()
                .unwrap()
                .control_at_crossing
        );
    }
    assert_eq!(
        r.final_state.puck,
        Puck::Controlled {
            player: "p2".into()
        }
    );
}
#[test]
fn failed_receipt_does_not_erase_physical_crossing_or_award_possession() {
    let mut d = draft("pass-route");
    d["episode"]["steps"][2]["action"]["plan"]["sides"][0]["exit"]["movement"]["receipt"]
        ["target"] = json!(40);
    let r = run(&build(d)).unwrap();
    assert_eq!(complete(&r), None);
    assert_eq!(r.final_state.puck, Puck::Loose);
    let t = territories(&r);
    assert_eq!(t.last().unwrap().band, Band::Neutral);
    assert!(t.iter().any(|a| a.outcome == "pass_crossing_in_flight"));
}
#[test]
fn clearance_is_an_exit_but_opponent_can_control_in_neutral_zone() {
    let r = case("clear-counter");
    assert_eq!(complete(&r), Some(Side::B));
    let t = territories(&r);
    let clear = t
        .iter()
        .find(|a| a.phase == "clear" && a.crossing.is_some())
        .unwrap();
    assert_eq!(clear.band, Band::Neutral);
    assert_eq!(clear.controlling_side, None);
    assert_eq!(t.iter().filter(|a| a.phase == "exit").count(), 0);
    assert!(t
        .iter()
        .any(|a| a.phase == "turnover" && a.controlling_side == Some(Side::B)));
}
#[test]
fn dump_requires_collection_before_attack_is_established() {
    let r = case("dump-recovered");
    assert_eq!(complete(&r), Some(Side::A));
    assert_eq!(
        r.events
            .iter()
            .filter(|e| e.spatial_collection.is_some())
            .count(),
        2
    );
    assert_eq!(
        r.final_state.puck,
        Puck::Controlled {
            player: "p1".into()
        }
    );
    let t = territories(&r);
    assert!(t
        .iter()
        .any(|a| a.phase == "dump" && a.controller.is_none()));
}
#[test]
fn lost_dump_requires_opponent_defensive_exit_before_counter_entry() {
    let r = case("dump-lost-counter");
    assert_eq!(complete(&r), Some(Side::B));
    assert_eq!(r.final_state.seconds_remaining, 588);
    let t = territories(&r);
    let reverse = t.iter().position(|a| a.phase == "turnover").unwrap();
    assert_eq!(t[reverse].band, Band::EastEnd);
    assert_eq!(t[reverse].zone_for_b, Zone::Defensive);
    let exit = t.iter().skip(reverse).find(|a| a.phase == "exit").unwrap();
    assert_eq!(exit.controlling_side, Some(Side::B));
    assert_eq!(t.last().unwrap().band, Band::WestEnd);
}
#[test]
fn regroup_returns_to_defensive_zone_and_requires_second_exit() {
    let r = case("regroup");
    assert_eq!(complete(&r), Some(Side::A));
    let t = territories(&r);
    assert_eq!(
        t.iter()
            .filter(|a| a.phase == "exit" && a.outcome == "control_retained")
            .count(),
        2
    );
    assert!(t
        .iter()
        .any(|a| a.phase == "regroup" && a.zone_for_a == Zone::Defensive));
    assert_eq!(
        r.final_state.puck,
        Puck::Controlled {
            player: "p1".into()
        }
    );
    assert_eq!(r.final_state.seconds_remaining, 590);
}
#[test]
fn unknown_crossing_stops_without_assumed_entry_or_extra_time() {
    let r = case("unknown-entry");
    assert!(r
        .final_state
        .unresolved
        .as_ref()
        .unwrap()
        .contains("ruling unknown"));
    assert_eq!(complete(&r), None);
    assert_eq!(r.final_state.seconds_remaining, 594);
    assert_eq!(territories(&r).last().unwrap().band, Band::Neutral);
}
#[test]
fn attack_direction_is_declared_by_frame_not_period_parity_guess() {
    let r = case("reverse-direction");
    assert_eq!(complete(&r), Some(Side::A));
    assert_eq!(territories(&r).last().unwrap().band, Band::WestEnd);
}
#[test]
fn window_stop_cannot_invent_an_entry_or_consume_its_time() {
    let r = case("window-stop");
    assert_eq!(complete(&r), None);
    assert_eq!(r.final_state.seconds_remaining, 594);
    assert!(r.events.iter().any(|e| e.kind == "transport_window_ended"));
    assert!(!r
        .events
        .iter()
        .filter_map(|e| e.transport_task.as_ref())
        .any(|a| a.task == "entry_carry"));
}
#[test]
fn contested_entry_has_both_outcomes_without_automatic_defender_control() {
    let mut wins = 0;
    let mut losses = 0;
    for seed in 0..100 {
        let mut q = build(draft("contested-entry"));
        q.seed = seed;
        let r = run(&q).unwrap();
        let a = r
            .events
            .iter()
            .filter_map(|e| e.transport_task.as_ref())
            .find(|a| a.task == "entry_carry")
            .unwrap();
        assert_eq!((a.exact_successes, a.combinations), (190, 400));
        if a.success {
            wins += 1;
            assert_eq!(complete(&r), Some(Side::A));
        } else {
            losses += 1;
            assert_eq!(r.final_state.puck, Puck::Loose);
            assert_eq!(territories(&r).last().unwrap().band, Band::Neutral);
        }
    }
    assert!(wins > 0 && losses > 0);
}
#[test]
fn reached_attributes_change_exact_task_chances_without_changing_route_policy() {
    let mut d = draft("contested-entry");
    d["members"][0]["build"]["scores"]["scores"][1] = json!(16);
    let r = run(&build(d)).unwrap();
    let a = r
        .events
        .iter()
        .filter_map(|e| e.transport_task.as_ref())
        .find(|a| a.task == "entry_carry")
        .unwrap();
    assert_eq!((a.exact_successes, a.combinations), (247, 400));
}
#[test]
fn validator_rejects_incomplete_scenes_wrong_sides_and_missing_profiles() {
    for kind in 0..6 {
        let mut d = draft("carry-route");
        let p = &mut d["episode"]["steps"][2]["action"]["plan"];
        match kind {
            0 => {
                p["scenes"][0]["actors"].as_array_mut().unwrap().pop();
            }
            1 => p["sides"][1]["side"] = json!("a"),
            2 => p["sides"][0]["neutral"]["context"] = json!({"kind":"defended","defender":"p1"}),
            3 => p["scenes"][0]["actors"][0]["offset"]["x"] = json!(40),
            4 => p["start_puck"]["x"] = json!(0),
            _ => {
                d["members"].as_array_mut().unwrap().remove(1);
            }
        }
        assert!(
            collection::build(serde_json::from_value(d).unwrap()).is_err(),
            "kind {kind}"
        );
    }
}
#[test]
fn actual_holder_cannot_pass_to_self() {
    let mut d = draft("pass-route");
    d["episode"]["steps"][2]["action"]["plan"]["sides"][0]["exit"]["movement"]["receiver"] =
        json!("p0");
    assert!(run(&build(d)).unwrap_err().contains("actual holder"));
}
#[test]
fn period_boundary_stops_before_crossing() {
    let mut q = build(draft("carry-route"));
    q.seconds_remaining = 5;
    let r = run(&q).unwrap();
    assert!(r.final_state.unresolved.is_some());
    assert_eq!(complete(&r), None);
    assert!(!territories(&r).iter().any(|a| a.crossing.is_some()));
}
#[test]
fn protocol_is_explicit_and_old_access_schema_still_valid() {
    let mut d = draft("carry-route");
    d["schema"] = json!("rinkcraft.event-draft.0.8");
    d["episode"]["schema"] = json!("rinkcraft.event-episode.0.8");
    assert!(collection::build(serde_json::from_value(d).unwrap()).is_err());
}
#[test]
fn every_neutral_fixture_replays_exactly_and_keeps_goal_ledger_empty() {
    for name in [
        "carry-route",
        "pass-route",
        "clear-counter",
        "dump-recovered",
        "dump-lost-counter",
        "regroup",
        "unknown-entry",
        "reverse-direction",
        "contested-entry",
        "window-stop",
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
fn goalie_first_arrival_remains_explicitly_unmapped_in_route() {
    let mut d = draft("carry-route");
    let a = &mut d["episode"]["steps"][2]["action"]["plan"]["scenes"][0]["actors"][5];
    a["offset"] = json!({"x":0,"y":0});
    let r = run(&build(d)).unwrap();
    assert!(r
        .final_state
        .unresolved
        .as_ref()
        .unwrap()
        .contains("goalie reaches first"));
    assert_eq!(complete(&r), None);
}
#[test]
fn retrieval_deadline_does_not_invent_control() {
    let mut d = draft("carry-route");
    d["episode"]["steps"][2]["seconds"] = json!(1);
    let r = run(&build(d)).unwrap();
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert_eq!(r.final_state.seconds_remaining, 597);
    assert!(r.events.iter().all(|e| e.transport_task.is_none()));
}
#[test]
fn transition_limit_stops_in_actual_zone_without_fake_completion() {
    let mut d = draft("carry-route");
    d["episode"]["steps"][2]["action"]["plan"]["max_transitions"] = json!(1);
    let r = run(&build(d)).unwrap();
    assert_eq!(complete(&r), None);
    assert_eq!(territories(&r).last().unwrap().outcome, "transition_limit");
    assert_eq!(territories(&r).last().unwrap().band, Band::Neutral);
}
#[test]
fn exact_penalty_expiry_interrupts_transport_for_new_positions() {
    let mut q = build(draft("carry-route"));
    let macro_step = q.steps[2].clone();
    q.steps = request(vec![
        foul(),
        collect("p6"),
        json!({"kind":"restart","winner":"p0"}),
        release(false),
        flight(json!({"kind":"loose"})),
    ])
    .steps;
    q.steps.push(macro_step);
    // Initial retrieval costs 2, exit 1, neutral 1. Force expiry during entry.
    let wait = Step {
        seconds: 113,
        action: Action::Wait,
    };
    q.steps.insert(3, wait);
    let r = run(&q).unwrap();
    assert!(r
        .final_state
        .unresolved
        .as_ref()
        .unwrap()
        .contains("new snapshot required"));
    assert_eq!(complete(&r), None);
    assert!(r.final_state.active.contains(&"p6".into()));
}
#[test]
fn protocol_nine_preserves_existing_shot_and_rebound_followup_semantics() {
    let d: collection::Draft = serde_json::from_str(include_str!(
        "../../../examples/fixtures/rinkcraft-rebound-followup/neutral.draft.json"
    ))
    .unwrap();
    let old = collection::build(d).unwrap();
    let mut q = old.clone();
    q.schema = "rinkcraft.event-episode.0.9".into();
    let a = run(&old).unwrap();
    let b = run(&q).unwrap();
    assert_eq!(
        serde_json::to_value(a.events).unwrap(),
        serde_json::to_value(b.events).unwrap()
    );
}
