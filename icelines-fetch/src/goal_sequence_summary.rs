//! Descriptive comparisons along candidate credited-actor alignments.
use serde_json::{json, Value};

fn nearest(rows: &[Value], field: &str) -> Value {
    let valid: Vec<_> = rows
        .iter()
        .filter(|r| r["status"] == "available")
        .filter_map(|r| r[field].as_f64().map(|n| (r, n)))
        .collect();
    let minimum = valid.iter().map(|(_, n)| *n).reduce(f64::min);
    json!({"status":if minimum.is_none(){"unavailable"}else if valid.len()!=rows.len(){"partial_observed_markers"}else{"available_observed_markers"},
        "distance_feet_derived":minimum,
        "markers":valid.iter().filter(|(_,n)|Some(*n)==minimum).map(|(r,_)|json!({"player_id":r["player_id"],"name":r["name"]})).collect::<Vec<_>>()})
}

fn compare(intervals: &[Value], start: usize, end: usize) -> Value {
    if start > end || end > intervals.len() {
        return json!({"status":"invalid_sample_range","from_frame_index":start,"to_frame_index":end});
    }
    // Use exactly one adjacent pair per endpoint; missing rows are not borrowed
    // from another interval and gaps are not filled with nearby observations.
    let endpoint = |frame: usize| {
        let (row, field) = if frame < intervals.len() {
            (
                &intervals[frame],
                "scorer_marker_separation_start_feet_derived",
            )
        } else if let Some(last) = intervals.last() {
            (last, "scorer_marker_separation_end_feet_derived")
        } else {
            return json!({"status":"unavailable","markers":[]});
        };
        nearest(
            row["markers"].as_array().map(Vec::as_slice).unwrap_or(&[]),
            field,
        )
    };
    let first = endpoint(start);
    let last = endpoint(end);
    let mut route_rows = vec![];
    for interval in &intervals[start..end] {
        if interval["markers"].as_array().is_none_or(|v| v.is_empty()) {
            route_rows.push(json!({"status":"no_observed_markers"}));
        }
        for marker in interval["markers"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or(&[])
        {
            let mut row = marker.clone();
            row["from_frame_index"] = interval["from_frame_index"].clone();
            row["to_frame_index"] = interval["to_frame_index"].clone();
            route_rows.push(row);
        }
    }
    let mut route = nearest(
        &route_rows,
        "puck_marker_minimum_separation_feet_linear_derived",
    );
    let minimum = route["distance_feet_derived"].as_f64();
    route["encounters"]=json!(route_rows.iter().filter(|r|r["status"]=="available" && minimum.is_some() && r["puck_marker_minimum_separation_feet_linear_derived"].as_f64()==minimum).map(|r|json!({"player_id":r["player_id"],"name":r["name"],"from_frame_index":r["from_frame_index"],"to_frame_index":r["to_frame_index"],"closest_approach_fraction_linear":r["closest_approach_fraction_linear"]})).collect::<Vec<_>>());
    let delta = last["distance_feet_derived"]
        .as_f64()
        .zip(first["distance_feet_derived"].as_f64())
        .map(|(b, a)| b - a);
    let same = if first["markers"].as_array().is_some_and(|v| !v.is_empty())
        && last["markers"].as_array().is_some_and(|v| !v.is_empty())
    {
        Some(
            first["markers"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["player_id"].clone())
                .collect::<Vec<_>>()
                == last["markers"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| r["player_id"].clone())
                    .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    json!({"status":"sample_comparison","from_frame_index":start,"to_frame_index":end,
        "nearest_to_scorer_start":first,"nearest_to_scorer_end":last,
        "nearest_scorer_separation_change_feet_derived":delta,"same_nearest_marker_set_at_endpoints":same,
        "puck_route_minimum_linear":route})
}

/// Summarize stored alignment examples; never select one as the true touch chain.
pub fn assess(opposition: &Value, alignments: &Value) -> Value {
    let intervals = opposition["intervals"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let examples: Vec<_> = alignments["examples"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or(&[])
        .iter()
        .map(|example| {
            let mut phases = vec![];
            let anchors = example["anchors"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            for (i, anchor) in anchors.iter().enumerate() {
                let Some((first, last)) = anchor["first_frame"]
                    .as_u64()
                    .zip(anchor["last_frame"].as_u64())
                else {
                    continue;
                };
                if i > 0 {
                    if let Some(previous) = anchors[i - 1]["last_frame"].as_u64() {
                        let mut bridge = compare(intervals, previous as usize, first as usize);
                        bridge["kind"] = json!("between_actor_candidates");
                        bridge["from_actor_id"] = anchors[i - 1]["player_id"].clone();
                        bridge["to_actor_id"] = anchor["player_id"].clone();
                        phases.push(bridge);
                    }
                }
                let mut phase = compare(intervals, first as usize, last as usize);
                phase["kind"] = json!("actor_candidate_interval");
                phase["actor_id"] = anchor["player_id"].clone();
                phases.push(phase);
            }
            json!({"phases":phases})
        })
        .collect();
    json!({"schema":"icelines.goal-sequence-summary.v0.1","alignment_candidate_count":alignments["candidate_count"],"stored_example_count":examples.len(),"examples":examples,
        "interpretation":"Descriptive summaries of retained nearest-body alignment examples before each candidate net-plane crossing. Actor intervals are not verified setup, pass, release or finish times. Endpoint nearest-marker sets may change; separation change is not one defender's movement or evidence of marking. Route minimum is linear model geometry. Unknown duty, active occupancy, contact, height and goalie vision remain unknown. No true chain, causal breakdown or ability selected."})
}

#[cfg(test)]
mod tests {
    use super::*;
    fn marker(id: u64, start: f64, end: f64, route: f64) -> Value {
        json!({"status":"available","player_id":id,"name":format!("P{id}"),"scorer_marker_separation_start_feet_derived":start,"scorer_marker_separation_end_feet_derived":end,"puck_marker_minimum_separation_feet_linear_derived":route})
    }
    #[test]
    fn changing_nearest_actor_is_not_a_single_defender_movement() {
        let intervals = vec![
            json!({"from_frame_index":0,"to_frame_index":1,"markers":[marker(1,2.,8.,5.),marker(2,9.,3.,1.)]}),
        ];
        let a = compare(&intervals, 0, 1);
        assert_eq!(a["nearest_scorer_separation_change_feet_derived"], 1.);
        assert_eq!(a["same_nearest_marker_set_at_endpoints"], false);
        assert_eq!(
            a["puck_route_minimum_linear"]["encounters"][0]["player_id"],
            2
        );
    }
    #[test]
    fn tied_markers_and_partial_context_remain_visible() {
        let a = nearest(
            &[
                marker(1, 2., 3., 1.),
                marker(2, 2., 3., 1.),
                json!({"status":"marker_endpoint_missing"}),
            ],
            "scorer_marker_separation_start_feet_derived",
        );
        assert_eq!(a["markers"].as_array().unwrap().len(), 2);
        assert_eq!(a["status"], "partial_observed_markers");
    }
    #[test]
    fn one_sample_has_no_route_interval_and_invalid_range_stays_unknown() {
        let intervals = vec![json!({"markers":[marker(1,2.,3.,1.)]})];
        assert_eq!(
            compare(&intervals, 0, 0)["puck_route_minimum_linear"]["status"],
            "unavailable"
        );
        assert_eq!(compare(&intervals, 0, 2)["status"], "invalid_sample_range");
    }
    #[test]
    fn empty_interval_is_not_silently_counted_as_complete() {
        let intervals = vec![
            json!({"markers":[marker(1,2.,3.,1.)]}),
            json!({"markers":[]}),
        ];
        let a = compare(&intervals, 0, 2);
        assert_eq!(
            a["puck_route_minimum_linear"]["status"],
            "partial_observed_markers"
        );
        assert!(a["nearest_scorer_separation_change_feet_derived"].is_null());
        assert!(a["same_nearest_marker_set_at_endpoints"].is_null());
    }
    #[test]
    fn all_retained_examples_and_count_are_preserved() {
        let opposition = json!({"intervals":[{"markers":[marker(1,2.,3.,1.)]}]});
        let example = json!({"anchors":[{"player_id":4,"first_frame":0,"last_frame":0},{"player_id":5,"first_frame":1,"last_frame":1}]});
        let a = assess(
            &opposition,
            &json!({"candidate_count":20,"examples":[example.clone(),example]}),
        );
        assert_eq!(a["alignment_candidate_count"], 20);
        assert_eq!(a["examples"].as_array().unwrap().len(), 2);
        assert_eq!(a["examples"][0]["phases"].as_array().unwrap().len(), 3);
        assert_eq!(
            a["examples"][0]["phases"][1]["kind"],
            "between_actor_candidates"
        );
    }
}
