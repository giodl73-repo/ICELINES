//! Fixed-goal geometry derived from the official renderer and regulation rink.
//! Raw tracking is preserved; these coordinates do not identify release/contact.
use serde_json::{json, Value};

pub const GEOMETRY_SCHEMA: &str = "icelines.goal-relative-geometry.v0.1";
pub const RENDERER_URL: &str = "https://wsr.nhle.com/static/js/main.bundle.js?rel=4.6.1-333420b8";

pub fn assess(event: &Value, summary: &Value) -> Value {
    let scoring = event["goal"]["team_abbrev"].as_str();
    let home = summary["home_team"].as_str();
    let away = summary["away_team"].as_str();
    let side = event["goal"]["home_team_defending_side"].as_str();
    let is_home = match (scoring, home, away) {
        (Some(s), Some(h), Some(a)) if h != a && s == h => Some(true),
        (Some(s), Some(h), Some(a)) if h != a && s == a => Some(false),
        _ => None,
    };
    let right = match (is_home, side) {
        (Some(true), Some("left")) | (Some(false), Some("right")) => true,
        (Some(true), Some("right")) | (Some(false), Some("left")) => false,
        _ => {
            return json!({"schema":GEOMETRY_SCHEMA,"status":"unavailable","reason":"Scoring-team identity or recorded defending side is unknown; no attacking goal is guessed.","frames":[]})
        }
    };
    // Ice-facing edges of the official SVG goal-line bands: left130..132,
    // right2268..2270. Centreline y510; rink dimensions2400 by1020.
    let target_x = if right { 2268.0 } else { 132.0 };
    let direction = if right { 1.0 } else { -1.0 };
    let mut last_side = 0i8;
    let mut previous: Option<(f64, f64)> = None;
    let rows:Vec<_>=event["frames"].as_array().map(Vec::as_slice).unwrap_or(&[]).iter().map(|frame|{
        let Some((x,y))=frame["puck"]["x"].as_f64().zip(frame["puck"]["y"].as_f64()) else {
            last_side=0;previous=None;
            return json!({"frame":frame["frame"],"status":"unavailable","reason":"Puck coordinates missing"});
        };
        let depth=direction*(target_x-x)/12.0;let lateral=(y-510.0)/12.0;
        let distance=depth.hypot(lateral);let bearing=if distance==0.0{None}else{Some(lateral.abs().atan2(depth).to_degrees())};
        let side=if lateral>0.0{1}else if lateral<0.0{-1}else{0};
        let crossed=side!=0 && last_side!=0 && side!=last_side;
        if side!=0 {last_side=side;}
        let distance_change=previous.map(|p|distance-p.0);
        let lateral_change=previous.map(|p|lateral-p.1);previous=Some((distance,lateral));
        json!({"frame":frame["frame"],"status":"available","puck":frame["puck"],
            "distance_to_goal_center_feet_derived":distance,"depth_to_goal_line_feet_derived":depth,
            "lateral_offset_from_centerline_feet_derived":lateral,"goal_relative_bearing_degrees":bearing,
            "in_front_of_goal_line":depth>=0.0,"sampled_centerline_side_change":crossed,
            "distance_change_from_previous_frame_feet_derived":distance_change,"lateral_change_from_previous_frame_feet_derived":lateral_change})
    }).collect();
    json!({"schema":GEOMETRY_SCHEMA,"status":"available","scoring_team":scoring,"attacking_goal_side":if right{"right"}else{"left"},
        "target_goal_reference":{"x":target_x,"y":510.0,"basis":"ice-facing edge of goal-line band in official rink SVG; fixed goal, not goalie position"},
        "rink_reference":{"length_native":2400,"width_native":1020,"native_units_per_foot_derived":12,"goal_line_from_end_native":132},
        "reference_status":"official-renderer geometry; physical scale derived from regulation dimensions, not independently surveyed tracking accuracy",
        "reference_url":RENDERER_URL,"orientation_basis":"scoring-team identity and recorded homeTeamDefendingSide for the goal period",
        "interpretation":"Distance and bearing relative to a fixed goal-line reference. Bearing is not a release-aligned shot angle or an xG estimate. Positive depth is in front; negative is behind. Lateral sign follows native y, not a broadcast-view left/right. Crossing the rink centreline does not establish a pass or chance quality. Exact contact/release/goal alignment, puck height and goalie vision remain unknown.","frames":rows})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input(home: bool, side: &str, x: f64, y: f64) -> (Value, Value) {
        (
            json!({"goal":{"team_abbrev":if home{"HOME"}else{"AWAY"},"home_team_defending_side":side},"frames":[{"frame":0,"puck":{"x":x,"y":y}}]}),
            json!({"home_team":"HOME","away_team":"AWAY"}),
        )
    }
    #[test]
    fn both_attack_directions_use_fixed_goal_not_goalie() {
        let (e, s) = input(true, "left", 2148., 510.);
        let a = assess(&e, &s);
        assert_eq!(a["target_goal_reference"]["x"], 2268.);
        assert_eq!(a["frames"][0]["distance_to_goal_center_feet_derived"], 10.);
        let (e, s) = input(true, "right", 252., 510.);
        assert_eq!(
            assess(&e, &s)["frames"][0]["distance_to_goal_center_feet_derived"],
            10.
        );
        let (e, s) = input(false, "right", 2148., 510.);
        assert_eq!(assess(&e, &s)["attacking_goal_side"], "right");
    }
    #[test]
    fn bearing_and_behind_goal_are_explicit() {
        let (e, s) = input(true, "left", 2148., 630.);
        let a = assess(&e, &s);
        assert_eq!(a["frames"][0]["goal_relative_bearing_degrees"], 45.);
        let (e, s) = input(true, "left", 2280., 522.);
        let a = assess(&e, &s);
        assert_eq!(a["frames"][0]["in_front_of_goal_line"], false);
        assert!(
            a["frames"][0]["goal_relative_bearing_degrees"]
                .as_f64()
                .unwrap()
                > 90.
        );
        let (e, s) = input(true, "left", 2268., 510.);
        assert!(assess(&e, &s)["frames"][0]["goal_relative_bearing_degrees"].is_null());
    }
    #[test]
    fn unknown_orientation_does_not_guess() {
        let (e, s) = input(true, "unknown", 2148., 510.);
        assert_eq!(assess(&e, &s)["status"], "unavailable");
        let (mut e, s) = input(true, "left", 2148., 510.);
        e["goal"]["team_abbrev"] = json!("OTHER");
        assert_eq!(assess(&e, &s)["status"], "unavailable");
    }
    #[test]
    fn centerline_crossing_survives_a_center_sample_but_not_missing_data() {
        let (mut e, s) = input(true, "left", 2000., 500.);
        e["frames"] = json!([{"frame":0,"puck":{"x":2000.,"y":500.}},{"frame":1,"puck":{"x":2000.,"y":510.}},{"frame":2,"puck":{"x":2000.,"y":520.}},{"frame":3,"puck":null},{"frame":4,"puck":{"x":2000.,"y":500.}}]);
        let a = assess(&e, &s);
        assert_eq!(a["frames"][2]["sampled_centerline_side_change"], true);
        assert_eq!(a["frames"][4]["sampled_centerline_side_change"], false);
    }
}
