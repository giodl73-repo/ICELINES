//! Official scoring credits and separately sourced action descriptions.
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::collections::BTreeSet;

#[derive(Clone)]
struct Run {
    player: Option<u64>,
    first: u64,
    last: u64,
}

pub(super) fn ordered_anchors(frames: &[Value], order: &[u64]) -> Value {
    let mut runs: Vec<Run> = vec![];
    for frame in frames {
        let player = frame["nearest_puck_observation"]["player_id"].as_u64();
        let index = frame["frame"].as_u64().expect("audited frame index");
        if let Some(last) = runs.last_mut().filter(|last| last.player == player) {
            last.last = index;
        } else {
            runs.push(Run {
                player,
                first: index,
                last: index,
            });
        }
    }
    // Count every ordered match, preserving ambiguity instead of choosing a
    // 'best' contact from body-point proximity. Zero matches do not refute credit.
    let mut ways = vec![1u64; runs.len() + 1];
    let mut saturated = false;
    for player in order {
        let mut next = vec![0u64; runs.len() + 1];
        for (i, run) in runs.iter().enumerate() {
            let extra = if run.player == Some(*player) {
                ways[i]
            } else {
                0
            };
            next[i + 1] = next[i].checked_add(extra).unwrap_or_else(|| {
                saturated = true;
                u64::MAX
            });
        }
        ways = next;
    }
    fn examples(
        runs: &[Run],
        order: &[u64],
        start: usize,
        path: &mut Vec<usize>,
        out: &mut Vec<Value>,
    ) {
        if out.len() == 16 {
            return;
        }
        if path.len() == order.len() {
            let anchors:Vec<_>=path.iter().map(|i|json!({"player_id":runs[*i].player,"first_frame":runs[*i].first,"last_frame":runs[*i].last})).collect();
            let bridges:Vec<_>=path.windows(2).map(|pair|{
                let middle:Vec<_>=runs[pair[0]+1..pair[1]].iter().map(|r|json!({"player_id":r.player,"first_frame":r.first,"last_frame":r.last})).collect();
                json!({"from_player_id":runs[pair[0]].player,"to_player_id":runs[pair[1]].player,"intervening_nearest_body_runs":middle})
            }).collect();
            out.push(json!({"anchors":anchors,"bridges":bridges}));
            return;
        }
        for i in start..runs.len() {
            if runs[i].player == Some(order[path.len()]) {
                path.push(i);
                examples(runs, order, i + 1, path, out);
                path.pop();
                if out.len() == 16 {
                    return;
                }
            }
        }
    }
    let count = *ways.last().unwrap();
    let mut matches = vec![];
    if count > 0 {
        examples(&runs, order, 0, &mut vec![], &mut matches);
    }
    json!({"status":if count>0{"ordered_geometry_candidates"}else{"no_complete_nearest_body_alignment"},
        "candidate_count":count,"count_saturated":saturated,"example_limit":16,"examples":matches,
        "interpretation":"Official credited touch order constrains the search. These are candidate nearest-body intervals, not contact times or a pass detector. Other nearby players do not establish intervening touches. Missing alignment does not contradict official credits; goal-frame alignment remains unverified."})
}

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
    let touch_order: Vec<_> = slots.iter().map(|(id, _, _)| *id).collect();
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
        json!({"schema":"icelines.goal-contributors.v0.2","status":"official_credited_touch_order_verified","credited_touch_order":touch_order,"participants":participants,
        "ordered_tracking_anchors":ordered_anchors(frames,&touch_order),
        "interpretation":"Secondary assist precedes primary assist, which precedes the scoring touch in the credited touch order. This does not enumerate every touch or identify each action, exact contact time or creative importance. Unassisted does not mean unsupported."}),
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
    fn frame(i: u64, id: Option<u64>) -> Value {
        json!({"frame":i,"nearest_puck_observation":{"player_id":id}})
    }
    #[test]
    fn credited_order_preserves_multiple_candidates_and_intervening_proximity() {
        let frames = vec![
            frame(0, Some(3)),
            frame(1, Some(9)),
            frame(2, Some(3)),
            frame(3, Some(2)),
            frame(4, Some(8)),
            frame(5, Some(1)),
        ];
        let a = ordered_anchors(&frames, &[3, 2, 1]);
        assert_eq!(a["candidate_count"], 2);
        assert_eq!(a["examples"][1]["anchors"][0]["first_frame"], 2);
        assert_eq!(
            a["examples"][1]["bridges"][1]["intervening_nearest_body_runs"][0]["player_id"],
            8
        );
        assert_eq!(ordered_anchors(&frames, &[2, 3, 1])["candidate_count"], 0);
    }
    #[test]
    fn missing_primary_anchor_does_not_invent_a_touch() {
        let a = ordered_anchors(
            &[frame(0, Some(3)), frame(1, None), frame(2, Some(1))],
            &[3, 2, 1],
        );
        assert_eq!(a["candidate_count"], 0);
        assert!(a["examples"].as_array().unwrap().is_empty());
        assert_eq!(ordered_anchors(&[], &[1])["candidate_count"], 0);
    }
    #[test]
    fn examples_are_bounded_while_all_alternatives_are_counted() {
        let mut f = vec![];
        for i in 0..20 {
            f.push(frame(i * 2, Some(3)));
            f.push(frame(i * 2 + 1, Some(9)));
        }
        f.push(frame(40, Some(2)));
        f.push(frame(41, Some(1)));
        let a = ordered_anchors(&f, &[3, 2, 1]);
        assert_eq!(a["candidate_count"], 20);
        assert_eq!(a["examples"].as_array().unwrap().len(), 16);
    }
}
