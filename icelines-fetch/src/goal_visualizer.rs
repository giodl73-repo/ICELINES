//! NHL EDGE Goal Visualizer discovery and feed-native tracking frames.
//!
//! The normal play-by-play endpoint exposes event coordinates, not player and
//! puck movement. Selected goals in the Gamecenter landing response carry a
//! `pptReplayUrl` whose public JSON payload contains timestamped `onIce`
//! objects. Coverage is substantial but optional. Coordinates are deliberately
//! preserved in the feed's undocumented native system; this module does not
//! claim rink units or normalize orientation.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

pub const BUNDLE_SCHEMA: &str = "icelines.goal-visualizer.v0.1";
pub const COORDINATE_SYSTEM: &str = "nhl-edge-goal-visualizer-feed-native-unspecified";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalVisualizerAssist {
    pub player_id: Option<u32>,
    pub name: Option<String>,
    pub sweater_number: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalVisualizerGoal {
    pub event_id: u32,
    pub period: u8,
    pub period_type: String,
    pub time_in_period: String,
    pub situation_code: Option<String>,
    pub strength: Option<String>,
    pub scorer_id: Option<u32>,
    pub scorer_name: Option<String>,
    pub team_abbrev: Option<String>,
    pub shot_type: Option<String>,
    pub home_team_defending_side: Option<String>,
    pub assists: Vec<GoalVisualizerAssist>,
    pub replay_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalTrackedObject {
    pub id: u64,
    #[serde(rename = "playerId")]
    pub player_id: Option<u32>,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "sweaterNumber")]
    pub sweater_number: Option<u16>,
    #[serde(rename = "teamId")]
    pub team_id: Option<u32>,
    #[serde(rename = "teamAbbrev")]
    pub team_abbrev: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalTrackingFrame {
    #[serde(rename = "timeStamp")]
    pub timestamp: u64,
    #[serde(rename = "onIce")]
    pub on_ice: BTreeMap<String, GoalTrackedObject>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalVisualizerEvent {
    pub goal: GoalVisualizerGoal,
    pub source_url: String,
    #[serde(default)]
    pub source_sha256: String,
    pub frame_count: usize,
    pub tracked_player_ids: Vec<u32>,
    pub puck_object_observed: bool,
    pub frames: Vec<GoalTrackingFrame>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GoalVisualizerBundle {
    pub schema: String,
    pub game_id: u64,
    pub coordinate_system: String,
    pub coordinate_note: String,
    pub fetched_at: DateTime<Utc>,
    pub events: Vec<GoalVisualizerEvent>,
}

impl GoalVisualizerBundle {
    pub fn new(game_id: u64, fetched_at: DateTime<Utc>) -> Self {
        Self {
            schema: BUNDLE_SCHEMA.to_string(),
            game_id,
            coordinate_system: COORDINATE_SYSTEM.to_string(),
            coordinate_note: "Coordinates are preserved exactly as supplied; no rink-unit or orientation transform is asserted.".to_string(),
            fetched_at,
            events: Vec::new(),
        }
    }

    pub fn upsert_event(&mut self, event: GoalVisualizerEvent) {
        self.events
            .retain(|existing| existing.goal.event_id != event.goal.event_id);
        self.events.push(event);
        self.events
            .sort_by_key(|event| (event.goal.period, event.goal.event_id));
    }
}

#[derive(Debug, thiserror::Error)]
pub enum GoalVisualizerError {
    #[error("Gamecenter landing payload has no summary.scoring array")]
    MissingScoringSummary,
    #[error("Goal Visualizer replay URL is not an allowed NHL sprite URL: {0}")]
    ReplayUrlNotAllowed(String),
    #[error("Goal Visualizer payload must be a non-empty JSON frame array")]
    EmptyOrInvalidFrames,
    #[error("Goal Visualizer frame {frame} is missing {field}")]
    MissingFrameField { frame: usize, field: &'static str },
    #[error("Goal Visualizer frame timestamps move backwards at frame {frame}")]
    NonMonotonicTimestamps { frame: usize },
    #[error("Goal Visualizer frame {frame} object '{object}' is missing {field}")]
    MissingObjectField {
        frame: usize,
        object: String,
        field: &'static str,
    },
}

pub fn discover_goals(
    landing: &serde_json::Value,
) -> std::result::Result<Vec<GoalVisualizerGoal>, GoalVisualizerError> {
    let scoring = landing["summary"]["scoring"]
        .as_array()
        .ok_or(GoalVisualizerError::MissingScoringSummary)?;
    let mut goals = Vec::new();
    for period_block in scoring {
        let period = period_block["periodDescriptor"]["number"]
            .as_u64()
            .unwrap_or(0) as u8;
        let period_type = period_block["periodDescriptor"]["periodType"]
            .as_str()
            .unwrap_or("REG")
            .to_string();
        let Some(rows) = period_block["goals"].as_array() else {
            continue;
        };
        for row in rows {
            let Some(event_id) = row["eventId"].as_u64() else {
                continue;
            };
            let assists = row["assists"]
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|assist| GoalVisualizerAssist {
                            player_id: value_as_u64(&assist["playerId"]).map(|v| v as u32),
                            name: localized_name(assist),
                            sweater_number: value_as_u64(&assist["sweaterNumber"])
                                .map(|v| v as u16),
                        })
                        .collect()
                })
                .unwrap_or_default();
            goals.push(GoalVisualizerGoal {
                event_id: event_id as u32,
                period,
                period_type: period_type.clone(),
                time_in_period: row["timeInPeriod"].as_str().unwrap_or("").to_string(),
                situation_code: row["situationCode"].as_str().map(str::to_string),
                strength: row["strength"].as_str().map(str::to_string),
                scorer_id: value_as_u64(&row["playerId"]).map(|v| v as u32),
                scorer_name: localized_name(row),
                team_abbrev: localized_default(&row["teamAbbrev"]),
                shot_type: row["shotType"].as_str().map(str::to_string),
                home_team_defending_side: row["homeTeamDefendingSide"].as_str().map(str::to_string),
                assists,
                replay_url: row["pptReplayUrl"].as_str().map(str::to_string),
            });
        }
    }
    Ok(goals)
}

pub fn parse_frames(
    raw: &serde_json::Value,
) -> std::result::Result<Vec<GoalTrackingFrame>, GoalVisualizerError> {
    let rows = raw
        .as_array()
        .filter(|rows| !rows.is_empty())
        .ok_or(GoalVisualizerError::EmptyOrInvalidFrames)?;
    let mut frames = Vec::with_capacity(rows.len());
    let mut previous_timestamp = None;
    for (frame_index, row) in rows.iter().enumerate() {
        let timestamp =
            value_as_u64(&row["timeStamp"]).ok_or(GoalVisualizerError::MissingFrameField {
                frame: frame_index,
                field: "timeStamp",
            })?;
        if previous_timestamp.is_some_and(|previous| timestamp < previous) {
            return Err(GoalVisualizerError::NonMonotonicTimestamps { frame: frame_index });
        }
        previous_timestamp = Some(timestamp);
        let on_ice = row["onIce"]
            .as_object()
            .ok_or(GoalVisualizerError::MissingFrameField {
                frame: frame_index,
                field: "onIce",
            })?;
        let mut objects = BTreeMap::new();
        for (key, value) in on_ice {
            let id = value_as_u64(&value["id"])
                .or_else(|| key.parse::<u64>().ok())
                .ok_or_else(|| GoalVisualizerError::MissingObjectField {
                    frame: frame_index,
                    object: key.clone(),
                    field: "id",
                })?;
            let x = value["x"]
                .as_f64()
                .ok_or_else(|| GoalVisualizerError::MissingObjectField {
                    frame: frame_index,
                    object: key.clone(),
                    field: "x",
                })?;
            let y = value["y"]
                .as_f64()
                .ok_or_else(|| GoalVisualizerError::MissingObjectField {
                    frame: frame_index,
                    object: key.clone(),
                    field: "y",
                })?;
            objects.insert(
                key.clone(),
                GoalTrackedObject {
                    id,
                    player_id: value_as_u64(&value["playerId"]).map(|v| v as u32),
                    x,
                    y,
                    sweater_number: value_as_u64(&value["sweaterNumber"]).map(|v| v as u16),
                    team_id: value_as_u64(&value["teamId"]).map(|v| v as u32),
                    team_abbrev: non_empty_string(&value["teamAbbrev"]),
                },
            );
        }
        frames.push(GoalTrackingFrame {
            timestamp,
            on_ice: objects,
        });
    }
    Ok(frames)
}

pub fn build_event(
    goal: GoalVisualizerGoal,
    raw: &serde_json::Value,
    source_sha256: String,
) -> std::result::Result<GoalVisualizerEvent, GoalVisualizerError> {
    let source_url = goal
        .replay_url
        .clone()
        .ok_or_else(|| GoalVisualizerError::ReplayUrlNotAllowed("<missing>".to_string()))?;
    validate_replay_url(&source_url)?;
    let frames = parse_frames(raw)?;
    let tracked_player_ids = frames
        .iter()
        .flat_map(|frame| frame.on_ice.values().filter_map(|object| object.player_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let puck_object_observed = frames
        .iter()
        .any(|frame| frame.on_ice.values().any(|object| object.id == 1));
    Ok(GoalVisualizerEvent {
        goal,
        source_url,
        source_sha256,
        frame_count: frames.len(),
        tracked_player_ids,
        puck_object_observed,
        frames,
    })
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn validate_replay_url(url: &str) -> std::result::Result<(), GoalVisualizerError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| GoalVisualizerError::ReplayUrlNotAllowed(url.to_string()))?;
    let allowed = parsed.scheme() == "https"
        && parsed.host_str() == Some("wsr.nhle.com")
        && parsed.path().starts_with("/sprites/")
        && parsed.path().ends_with(".json")
        && parsed.username().is_empty()
        && parsed.password().is_none();
    if allowed {
        Ok(())
    } else {
        Err(GoalVisualizerError::ReplayUrlNotAllowed(url.to_string()))
    }
}

pub fn nhl_web_headers() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "Accept".to_string(),
            "application/json,text/plain,*/*".to_string(),
        ),
        ("Origin".to_string(), "https://www.nhl.com".to_string()),
        ("Referer".to_string(), "https://www.nhl.com/".to_string()),
        ("User-Agent".to_string(), "Mozilla/5.0".to_string()),
    ])
}

