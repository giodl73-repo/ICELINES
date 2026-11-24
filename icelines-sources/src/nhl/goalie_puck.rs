//! Credited goalie puck events, not a complete touch log or handling rating.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Goalie {
    pub player_id: u64,
    pub name: String,
    pub team_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Event {
    pub game_id: u64,
    pub event_id: u64,
    pub player_id: u64,
    pub team_id: u64,
    pub period: u64,
    pub time_in_period: String,
    pub kind: String,
    pub penalty_description: Option<String>,
    /// First opponent shot on goal / goal within ten seconds, before a stoppage.
    /// Temporal context only: this does not establish possession or causation.
    pub following_opponent_attempt: Option<Attempt>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Attempt {
    pub event_id: u64,
    pub kind: String,
    pub seconds_after: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Game {
    pub game_id: u64,
    pub goalies_on_roster: Vec<Goalie>,
    pub events: Vec<Event>,
}
fn clock(v: &Value) -> Result<u64, String> {
    let (m, s) = v
        .as_str()
        .and_then(|s| s.split_once(':'))
        .ok_or("Missing game clock")?;
    let m: u64 = m.parse().map_err(|_| "Invalid minutes")?;
    let s: u64 = s.parse().map_err(|_| "Invalid seconds")?;
    if m > 20 || s >= 60 || (m == 20 && s != 0) {
        return Err("Invalid game clock".into());
    }
    Ok(m * 60 + s)
}
pub fn parse(bytes: &[u8], game_id: u64, season: u32) -> Result<Game, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if v["id"].as_u64() != Some(game_id)
        || v["season"].as_u64() != Some(season as u64)
        || v["gameType"].as_u64() != Some(2)
        || !matches!(v["gameState"].as_str(), Some("OFF" | "FINAL"))
    {
        return Err("Expected the requested finalized regular-season game".into());
    }
    let mut goalies = BTreeMap::new();
    let mut actors = BTreeSet::new();
    for r in v["rosterSpots"].as_array().ok_or("Missing roster")? {
        let id = r["playerId"].as_u64().ok_or("Missing roster player")?;
        actors.insert(id);
        if r["positionCode"] == "G" {
            let goalie = Goalie {
                player_id: id,
                name: format!(
                    "{} {}",
                    r["firstName"]["default"].as_str().unwrap_or(""),
                    r["lastName"]["default"].as_str().unwrap_or("")
                ),
                team_id: r["teamId"].as_u64().ok_or("Missing goalie team")?,
            };
            if goalies.insert(id, goalie).is_some() {
                return Err("Duplicate goalie roster entry".into());
            }
        }
    }
    if goalies.is_empty() {
        return Err("No goalie identities in roster".into());
    }
    let mut plays: Vec<&Value> = v["plays"]
        .as_array()
        .ok_or("Missing plays")?
        .iter()
        .collect();
    if plays.is_empty() {
        return Err("Finalized game has an empty play log".into());
    }
    plays.sort_by_key(|p| p["sortOrder"].as_u64().unwrap_or(u64::MAX));
    let mut seen = BTreeSet::new();
    let mut orders = BTreeSet::new();
    for p in &plays {
        let id = p["eventId"].as_u64().ok_or("Missing event identity")?;
        if !seen.insert(id) || !p["sortOrder"].as_u64().is_some_and(|n| orders.insert(n)) {
            return Err("Duplicate/missing event order".into());
        }
    }
    let mut events = vec![];
    for (index, p) in plays.iter().enumerate() {
        // Shootout attempts are not regular gameplay goals or puck-handling events.
        if p["periodDescriptor"]["periodType"] == "SO" {
            continue;
        }
        let d = &p["details"];
        let credited: &[(&str, &str)] = match p["typeDescKey"].as_str() {
            Some("giveaway") => &[("playerId", "giveaway")],
            Some("takeaway") => &[("playerId", "takeaway")],
            Some("goal") => &[
                ("scoringPlayerId", "goal"),
                ("assist1PlayerId", "primary_assist"),
                ("assist2PlayerId", "secondary_assist"),
            ],
            Some("penalty") => &[("committedByPlayerId", "penalty")],
            _ => continue,
        };
        for &(field, kind) in credited {
            let Some(id) = d[field].as_u64() else {
                if matches!(kind, "giveaway" | "takeaway" | "goal") {
                    return Err("Credited event has no actor".into());
                }
                continue; // assists are optional; team penalties have no player actor.
            };
            if !actors.contains(&id) {
                return Err(format!("Credited actor {id} absent from roster"));
            }
            let Some(g) = goalies.get(&id) else {
                continue;
            };
            if d["eventOwnerTeamId"].as_u64() != Some(g.team_id) {
                return Err("Goalie event team differs from roster".into());
            }
            let period = p["periodDescriptor"]["number"]
                .as_u64()
                .ok_or("Missing period")?;
            let time = clock(&p["timeInPeriod"])?;
            let mut following = None;
            if kind == "giveaway" {
                for next in plays.iter().skip(index + 1) {
                    if next["periodDescriptor"]["number"].as_u64() != Some(period) {
                        break;
                    }
                    let t = clock(&next["timeInPeriod"])?;
                    if t < time {
                        return Err("Game clock moved backwards".into());
                    }
                    if t - time > 10 {
                        break;
                    }
                    let k = next["typeDescKey"].as_str().unwrap_or("");
                    if matches!(
                        k,
                        "stoppage" | "faceoff" | "period-end" | "game-end" | "penalty"
                    ) {
                        break;
                    }
                    if matches!(k, "shot-on-goal" | "goal") {
                        if next["details"]["eventOwnerTeamId"]
                            .as_u64()
                            .is_some_and(|team| team != g.team_id)
                        {
                            following = Some(Attempt {
                                event_id: next["eventId"].as_u64().ok_or("Missing attempt id")?,
                                kind: k.into(),
                                seconds_after: t - time,
                            });
                        }
                        if following.is_some() || k == "goal" {
                            break;
                        }
                    }
                }
            }
            events.push(Event {
                game_id,
                event_id: p["eventId"].as_u64().unwrap(),
                player_id: id,
                team_id: g.team_id,
                period,
                time_in_period: p["timeInPeriod"].as_str().unwrap().into(),
                kind: kind.into(),
                penalty_description: if kind == "penalty" {
                    d["descKey"].as_str().map(str::to_owned)
                } else {
                    None
                },
                following_opponent_attempt: following,
            });
        }
    }
    Ok(Game {
        game_id,
        goalies_on_roster: goalies.into_values().collect(),
        events,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn fixture() -> Value {
        json!({"id":2026020001_u64,"season":20262027,"gameType":2,"gameState":"OFF",
        "rosterSpots":[{"playerId":1,"positionCode":"G","teamId":10,"firstName":{"default":"Test"},"lastName":{"default":"Goalie"}},
        {"playerId":2,"positionCode":"C","teamId":10}],"plays":[
        {"eventId":1,"sortOrder":1,"periodDescriptor":{"number":1,"periodType":"REG"},"timeInPeriod":"01:00","typeDescKey":"giveaway","details":{"playerId":1,"eventOwnerTeamId":10}},
        {"eventId":2,"sortOrder":2,"periodDescriptor":{"number":1,"periodType":"REG"},"timeInPeriod":"01:05","typeDescKey":"shot-on-goal","details":{"eventOwnerTeamId":20}},
        {"eventId":3,"sortOrder":3,"periodDescriptor":{"number":1,"periodType":"REG"},"timeInPeriod":"02:00","typeDescKey":"goal","details":{"scoringPlayerId":2,"assist2PlayerId":1,"eventOwnerTeamId":10}}
        ]})
    }
    fn run(v: &Value) -> Result<Game, String> {
        parse(&serde_json::to_vec(v).unwrap(), 2026020001, 20262027)
    }
    #[test]
    fn credited_goalie_actors_and_temporal_context() {
        let g = run(&fixture()).unwrap();
        assert_eq!(g.events.len(), 2);
        assert_eq!(g.events[1].kind, "secondary_assist");
        assert_eq!(
            g.events[0]
                .following_opponent_attempt
                .as_ref()
                .unwrap()
                .seconds_after,
            5
        );
    }
    #[test]
    fn stoppage_late_attempt_and_period_change_break_context() {
        for (key, value) in [
            ("typeDescKey", json!("stoppage")),
            ("timeInPeriod", json!("01:11")),
            ("periodDescriptor", json!({"number":2,"periodType":"REG"})),
        ] {
            let mut v = fixture();
            v["plays"][1][key] = value;
            assert!(run(&v).unwrap().events[0]
                .following_opponent_attempt
                .is_none());
        }
    }
    #[test]
    fn rejects_wrong_scope_duplicates_and_unknown_actors() {
        for (key, value) in [
            ("id", json!(3)),
            ("season", json!(20252026)),
            ("gameType", json!(3)),
            ("gameState", json!("LIVE")),
        ] {
            let mut v = fixture();
            v[key] = value;
            assert!(run(&v).is_err());
        }
        let mut v = fixture();
        v["plays"][1]["eventId"] = json!(1);
        assert!(run(&v).is_err());
        let mut v = fixture();
        v["plays"][0]["details"]["playerId"] = json!(999);
        assert!(run(&v).is_err());
    }
    #[test]
    fn backup_roster_is_not_an_appearance_and_shootout_is_excluded() {
        let mut v = fixture();
        v["plays"][0]["details"]["playerId"] = json!(2);
        v["plays"][2]["periodDescriptor"]["periodType"] = json!("SO");
        let g = run(&v).unwrap();
        assert_eq!(g.goalies_on_roster.len(), 1);
        assert!(g.events.is_empty());
    }
}
