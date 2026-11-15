use super::*;
use serde_json::{json, Value};
#[path = "collection_tests.rs"]
mod collection_tests;
#[path = "followup_tests.rs"]
mod followup_tests;
#[path = "pass_context_tests.rs"]
mod pass_context_tests;
#[path = "passing_tests.rs"]
mod passing_tests;
#[path = "rebound_tests.rs"]
mod rebound_tests;
#[path = "shot_tests.rs"]
mod shot_tests;
#[path = "space_tests.rs"]
mod space_tests;
#[path = "zones_tests.rs"]
mod zones_tests;
#[path = "recovery_tests.rs"]
mod recovery_tests;
#[path = "team_intent_tests.rs"]
mod team_intent_tests;
fn request(actions: Vec<Value>) -> Request {
    serde_json::from_value(json!({"schema":"rinkcraft.event-episode.0.1","seed":17,
        "players":(0..12).map(|i| json!({"id":format!("p{i}"),"side":if i<6 {"a"} else {"b"},"goalie":i==5||i==11})).collect::<Vec<_>>(),
        "controller":"p0","seconds_remaining":600,
        "steps":actions.into_iter().map(|action| json!({"seconds":if action["kind"]=="restart" {0} else {1},"action":action})).collect::<Vec<_>>() })).unwrap()
}
fn release(shot: bool) -> Value {
    json!({"kind":"release","actor":"p0","shot":shot})
}
fn flight(contact: Value) -> Value {
    json!({"kind":"flight","choices":[{"weight":10000,"outcome":contact}]})
}
fn collect(player: &str) -> Value {
    json!({"kind":"collect","choices":[{"weight":10000,"outcome":{"kind":"control","player":player}}]})
}
fn foul() -> Value {
    json!({"kind":"challenge","actor":"p6","victim":"p0","choices":[{"weight":10000,"outcome":{"dislodge":true,"ruling":{"kind":"called_minor","infraction":"tripping; ordinary minor adjudicated"}}}]})
}
fn service() -> Vec<Value> {
    vec![
        foul(),
        collect("p6"),
        json!({"kind":"restart","winner":"p0"}),
    ]
}
fn kinds(r: &ResultSet) -> Vec<&str> {
    r.events.iter().map(|e| e.kind.as_str()).collect()
}