pub async fn fetch_replay_bytes(
    game_id: u64,
    event_id: u32,
    replay_url: &str,
    cache_root: impl Into<std::path::PathBuf>,
    force: bool,
) -> Result<Vec<u8>> {
    validate_replay_url(replay_url)?;
    crate::fletch::fetch_generic_http_bytes_with_headers_async(
        format!("icelines.goal-visualizer.{game_id}.{event_id}"),
        replay_url.to_string(),
        nhl_web_headers(),
        cache_root,
        force,
    )
    .await
    .with_context(|| format!("fetching Goal Visualizer game {game_id} event {event_id}"))
}

pub fn load_bundle(path: &Path) -> Result<GoalVisualizerBundle> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("reading Goal Visualizer bundle {}", path.display()))?;
    let bundle: GoalVisualizerBundle = serde_json::from_slice(&bytes)
        .with_context(|| format!("parsing Goal Visualizer bundle {}", path.display()))?;
    anyhow::ensure!(
        bundle.schema == BUNDLE_SCHEMA,
        "unsupported Goal Visualizer bundle schema '{}'",
        bundle.schema
    );
    Ok(bundle)
}

pub fn merge_bundle(
    path: &Path,
    game_id: u64,
    fetched_at: DateTime<Utc>,
    events: impl IntoIterator<Item = GoalVisualizerEvent>,
) -> Result<GoalVisualizerBundle> {
    let mut bundle = if path.exists() {
        let existing = load_bundle(path)?;
        anyhow::ensure!(
            existing.game_id == game_id,
            "existing Goal Visualizer bundle game id mismatch"
        );
        existing
    } else {
        GoalVisualizerBundle::new(game_id, fetched_at)
    };
    for event in events {
        bundle.upsert_event(event);
    }
    bundle.fetched_at = fetched_at;
    crate::atomic_write::write_json_atomic(path, &bundle)
        .with_context(|| format!("writing Goal Visualizer bundle {}", path.display()))?;
    Ok(bundle)
}

