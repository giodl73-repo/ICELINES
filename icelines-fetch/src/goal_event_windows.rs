//! Candidate 2D net-opening encounters, not verified goal/contact timestamps.
use serde_json::{json, Value};
const SCHEMA: &str = "icelines.goal-event-windows.v0.1";

pub fn assess(event: &Value, geometry: &Value) -> Value {
    let reference = geometry["target_goal_reference"]["x"]
        .as_f64()
        .zip(geometry["target_goal_reference"]["y"].as_f64())
        .zip(geometry["rink_reference"]["native_units_per_foot_derived"].as_f64());
    let direction = match geometry["attacking_goal_side"].as_str() {
        Some("right") => 1.0,
        Some("left") => -1.0,
        _ => 0.0,
    };
    let Some(((gx, gy), units)) = reference
        .filter(|(_, units)| *units > 0.0 && geometry["status"] == "available" && direction != 0.0)
    else {
        return json!({"schema":SCHEMA,"status":"unavailable","reason":"Fixed goal reference, scale or direction unknown.","candidates":[]});
    };
    let frames = event["frames"].as_array().map(Vec::as_slice).unwrap_or(&[]);
    let mut candidates = vec![];
    let mut outside_opening = vec![];
    let mut valid_pairs = 0;
    let minimum_plane_distance = frames
        .iter()
        .filter_map(|f| f["puck"]["x"].as_f64())
        .map(|x| (gx - x).abs() / units)
        .reduce(f64::min);
    for (i, pair) in frames.windows(2).enumerate() {
        let coords = pair[0]["puck"]["x"]
            .as_f64()
            .zip(pair[0]["puck"]["y"].as_f64())
            .zip(
                pair[1]["puck"]["x"]
                    .as_f64()
                    .zip(pair[1]["puck"]["y"].as_f64()),
            );
        let Some(((ax, ay), (bx, by))) = coords else {
            continue;
        };
        valid_pairs += 1;
        let before = direction * (gx - ax);
        let after = direction * (gx - bx);
        // Require entry beyond the reference plane. Merely reaching it and
        // turning back does not qualify, nor does a return towards the ice.
        if before < 0.0 || after >= 0.0 {
            continue;
        }
        let fraction = before / (before - after);
        let y = ay + fraction * (by - ay);
        let encounter = json!({"from_frame_index":i,"to_frame_index":i+1,
            "fraction_within_sample_interval_linear":fraction,"fractional_frame_index_linear":i as f64+fraction,
            "native_tick_linear":pair[0]["elapsed_native_ticks"].as_f64().zip(pair[1]["elapsed_native_ticks"].as_f64()).map(|(a,b)|a+fraction*(b-a)),
            "intersection_native":{"x":gx,"y":y},"lateral_from_goal_centre_feet_derived":(y-gy)/units,
            "comparison_start_frame_index":i.saturating_sub(10),"comparison_end_frame_index":i});
        if (y - gy).abs() > 3.0 * units {
            outside_opening.push(encounter);
        } else {
            candidates.push(encounter);
        }
    }
    json!({"schema":SCHEMA,"status":if candidates.is_empty(){"no_sampled_opening_crossing"}else{"candidate_windows_available"},
        "candidate_count":candidates.len(),"candidates":candidates,"opening_half_width_feet_reference":3,
        "outside_opening_forward_encounters":outside_opening,
        "minimum_absolute_sample_plane_distance_feet_derived":minimum_plane_distance,
        "missing_candidate_reason":if !candidates.is_empty(){None}else if valid_pairs==0{Some("no_valid_adjacent_samples")}else if outside_opening.is_empty(){Some("no_forward_reference_plane_encounter")}else{Some("forward_encounters_outside_opening_only")},
        "reference_url":"https://media.nhl.com/site/asset/public/ext/2023-24/2023-24Rulebook.pdf",
        "puck_height_observed":false,"puck_radius_accounted_for":false,"whole_puck_crossing_verified":false,
        "exact_goal_frame_verified":false,
        "interpretation":"All forward crossings of a regulation-derived 2D net-opening reference using linear interpolation between adjacent samples. Includes possible later crossings; none is selected as the true goal. Height, puck volume, posts, true between-sample path and exact goal/release/contact timing are unverified. Missing crossing does not contradict an official goal. Comparisons end at the pre-crossing sample, using up to ten earlier frames; they are not reaction-time windows."})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event(points: &[(f64, f64)]) -> Value {
        json!({"frames":points.iter().enumerate().map(|(i,(x,y))|json!({"frame":i,"elapsed_native_ticks":i,"puck":{"x":x,"y":y}})).collect::<Vec<_>>()})
    }
    fn geometry() -> Value {
        json!({"status":"available","attacking_goal_side":"right","target_goal_reference":{"x":2268.,"y":510.},"rink_reference":{"native_units_per_foot_derived":12}})
    }
    #[test]
    fn intersects_reference_opening_and_retains_bracket() {
        let a = assess(&event(&[(2256., 500.), (2280., 520.)]), &geometry());
        let c = &a["candidates"][0];
        assert_eq!(c["fractional_frame_index_linear"], 0.5);
        assert_eq!(c["intersection_native"]["y"], 510.);
        assert_eq!(c["from_frame_index"], 0);
        assert_eq!(a["exact_goal_frame_verified"], false);
    }
    #[test]
    fn outside_opening_and_reverse_direction_do_not_match() {
        assert_eq!(
            assess(&event(&[(2256., 560.), (2280., 560.)]), &geometry())["candidate_count"],
            0
        );
        let outside = assess(&event(&[(2256., 560.), (2280., 560.)]), &geometry());
        assert_eq!(
            outside["outside_opening_forward_encounters"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            outside["missing_candidate_reason"],
            "forward_encounters_outside_opening_only"
        );
        assert_eq!(
            assess(&event(&[(2280., 510.), (2256., 510.)]), &geometry())["candidate_count"],
            0
        );
    }
    #[test]
    fn repeat_forward_encounters_are_not_collapsed_into_one_goal() {
        let a = assess(
            &event(&[(2256., 510.), (2280., 510.), (2256., 510.), (2280., 510.)]),
            &geometry(),
        );
        assert_eq!(a["candidate_count"], 2);
    }
    #[test]
    fn line_arrival_then_departure_is_not_double_counted() {
        let a = assess(
            &event(&[(2256., 510.), (2268., 510.), (2280., 510.)]),
            &geometry(),
        );
        assert_eq!(a["candidate_count"], 1);
        assert_eq!(a["candidates"][0]["from_frame_index"], 1);
        assert_eq!(
            assess(
                &event(&[(2256., 510.), (2268., 510.), (2256., 510.)]),
                &geometry()
            )["candidate_count"],
            0
        );
    }
    #[test]
    fn unknown_samples_are_not_bridged() {
        let mut e = event(&[(2256., 510.), (2268., 510.), (2280., 510.)]);
        e["frames"][1]["puck"] = Value::Null;
        assert_eq!(assess(&e, &geometry())["candidate_count"], 0);
        let mut g = geometry();
        g["attacking_goal_side"] = Value::Null;
        assert_eq!(assess(&e, &g)["status"], "unavailable");
    }
    #[test]
    fn mirrors_left_goal_and_keeps_lookback_bounded() {
        let mut g = geometry();
        g["attacking_goal_side"] = json!("left");
        g["target_goal_reference"]["x"] = json!(132.);
        let a = assess(&event(&[(144., 510.), (120., 510.)]), &g);
        assert_eq!(a["candidate_count"], 1);
        assert_eq!(a["candidates"][0]["comparison_start_frame_index"], 0);
    }
    #[test]
    fn near_plane_sample_is_not_silently_promoted_to_a_crossing() {
        let a = assess(&event(&[(2256., 510.), (2267.25, 510.)]), &geometry());
        assert_eq!(a["candidate_count"], 0);
        assert_eq!(
            a["minimum_absolute_sample_plane_distance_feet_derived"],
            0.0625
        );
        assert_eq!(
            a["missing_candidate_reason"],
            "no_forward_reference_plane_encounter"
        );
    }
}