#[test]
fn post_tip_goal_is_one_attempt_and_one_goal() {
    let r = run(&request(vec![
        release(true),
        flight(json!({"kind":"post"})),
        flight(json!({"kind":"deflect","player":"p1"})),
        flight(json!({"kind":"goal"})),
    ]))
    .unwrap();
    assert_eq!(r.final_state.score, [1, 0]);
    assert_eq!(r.goals.len(), 1);
    assert_eq!(r.goals[0].attempt, 1);
    assert_eq!(r.final_state.last_touch.as_deref(), Some("p1"));
    assert_eq!(r.final_state.last_deliberate.as_deref(), Some("p0"));
    assert!(r
        .events
        .iter()
        .filter(|e| e.kind == "post_contact" || e.kind == "player_deflection")
        .all(|e| e.attempt == Some(1)));
}
#[test]
fn rebound_is_loose_until_collected_then_new_attempt_links_parent() {
    let r = run(&request(vec![
        release(true),
        flight(json!({"kind":"goalie","player":"p11","hold":false})),
        collect("p0"),
        release(true),
        flight(json!({"kind":"goal"})),
    ]))
    .unwrap();
    assert_eq!(
        r.events
            .iter()
            .find(|e| e.kind == "goalie_contact")
            .unwrap()
            .state
            .puck,
        Puck::Loose
    );
    let shot = r
        .events
        .iter()
        .filter(|e| e.kind == "shot_released")
        .last()
        .unwrap();
    assert_eq!((shot.attempt, shot.parent_attempt), (Some(2), Some(1)));
    assert_eq!(r.goals[0].attempt, 2);
}
#[test]
fn hold_whistles_and_requires_restart() {
    let mut q = request(vec![
        release(true),
        flight(json!({"kind":"goalie","player":"p11","hold":true})),
    ]);
    assert_eq!(run(&q).unwrap().final_state.puck, Puck::Dead);
    q.steps.push(Step {
        seconds: 1,
        action: Action::Wait,
    });
    assert!(run(&q).unwrap_err().contains("after whistle"));
}
#[test]
fn delayed_call_does_not_award_control_to_challenger() {
    let r = run(&request(vec![foul()])).unwrap();
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert!(r.final_state.penalty.as_ref().unwrap().delayed);
    assert_eq!(r.final_state.active.len(), 12);
    assert_eq!(r.final_state.penalty.as_ref().unwrap().victim, "p0");
}
#[test]
fn offending_control_stops_play_then_variable_manpower_restarts() {
    let r = run(&request(service())).unwrap();
    assert_eq!(r.final_state.active.len(), 11);
    assert!(!r.final_state.active.contains(&"p6".into()));
    assert!(!r.final_state.penalty.unwrap().delayed);
    assert!(kinds(&run(&request(service())).unwrap()).contains(&"minor_service_started"));
}
#[test]
fn pending_minor_washed_on_nonoffending_goal() {
    let r = run(&request(vec![
        foul(),
        collect("p0"),
        release(true),
        flight(json!({"kind":"goal"})),
    ]))
    .unwrap();
    assert!(r.final_state.penalty.is_none());
    assert_eq!(r.final_state.active.len(), 12);
    assert!(kinds(&r).contains(&"pending_minor_washed"));
}
#[test]
fn powerplay_goal_releases_single_serving_minor() {
    let mut steps = service();
    steps.extend([release(true), flight(json!({"kind":"goal"}))]);
    let r = run(&request(steps)).unwrap();
    assert_eq!(r.final_state.active.len(), 12);
    assert!(r.final_state.penalty.is_none());
    assert!(kinds(&r).contains(&"minor_ended_by_goal"));
}
#[test]
fn penalty_expires_only_on_live_clock() {
    let mut q = request(service());
    q.steps.push(Step {
        seconds: 119,
        action: Action::Wait,
    });
    let r = run(&q).unwrap();
    assert_eq!(r.final_state.penalty.unwrap().remaining, 1);
    q.steps.push(Step {
        seconds: 1,
        action: Action::Wait,
    });
    let r = run(&q).unwrap();
    assert!(r.final_state.penalty.is_none());
    assert_eq!(r.final_state.active.len(), 12);
}
#[test]
fn penalized_player_is_not_a_candidate_even_if_not_selected() {
    let mut steps = service();
    steps.push(release(false));
    steps.push(json!({"kind":"flight","choices":[{"weight":9999,"outcome":{"kind":"loose"}},{"weight":1,"outcome":{"kind":"receive","player":"p6"}}]}));
    assert!(run(&request(steps))
        .unwrap_err()
        .contains("inactive player"));
}
#[test]
fn legal_disruption_and_foul_are_separate_from_puck_effect() {
    let mut action = foul();
    action["choices"][0]["outcome"]["ruling"] = json!({"kind":"legal"});
    let r = run(&request(vec![action])).unwrap();
    assert_eq!(r.final_state.puck, Puck::Loose);
    assert!(r.final_state.penalty.is_none());
    assert!(kinds(&r).contains(&"contact_legal"));
}
#[test]
fn icing_stops_unresolved_without_fabricated_goal_or_restart() {
    let r = run(&request(vec![
        release(false),
        flight(json!({"kind":"icing_candidate"})),
        flight(json!({"kind":"goal"})),
    ]))
    .unwrap();
    assert!(r.final_state.unresolved.unwrap().contains("hybrid icing"));
    assert!(r.goals.is_empty());
    assert!(r.final_state.restart.is_none());
}
#[test]
fn direct_and_deflected_exits_remain_distinct_and_unknown_law_stops() {
    for deflected in [true, false] {
        let r=run(&request(vec![release(false),flight(json!({"kind":"out","deflected":deflected,"ruling":{"kind":"unresolved","reason":"Rule 63.2 boundary/origin exceptions not adjudicated"}}))])).unwrap();
        assert!(r.final_state.unresolved.is_some());
        assert!(kinds(&r).contains(&if deflected {
            "out_deflected"
        } else {
            "out_direct"
        }));
        assert!(r.final_state.penalty.is_none());
    }
}
#[test]
fn declared_ordinary_out_has_explicit_restart() {
    let r=run(&request(vec![release(false),flight(json!({"kind":"out","deflected":true,"ruling":{"kind":"ordinary","restart":"declared_neutral_dot"}}))])).unwrap();
    assert_eq!(r.final_state.puck, Puck::Dead);
    assert_eq!(
        r.final_state.restart.as_deref(),
        Some("declared_neutral_dot")
    );
}
#[test]
fn competing_recipients_replay_and_conserve_puck_clock_score_and_ids() {
    let q = request(vec![
        release(false),
        json!({"kind":"flight","choices":[
        {"weight":2500,"outcome":{"kind":"receive","player":"p1"}},
        {"weight":2500,"outcome":{"kind":"receive","player":"p6"}},
        {"weight":2500,"outcome":{"kind":"receive","player":"p0"}},
        {"weight":2500,"outcome":{"kind":"loose"}}]}),
    ]);
    for seed in 0..100 {
        let mut q = q.clone();
        q.seed = seed;
        let r = run(&q).unwrap();
        assert_eq!(
            serde_json::to_vec(&r).unwrap(),
            serde_json::to_vec(&run(&q).unwrap()).unwrap()
        );
        assert_eq!(r.final_state.seconds_remaining, 598);
        assert_eq!(r.final_state.score, [0, 0]);
        for (i, e) in r.events.iter().enumerate() {
            assert_eq!(e.id, i as u32 + 1);
            assert_eq!(e.cause, if i == 0 { None } else { Some(i as u32) });
            if let Puck::Controlled { player } = &e.state.puck {
                assert!(e.state.active.contains(player));
            }
        }
    }
}
#[test]
fn malformed_or_ineligible_distributions_rejected() {
    for c in [
        json!([{"weight":9999,"outcome":{"kind":"loose"}}]),
        json!([{"weight":5000,"outcome":{"kind":"loose"}},{"weight":5000,"outcome":{"kind":"loose"}}]),
        json!([{"weight":10000,"outcome":{"kind":"receive","player":"bench"}}]),
    ] {
        assert!(run(&request(vec![
            release(false),
            json!({"kind":"flight","choices":c})
        ]))
        .is_err());
    }
    assert!(run(&request(vec![
        release(false),
        flight(json!({"kind":"goal"}))
    ]))
    .is_err());
    assert!(run(&request(vec![
        release(true),
        flight(json!({"kind":"goalie","player":"p5","hold":false}))
    ]))
    .is_err());
}
#[test]
fn invalid_clock_lineup_and_unsupported_penalty_are_explicit() {
    let mut q = request(vec![json!({"kind":"wait"})]);
    q.steps[0].seconds = 0;
    assert!(run(&q).is_err());
    q.steps[0].seconds = 600;
    assert!(run(&q).unwrap().final_state.unresolved.is_some());
    q.players[1].id = "p0".into();
    assert!(run(&q).is_err());
    let mut action = foul();
    action["choices"][0]["outcome"]["ruling"] =
        json!({"kind":"unresolved","reason":"potential penalty shot"});
    let r = run(&request(vec![action])).unwrap();
    assert_eq!(
        r.final_state.unresolved.as_deref(),
        Some("potential penalty shot")
    );
    assert!(r.final_state.penalty.is_none());
}