fn localized_default(value: &serde_json::Value) -> Option<String> {
    value["default"]
        .as_str()
        .or_else(|| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn localized_name(value: &serde_json::Value) -> Option<String> {
    if let Some(name) = localized_default(&value["name"]) {
        return Some(name);
    }
    let first = localized_default(&value["firstName"]);
    let last = localized_default(&value["lastName"]);
    match (first, last) {
        (Some(first), Some(last)) => Some(format!("{first} {last}")),
        (Some(name), None) | (None, Some(name)) => Some(name),
        (None, None) => None,
    }
}

fn non_empty_string(value: &serde_json::Value) -> Option<String> {
    value
        .as_str()
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
}

fn value_as_u64(value: &serde_json::Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().filter(|s| !s.is_empty())?.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn l0_goal_visualizer_discovers_optional_replay_urls_and_assists() {
        let landing = json!({"summary":{"scoring":[{
            "periodDescriptor":{"number":3,"periodType":"REG"},
            "goals":[
                {"eventId":996,"timeInPeriod":"11:14","strength":"sh","playerId":8477839,
                 "firstName":{"default":"Conor"},"lastName":{"default":"Sheary"},
                 "teamAbbrev":{"default":"NYR"},"pptReplayUrl":"https://wsr.nhle.com/sprites/20252026/2025021167/ev996.json",
                 "assists":[{"playerId":8476468,"name":{"default":"J. Miller"},"sweaterNumber":8}]},
                {"eventId":1200,"timeInPeriod":"20:00","strength":"so"}
            ]
        }]}});
        let goals = discover_goals(&landing).expect("discover goals");
        assert_eq!(goals.len(), 2);
        assert_eq!(goals[0].event_id, 996);
        assert_eq!(goals[0].period, 3);
        assert_eq!(goals[0].assists[0].player_id, Some(8476468));
        assert!(goals[0].replay_url.is_some());
        assert!(
            goals[1].replay_url.is_none(),
            "missing coverage is data, not a fabricated URL"
        );
    }

    #[test]
    fn l0_goal_visualizer_parses_players_and_empty_string_puck_fields() {
        let raw = json!([
            {"timeStamp":17748118960u64,"onIce":{
                "3024":{"id":3024,"playerId":8481789,"x":1600.594,"y":754.2232,"sweaterNumber":24,"teamId":3,"teamAbbrev":"NYR"},
                "1":{"id":1,"playerId":"","x":1359.1266,"y":986.9048,"sweaterNumber":"","teamId":"","teamAbbrev":""}
            }},
            {"timeStamp":17748118961u64,"onIce":{
                "3024":{"id":3024,"playerId":8481789,"x":1601.0,"y":753.0,"sweaterNumber":24,"teamId":3,"teamAbbrev":"NYR"},
                "1":{"id":1,"playerId":"","x":1360.0,"y":985.0,"sweaterNumber":"","teamId":"","teamAbbrev":""}
            }}
        ]);
        let frames = parse_frames(&raw).expect("parse frames");
        assert_eq!(frames.len(), 2);
        assert_eq!(frames[0].on_ice["3024"].player_id, Some(8481789));
        assert_eq!(frames[0].on_ice["1"].player_id, None);
        assert_eq!(frames[0].on_ice["1"].id, 1);
    }

    #[test]
    fn l0_goal_visualizer_rejects_backwards_timestamps() {
        let raw = json!([
            {"timeStamp":2,"onIce":{"1":{"id":1,"x":1.0,"y":1.0}}},
            {"timeStamp":1,"onIce":{"1":{"id":1,"x":2.0,"y":2.0}}}
        ]);
        assert!(matches!(
            parse_frames(&raw),
            Err(GoalVisualizerError::NonMonotonicTimestamps { frame: 1 })
        ));
    }

    #[test]
    fn l0_goal_visualizer_restricts_discovered_fetch_url() {
        assert!(
            validate_replay_url("https://wsr.nhle.com/sprites/20252026/2025021167/ev996.json")
                .is_ok()
        );
        assert!(validate_replay_url("http://wsr.nhle.com/sprites/x.json").is_err());
        assert!(validate_replay_url("https://example.com/sprites/x.json").is_err());
        assert!(validate_replay_url("https://wsr.nhle.com/other/x.json").is_err());
    }

    #[test]
    fn l0_goal_visualizer_declares_required_public_nhl_headers() {
        let headers = nhl_web_headers();
        assert_eq!(
            headers.get("Origin").map(String::as_str),
            Some("https://www.nhl.com")
        );
        assert_eq!(
            headers.get("Referer").map(String::as_str),
            Some("https://www.nhl.com/")
        );
        assert!(headers.contains_key("User-Agent"));
    }

    #[test]
    fn l0_goal_visualizer_merge_reloads_and_preserves_existing_events() {
        fn event(event_id: u32) -> GoalVisualizerEvent {
            GoalVisualizerEvent {
                goal: GoalVisualizerGoal {
                    event_id,
                    period: 1,
                    period_type: "REG".to_string(),
                    time_in_period: "01:00".to_string(),
                    situation_code: None,
                    strength: None,
                    scorer_id: None,
                    scorer_name: None,
                    team_abbrev: None,
                    shot_type: None,
                    home_team_defending_side: None,
                    assists: Vec::new(),
                    replay_url: Some(format!(
                        "https://wsr.nhle.com/sprites/20252026/2025021167/ev{event_id}.json"
                    )),
                },
                source_url: format!(
                    "https://wsr.nhle.com/sprites/20252026/2025021167/ev{event_id}.json"
                ),
                source_sha256: format!("sha-{event_id}"),
                frame_count: 0,
                tracked_player_ids: Vec::new(),
                puck_object_observed: false,
                frames: Vec::new(),
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("2025021167.json");
        merge_bundle(&path, 2025021167, Utc::now(), [event(100)]).unwrap();
        let merged = merge_bundle(&path, 2025021167, Utc::now(), [event(200)]).unwrap();
        assert_eq!(
            merged
                .events
                .iter()
                .map(|event| event.goal.event_id)
                .collect::<Vec<_>>(),
            vec![100, 200]
        );
    }
}
