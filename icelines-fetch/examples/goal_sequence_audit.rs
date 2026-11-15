//! Audit existing typed Goal Visualizer bundles without inventing units,
//! possession, active-ice membership, goal-frame alignment or coaching duties.
use anyhow::{Context, Result};
use icelines_fetch::goal_visualizer::{load_bundle, parse_frames, GoalTrackedObject};
use serde_json::{json, Value};
use std::{collections::BTreeMap, path::Path};
fn read(path: &str) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(path)?)?)
}
fn distance(a: &GoalTrackedObject, b: &GoalTrackedObject) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn main() -> Result<()> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        a.len() == 5,
        "usage: goal_sequence_audit BUNDLE LANDING BOXSCORE PLAY_BY_PLAY OUTPUT"
    );
    let b = load_bundle(Path::new(&a[0]))?;
    let landing = read(&a[1])?;
    let boxscore = read(&a[2])?;
    let pbp = read(&a[3])?;
    anyhow::ensure!(
        landing["id"].as_u64() == Some(b.game_id)
            && boxscore["id"].as_u64() == Some(b.game_id)
            && pbp["id"].as_u64() == Some(b.game_id),
        "all inputs must refer to the same game"
    );
    let mut roster = BTreeMap::new();
    for (side, groups) in boxscore["playerByGameStats"]
        .as_object()
        .context("box roster")?
    {
        for (group, players) in groups.as_object().context("roster groups")? {
            for p in players.as_array().context("roster rows")? {
                let id = p["playerId"].as_u64().context("roster player id")?;
                roster.insert(id,json!({"player_id":id,"name":p["name"]["default"],"position":p["position"],"team":boxscore[side]["abbrev"],"goalie":group=="goalies","sweater_number":p["sweaterNumber"]}));
            }
        }
    }
    let mut results = vec![];
    for e in &b.events {
        let frames = parse_frames(&serde_json::to_value(&e.frames)?)?;
        let first = frames.first().context("no frames")?.timestamp;
        let goal = landing["summary"]["scoring"]
            .as_array()
            .context("scoring")?
            .iter()
            .flat_map(|p| p["goals"].as_array().into_iter().flatten())
            .find(|g| g["eventId"].as_u64() == Some(u64::from(e.goal.event_id)))
            .context("goal metadata")?;
        let official = pbp["plays"]
            .as_array()
            .context("plays")?
            .iter()
            .find(|p| {
                p["eventId"].as_u64() == Some(u64::from(e.goal.event_id))
                    && p["typeDescKey"] == "goal"
            })
            .context("goal in play-by-play")?;
        let scorer = e.goal.scorer_id.context("unknown scorer")?;
        let mut deltas = BTreeMap::<u64, usize>::new();
        let mut counts = BTreeMap::<usize, usize>::new();
        let mut observations = BTreeMap::<u32, Vec<usize>>::new();
        let mut rows = vec![];
        for (i, f) in frames.iter().enumerate() {
            if i > 0 {
                *deltas
                    .entry(f.timestamp - frames[i - 1].timestamp)
                    .or_default() += 1;
            }
            let players: Vec<_> = f
                .on_ice
                .values()
                .filter(|o| o.player_id.is_some())
                .collect();
            *counts.entry(players.len()).or_default() += 1;
            let pucks: Vec<_> = f
                .on_ice
                .values()
                .filter(|o| o.id == 1 && o.player_id.is_none())
                .collect();
            anyhow::ensure!(
                pucks.len() == 1,
                "expected exactly one identified puck in event {}, frame {i}",
                e.goal.event_id
            );
            let puck = pucks[0];
            let scorer_obj = players
                .iter()
                .find(|o| o.player_id == Some(scorer))
                .copied();
            let mut near: Vec<_> = players
                .iter()
                .map(|o| (distance(o, puck), o.player_id.unwrap()))
                .collect();
            near.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
            let mut out = vec![];
            for o in &players {
                let id = o.player_id.unwrap();
                anyhow::ensure!(
                    !observations.entry(id).or_default().contains(&i),
                    "duplicate player observation"
                );
                observations.get_mut(&id).unwrap().push(i);
                let metadata = roster
                    .get(&u64::from(id))
                    .context("tracked player absent from official roster")?;
                out.push(json!({"player_id":id,"team":o.team_abbrev,"x":o.x,"y":o.y,"roster":metadata,"distance_to_puck_native":distance(o,puck)}));
            }
            let nearest_defender = scorer_obj.and_then(|s| {
                players
                    .iter()
                    .filter(|o| {
                        o.team_id != s.team_id
                            && !roster[&u64::from(o.player_id.unwrap())]["goalie"]
                                .as_bool()
                                .unwrap()
                    })
                    .map(|o| (distance(s, o), o.player_id.unwrap()))
                    .min_by(|a, b| a.0.total_cmp(&b.0))
            });
            rows.push(json!({"frame":i,"timestamp_native":f.timestamp,"elapsed_native_ticks":f.timestamp-first,"puck":{"x":puck.x,"y":puck.y},"observations":out,
     "nearest_puck_observation":near.first().map(|(d,id)|json!({"player_id":id,"distance_native":d,"possession_inferred":false})),
     "scorer_observed":scorer_obj.is_some(),"scorer_nearest_opposing_skater":nearest_defender.map(|(d,id)|json!({"player_id":id,"distance_native":d,"active_status_verified":false}))}));
        }
        let presence:Vec<_>=observations.iter().map(|(id,indices)|json!({"player_id":id,"roster":roster[&u64::from(*id)],"first_observed_frame":indices[0],"last_observed_frame":indices.last(),"observed_frames":indices.len(),"active_ice_status":null})).collect();
        results.push(json!({"goal":e.goal,"goal_modifier":goal["goalModifier"],"official_goal_details":official["details"],"visualizer_url":format!("https://www.nhl.com/ppt-replay/goal/{}/{}",b.game_id,e.goal.event_id),"tracking_url":e.source_url,"tracking_source_sha256":e.source_sha256,
   "frames":rows,"frame_count":frames.len(),"timestamp_delta_counts":deltas,"native_tick_span":frames.last().unwrap().timestamp-first,"player_observation_counts":counts,"players":presence,"exact_goal_frame":null,"coordinate_units":null,"timestamp_seconds_per_tick":null}));
    }
    results.sort_by_key(|e| {
        (
            e["goal"]["period"].as_u64(),
            e["goal"]["time_in_period"]
                .as_str()
                .unwrap_or("")
                .to_owned(),
        )
    });
    let out = json!({"schema":"icelines.goal-sequence-audit.0.1","game_id":b.game_id,"date":landing["gameDate"],"coordinate_system":b.coordinate_system,"coordinate_note":b.coordinate_note,"events":results,
  "limitations":["Feed-native coordinates/timestamps are not converted to physical units or seconds.","Nearest observed object is not inferred possession, pressure, stick coverage or active-ice status.","Goal moment is not automatically aligned; do not treat final frame as the shot or goal.","Goal clips are outcome-selected and cannot estimate tactic success or failure rates.","Movement alone does not establish a called assignment, awareness, stick/body orientation, puck height or goalie vision."]});
    std::fs::write(&a[4], serde_json::to_vec_pretty(&out)?)?;
    println!(
        "Audited {} timed goal sequences; preserved native units and unknown assignments",
        b.events.len()
    );
    Ok(())
}
