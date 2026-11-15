//! Official scoring credits and separately sourced action descriptions.
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

pub fn assess(event: &Value, annotation: &Value) -> Result<Value> {
    let scorer = event["goal"]["scorer_id"].as_u64().context("scorer")?;
    let assists = event["goal"]["assists"].as_array().context("assists")?;
    let details = &event["official_goal_details"];
    anyhow::ensure!(
        details["scoringPlayerId"].as_u64() == Some(scorer),
        "Conflicting scorer identities"
    );
    let primary = details["assist1PlayerId"].as_u64();
    let secondary = details["assist2PlayerId"].as_u64();
    anyhow::ensure!(
        secondary.is_none() || primary.is_some(),
        "Secondary assist without primary"
    );
    let official: Vec<_> = [primary, secondary].into_iter().flatten().collect();
    let ids: BTreeSet<_> = official.iter().copied().collect();
    let listed: BTreeSet<_> = assists
        .iter()
        .filter_map(|a| a["player_id"].as_u64())
        .collect();
    anyhow::ensure!(
        ids.len() == official.len()
            && !ids.contains(&scorer)
            && ids == listed
            && listed.len() == assists.len(),
        "Conflicting assist identities across official sources"
    );
    let mut slots = vec![];
    if let Some(id) = secondary {
        slots.push((id, "secondary assist", "assist2PlayerId"));
    }
    if let Some(id) = primary {
        slots.push((id, "primary assist", "assist1PlayerId"));
    }
    slots.push((scorer, "goal scorer", "scoringPlayerId"));
    let notes = annotation["contributor_actions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let mut seen = BTreeSet::new();
    for note in &notes {
        let id = note["player_id"]
            .as_u64()
            .context("contributor player_id")?;
        anyhow::ensure!(
            seen.insert(id) && slots.iter().any(|(p, _, _)| *p == id),
            "Duplicate or uncredited contributor action"
        );
        let sources = note["source_ids"]
            .as_array()
            .context("contributor source_ids")?;
        anyhow::ensure!(
            !sources.is_empty()
                && sources.iter().all(|source| annotation["sources"]
                    .as_array()
                    .is_some_and(|all| all.iter().any(|s| s["id"] == *source))),
            "Contributor action lacks a linked annotation source"
        );
        anyhow::ensure!(
            note["action"]
                .as_str()
                .is_some_and(|a| !a.trim().is_empty()),
            "Empty contributor action"
        );
    }
    let frames = event["frames"].as_array().context("frames")?;
    let participants:Vec<_>=slots.into_iter().map(|(id,credit,field)|{
        let name=if id==scorer {event["goal"]["scorer_name"].clone()} else {assists.iter().find(|a|a["player_id"].as_u64()==Some(id)).unwrap()["name"].clone()};
        let note=notes.iter().find(|n|n["player_id"].as_u64()==Some(id));
        let observed:Vec<_>=frames.iter().filter(|f|f["observations"].as_array().is_some_and(|o|o.iter().any(|p|p["player_id"].as_u64()==Some(id)))).map(|f|f["frame"].clone()).collect();
        let nearest:Vec<_>=frames.iter().filter(|f|f["nearest_puck_observation"]["player_id"].as_u64()==Some(id)).map(|f|f["frame"].clone()).collect();
        json!({"player_id":id,"name":name,"official_credit":credit,"credit_source_field":field,
            "action_status":if note.is_some(){"documentary-supported"}else{"unassessed"},
            "action":note.map(|n|n["action"].clone()),"action_source_ids":note.map(|n|n["source_ids"].clone()),
            "observed_frames":observed,"puck_nearest_frames":nearest,
            "observation_interpretation":"Frame presence and nearest body point are not possession, an inferred pass, an assignment or verified ice membership."})
    }).collect();
    Ok(
        json!({"schema":"icelines.goal-contributors.v0.1","status":"official_credit_order_verified","participants":participants,
        "interpretation":"Official credit order, not a reconstructed sequence of passes or causal importance ranking. Contributor actions need separate evidence; unassisted does not mean unsupported."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn event() -> Value {
        json!({"goal":{"scorer_id":1,"scorer_name":"Finisher","assists":[{"player_id":3,"name":"Earlier"},{"player_id":2,"name":"Last"}]},"official_goal_details":{"scoringPlayerId":1,"assist1PlayerId":2,"assist2PlayerId":3},"frames":[]})
    }
    #[test]
    fn official_slots_override_array_order_without_inventing_actions() {
        let a = assess(&event(), &Value::Null).unwrap();
        let p = a["participants"].as_array().unwrap();
        assert_eq!(
            p.iter()
                .map(|p| p["player_id"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            vec![3, 2, 1]
        );
        assert!(p
            .iter()
            .all(|p| p["action"].is_null() && p["action_status"] == "unassessed"));
    }
    #[test]
    fn action_sources_and_credit_identities_must_agree() {
        let mut e = event();
        let note = json!({"sources":[{"id":"SOURCE"}],"contributor_actions":[{"player_id":2,"action":"Shot delivery","source_ids":["SOURCE"]}]});
        assert_eq!(
            assess(&e, &note).unwrap()["participants"][1]["action"],
            "Shot delivery"
        );
        e["official_goal_details"]["assist2PlayerId"] = json!(2);
        assert!(assess(&e, &note).is_err());
        let mut note = note;
        note["contributor_actions"][0]["source_ids"] = json!(["MISSING"]);
        assert!(assess(&event(), &note).is_err());
    }
    #[test]
    fn unassisted_goal_keeps_finisher_without_claiming_no_support() {
        let mut e = event();
        e["goal"]["assists"] = json!([]);
        e["official_goal_details"] = json!({"scoringPlayerId":1});
        let a = assess(&e, &Value::Null).unwrap();
        assert_eq!(a["participants"].as_array().unwrap().len(), 1);
        assert_eq!(a["participants"][0]["official_credit"], "goal scorer");
    }
}