#[test]
fn expiration_inside_long_step_has_exact_event_time() {
    let mut q = request(service());
    q.steps.push(Step {
        seconds: 125,
        action: Action::Wait,
    });
    let r = run(&q).unwrap();
    let expired = r.events.iter().find(|e| e.kind == "minor_expired").unwrap();
    assert_eq!(expired.state.seconds_remaining, 478);
    assert_eq!(r.final_state.seconds_remaining, 473);
}
#[test]
fn short_handed_goal_does_not_release_minor() {
    let mut steps = service();
    steps.push(release(false));
    steps.extend([
        flight(json!({"kind":"receive","player":"p7"})),
        json!({"kind":"release","actor":"p7","shot":true}),
        flight(json!({"kind":"goal"})),
    ]);
    let r = run(&request(steps)).unwrap();
    assert_eq!(r.final_state.score, [0, 1]);
    assert!(r.final_state.penalty.is_some());
    assert_eq!(r.final_state.active.len(), 11);
}
#[test]
fn successive_goals_reconcile_and_have_unique_attempts() {
    let r = run(&request(vec![
        release(true),
        flight(json!({"kind":"goal"})),
        json!({"kind":"restart","winner":"p7"}),
        json!({"kind":"release","actor":"p7","shot":true}),
        flight(json!({"kind":"goal"})),
    ]))
    .unwrap();
    let mut tally = [0, 0];
    let mut attempts = BTreeSet::new();
    for goal in &r.goals {
        tally[goal.side.index()] += 1;
        assert!(attempts.insert(goal.attempt));
        assert_eq!(r.events[(goal.event - 1) as usize].kind, "goal");
    }
    assert_eq!(tally, [1, 1]);
    assert_eq!(r.final_state.score, tally);
}
#[test]
fn second_minor_and_synthetic_goalie_are_rejected() {
    let mut steps = service();
    let mut second = foul();
    second["actor"] = json!("p7");
    steps.push(second);
    assert!(run(&request(steps)).unwrap_err().contains("one minor"));
    assert!(run(&request(vec![
        release(true),
        flight(json!({"kind":"goalie","player":"p7","hold":false}))
    ]))
    .is_err());
}
