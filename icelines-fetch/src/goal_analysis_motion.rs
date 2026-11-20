//! Endpoint geometry only: no inferred possession, duty, speed or physical units.
use serde_json::{json, Value};

fn displacement(before: &Value, after: &Value) -> Option<Value> {
    let dx = after["x"].as_f64()? - before["x"].as_f64()?;
    let dy = after["y"].as_f64()? - before["y"].as_f64()?;
    Some(json!({"dx_native":dx,"dy_native":dy,"distance_native":dx.hypot(dy)}))
}
fn separation(a: &Value, b: &Value) -> Option<f64> {
    Some((a["x"].as_f64()? - b["x"].as_f64()?).hypot(a["y"].as_f64()? - b["y"].as_f64()?))
}
fn player(frame: &Value, id: u64) -> Option<&Value> {
    frame["observations"]
        .as_array()?
        .iter()
        .find(|o| o["player_id"].as_u64() == Some(id))
}

pub(super) fn comparisons(event: &Value) -> Value {
    let frames = event["frames"].as_array().expect("audited frames");
    let scorer = event["goal"]["scorer_id"].as_u64().expect("audited scorer");
    let rows: Vec<_> = frames.iter().enumerate().map(|(i, current)| {
        let start = i.saturating_sub(10);
        let before = &frames[start];
        let old_scorer = player(before, scorer);
        let new_scorer = player(current, scorer);
        let mut matched = vec![];
        let mut appeared = vec![];
        for now in current["observations"].as_array().expect("audited observations") {
            let id = now["player_id"].as_u64().expect("audited player");
            if let Some(old) = player(before, id) {
                let old_gap = old_scorer.and_then(|s| separation(old, s));
                let new_gap = new_scorer.and_then(|s| separation(now, s));
                matched.push(json!({"player_id":id,"team":now["team"],"goalie":now["roster"]["goalie"],
                    "displacement":displacement(old,now),
                    "scorer_distance_before_native":old_gap,"scorer_distance_after_native":new_gap,
                    "scorer_distance_change_native":old_gap.zip(new_gap).map(|(a,b)|b-a),
                    "observed_in_all_window_frames":frames[start..=i].iter().all(|f|player(f,id).is_some())}));
            } else { appeared.push(id); }
        }
        let disappeared: Vec<_> = before["observations"].as_array().expect("audited observations").iter()
            .filter_map(|o|o["player_id"].as_u64()).filter(|id|player(current,*id).is_none()).collect();
        let old_nearest = before["scorer_nearest_opposing_skater"]["player_id"].as_u64();
        let new_nearest = current["scorer_nearest_opposing_skater"]["player_id"].as_u64();
        json!({"frame":i,"reference_frame":start,"window_frames":i-start,
            "elapsed_native_ticks":current["elapsed_native_ticks"].as_f64().zip(before["elapsed_native_ticks"].as_f64()).map(|(b,a)|b-a),
            "puck_displacement":displacement(&before["puck"],&current["puck"]),
            "scorer_displacement":old_scorer.zip(new_scorer).and_then(|(a,b)|displacement(a,b)),
            "nearest_opponent_before":old_nearest,"nearest_opponent_after":new_nearest,
            "nearest_opponent_changed":old_nearest.zip(new_nearest).map(|(a,b)|a!=b),
            "matched_players":matched,"newly_observed_ids":appeared,"no_longer_observed_ids":disappeared})
    }).collect();
    json!({"schema":"icelines.goal-geometry-change.v0.1","reference_window_frames":10,
        "interpretation":"Endpoint displacement and same-player separation in native units. Not path length, speed, possession, coverage, a pass or a cause. Observation changes are not line changes; frames may include post-goal movement.",
        "frames":rows})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn observation(id: u64, x: f64, y: f64) -> Value {
        json!({"player_id":id,"x":x,"y":y,"team":"AWAY","roster":{"goalie":false}})
    }
    fn frame(t: u64, observations: Vec<Value>, nearest: u64) -> Value {
        json!({"elapsed_native_ticks":t,"puck":{"x":t as f64,"y":0.},"observations":observations,"scorer_nearest_opposing_skater":{"player_id":nearest}})
    }
    #[test]
    fn nearest_switch_does_not_substitute_for_same_player_change() {
        let event = json!({"goal":{"scorer_id":1},"frames":[
            frame(0,vec![observation(1,0.,0.),observation(2,3.,0.),observation(3,8.,0.)],2),
            frame(1,vec![observation(1,0.,4.),observation(2,6.,4.),observation(3,2.,4.)],3)]});
        let out = comparisons(&event);
        let row = &out["frames"][1];
        assert_eq!(row["nearest_opponent_changed"], true);
        assert_eq!(row["scorer_displacement"]["distance_native"], 4.);
        assert_eq!(
            row["matched_players"][1]["scorer_distance_change_native"],
            3.
        );
        assert_eq!(
            row["matched_players"][2]["scorer_distance_change_native"],
            -6.
        );
        assert_eq!(
            row["matched_players"][1]["displacement"]["distance_native"],
            5.
        );
    }
    #[test]
    fn missing_scorer_and_observation_gaps_stay_explicit() {
        let event = json!({"goal":{"scorer_id":1},"frames":[
            frame(0,vec![observation(2,1.,0.)],2),frame(1,vec![],2),
            frame(2,vec![observation(1,0.,0.),observation(2,4.,0.)],2)]});
        let out = comparisons(&event);
        let row = &out["frames"][2];
        assert!(row["scorer_displacement"].is_null());
        assert!(row["matched_players"][0]["scorer_distance_change_native"].is_null());
        assert_eq!(
            row["matched_players"][0]["observed_in_all_window_frames"],
            false
        );
        assert_eq!(row["newly_observed_ids"], json!([1]));
        assert_eq!(out["frames"][1]["no_longer_observed_ids"], json!([2]));
        assert_eq!(out["frames"][0]["window_frames"], 0);
    }
    #[test]
    fn window_is_bounded_and_geometry_is_translation_invariant() {
        let frames: Vec<_> = (0..12)
            .map(|i| {
                frame(
                    i,
                    vec![observation(1, i as f64, 0.), observation(2, 20., 4.)],
                    2,
                )
            })
            .collect();
        let mut event = json!({"goal":{"scorer_id":1},"frames":frames});
        let original = comparisons(&event);
        assert_eq!(original["frames"][11]["reference_frame"], 1);
        assert_eq!(original["frames"][11]["window_frames"], 10);
        assert_eq!(
            original["frames"][11]["scorer_displacement"]["distance_native"],
            10.
        );
        for frame in event["frames"].as_array_mut().unwrap() {
            for point in frame["observations"].as_array_mut().unwrap() {
                point["x"] = json!(point["x"].as_f64().unwrap() + 100.);
                point["y"] = json!(point["y"].as_f64().unwrap() - 70.);
            }
        }
        assert_eq!(original, comparisons(&event));
    }
}
