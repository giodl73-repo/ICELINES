//! Goalie-marker geometry, not facing, coverage or readiness assessment.
use serde_json::{json, Value};
const SCHEMA: &str = "icelines.goalie-reference-geometry.v0.1";
pub fn assess(event: &Value, goal: &Value) -> Value {
    if matches!(
        event["goal_modifier"].as_str(),
        Some("empty-net" | "awarded-empty-net" | "own-goal-empty-net")
    ) {
        return json!({"schema":SCHEMA,"status":"inapplicable_empty_net","frames":[]});
    }
    let Some(id) = event["official_goal_details"]["goalieInNetId"].as_u64() else {
        return json!({"schema":SCHEMA,"status":"unavailable","reason":"No recorded opposing goalie identity; absence is not assumed.","frames":[]});
    };
    let Some((gx, gy)) = goal["target_goal_reference"]["x"]
        .as_f64()
        .zip(goal["target_goal_reference"]["y"].as_f64())
        .filter(|_| goal["status"] == "available")
    else {
        return json!({"schema":SCHEMA,"status":"unavailable","reason":"Fixed attacking-goal reference unavailable.","frames":[]});
    };
    let direction = match goal["attacking_goal_side"].as_str() {
        Some("right") => 1.0,
        Some("left") => -1.0,
        _ => {
            return json!({"schema":SCHEMA,"status":"unavailable","reason":"Attacking-goal direction unknown.","frames":[]})
        }
    };
    let Some(units) = goal["rink_reference"]["native_units_per_foot_derived"]
        .as_f64()
        .filter(|n| *n > 0.0)
    else {
        return json!({"schema":SCHEMA,"status":"unavailable","reason":"Derived reference scale unknown.","frames":[]});
    };
    let name = event["players"]
        .as_array()
        .and_then(|p| p.iter().find(|p| p["player_id"].as_u64() == Some(id)))
        .map(|p| p["roster"]["name"].clone());
    let mut previous: Option<(f64, f64, f64)> = None;
    let rows:Vec<_>=event["frames"].as_array().map(Vec::as_slice).unwrap_or(&[]).iter().map(|frame|{
        let keeper=frame["observations"].as_array().and_then(|o|o.iter().find(|o|o["player_id"].as_u64()==Some(id)));
        let Some(k)=keeper else {previous=None;return json!({"frame":frame["frame"],"status":"goalie_not_observed"});};
        if k["roster"]["goalie"].is_null() {previous=None;return json!({"frame":frame["frame"],"status":"goalie_role_metadata_unavailable"});}
        if k["roster"]["goalie"]!=true || k["team"]==event["goal"]["team_abbrev"] {
            previous=None;return json!({"frame":frame["frame"],"status":"goalie_identity_context_conflict"});
        }
        let coords=frame["puck"]["x"].as_f64().zip(frame["puck"]["y"].as_f64()).zip(k["x"].as_f64().zip(k["y"].as_f64()));
        let Some(((px,py),(kx,ky)))=coords else {previous=None;return json!({"frame":frame["frame"],"status":"coordinates_unavailable"});};
        let dx=px-gx;let dy=py-gy;let length=dx.hypot(dy);let depth=direction*(gx-kx)/units;
        if length==0.0 {previous=None;return json!({"frame":frame["frame"],"status":"reference_ray_undefined","goalie_marker_depth_feet_derived":depth});}
        let vx=kx-gx;let vy=ky-gy;let projection=(vx*dx+vy*dy)/(length*length);
        let offset=(dx*vy-dy*vx).abs()/length/units;
        let movement=previous.map(|(x,y,_)|(kx-x).hypot(ky-y)/units);
        let offset_change=previous.map(|(_,_,d)|offset-d);previous=Some((kx,ky,offset));
        json!({"frame":frame["frame"],"status":"available","goalie_marker":{"x":kx,"y":ky},
            "goalie_marker_depth_feet_derived":depth,"goalie_marker_offset_from_puck_goal_line_feet_derived":offset,
            "goalie_projection_fraction":projection,"projection_between_goal_and_puck":(0.0..=1.0).contains(&projection),
            "puck_to_goalie_marker_feet_derived":(px-kx).hypot(py-ky)/units,
            "goalie_marker_displacement_since_previous_frame_feet_derived":movement,
            "reference_line_offset_change_since_previous_frame_feet_derived":offset_change})
    }).collect();
    json!({"schema":SCHEMA,"status":"recorded_goalie_reference","recorded_goalie_id":id,"recorded_goalie_name":name,
        "identity_basis":"goalieInNetId at the goal event; frame presence does not verify active occupancy",
        "interpretation":"Observed goalie marker versus the puck-to-fixed-goal-centre reference line. This is not the actual shot target or goalie facing, body coverage, skate depth, vision, readiness or an optimal-position grade. The line moves with the puck, so its offset can change while the goalie stays still. Distances use the derived rink reference; release/contact/goal timing remains unaligned.","frames":rows})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> (Value, Value) {
        (
            json!({"goal":{"team_abbrev":"HOME"},"goal_modifier":"none","official_goal_details":{"goalieInNetId":2},"players":[{"player_id":2,"roster":{"name":"Keeper"}}],"frames":[{"frame":0,"puck":{"x":2148.,"y":510.},"observations":[{"player_id":2,"team":"AWAY","x":2208.,"y":510.,"roster":{"goalie":true}}]}]}),
            json!({"status":"available","attacking_goal_side":"right","rink_reference":{"native_units_per_foot_derived":12},"target_goal_reference":{"x":2268.,"y":510.}}),
        )
    }
    #[test]
    fn aligned_marker_has_depth_and_zero_reference_offset() {
        let (e, g) = input();
        let a = assess(&e, &g);
        let f = &a["frames"][0];
        assert_eq!(f["goalie_marker_depth_feet_derived"], 5.);
        assert_eq!(
            f["goalie_marker_offset_from_puck_goal_line_feet_derived"],
            0.
        );
        assert_eq!(f["goalie_projection_fraction"], 0.5);
        assert_eq!(f["projection_between_goal_and_puck"], true);
    }
    #[test]
    fn moving_puck_changes_reference_offset_with_stationary_goalie() {
        let (mut e, g) = input();
        let mut f = e["frames"][0].clone();
        f["frame"] = json!(1);
        f["puck"]["y"] = json!(630.);
        e["frames"].as_array_mut().unwrap().push(f);
        let a = assess(&e, &g);
        let f = &a["frames"][1];
        assert_eq!(
            f["goalie_marker_displacement_since_previous_frame_feet_derived"],
            0.
        );
        assert!(
            f["reference_line_offset_change_since_previous_frame_feet_derived"]
                .as_f64()
                .unwrap()
                > 0.
        );
    }
    #[test]
    fn missing_observation_breaks_movement_continuity() {
        let (mut e, g) = input();
        let first = e["frames"][0].clone();
        e["frames"] =
            json!([first.clone(),{"frame":1,"puck":{"x":2100.,"y":510.},"observations":[]},first]);
        let a = assess(&e, &g);
        assert_eq!(a["frames"][1]["status"], "goalie_not_observed");
        assert!(
            a["frames"][2]["goalie_marker_displacement_since_previous_frame_feet_derived"]
                .is_null()
        );
    }
    #[test]
    fn empty_net_unknown_identity_and_undefined_ray_stay_distinct() {
        let (mut e, g) = input();
        e["goal_modifier"] = json!("empty-net");
        assert_eq!(assess(&e, &g)["status"], "inapplicable_empty_net");
        e["goal_modifier"] = json!("none");
        e["frames"][0]["puck"] = json!({"x":2268.,"y":510.});
        assert_eq!(
            assess(&e, &g)["frames"][0]["status"],
            "reference_ray_undefined"
        );
        e["official_goal_details"] = json!({});
        assert_eq!(assess(&e, &g)["status"], "unavailable");
    }
    #[test]
    fn mirrored_direction_has_same_depth_and_offset() {
        let (mut e, mut g) = input();
        e["frames"][0]["puck"] = json!({"x":252.,"y":510.});
        e["frames"][0]["observations"][0]["x"] = json!(192.);
        g["target_goal_reference"]["x"] = json!(132.);
        g["attacking_goal_side"] = json!("left");
        let a = assess(&e, &g);
        assert_eq!(a["frames"][0]["goalie_marker_depth_feet_derived"], 5.);
        assert_eq!(
            a["frames"][0]["goalie_marker_offset_from_puck_goal_line_feet_derived"],
            0.
        );
    }
    #[test]
    fn zero_offset_behind_net_is_not_interposition() {
        let (mut e, g) = input();
        e["frames"][0]["observations"][0]["x"] = json!(2280.);
        let a = assess(&e, &g);
        let f = &a["frames"][0];
        assert_eq!(
            f["goalie_marker_offset_from_puck_goal_line_feet_derived"],
            0.
        );
        assert_eq!(f["goalie_marker_depth_feet_derived"], -1.);
        assert_eq!(f["projection_between_goal_and_puck"], false);
    }
    #[test]
    fn conflicting_identity_or_unknown_reference_stays_unavailable() {
        let (mut e, mut g) = input();
        e["frames"][0]["observations"][0]["team"] = json!("HOME");
        assert_eq!(
            assess(&e, &g)["frames"][0]["status"],
            "goalie_identity_context_conflict"
        );
        e["frames"][0]["observations"][0]["team"] = json!("AWAY");
        e["frames"][0]["observations"][0]["roster"]["goalie"] = Value::Null;
        assert_eq!(
            assess(&e, &g)["frames"][0]["status"],
            "goalie_role_metadata_unavailable"
        );
        g["attacking_goal_side"] = Value::Null;
        assert_eq!(assess(&e, &g)["status"], "unavailable");
    }
}
