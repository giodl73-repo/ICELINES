//! Every observed player's distance to the puck across a source sample window.
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
fn point(v: &Value) -> Option<(f64, f64)> {
    v["x"].as_f64().zip(v["y"].as_f64())
}
pub fn assess(scene: &Value, geometry: &Value) -> Value {
    let mut out = json!({"schema":"icelines.goal-puck-distance-history.v0.1","status":"unavailable","frames":[],"roster":[],
        "interpretation":"All source samples and both teams' observations, including goalies. Derived point-to-puck distance and consecutive-sample same-ID distance change are not possession, speed, pressure, checking, assigned coverage or active-unit validation. Unknowns break continuity; native sample spacing is not calibrated seconds."});
    let Some(units) = geometry["rink_reference"]["native_units_per_foot_derived"]
        .as_f64()
        .filter(|u| u.is_finite() && *u > 0.)
    else {
        out["reason"] = json!("derived_scale_unavailable");
        return out;
    };
    let Some(frames) = scene["frames"]
        .as_array()
        .filter(|_| scene["status"] == "source_samples_available")
    else {
        out["reason"] = json!("source_samples_unavailable");
        return out;
    };
    let mut roster = BTreeMap::new();
    let mut previous: BTreeMap<u64, f64> = BTreeMap::new();
    let mut previous_index: Option<u64> = None;
    let rows:Vec<_>=frames.iter().map(|frame|{
        let index=frame["frame_index"].as_u64();
        if previous_index.zip(index).is_none_or(|(a,b)|a.checked_add(1)!=Some(b)){previous.clear()}
        let puck=point(&frame["puck"]["marker"]);let players=frame["players"].as_array().map(Vec::as_slice).unwrap_or(&[]);let mut current=BTreeMap::new();
        let measured:Vec<_>=players.iter().map(|p|{
            let id=p["player_id"].as_u64();
            if let Some(id)=id {roster.entry(id).or_insert_with(||json!({"player_id":id,"name":p["name"],"team":p["team"],"goalie":p["goalie"]}));}
            let status=if id.is_none(){"identity_unavailable"}else if players.iter().filter(|q|q["player_id"].as_u64()==id).count()!=1{"identity_ambiguous"}else if point(&p["marker"]).is_none(){"player_coordinates_unavailable"}else if puck.is_none(){"puck_coordinates_unavailable"}else{"available"};
            let distance=if status=="available"{point(&p["marker"]).zip(puck).map(|((x,y),(u,v))|(x-u).hypot(y-v)/units)}else{None};
            let change=id.zip(distance).and_then(|(id,d)|previous.get(&id).map(|before|d-before));
            if let Some((id,d))=id.zip(distance){current.insert(id,d);}
            json!({"source_observation_index":p["source_observation_index"],"player_id":p["player_id"],"team":p["team"],"goalie":p["goalie"],"status":status,"puck_separation_feet_derived":distance,"change_since_previous_sample_feet_derived":change})
        }).collect();
        previous=current;previous_index=index;
        json!({"frame_index":frame["frame_index"],"elapsed_native_ticks":frame["elapsed_native_ticks"],"puck_status":if puck.is_some(){"available"}else{"coordinates_unavailable"},"players":measured})
    }).collect();
    // Flag changing metadata rather than silently treating first-known labels as
    // a stable roster identity. Per-sample metadata is always retained above.
    for (id, row) in &mut roster {
        let contexts: BTreeSet<_> = frames
            .iter()
            .flat_map(|f| f["players"].as_array().map(Vec::as_slice).unwrap_or(&[]))
            .filter(|p| p["player_id"].as_u64() == Some(*id))
            .map(|p| format!("{}|{}|{}", p["name"], p["team"], p["goalie"]))
            .collect();
        row["metadata_changes_within_window"] = json!(contexts.len() > 1);
    }
    out["status"] = json!("all_player_distances_available");
    out["frames"] = json!(rows);
    out["roster"] = json!(roster.into_values().collect::<Vec<_>>());
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> Value {
        json!({"status":"source_samples_available","frames":[{"frame_index":0,"puck":{"marker":{"x":0.,"y":0.}},"players":[{"player_id":1,"team":"A","goalie":false,"marker":{"x":3.,"y":4.}},{"player_id":2,"team":"B","goalie":true,"marker":{"x":6.,"y":8.}}]},{"frame_index":1,"puck":{"marker":{"x":0.,"y":0.}},"players":[{"player_id":1,"team":"A","goalie":false,"marker":{"x":0.,"y":4.}}]}]})
    }
    fn g() -> Value {
        json!({"rink_reference":{"native_units_per_foot_derived":1.}})
    }
    #[test]
    fn both_teams_and_goalies_with_same_identity_delta() {
        let r = assess(&scene(), &g());
        assert_eq!(
            r["frames"][0]["players"][1]["puck_separation_feet_derived"],
            10.
        );
        assert_eq!(
            r["frames"][1]["players"][0]["change_since_previous_sample_feet_derived"],
            -1.
        );
        assert_eq!(r["roster"].as_array().unwrap().len(), 2);
    }
    #[test]
    fn missing_puck_breaks_continuity_without_zero_fill() {
        let mut s = scene();
        let last = s["frames"][1].clone();
        s["frames"][1]["puck"]["marker"] = Value::Null;
        let mut f = last;
        f["frame_index"] = json!(2);
        s["frames"].as_array_mut().unwrap().push(f);
        let r = assess(&s, &g());
        assert!(r["frames"][1]["players"][0]["puck_separation_feet_derived"].is_null());
        assert!(
            r["frames"][2]["players"][0]["change_since_previous_sample_feet_derived"].is_null()
        );
    }
    #[test]
    fn duplicate_ids_and_index_gaps_break_continuity() {
        let mut s = scene();
        let p = s["frames"][0]["players"][0].clone();
        s["frames"][0]["players"].as_array_mut().unwrap().push(p);
        let r = assess(&s, &g());
        assert_eq!(r["frames"][0]["players"][0]["status"], "identity_ambiguous");
        assert!(
            r["frames"][1]["players"][0]["change_since_previous_sample_feet_derived"].is_null()
        );
        let mut s = scene();
        s["frames"][1]["frame_index"] = json!(2);
        assert!(assess(&s, &g())["frames"][1]["players"][0]
            ["change_since_previous_sample_feet_derived"]
            .is_null());
    }
    #[test]
    fn scale_and_metadata_changes_are_explicit() {
        let mut s = scene();
        s["frames"][1]["players"][0]["team"] = json!("B");
        assert_eq!(
            assess(&s, &g())["roster"][0]["metadata_changes_within_window"],
            true
        );
        assert_eq!(
            assess(&s, &json!({}))["reason"],
            "derived_scale_unavailable"
        );
    }
}
