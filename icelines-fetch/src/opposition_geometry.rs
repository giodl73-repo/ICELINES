//! Adjacent-sample opponent geometry; never possession, contact or coverage duty.
use serde_json::{json, Value};
use std::collections::BTreeSet;

fn point(v: &Value) -> Option<(f64, f64)> {
    v["x"].as_f64().zip(v["y"].as_f64())
}
fn separation(a: &Value, b: &Value, units: f64) -> Option<f64> {
    point(a)
        .zip(point(b))
        .map(|((x, y), (u, v))| (x - u).hypot(y - v) / units)
}
fn observations(frame: &Value) -> &[Value] {
    frame["observations"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

pub fn assess(event: &Value, geometry: &Value) -> Value {
    let Some(units) = geometry["rink_reference"]["native_units_per_foot_derived"]
        .as_f64()
        .filter(|u| u.is_finite() && *u > 0.)
    else {
        return json!({"schema":"icelines.opposition-geometry.v0.1","status":"unavailable","reason":"Derived scale unavailable.","intervals":[]});
    };
    let Some(team) = event["goal"]["team_abbrev"]
        .as_str()
        .filter(|s| !s.is_empty())
    else {
        return json!({"schema":"icelines.opposition-geometry.v0.1","status":"unavailable","reason":"Scoring team unknown.","intervals":[]});
    };
    let scorer = event["goal"]["scorer_id"].as_u64();
    let frames = event["frames"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let intervals: Vec<_> = frames
        .windows(2)
        .enumerate()
        .map(|(i, pair)| {
            let a = observations(&pair[0]);
            let b = observations(&pair[1]);
            // Include the union so a missing endpoint never silently removes an actor.
            let ids: BTreeSet<_> = a
                .iter()
                .chain(b)
                .filter_map(|o| o["player_id"].as_u64())
                .collect();
            let receiver0 = a
                .iter()
                .find(|o| o["player_id"].as_u64() == scorer && scorer.is_some());
            let receiver1 = b
                .iter()
                .find(|o| o["player_id"].as_u64() == scorer && scorer.is_some());
            let markers: Vec<_> = ids
                .into_iter()
                .filter_map(|id| {
                    let x = a.iter().find(|o| o["player_id"].as_u64() == Some(id));
                    let y = b.iter().find(|o| o["player_id"].as_u64() == Some(id));
                    let o = x.or(y).unwrap();
                    if x.into_iter()
                        .chain(y)
                        .all(|v| v["team"].as_str() == Some(team))
                        || x.into_iter()
                            .chain(y)
                            .all(|v| v["roster"]["goalie"] == true)
                    {
                        return None;
                    }
                    let mut row =
                        json!({"player_id":id,"name":o["roster"]["name"],"status":"available"});
                    let Some((x, y)) = x.zip(y) else {
                        row["status"] = json!("marker_endpoint_missing");
                        return Some(row);
                    };
                    if x["team"].as_str().is_none()
                        || y["team"].as_str().is_none()
                        || x["roster"]["goalie"].as_bool().is_none()
                        || y["roster"]["goalie"].as_bool().is_none()
                    {
                        row["status"] = json!("identity_context_unavailable");
                        return Some(row);
                    }
                    if x["team"] != y["team"] || x["roster"]["goalie"] != y["roster"]["goalie"] {
                        row["status"] = json!("identity_context_conflict");
                        return Some(row);
                    }
                    row["scorer_marker_separation_start_feet_derived"] =
                        json!(receiver0.and_then(|s| separation(x, s, units)));
                    row["scorer_marker_separation_end_feet_derived"] =
                        json!(receiver1.and_then(|s| separation(y, s, units)));
                    let coords = point(&pair[0]["puck"])
                        .zip(point(&pair[1]["puck"]))
                        .zip(point(x).zip(point(y)));
                    let Some((((px, py), (qx, qy)), ((ax, ay), (bx, by)))) = coords else {
                        row["status"] = json!("coordinates_unavailable");
                        return Some(row);
                    };
                    let (rx, ry) = (px - ax, py - ay);
                    let (vx, vy) = (qx - px - bx + ax, qy - py - by + ay);
                    let v2 = vx * vx + vy * vy;
                    let t = if v2 == 0. {
                        0.
                    } else {
                        (-(rx * vx + ry * vy) / v2).clamp(0., 1.)
                    };
                    row["closest_approach_fraction_linear"] = json!(t);
                    row["puck_marker_minimum_separation_feet_linear_derived"] =
                        json!((rx + t * vx).hypot(ry + t * vy) / units);
                    Some(row)
                })
                .collect();
            json!({"from_frame_index":i,"to_frame_index":i+1,"markers":markers})
        })
        .collect();
    json!({"schema":"icelines.opposition-geometry.v0.1","status":"sample_intervals_available","scorer_id":scorer,"intervals":intervals,
        "interpretation":"All opposing non-goalie markers, including forwards and defenders. Linear simultaneous motion between adjacent samples gives a model minimum puck–marker separation; scorer separation uses observed endpoints. No contact, stick reach, puck height, active-ice membership, assigned coverage, pressure or fault is established. Missing endpoints/context remain explicit; no gap bridging. No temporal calibration or goal alignment is implied."})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (Value, Value) {
        let obs = |x, y| json!({"player_id":2,"team":"B","x":x,"y":y,"roster":{"goalie":false,"name":"Opponent"}});
        (
            json!({"goal":{"team_abbrev":"A","scorer_id":1},"frames":[
            {"puck":{"x":0.,"y":0.},"observations":[obs(5.,5.)]},
            {"puck":{"x":10.,"y":0.},"observations":[obs(5.,-5.)]}]}),
            json!({"rink_reference":{"native_units_per_foot_derived":1.}}),
        )
    }
    #[test]
    fn simultaneous_crossing_is_distinct_from_endpoint_distance() {
        let (e, g) = inputs();
        let a = assess(&e, &g);
        let m = &a["intervals"][0]["markers"][0];
        assert_eq!(m["closest_approach_fraction_linear"], 0.5);
        assert_eq!(m["puck_marker_minimum_separation_feet_linear_derived"], 0.);
        assert!(m["scorer_marker_separation_start_feet_derived"].is_null());
    }
    #[test]
    fn spatial_intersection_at_different_times_is_not_zero_separation() {
        let (mut e, g) = inputs();
        e["frames"][0]["observations"][0]["y"] = json!(10.);
        e["frames"][1]["observations"][0]["y"] = json!(0.);
        let a = assess(&e, &g);
        let m = &a["intervals"][0]["markers"][0];
        assert_eq!(m["closest_approach_fraction_linear"], 0.75);
        assert!(
            (m["puck_marker_minimum_separation_feet_linear_derived"]
                .as_f64()
                .unwrap()
                - 12.5f64.sqrt())
            .abs()
                < 1e-10
        );
    }
    #[test]
    fn closest_approach_is_clamped_to_interval() {
        let (mut e, g) = inputs();
        e["frames"][0]["observations"][0]["x"] = json!(20.);
        e["frames"][1]["observations"][0]["x"] = json!(20.);
        e["frames"][1]["observations"][0]["y"] = json!(5.);
        assert_eq!(
            assess(&e, &g)["intervals"][0]["markers"][0]["closest_approach_fraction_linear"],
            1.
        );
    }
    #[test]
    fn constant_relative_motion_retains_nonzero_distance() {
        let (mut e, g) = inputs();
        e["frames"][1]["observations"][0]["x"] = json!(15.);
        e["frames"][1]["observations"][0]["y"] = json!(5.);
        let a = assess(&e, &g);
        let m = &a["intervals"][0]["markers"][0];
        assert_eq!(m["closest_approach_fraction_linear"], 0.);
        assert!(
            (m["puck_marker_minimum_separation_feet_linear_derived"]
                .as_f64()
                .unwrap()
                - 50f64.sqrt())
            .abs()
                < 1e-10
        );
    }
    #[test]
    fn missing_endpoint_does_not_bridge_or_disappear() {
        let (mut e, g) = inputs();
        e["frames"][0]["observations"] = json!([]);
        assert_eq!(
            assess(&e, &g)["intervals"][0]["markers"][0]["status"],
            "marker_endpoint_missing"
        );
        e["frames"][1]["puck"] = Value::Null;
        e["frames"][0]["observations"] = e["frames"][1]["observations"].clone();
        assert_eq!(
            assess(&e, &g)["intervals"][0]["markers"][0]["status"],
            "coordinates_unavailable"
        );
    }
    #[test]
    fn goalie_and_teammate_excluded_unknown_metadata_retained() {
        let (mut e, g) = inputs();
        e["frames"][0]["observations"][0]["roster"]["goalie"] = json!(true);
        assert_eq!(
            assess(&e, &g)["intervals"][0]["markers"][0]["status"],
            "identity_context_conflict"
        );
        e["frames"][0]["observations"][0]["roster"]["goalie"] = Value::Null;
        assert_eq!(
            assess(&e, &g)["intervals"][0]["markers"][0]["status"],
            "identity_context_unavailable"
        );
        e["frames"][0]["observations"][0]["team"] = json!("A");
        e["frames"][1]["observations"][0]["team"] = json!("A");
        assert!(assess(&e, &g)["intervals"][0]["markers"]
            .as_array()
            .unwrap()
            .is_empty());
    }
    #[test]
    fn scorer_distance_is_separate_and_scale_required() {
        let (mut e, mut g) = inputs();
        e["frames"][0]["observations"]
            .as_array_mut()
            .unwrap()
            .push(json!({"player_id":1,"team":"A","x":5.,"y":7.,"roster":{"goalie":false}}));
        assert_eq!(
            assess(&e, &g)["intervals"][0]["markers"][0]
                ["scorer_marker_separation_start_feet_derived"],
            2.
        );
        g["rink_reference"] = Value::Null;
        assert_eq!(assess(&e, &g)["status"], "unavailable");
    }
}
