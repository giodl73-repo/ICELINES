//! Same-sample point separations, not contact, pressure or assigned coverage.
use serde_json::{json, Value};
fn point(v: &Value) -> Option<(f64, f64)> {
    v["x"].as_f64().zip(v["y"].as_f64())
}
fn separation(a: Option<(f64, f64)>, b: Option<(f64, f64)>, units: f64) -> Option<f64> {
    a.zip(b)
        .map(|((x, y), (u, v))| (x - u).hypot(y - v) / units)
}
fn nearest(rows: &[Value], field: &str) -> Value {
    let possible: Vec<_> = rows.iter().filter(|r| r["goalie"] != true).collect();
    let measured: Vec<_> = possible
        .iter()
        .filter(|r| r["status"] == "available")
        .filter_map(|r| r[field].as_f64().map(|n| (*r, n)))
        .collect();
    let minimum = measured.iter().map(|(_, n)| *n).reduce(f64::min);
    json!({"status":if minimum.is_none(){"unavailable"}else if measured.len()!=possible.len(){"partial_observed_skater_set"}else{"available_observed_skater_set"},"distance_feet_derived":minimum,
        "markers":measured.iter().filter(|(_,n)|Some(*n)==minimum).map(|(r,_)|json!({"source_observation_index":r["source_observation_index"],"player_id":r["player_id"],"name":r["name"]})).collect::<Vec<_>>()})
}
pub fn assess(scene: &Value, geometry: &Value) -> Value {
    let mut result = json!({"schema":"icelines.goal-sample-proximity.v0.1","status":"unavailable","frames":[],"interpretation":"Same-source-sample opposing-player point separations using the regulation-derived scale. No contact, stick/body reach, possession, pressure, called duty, active occupancy, ability or probability assessment. Observed goalies have distances but are excluded from skater minima. Unknown/ambiguous context remains unavailable; no position substitution."});
    let Some(units) = geometry["rink_reference"]["native_units_per_foot_derived"]
        .as_f64()
        .filter(|u| u.is_finite() && *u > 0.)
    else {
        result["reason"] = json!("derived_scale_unavailable");
        return result;
    };
    let Some(team) = scene["scoring_team"].as_str().filter(|t| !t.is_empty()) else {
        result["reason"] = json!("scoring_team_unavailable");
        return result;
    };
    let Some(frames) = scene["frames"]
        .as_array()
        .filter(|_| scene["status"] == "source_samples_available")
    else {
        result["reason"] = json!("source_samples_unavailable");
        return result;
    };
    let scorer = scene["scorer_id"].as_u64();
    let rows:Vec<_>=frames.iter().map(|frame|{
        let players=frame["players"].as_array().map(Vec::as_slice).unwrap_or(&[]);
        let matches:Vec<_>=players.iter().filter(|p|scorer.is_some()&&p["player_id"].as_u64()==scorer).collect();
        let scorer_status=if scorer.is_none(){"identity_unavailable"}else if matches.is_empty(){"not_observed"}else if matches.len()!=1{"identity_ambiguous"}else if matches[0]["team"].as_str()!=Some(team){"team_context_unavailable"}else if point(&matches[0]["marker"]).is_none(){"coordinates_unavailable"}else{"available"};
        let scorer_point=if scorer_status=="available"{point(&matches[0]["marker"])}else{None};let puck=point(&frame["puck"]["marker"]);
        let opponents:Vec<_>=players.iter().filter(|p|p["team"].as_str()!=Some(team)).map(|p|{
            let id=p["player_id"].as_u64();
            let status=if p["team"].as_str().is_none_or(|s|s.is_empty()){"team_context_unavailable"}else if id.is_none(){"identity_unavailable"}else if players.iter().filter(|q|q["player_id"].as_u64()==id).count()!=1{"identity_ambiguous"}else if p["goalie"].as_bool().is_none(){"role_context_unavailable"}else if point(&p["marker"]).is_none(){"coordinates_unavailable"}else{"available"};
            let marker=if status=="available"{point(&p["marker"])}else{None};
            json!({"source_observation_index":p["source_observation_index"],"player_id":p["player_id"],"name":p["name"],"team":p["team"],"goalie":p["goalie"],"status":status,
                "puck_separation_feet_derived":separation(marker,puck,units),"scorer_separation_feet_derived":separation(marker,scorer_point,units)})
        }).collect();
        json!({"frame_index":frame["frame_index"],"scorer_status":scorer_status,"puck_status":if puck.is_some(){"available"}else{"coordinates_unavailable"},"nearest_puck_skater":nearest(&opponents,"puck_separation_feet_derived"),"nearest_scorer_skater":nearest(&opponents,"scorer_separation_feet_derived"),"opponents":opponents})
    }).collect();
    result["status"] = json!("sample_separations_available");
    result["frames"] = json!(rows);
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    fn inputs() -> (Value, Value) {
        let p = |id, team, x, goalie| json!({"player_id":id,"team":team,"name":format!("P{id}"),"source_observation_index":id,"marker":{"x":x,"y":0.},"goalie":goalie});
        (
            json!({"status":"source_samples_available","scoring_team":"A","scorer_id":1,"frames":[{"frame_index":7,"puck":{"marker":{"x":10.,"y":0.}},"players":[p(1,"A",0.,false),p(2,"B",9.,false),p(3,"B",2.,false),p(4,"B",10.,true)]}]}),
            json!({"rink_reference":{"native_units_per_foot_derived":1.}}),
        )
    }
    #[test]
    fn different_skater_minima_and_goalie_exclusion_are_explicit() {
        let (s, g) = inputs();
        let r = assess(&s, &g);
        let f = &r["frames"][0];
        assert_eq!(f["nearest_puck_skater"]["markers"][0]["player_id"], 2);
        assert_eq!(f["nearest_scorer_skater"]["markers"][0]["player_id"], 3);
        assert_eq!(f["opponents"][2]["puck_separation_feet_derived"], 0.);
        assert_eq!(f["opponents"][2]["goalie"], true);
    }
    #[test]
    fn missing_puck_does_not_erase_valid_scorer_separations() {
        let (mut s, g) = inputs();
        s["frames"][0]["puck"]["marker"] = Value::Null;
        let r = assess(&s, &g);
        let f = &r["frames"][0];
        assert_eq!(f["nearest_puck_skater"]["status"], "unavailable");
        assert_eq!(f["nearest_scorer_skater"]["distance_feet_derived"], 2.);
    }
    #[test]
    fn duplicates_make_minima_partial_without_substitution() {
        let (mut s, g) = inputs();
        let mut dupe = s["frames"][0]["players"][1].clone();
        dupe["source_observation_index"] = json!(9);
        s["frames"][0]["players"].as_array_mut().unwrap().push(dupe);
        let r = assess(&s, &g);
        let f = &r["frames"][0];
        assert_eq!(f["opponents"][0]["status"], "identity_ambiguous");
        assert!(f["opponents"][0]["puck_separation_feet_derived"].is_null());
        assert_eq!(
            f["nearest_puck_skater"]["status"],
            "partial_observed_skater_set"
        );
    }
    #[test]
    fn ties_scale_and_missing_scorer_are_preserved() {
        let (mut s, mut g) = inputs();
        s["frames"][0]["players"][2]["marker"]["x"] = json!(11.);
        g["rink_reference"]["native_units_per_foot_derived"] = json!(2.);
        let r = assess(&s, &g);
        assert_eq!(
            r["frames"][0]["nearest_puck_skater"]["markers"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            r["frames"][0]["nearest_puck_skater"]["distance_feet_derived"],
            0.5
        );
        s["scorer_id"] = Value::Null;
        assert_eq!(
            assess(&s, &g)["frames"][0]["nearest_scorer_skater"]["status"],
            "unavailable"
        );
        g["rink_reference"] = Value::Null;
        assert_eq!(assess(&s, &g)["reason"], "derived_scale_unavailable");
    }
}
