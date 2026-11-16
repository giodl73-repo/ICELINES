//! Same-identity endpoint displacement, not path length, speed or tactical duty.
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn point(v: &Value) -> Option<(f64, f64)> {
    v["x"].as_f64().zip(v["y"].as_f64())
}
fn observations(frame: &Value) -> &[Value] {
    frame["observations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}
fn distance(a: (f64, f64), b: (f64, f64), units: f64) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1) / units
}
fn native_point(v: &Value) -> Value {
    point(v)
        .map(|(x, y)| json!({"x":x,"y":y}))
        .unwrap_or(Value::Null)
}

/// Compare raw observed endpoints for the scorer and every opposing skater.
pub fn assess(event: &Value, geometry: &Value, start: usize, end: usize) -> Value {
    let mut result = json!({"schema":"icelines.goal-marker-motion.v0.1","status":"unavailable",
        "from_frame_index":start,"to_frame_index":end,"markers":[],
        "interpretation":"Same-identity observed endpoint displacements only, not travelled distance, speed, acceleration, reaction or causal attribution. Intermediate paths are not reconstructed or verified. Separation change compares the same scorer/opponent pair; nearest-opponent identity may change elsewhere. Derived rink scale, contact, active occupancy, called duty, height and physical tracking accuracy remain unverified."});
    let frames = event["frames"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    if start > end || end >= frames.len() {
        result["reason"] = json!("invalid_sample_range");
        return result;
    }
    // Keep source points and unknowns; do not interpolate or infer contacts.
    result["synchronized_scene"] = crate::goal_sample_scene::assess(event,start,end);
    result["puck_samples_native"] = json!(frames[start..=end].iter().enumerate().map(|(i,frame)| {
        let marker=native_point(&frame["puck"]);
        json!({"frame_index":start+i,"elapsed_native_ticks":frame["elapsed_native_ticks"],
            "status":if marker.is_null(){"coordinates_unavailable"}else{"observed"},"marker":marker})
    }).collect::<Vec<_>>());
    let Some(units) = geometry["rink_reference"]["native_units_per_foot_derived"]
        .as_f64()
        .filter(|u| u.is_finite() && *u > 0.)
    else {
        result["reason"] = json!("derived_scale_unavailable");
        return result;
    };
    let Some(scorer) = event["goal"]["scorer_id"].as_u64() else {
        result["reason"] = json!("scorer_identity_unavailable");
        return result;
    };
    let Some(team) = event["goal"]["team_abbrev"]
        .as_str()
        .filter(|t| !t.is_empty())
    else {
        result["reason"] = json!("scoring_team_unavailable");
        return result;
    };
    let a = observations(&frames[start]);
    let b = observations(&frames[end]);
    let s0 = a.iter().find(|o| o["player_id"].as_u64() == Some(scorer));
    let s1 = b.iter().find(|o| o["player_id"].as_u64() == Some(scorer));
    let Some((s0, s1)) = s0.zip(s1) else {
        result["reason"] = json!("scorer_endpoint_missing");
        return result;
    };
    if s0["team"].as_str() != Some(team) || s1["team"].as_str() != Some(team) {
        result["reason"] = json!("scorer_identity_context_unverified");
        return result;
    }
    let Some((p0, p1)) = point(s0).zip(point(s1)) else {
        result["reason"] = json!("scorer_coordinates_unavailable");
        return result;
    };
    result["status"] = json!("observed_endpoint_comparison");
    result["scorer_id"] = json!(scorer);
    result["scorer_marker_displacement_feet_derived"] = json!(distance(p0, p1, units));
    result["scorer_marker_endpoints_native"] =
        json!({"start":native_point(s0),"end":native_point(s1)});
    result["puck_endpoints_native"] = json!({"start":native_point(&frames[start]["puck"]),"end":native_point(&frames[end]["puck"])});
    result["target_goal_reference_native"] = if geometry["status"] == "available" {
        native_point(&geometry["target_goal_reference"])
    } else {
        Value::Null
    };
    let ids: BTreeSet<_> = a
        .iter()
        .chain(b)
        .filter_map(|o| o["player_id"].as_u64())
        .collect();
    let rows:Vec<_>=ids.into_iter().filter_map(|id| {
        let x=a.iter().find(|o|o["player_id"].as_u64()==Some(id));let y=b.iter().find(|o|o["player_id"].as_u64()==Some(id));
        if x.into_iter().chain(y).all(|v|v["team"].as_str()==Some(team)) || x.into_iter().chain(y).all(|v|v["roster"]["goalie"]==true) {return None}
        let mut row=json!({"player_id":id,"name":x.or(y).unwrap()["roster"]["name"],"status":"available"});
        let Some((x,y))=x.zip(y) else {row["status"]=json!("marker_endpoint_missing");return Some(row)};
        if x["team"].as_str().is_none_or(|t|t.is_empty()) || y["team"].as_str().is_none_or(|t|t.is_empty()) || x["roster"]["goalie"].as_bool().is_none() || y["roster"]["goalie"].as_bool().is_none() {row["status"]=json!("identity_context_unavailable");return Some(row)}
        if x["team"]!=y["team"] || x["roster"]["goalie"]!=y["roster"]["goalie"] {row["status"]=json!("identity_context_conflict");return Some(row)}
        let Some((q0,q1))=point(x).zip(point(y)) else {row["status"]=json!("marker_coordinates_unavailable");return Some(row)};
        row["marker_endpoints_native"]=json!({"start":native_point(x),"end":native_point(y)});
        let d0=distance(p0,q0,units);let d1=distance(p1,q1,units);
        row["marker_displacement_feet_derived"]=json!(distance(q0,q1,units));
        row["scorer_separation_start_feet_derived"]=json!(d0);row["scorer_separation_end_feet_derived"]=json!(d1);
        row["same_pair_separation_change_feet_derived"]=json!(d1-d0);Some(row)
    }).collect();
    result["markers"] = json!(rows);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (Value, Value) {
        let s = |x| json!({"player_id":1,"team":"A","x":x,"y":0.,"roster":{"goalie":false}});
        let o = |x| json!({"player_id":2,"team":"B","x":x,"y":0.,"roster":{"goalie":false}});
        (
            json!({"goal":{"scorer_id":1,"team_abbrev":"A"},"frames":[{"observations":[s(0.),o(10.)]},{"observations":[s(3.),o(8.)]}]}),
            json!({"rink_reference":{"native_units_per_foot_derived":1.}}),
        )
    }
    #[test]
    fn both_markers_move_and_same_pair_closes() {
        let (e, g) = inputs();
        let r = assess(&e, &g, 0, 1);
        let o = &r["markers"][0];
        assert_eq!(r["scorer_marker_displacement_feet_derived"], 3.);
        assert_eq!(o["marker_displacement_feet_derived"], 2.);
        assert_eq!(o["scorer_separation_start_feet_derived"], 10.);
        assert_eq!(o["scorer_separation_end_feet_derived"], 5.);
        assert_eq!(o["same_pair_separation_change_feet_derived"], -5.);
        assert_eq!(
            r["scorer_marker_endpoints_native"]["start"],
            json!({"x":0.,"y":0.})
        );
        assert_eq!(o["marker_endpoints_native"]["end"], json!({"x":8.,"y":0.}));
        assert!(r["puck_endpoints_native"]["start"].is_null());
        assert!(r["target_goal_reference_native"].is_null());
    }
    #[test]
    fn diagram_coordinates_preserve_samples_and_unknowns() {
        let (mut e, mut g) = inputs();
        e["frames"][0]["puck"] = json!({"x":4.,"y":5.});
        g["status"] = json!("available");
        g["target_goal_reference"] = json!({"x":132.,"y":510.});
        let r = assess(&e, &g, 0, 1);
        assert_eq!(r["puck_endpoints_native"]["start"], json!({"x":4.,"y":5.}));
        assert!(r["puck_endpoints_native"]["end"].is_null());
        assert_eq!(
            r["target_goal_reference_native"],
            g["target_goal_reference"]
        );
    }
    #[test]
    fn sampled_points_keep_detours_unknowns_and_single_sample_ranges() {
        let (mut e, g) = inputs();
        let mut middle = e["frames"][0].clone();
        e["frames"][0]["puck"] = json!({"x":0.,"y":0.});
        middle["puck"] = json!({"x":50.,"y":80.});
        middle["elapsed_native_ticks"] = json!(17);
        e["frames"].as_array_mut().unwrap().insert(1, middle);
        let r = assess(&e, &g, 0, 2);
        let samples = r["puck_samples_native"].as_array().unwrap();
        assert_eq!(samples.len(), 3);
        assert_eq!(samples[1]["marker"], json!({"x":50.,"y":80.}));
        assert_eq!(samples[1]["elapsed_native_ticks"], 17);
        assert_eq!(samples[2]["status"], "coordinates_unavailable");
        assert_eq!(
            assess(&e, &g, 1, 1)["puck_samples_native"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
    #[test]
    fn joint_translation_preserves_gap() {
        let (mut e, g) = inputs();
        e["frames"][1]["observations"][1]["x"] = json!(13.);
        assert_eq!(
            assess(&e, &g, 0, 1)["markers"][0]["same_pair_separation_change_feet_derived"],
            0.
        );
    }
    #[test]
    fn missing_actor_stays_explicit_and_goalie_excluded() {
        let (mut e, g) = inputs();
        e["frames"][1]["observations"].as_array_mut().unwrap().pop();
        assert_eq!(
            assess(&e, &g, 0, 1)["markers"][0]["status"],
            "marker_endpoint_missing"
        );
        e["frames"][0]["observations"][1]["roster"]["goalie"] = json!(true);
        assert!(assess(&e, &g, 0, 1)["markers"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    #[test]
    fn missing_scorer_unknown_context_and_bad_range_are_distinct() {
        let (mut e, g) = inputs();
        assert_eq!(assess(&e, &g, 1, 0)["reason"], "invalid_sample_range");
        e["frames"][1]["observations"][0]["team"] = Value::Null;
        assert_eq!(
            assess(&e, &g, 0, 1)["reason"],
            "scorer_identity_context_unverified"
        );
        e["frames"][1]["observations"]
            .as_array_mut()
            .unwrap()
            .remove(0);
        assert_eq!(assess(&e, &g, 0, 1)["reason"], "scorer_endpoint_missing");
    }
    #[test]
    fn one_sample_is_zero_displacement_not_an_inferred_path() {
        let (e, g) = inputs();
        let r = assess(&e, &g, 0, 0);
        assert_eq!(r["scorer_marker_displacement_feet_derived"], 0.);
        assert_eq!(r["markers"][0]["marker_displacement_feet_derived"], 0.);
    }
    #[test]
    fn derived_scale_and_unknown_marker_context_remain_explicit() {
        let (mut e, mut g) = inputs();
        g["rink_reference"]["native_units_per_foot_derived"] = json!(2.);
        assert_eq!(
            assess(&e, &g, 0, 1)["scorer_marker_displacement_feet_derived"],
            1.5
        );
        e["frames"][1]["observations"][1]["roster"]["goalie"] = Value::Null;
        assert_eq!(
            assess(&e, &g, 0, 1)["markers"][0]["status"],
            "identity_context_unavailable"
        );
        g["rink_reference"] = Value::Null;
        assert_eq!(assess(&e, &g, 0, 1)["reason"], "derived_scale_unavailable");
    }
    #[test]
    fn intermediate_detour_is_not_endpoint_displacement() {
        let (mut e, g) = inputs();
        let mut middle = e["frames"][0].clone();
        middle["observations"][0]["y"] = json!(100.);
        e["frames"].as_array_mut().unwrap().insert(1, middle);
        assert_eq!(
            assess(&e, &g, 0, 2)["scorer_marker_displacement_feet_derived"],
            3.
        );
    }
}
