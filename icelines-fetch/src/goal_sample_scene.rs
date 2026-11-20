//! Same-source-sample coordinates, without interpolation or active-unit inference.
use serde_json::{json, Value};

fn marker(v: &Value) -> Value {
    v["x"]
        .as_f64()
        .zip(v["y"].as_f64())
        .map(|(x, y)| json!({"x":x,"y":y}))
        .unwrap_or(Value::Null)
}
pub fn assess(event: &Value, start: usize, end: usize) -> Value {
    let frames = event["frames"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut out = json!({"schema":"icelines.goal-sample-scene.v0.1","status":"unavailable","frames":[],
        "scorer_id":event["goal"]["scorer_id"],"scoring_team":event["goal"]["team_abbrev"],
        "interpretation":"Coordinates and metadata from the same published sample. No interpolation, clock/physical synchronization calibration, possession, contacts, active-ice membership, roster duty or coverage inference. An event-recorded goalie/empty-net designation does not establish occupancy at every earlier sample. Duplicate/unknown identities and missing coordinates remain explicit."});
    if start > end || end >= frames.len() {
        out["reason"] = json!("invalid_sample_range");
        return out;
    }
    let rows:Vec<_>=frames[start..=end].iter().enumerate().map(|(offset,frame)| {
        let puck=marker(&frame["puck"]);
        let players:Vec<_>=frame["observations"].as_array().map(Vec::as_slice).unwrap_or(&[]).iter().enumerate().map(|(index,o)| {
            let point=marker(o);
            json!({"source_observation_index":index,"player_id":o["player_id"],"name":o["roster"]["name"],
                "team":o["team"],"goalie":o["roster"]["goalie"].as_bool(),
                "identity_status":if o["player_id"].as_u64().is_some(){"recorded_id"}else{"identity_unavailable"},
                "coordinate_status":if point.is_null(){"coordinates_unavailable"}else{"observed"},"marker":point})
        }).collect();
        json!({"frame_index":start+offset,"elapsed_native_ticks":frame["elapsed_native_ticks"],
            "puck":{"coordinate_status":if puck.is_null(){"coordinates_unavailable"}else{"observed"},"marker":puck},"players":players})
    }).collect();
    out["status"] = json!("source_samples_available");
    out["frames"] = json!(rows);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keeps_every_sample_and_observation_without_carrying_positions_forward() {
        let e = json!({"frames":[{"puck":{"x":1.,"y":2.},"observations":[{"player_id":3,"team":"A","x":10.,"y":20.,"roster":{"goalie":true}}]}, {"puck":{"x":4.,"y":5.},"observations":[]}]});
        let s = assess(&e, 0, 1);
        assert_eq!(
            s["frames"][0]["players"][0]["marker"],
            json!({"x":10.,"y":20.})
        );
        assert_eq!(s["frames"][1]["puck"]["marker"], json!({"x":4.,"y":5.}));
        assert!(s["frames"][1]["players"].as_array().unwrap().is_empty());
    }
    #[test]
    fn unknown_identity_role_and_coordinates_remain_distinct() {
        let e = json!({"frames":[{"observations":[{"team":"A","x":1.,"y":2.},{"player_id":3,"roster":{"goalie":false}}]}]});
        let s = assess(&e, 0, 0);
        let rows = &s["frames"][0]["players"];
        assert_eq!(rows[0]["identity_status"], "identity_unavailable");
        assert!(rows[0]["goalie"].is_null());
        assert_eq!(rows[1]["coordinate_status"], "coordinates_unavailable");
        assert_eq!(rows[1]["goalie"], false);
        assert!(s["frames"][0]["puck"]["marker"].is_null());
    }
    #[test]
    fn duplicate_ids_and_native_ticks_are_preserved_not_deduplicated() {
        let e = json!({"frames":[{"elapsed_native_ticks":17,"observations":[{"player_id":3,"x":1.,"y":2.},{"player_id":3,"x":4.,"y":5.}]}]});
        let s = assess(&e, 0, 0);
        assert_eq!(s["frames"][0]["players"].as_array().unwrap().len(), 2);
        assert_eq!(s["frames"][0]["players"][1]["source_observation_index"], 1);
        assert_eq!(s["frames"][0]["elapsed_native_ticks"], 17);
    }
    #[test]
    fn invalid_ranges_are_not_clamped_or_replaced() {
        assert_eq!(
            assess(&json!({"frames":[]}), 0, 0)["reason"],
            "invalid_sample_range"
        );
        assert_eq!(
            assess(&json!({"frames":[{}]}), 1, 0)["reason"],
            "invalid_sample_range"
        );
    }
}
