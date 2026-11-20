use super::*;
fn draft(name: &str) -> Value {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fixtures/rinkcraft-team-intent")
        .join(format!("{name}.draft.json"));
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn build(d: Value) -> Request {
    collection::build(serde_json::from_value(d).unwrap()).unwrap()
}
fn case(name: &str) -> ResultSet {
    run(&build(draft(name))).unwrap()
}
fn audit(r: &ResultSet) -> &team_intent::Audit {
    r.events
        .iter()
        .find_map(|e| e.team_intent.as_ref())
        .unwrap()
}
fn complete(r: &ResultSet) -> Option<Side> {
    r.events
        .iter()
        .filter_map(|e| e.territory.as_ref())
        .find(|a| a.outcome == "attack_established")
        .and_then(|a| a.controlling_side)
}
fn intent(d: &mut Value) -> &mut Value {
    &mut d["episode"]["steps"][2]["action"]["plan"]["scenes"][1]["team_intent"]
}
#[test]
fn timely_forecast_permits_named_pinch_and_actual_collection() {
    let r = case("timely-cover");
    let a = audit(&r);
    assert_eq!(r.schema, "rinkcraft.event-result.0.11");
    assert_eq!(a.decision, "pinch_permitted");
    assert_eq!(a.cover_arrival_seconds, Some(1));
    assert_eq!(a.cover_deadline_seconds, Some(1));
    assert!(!a.executed_cover_confirmed);
    assert_eq!(complete(&r), Some(Side::A));
    assert_eq!(r.final_state.seconds_remaining, 589);
    let access = r
        .events
        .iter()
        .filter_map(|e| e.access_task.as_ref())
        .nth(1)
        .unwrap();
    assert_eq!(access.winner.as_deref(), Some("p0"));
}
#[test]
fn late_cover_aborts_chase_and_defender_can_recover_and_counter() {
    let r = case("late-cover");
    let a = audit(&r);
    assert_eq!(a.decision, "pinch_aborted");
    assert_eq!(a.cover_arrival_seconds, Some(4));
    assert_eq!(complete(&r), Some(Side::B));
    assert_eq!(r.final_state.seconds_remaining, 584);
    let access = r
        .events
        .iter()
        .filter_map(|e| e.access_task.as_ref())
        .nth(1)
        .unwrap();
    assert_eq!(access.winner.as_deref(), Some("p6"));
    assert!(access
        .arrivals
        .iter()
        .find(|a| a.player == "p0")
        .unwrap()
        .seconds
        .is_none());
}
#[test]
fn cover_is_forecast_not_invented_executed_motion() {
    let r = case("timely-cover");
    let a = audit(&r);
    assert_eq!(a.cover_status, "projected_timely");
    assert!(!a.executed_cover_confirmed);
    let access = r
        .events
        .iter()
        .filter_map(|e| e.access_task.as_ref())
        .nth(1)
        .unwrap();
    let cover = access.arrivals.iter().find(|a| a.player == "p1").unwrap();
    assert_eq!(cover.reason, "holding_outside_reach");
    assert_eq!(cover.seconds, None);
}
#[test]
fn inactive_trigger_does_not_require_unknown_cover_or_start_chase() {
    let mut d = draft("inactive-trigger");
    intent(&mut d)["cover"] = json!({"kind":"unknown","reason":"Not required in inactive branch"});
    let r = run(&build(d)).unwrap();
    assert_eq!(audit(&r).decision, "not_triggered");
    assert_eq!(audit(&r).cover_status, "not_required");
    assert!(r.final_state.unresolved.is_none());
    assert_eq!(complete(&r), Some(Side::B));
}
#[test]
fn unknown_trigger_and_cover_stop_before_chase_clock_or_dice() {
    for name in ["unknown-cover", "unknown-trigger"] {
        let r = case(name);
        assert_eq!(audit(&r).decision, "unresolved");
        assert!(r.final_state.unresolved.is_some());
        assert_eq!(r.final_state.puck, Puck::Loose);
        assert_eq!(r.final_state.seconds_remaining, 593);
        assert_eq!(
            r.events.iter().filter(|e| e.access_task.is_some()).count(),
            1
        );
        assert_eq!(complete(&r), None);
    }
}
#[test]
fn outlet_presence_proximity_and_pass_lane_are_separate() {
    let clear = case("timely-cover");
    let pressured = case("pressured-outlet");
    let missing = case("missing-outlet");
    assert_eq!(audit(&clear).outlets[0].status, "at_station_unpressured");
    assert_eq!(audit(&pressured).outlets[0].status, "at_station_pressured");
    assert_eq!(audit(&pressured).outlets[0].pressure, ["p7"]);
    assert_eq!(audit(&missing).outlets[0].status, "not_at_station");
    for r in [&clear, &pressured, &missing] {
        assert_eq!(complete(r), Some(Side::A));
        assert_eq!(r.final_state.seconds_remaining, 589);
        assert!(r
            .request
            .profiles
            .as_ref()
            .unwrap()
            .members
            .iter()
            .all(|m| m.character.capabilities.iter().all(|c| c.score == 10)));
    }
    let rolls = |r: &ResultSet| {
        r.events
            .iter()
            .filter_map(|e| e.spatial_collection.as_ref())
            .map(|a| a.collector.die)
            .collect::<Vec<_>>()
    };
    assert_eq!(rolls(&clear), rolls(&pressured));
}
#[test]
fn local_f_labels_do_not_supply_universal_player_duties() {
    let mut d = draft("timely-cover");
    intent(&mut d)["legend"][1]["label"] = json!("first-back-cover");
    intent(&mut d)["legend"][3]["label"] = json!("F3-in-this-local-legend");
    let r = run(&build(d)).unwrap();
    assert_eq!(audit(&r).cover_player.as_deref(), Some("p1"));
    assert_eq!(audit(&r).decision, "pinch_permitted");
    assert_eq!(complete(&r), Some(Side::A));
}
#[test]
fn fixed_frame_reversal_preserves_permission_and_cover_time() {
    let r = case("reverse-direction");
    assert_eq!(audit(&r).cover_arrival_seconds, Some(1));
    assert_eq!(complete(&r), Some(Side::A));
    assert_eq!(
        r.events
            .iter()
            .filter_map(|e| e.territory.as_ref())
            .last()
            .unwrap()
            .band,
        zones::Band::WestEnd
    );
}
#[test]
fn goalie_outlet_pressure_is_unmapped_without_skater_substitution() {
    let mut d = draft("timely-cover");
    d["episode"]["steps"][2]["action"]["plan"]["scenes"][1]["actors"][11]["offset"] =
        json!({"x":1,"y":6});
    let r = run(&build(d)).unwrap();
    assert_eq!(audit(&r).outlets[0].status, "goalie_pressure_unmapped");
    assert_eq!(complete(&r), Some(Side::A));
}
#[test]
fn unavailable_cover_or_partner_cannot_be_assumed_replaced() {
    let q = build(draft("timely-cover"));
    let Action::ThreeZoneSequence { plan } = &q.steps[2].action else {
        panic!()
    };
    let scene = &plan.scenes[1];
    let grid = space::Plan {
        schema: "rinkcraft.access-grid.0.1".into(),
        snapshot_seconds_remaining: 593,
        basis: scene.basis.clone(),
        puck: space::Cell { x: 8, y: 0 },
        reach_cells: 0,
        collection_seconds: 1,
        open_target: 1,
        actors: scene
            .actors
            .iter()
            .map(|a| space::Located {
                player: a.player.clone(),
                cell: space::Cell {
                    x: 8 + a.offset.x,
                    y: a.offset.y,
                },
                facing: a.facing,
                intent: a.intent.clone(),
            })
            .collect(),
    };
    for missing in ["p1", "p4", "p5"] {
        let mut partial = grid.clone();
        partial.actors.retain(|a| a.player != missing);
        let a = team_intent::evaluate(&q, &partial, scene.team_intent.as_ref().unwrap()).unwrap();
        assert_eq!(a.decision, "unresolved");
        assert_eq!(a.cover_status, "named_actor_unavailable");
    }
}
#[test]
fn validation_requires_complete_legend_valid_duties_and_explicit_opt_in() {
    for kind in 0..9 {
        let mut d = draft("timely-cover");
        let t = intent(&mut d);
        match kind {
            0 => {
                t["legend"].as_array_mut().unwrap().pop();
            }
            1 => t["legend"][1]["label"] = json!("D1"),
            2 => t["legend"][1]["player"] = json!("p6"),
            3 => t["cover"]["deadline_seconds"] = json!(0),
            4 => t["cover"]["motion"]["seconds_per_cell"] = json!(0),
            5 => t["cover"]["slot_offset"]["x"] = json!(20),
            6 => t["chase"] = json!({"kind":"hold"}),
            7 => t["legend"][5]["duty"] = json!("cover"),
            _ => {
                d["schema"] = json!("rinkcraft.event-draft.0.10");
                d["episode"]["schema"] = json!("rinkcraft.event-episode.0.10");
            }
        }
        assert!(
            collection::build(serde_json::from_value(d).unwrap()).is_err(),
            "kind {kind}"
        );
    }
}
#[test]
fn unknown_runtime_branch_does_not_skip_static_validation() {
    let mut d = draft("unknown-trigger");
    intent(&mut d)["cover"]["motion"]["seconds_per_cell"] = json!(0);
    assert!(collection::build(serde_json::from_value(d).unwrap()).is_err());
}
#[test]
fn all_team_intent_cases_replay_and_leave_goal_ledger_unchanged() {
    for name in [
        "timely-cover",
        "late-cover",
        "inactive-trigger",
        "unknown-cover",
        "unknown-trigger",
        "pressured-outlet",
        "missing-outlet",
        "reverse-direction",
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
fn protocol_eleven_without_team_policy_preserves_old_recovery_events() {
    let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/fixtures/rinkcraft-transition-recovery/denied-entry-counter.draft.json");
    let q = collection::build(serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap())
        .unwrap();
    let a = run(&q).unwrap();
    let mut newer = q.clone();
    newer.schema = "rinkcraft.event-episode.0.11".into();
    let b = run(&newer).unwrap();
    assert_eq!(
        serde_json::to_value(a.events).unwrap(),
        serde_json::to_value(b.events).unwrap()
    );
}
