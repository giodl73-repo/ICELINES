//! Experimental geometric screens; no verified possession or carry assignments.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub distance_screens_feet: Vec<f64>,
    pub minimum_samples: usize,
    pub maximum_relative_offset_change_feet: f64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            distance_screens_feet: vec![4., 6., 8.],
            minimum_samples: 3,
            maximum_relative_offset_change_feet: 2.,
        }
    }
}
fn point(v: &Value) -> Option<(f64, f64)> {
    v["x"].as_f64().zip(v["y"].as_f64())
}
fn length(a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
pub fn assess(scene: &Value, geometry: &Value, policy: &Policy) -> Value {
    let mut out = json!({"schema":"icelines.goal-control-candidates.v0.1","status":"unavailable","policy":policy,"frames":[],
        "policy_status":"exploratory_not_validated","possession_verified":false,"carry_verified":false,
        "interpretation":"Experimental sustained-proximity/co-motion screens, not verified control, carry, contested possession, pass, shot or flight. The 4/6/8ft screens are sensitivity hypotheses; three samples and 2ft relative-offset tolerance are authored pilot choices. Native samples are not calibrated seconds. Opponent proximity is context, not proof of a battle. Initial possession and active personnel unknown. No previous verified holder, speed rule, confidence percentage or ability score is fabricated; this is not a replication of a published possession model."});
    if policy.minimum_samples < 2
        || policy.distance_screens_feet.is_empty()
        || policy
            .distance_screens_feet
            .iter()
            .any(|n| !n.is_finite() || *n <= 0.)
        || !policy.maximum_relative_offset_change_feet.is_finite()
        || policy.maximum_relative_offset_change_feet < 0.
    {
        out["reason"] = json!("invalid_policy");
        return out;
    }
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
    let mut previous_points: BTreeMap<u64, (f64, f64)> = BTreeMap::new();
    let mut previous_puck: Option<(f64, f64)> = None;
    let mut previous_index: Option<u64> = None;
    let mut previous_nearest = None;
    let mut streak = 0usize;
    let mut near_streaks = vec![0usize; policy.distance_screens_feet.len()];
    let mut recent_changes: VecDeque<f64> = VecDeque::new();
    let rows:Vec<_>=frames.iter().map(|frame| {
        let index=frame["frame_index"].as_u64();let adjacent=previous_index.zip(index).is_some_and(|(a,b)|a.checked_add(1)==Some(b));
        if !adjacent {previous_nearest=None;streak=0;near_streaks.fill(0);recent_changes.clear();previous_points.clear();previous_puck=None;}
        let puck=point(&frame["puck"]["marker"]);let players=frame["players"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        let valid:Vec<_>=players.iter().filter_map(|p|{let id=p["player_id"].as_u64()?;if players.iter().filter(|q|q["player_id"].as_u64()==Some(id)).count()!=1{return None}point(&p["marker"]).map(|position|(id,p,position))}).collect();
        let complete=!players.is_empty()&&valid.len()==players.len();
        let mut ranked:Vec<_>=valid.iter().filter_map(|(id,p,pos)|puck.map(|q|(*id,*p,*pos,length(*pos,q)/units))).collect();ranked.sort_by(|a,b|a.3.total_cmp(&b.3).then(a.0.cmp(&b.0)));
        let minimum=ranked.first().map(|r|r.3);let nearest:Vec<_>=ranked.iter().filter(|r|Some(r.3)==minimum).collect();
        let unique=if nearest.len()==1{Some(nearest[0])}else{None};let id=unique.map(|r|r.0);
        let continuing=complete&&puck.is_some()&&id.is_some()&&id==previous_nearest&&adjacent;
        if complete&&id.is_some(){streak=if continuing{streak+1}else{1};}else{streak=0;}
        if !continuing {recent_changes.clear();near_streaks.fill(0);}
        let motion=unique.and_then(|(id,_,position,_)|previous_points.get(id).copied().zip(previous_puck).zip(puck).map(|((before,old_puck),new_puck)|{
            let old_offset=(old_puck.0-before.0,old_puck.1-before.1);let new_offset=(new_puck.0-position.0,new_puck.1-position.1);
            (length(old_offset,new_offset)/units,length(before,*position)/units,length(old_puck,new_puck)/units)
        }));
        if continuing {if let Some((change,_,_))=motion {recent_changes.push_back(change);while recent_changes.len()>policy.minimum_samples-1{recent_changes.pop_front();}}else{recent_changes.clear();}}
        let recent_max=if recent_changes.len()==policy.minimum_samples-1{recent_changes.iter().copied().reduce(f64::max)}else{None};
        let variants:Vec<_>=policy.distance_screens_feet.iter().enumerate().map(|(i,radius)|{
            let inside=complete&&unique.is_some_and(|r|r.3<=*radius);
            near_streaks[i]=if inside {if continuing{near_streaks[i]+1}else{1}}else{0};
            let state=if puck.is_none(){"puck_unavailable"}else if !complete {"incomplete_observations"}else if ranked.is_empty(){"no_player_observations"}else if nearest.len()!=1{"ambiguous_nearest"}else if !inside{"no_nearby_candidate"}else if near_streaks[i]<policy.minimum_samples{"brief_proximity_unresolved"}else if recent_max.is_none(){"relative_motion_unavailable"}else if recent_max.unwrap()>policy.maximum_relative_offset_change_feet{"changing_offset_unresolved"}else{"sustained_control_geometry_candidate"};
            json!({"distance_screen_feet":radius,"state":state,"within_screen_streak_samples":near_streaks[i],"candidate_id":if state=="sustained_control_geometry_candidate"{id}else{None}})
        }).collect();
        let passed=variants.iter().filter(|v|!v["candidate_id"].is_null()).count();
        let opponent=unique.and_then(|r|r.1["team"].as_str().filter(|s|!s.is_empty()).map(|team|ranked.iter().filter(|v|v.1["team"].as_str().is_some_and(|t|!t.is_empty()&&t!=team)).collect::<Vec<_>>()));
        let opponent_min=opponent.as_ref().and_then(|rows|rows.iter().map(|r|r.3).reduce(f64::min));
        let nearest_opponents:Vec<_>=opponent.as_ref().map(|rows|rows.iter().filter(|r|Some(r.3)==opponent_min).map(|r|json!({"player_id":r.0,"name":r.1["name"],"goalie":r.1["goalie"]})).collect()).unwrap_or_default();
        let opponent_context=if unique.is_none_or(|r|r.1["team"].as_str().is_none_or(|s|s.is_empty())){"candidate_team_unknown"}else if ranked.iter().any(|r|r.1["team"].as_str().is_none_or(|s|s.is_empty())){"partial_team_metadata"}else{"recorded_team_metadata_available"};
        let row=json!({"frame_index":frame["frame_index"],"observation_set_status":if complete{"complete_source_observations"}else{"incomplete_source_observations"},
            "nearest_markers":nearest.iter().map(|r|json!({"player_id":r.0,"name":r.1["name"],"team":r.1["team"],"goalie":r.1["goalie"]})).collect::<Vec<_>>(),"nearest_distance_feet_derived":minimum,
            "runner_up_margin_feet_derived":ranked.get(1).zip(minimum).map(|(r,d)|r.3-d),"unique_nearest_streak_samples":streak,
            "opponent_context_status":opponent_context,"nearest_opponent_markers":nearest_opponents,"nearest_opponent_distance_feet_derived":opponent_min,"opponent_margin_feet_derived":opponent_min.zip(minimum).map(|(a,b)|a-b),
            "relative_offset_change_feet_derived":motion.map(|m|m.0),"player_displacement_feet_derived":motion.map(|m|m.1),"puck_displacement_feet_derived":motion.map(|m|m.2),"recent_relative_offset_change_maximum_feet_derived":recent_max,
            "radius_sensitivity":if passed==variants.len(){"candidate_passes_all_screens"}else if passed>0{"candidate_depends_on_distance_screen"}else{"no_screen_identifies_sustained_control"},"variants":variants});
        previous_points=valid.iter().map(|(id,_,p)|(*id,*p)).collect();previous_puck=puck;previous_index=index;previous_nearest=if complete{id}else{None};row
    }).collect();
    out["status"] = json!("experimental_candidates_available");
    out["frames"] = json!(rows);
    out
}
#[cfg(test)]
mod tests {
    use super::*;
    fn g() -> Value {
        json!({"rink_reference":{"native_units_per_foot_derived":1.}})
    }
    fn scene(pucks: &[(f64, f64)], positions: &[(f64, f64)]) -> Value {
        json!({"status":"source_samples_available","frames":pucks.iter().zip(positions).enumerate().map(|(i,(p,s))|json!({"frame_index":i,"puck":{"marker":{"x":p.0,"y":p.1}},"players":[{"player_id":1,"team":"A","goalie":true,"marker":{"x":s.0,"y":s.1}},{"player_id":2,"team":"B","goalie":false,"marker":{"x":100.,"y":100.}}]})).collect::<Vec<_>>()})
    }
    #[test]
    fn co_motion_builds_candidate_and_includes_goalies() {
        let s = scene(
            &[(1., 0.), (2., 0.), (3., 0.)],
            &[(0., 0.), (1., 0.), (2., 0.)],
        );
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(
            r["frames"][0]["variants"][1]["state"],
            "brief_proximity_unresolved"
        );
        assert_eq!(r["frames"][2]["variants"][1]["candidate_id"], 1);
        assert_eq!(r["frames"][2]["nearest_markers"][0]["goalie"], true);
        assert_eq!(r["possession_verified"], false);
    }
    #[test]
    fn constant_radial_distance_with_large_offset_change_is_not_co_motion() {
        let s = scene(&[(3., 0.), (0., 3.), (-3., 0.)], &[(0., 0.); 3]);
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(
            r["frames"][2]["variants"][1]["state"],
            "changing_offset_unresolved"
        );
        assert_eq!(r["frames"][2]["nearest_distance_feet_derived"], 3.);
    }
    #[test]
    fn distant_nearest_player_has_no_candidate() {
        let s = scene(&[(10., 0.); 3], &[(0., 0.); 3]);
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(
            r["frames"][2]["variants"][1]["state"],
            "no_nearby_candidate"
        );
        assert!(r["frames"][2]["variants"][1]["candidate_id"].is_null());
    }
    #[test]
    fn ties_missing_puck_and_gaps_do_not_carry_state() {
        let mut s = scene(&[(1., 0.); 5], &[(0., 0.); 5]);
        s["frames"][1]["puck"]["marker"] = Value::Null;
        s["frames"][3]["players"][1]["marker"] = json!({"x":2.,"y":0.});
        s["frames"][4]["frame_index"] = json!(8);
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(r["frames"][2]["unique_nearest_streak_samples"], 1);
        assert_eq!(r["frames"][3]["variants"][1]["state"], "ambiguous_nearest");
        assert_eq!(r["frames"][4]["unique_nearest_streak_samples"], 1);
    }
    #[test]
    fn radius_sensitivity_and_invalid_policy_are_explicit() {
        let s = scene(&[(5., 0.); 3], &[(0., 0.); 3]);
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(
            r["frames"][2]["radius_sensitivity"],
            "candidate_depends_on_distance_screen"
        );
        let mut policy = Policy::default();
        policy.minimum_samples = 0;
        assert_eq!(assess(&s, &g(), &policy)["reason"], "invalid_policy");
    }
    #[test]
    fn missing_or_duplicate_player_data_prevents_promotion() {
        let mut s = scene(&[(1., 0.); 3], &[(0., 0.); 3]);
        let duplicate = s["frames"][2]["players"][0].clone();
        s["frames"][2]["players"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(
            r["frames"][2]["variants"][1]["state"],
            "incomplete_observations"
        );
        assert!(r["frames"][2]["variants"][1]["candidate_id"].is_null());
    }
    #[test]
    fn teammate_runner_up_is_distinct_from_nearest_opponent() {
        let mut s = scene(&[(1., 0.); 3], &[(0., 0.); 3]);
        for f in s["frames"].as_array_mut().unwrap() {
            f["players"][1]["team"] = json!("A");
            f["players"][1]["marker"] = json!({"x":3.,"y":0.});
            f["players"]
                .as_array_mut()
                .unwrap()
                .push(json!({"player_id":3,"team":"B","marker":{"x":8.,"y":0.}}));
        }
        let r = assess(&s, &g(), &Policy::default());
        assert_eq!(r["frames"][2]["runner_up_margin_feet_derived"], 1.);
        assert_eq!(r["frames"][2]["opponent_margin_feet_derived"], 6.);
    }
}
