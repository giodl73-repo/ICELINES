use super::*;
fn source() -> (GoalTarget, gv::GoalVisualizerBundle, Value, Value, Value) {
    let t = GoalTarget::new(2019020001, 10).unwrap();
    let url = "https://wsr.nhle.com/sprites/20192020/2019020001/ev10.json";
    let landing = json!({"id":t.game_id,"gameDate":"2019-10-02","homeTeam":{"abbrev":"HOME"},"awayTeam":{"abbrev":"AWAY"},"summary":{"scoring":[{"periodDescriptor":{"number":1,"periodType":"REG"},"goals":[{"eventId":10,"timeInPeriod":"01:02","playerId":101,"name":{"default":"Test scorer"},"teamAbbrev":{"default":"HOME"},"strength":"sh","shotType":"tip-in","goalModifier":"none","pptReplayUrl":url}]}]}});
    let boxscore = json!({"id":t.game_id,"homeTeam":{"abbrev":"HOME"},"awayTeam":{"abbrev":"AWAY"},"playerByGameStats":{"homeTeam":{"forwards":[{"playerId":101,"name":{"default":"Test scorer"},"position":"C","sweaterNumber":9}],"goalies":[{"playerId":102,"name":{"default":"Home goalie"},"position":"G","sweaterNumber":1}]},"awayTeam":{"defense":[{"playerId":201,"name":{"default":"Test defender"},"position":"D","sweaterNumber":3}],"goalies":[{"playerId":202,"name":{"default":"Away goalie"},"position":"G","sweaterNumber":30}]}}});
    let pbp = json!({"id":t.game_id,"plays":[{"eventId":10,"typeDescKey":"goal","details":{"scoringPlayerId":101,"goalieInNetId":202}}]});
    let obj = |id, player, x, team, number| json!({"id":id,"playerId":player,"x":x,"y":2.,"teamId":team,"teamAbbrev":if team==3{"HOME"}else{"AWAY"},"sweaterNumber":number});
    let raw = json!([{"timeStamp":100,"onIce":{"1":{"id":1,"x":1.,"y":2.},"102":obj(102,102,10.,3,1),"201":obj(201,201,4.,13,3),"202":obj(202,202,-1.,13,30)}},{"timeStamp":101,"onIce":{"1":{"id":1,"x":2.,"y":2.},"101":obj(101,101,2.,3,9),"102":obj(102,102,10.,3,1),"201":obj(201,201,5.,13,3),"202":obj(202,202,-1.,13,30)}}]);
    let goal = gv::discover_goals(&landing).unwrap().remove(0);
    let mut bundle =
        gv::GoalVisualizerBundle::new(t.game_id, chrono::DateTime::from_timestamp(0, 0).unwrap());
    bundle.upsert_event(gv::build_event(goal, &raw, "0".repeat(64)).unwrap());
    (t, bundle, landing, boxscore, pbp)
}
#[test]
fn evidence_keeps_missing_observations_unknown_and_groups_no_possession() {
    let (t, b, l, x, p) = source();
    let r = analyze_sources(t, &b, &l, &x, &p).unwrap();
    let e = &r["audit"]["events"][0];
    assert_eq!(r["goal_geometry"]["status"], "unavailable");
    assert_eq!(r["geometry_changes"]["frames"].as_array().unwrap().len(), 2);
    assert!(r["geometry_changes"]["frames"][1]["scorer_displacement"].is_null());
    assert!(e["frames"][0]["scorer_nearest_opposing_skater"].is_null());
    assert_eq!(
        e["frames"][1]["scorer_nearest_opposing_skater"]["distance_native"],
        3.
    );
    assert!(
        e["coordinate_units"].is_null()
            && e["timestamp_seconds_per_tick"].is_null()
            && e["exact_goal_frame"].is_null()
    );
    assert_eq!(r["assignment_assessment"]["status"], "unresolved");
    assert_eq!(r["summary"]["goalkeeper_context"], "goalie_recorded");
    for run in r["nearest_observation_runs"].as_array().unwrap() {
        assert!(run["interpretation"]
            .as_str()
            .unwrap()
            .contains("not possession"));
    }
    assert!(r["questions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|q| q.as_str().unwrap().contains("shorthanded")));
}
#[test]
fn goal_reference_is_additive_and_preserves_raw_units() {
    let (t, mut b, l, x, p) = source();
    b.events[0].goal.home_team_defending_side = Some("left".into());
    let r = analyze_sources(t, &b, &l, &x, &p).unwrap();
    assert_eq!(r["goal_geometry"]["status"], "available");
    assert_eq!(r["goal_geometry"]["target_goal_reference"]["x"], 2268.);
    assert_eq!(r["goal_geometry"]["frames"].as_array().unwrap().len(), 2);
    assert!(r["audit"]["events"][0]["coordinate_units"].is_null());
}
#[test]
fn keeper_absence_is_not_automatically_empty_net() {
    let (t, b, mut l, x, mut p) = source();
    p["plays"][0]["details"]
        .as_object_mut()
        .unwrap()
        .remove("goalieInNetId");
    assert_eq!(
        analyze_sources(t, &b, &l, &x, &p).unwrap()["summary"]["goalkeeper_context"],
        "unknown"
    );
    l["summary"]["scoring"][0]["goals"][0]["goalModifier"] = json!("empty-net");
    assert_eq!(
        analyze_sources(t, &b, &l, &x, &p).unwrap()["summary"]["goalkeeper_context"],
        "empty_net_recorded"
    );
}
#[test]
fn target_source_game_scorer_and_capture_count_must_agree() {
    for kind in 0..6 {
        let (t, mut b, mut l, mut x, mut p) = source();
        match kind {
            0 => x["id"] = json!(2019020002),
            1 => p["plays"][0]["details"]["scoringPlayerId"] = json!(999),
            2 => l["summary"]["scoring"][0]["goals"][0]["playerId"] = json!(999),
            3 => b.events[0].source_url = "https://evil.test/data.json".into(),
            4 => b.events[0].frame_count = 99,
            _ => b.events[0].goal.event_id = 11,
        };
        assert!(analyze_sources(t, &b, &l, &x, &p).is_err(), "kind {kind}");
    }
}
#[tokio::test]
async fn offline_import_replays_and_seeds_cached_web_discovery_without_network() {
    let (t, b, l, x, p) = source();
    let input = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    for (kind, v) in [
        ("bundle", serde_json::to_value(b).unwrap()),
        ("landing", l),
        ("boxscore", x),
        ("play-by-play", p),
    ] {
        crate::atomic_write::write_json_atomic(&source_path(input.path(), t.game_id, kind), &v)
            .unwrap();
    }
    let a = prepare(t, root.path(), false, Some(input.path()))
        .await
        .unwrap();
    let b = prepare(t, root.path(), false, None).await.unwrap();
    assert_eq!(a["audit"], b["audit"]);
    assert_eq!(a["source_capture"]["origin"], "explicit_offline_capture");
    assert_eq!(b["source_capture"]["origin"], "stored_capture");
    for hash in b["source_capture"]["inputs"].as_array().unwrap() {
        let file = source_path(
            &root
                .path()
                .join("goal_analysis")
                .join(t.game_id.to_string())
                .join(t.event_id.to_string()),
            t.game_id,
            hash["kind"].as_str().unwrap(),
        );
        assert_eq!(
            hash["sha256"],
            gv::sha256_hex(&std::fs::read(file).unwrap())
        );
    }
    assert_eq!(
        discover_game(t.game_id, root.path(), false).await.unwrap()["id"],
        t.game_id
    );
}
#[tokio::test]
async fn concurrent_goal_imports_leave_one_valid_capture() {
    let (t, b, l, x, p) = source();
    let input = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    for (kind, v) in [
        ("bundle", serde_json::to_value(b).unwrap()),
        ("landing", l),
        ("boxscore", x),
        ("play-by-play", p),
    ] {
        crate::atomic_write::write_json_atomic(&source_path(input.path(), t.game_id, kind), &v)
            .unwrap();
    }
    let (a, b) = tokio::join!(
        prepare(t, root.path(), false, Some(input.path())),
        prepare(t, root.path(), false, Some(input.path()))
    );
    assert_eq!(a.unwrap()["audit"], b.unwrap()["audit"]);
}
