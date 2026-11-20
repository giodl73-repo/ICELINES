//! Captured NHL PPT per-game summary. Possession duration is exposure context,
//! not successful control, pass attempts, acceleration or decision quality.
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Serialize)]
pub struct Player {
    pub player_id: u32,
    pub name: String,
    pub position: String,
    pub team_id: String,
    pub source_entity_id: String,
    /// Preserve provider clocks separately; do not silently substitute one.
    pub time_on_seconds: Option<f64>,
    pub actual_time_on_seconds: Option<f64>,
    pub possession_seconds: Option<f64>,
    pub possession_even_strength_seconds: Option<f64>,
    pub possession_by_zone_seconds: BTreeMap<String, Option<f64>>,
    pub maximum_speed_mps: Option<f64>,
    pub average_speed_mps: Option<f64>,
    pub distance_metres: Option<f64>,
    pub provider_maximum_speed: Option<f64>,
    pub provider_average_speed: Option<f64>,
    pub provider_distance: Option<f64>,
    pub diagnostics: Vec<String>,
}
#[derive(Debug, Serialize)]
pub struct Report {
    pub schema: String,
    pub game_id: u64,
    pub summary_sha256: String,
    pub entities_sha256: String,
    pub event_time: Value,
    pub as_of_utc: Value,
    pub players: Vec<Player>,
    pub registered_players_without_summary: Vec<u32>,
    pub limits: Vec<String>,
}
fn root(raw: &[u8]) -> Result<Value, String> {
    let value: Value = serde_json::from_slice(raw).map_err(|e| e.to_string())?;
    if let Some(array) = value.as_array() {
        if array.len() == 1 && array[0].is_object() {
            return Ok(array[0].clone());
        }
        return Err("Expected exactly one tracking object in array".into());
    }
    if !value.is_object() {
        return Err("Expected one tracking object".into());
    }
    Ok(value)
}
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("Missing tracking string {key}"))
}
fn number(v: &Value, key: &str) -> Result<Option<f64>, String> {
    if v[key].is_null() {
        return Ok(None);
    }
    v[key]
        .as_f64()
        .filter(|v| v.is_finite() && *v >= 0.)
        .map(Some)
        .ok_or_else(|| format!("Invalid tracking number {key}"))
}
pub fn parse(summary_bytes: &[u8], entity_bytes: &[u8], game_id: u64) -> Result<Report, String> {
    let summary = root(summary_bytes)?;
    let registration = root(entity_bytes)?;
    if text(&summary, "OfficialCode")? != game_id.to_string()
        || text(&registration, "EventOfficialCode")? != game_id.to_string()
        || text(&summary, "EventId")? != text(&registration, "EventId")?
    {
        return Err("Tracking game identity mismatch".into());
    }
    let mut roster = BTreeMap::new();
    let mut official_ids = BTreeSet::new();
    for entity in registration["Entities"]
        .as_array()
        .ok_or("Missing entity registration")?
    {
        if entity["EntityType"].as_str() != Some("Player") {
            continue;
        }
        let id = text(entity, "EntityId")?;
        let official = text(entity, "OfficialId")?
            .parse::<u32>()
            .map_err(|_| "Invalid official player ID")?;
        if official == 0
            || !official_ids.insert(official)
            || roster.insert(id, (official, entity)).is_some()
        {
            return Err("Invalid/duplicate registered player".into());
        }
    }
    if roster.is_empty() {
        return Err("No registered players".into());
    }
    let mut seen = BTreeSet::new();
    let mut players = Vec::new();
    for row in summary["EntitySummaries"]
        .as_array()
        .ok_or("Missing entity summaries")?
    {
        let id = text(row, "EntityId")?;
        if !seen.insert(id) {
            return Err("Duplicate entity summary".into());
        }
        let Some((official, entity)) = roster.get(id) else {
            continue;
        };
        if let Some(row_id) = row["EntityOfficialId"].as_str().filter(|s| !s.is_empty()) {
            if row_id != official.to_string() {
                return Err("Summary/registration actor mismatch".into());
            }
        }
        let position = match text(entity, "Position")? {
            "LW" | "L" => "L",
            "RW" | "R" => "R",
            "C" => "C",
            "D" => "D",
            "G" => "G",
            _ => return Err("Unknown player position".into()),
        };
        let time_on = number(row, "TimeOnTotal")?;
        let actual = number(row, "ActualTimeOnTotal")?;
        let possession = number(row, "PossessionTime")?;
        let mut diagnostics = Vec::new();
        if time_on.zip(actual).is_some_and(|(a, b)| (a - b).abs() > 1.) {
            diagnostics.push("provider_time_on_clocks_differ_no_substitution".into());
        }
        let by_zone = [
            ("offensive", "PossessionTimeOZone"),
            ("neutral", "PossessionTimeNZone"),
            ("defensive", "PossessionTimeDZone"),
        ]
        .into_iter()
        .map(|(key, field)| Ok((key.into(), number(row, field)?)))
        .collect::<Result<BTreeMap<String, Option<f64>>, String>>()?;
        if let (Some(total), Some(parts)) = (
            possession,
            by_zone.values().copied().collect::<Option<Vec<f64>>>(),
        ) {
            if (parts.iter().sum::<f64>() - total).abs() > 1e-9 {
                diagnostics.push("possession_zone_sum_differs_from_total_no_reconciliation".into());
            }
        }
        players.push(Player {
            player_id: *official,
            name: format!(
                "{} {}",
                text(entity, "FirstName")?,
                text(entity, "LastName")?
            ),
            position: position.into(),
            team_id: text(entity, "EntityTeamId")?.into(),
            source_entity_id: id.into(),
            time_on_seconds: time_on,
            actual_time_on_seconds: actual,
            possession_seconds: possession,
            possession_even_strength_seconds: number(row, "PossessionTimeEvenStrength")?,
            possession_by_zone_seconds: by_zone,
            maximum_speed_mps: number(row, "SpeedMax")?.map(|v| v * 0.3048),
            average_speed_mps: number(row, "SpeedAvg")?.map(|v| v * 0.3048),
            distance_metres: number(row, "DistanceTotal")?.map(|v| v * 0.3048),
            provider_maximum_speed: number(row, "SpeedMax")?,
            provider_average_speed: number(row, "SpeedAvg")?,
            provider_distance: number(row, "DistanceTotal")?,
            diagnostics,
        });
    }
    players.sort_by_key(|p| p.player_id);
    Ok(Report { schema: "rinkcraft.tracking-game-context.v1".into(), game_id,
        summary_sha256: format!("{:x}", Sha256::digest(summary_bytes)), entities_sha256: format!("{:x}", Sha256::digest(entity_bytes)),
        event_time: summary["EventTime"].clone(), as_of_utc: summary["NowUTC"].clone(),
        registered_players_without_summary: roster.iter().filter(|(id, _)| !seen.contains(**id)).map(|(_, (official, _))| *official).collect(), players,
        limits: vec!["Captured summary as of provider timestamp; finality and full-game sensor coverage not established.".into(),
            "Feet and feet/second are adapter unit assumptions following the inspected client implementation, not units authenticated by this payload; raw values are retained. Conditional conversion uses exact 0.3048 metres/foot; calibration remains unverified.".into(),
            "Possession duration is context, not touches, successful control, pass attempts or handling capability.".into(),
            "Even strength is the provider category; do not relabel it five-on-five with both goalies present.".into(),
            "A single instantaneous location and game maximum do not form a motion sequence or measure agility, acceleration, anticipation or duty execution.".into()] })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn data() -> (Value, Value) {
        (
            json!({"OfficialCode":"2023020500","EventId":"synthetic","EntitySummaries":[{"EntityId":"1","EntityOfficialId":"123","PossessionTime":0,"SpeedMax":10,"TimeOnTotal":30,"ActualTimeOnTotal":40}]}),
            json!({"EventOfficialCode":"2023020500","EventId":"synthetic","Entities":[{"EntityType":"Player","EntityId":"1","OfficialId":"123","Position":"LW","FirstName":"Test","LastName":"Skater","EntityTeamId":"1"}]}),
        )
    }
    fn run(s: &Value, e: &Value) -> Result<Report, String> {
        parse(
            &serde_json::to_vec(s).unwrap(),
            &serde_json::to_vec(e).unwrap(),
            2023020500,
        )
    }
    #[test]
    fn units_zeros_missing_and_clocks_are_distinct() {
        let (s, e) = data();
        let r = run(&s, &e).unwrap();
        let p = &r.players[0];
        assert_eq!(p.possession_seconds, Some(0.));
        assert_eq!(p.possession_even_strength_seconds, None);
        assert_eq!(p.maximum_speed_mps, Some(3.048));
        assert_eq!(p.position, "L");
        assert_eq!(
            p.diagnostics,
            ["provider_time_on_clocks_differ_no_substitution"]
        );
        let wrapped = parse(
            &serde_json::to_vec(&json!([s])).unwrap(),
            &serde_json::to_vec(&json!([e])).unwrap(),
            2023020500,
        )
        .unwrap();
        assert_eq!(wrapped.players[0].player_id, 123);
        assert!(parse(b"[]", b"[]", 2023020500).is_err());
        assert!(parse(b"[{},{}]", b"[{},{}]", 2023020500).is_err());
    }
    #[test]
    fn identity_duplicates_and_bad_numbers_rejected() {
        let (mut s, e) = data();
        s["EntitySummaries"][0]["EntityOfficialId"] = json!("456");
        assert!(run(&s, &e).is_err());
        let (mut s, e) = data();
        s["EntitySummaries"][0]["SpeedMax"] = json!(-1);
        assert!(run(&s, &e).is_err());
        let (mut s, e) = data();
        let row = s["EntitySummaries"][0].clone();
        s["EntitySummaries"].as_array_mut().unwrap().push(row);
        assert!(run(&s, &e).is_err());
        let (mut s, e) = data();
        s["OfficialCode"] = json!("2023020501");
        assert!(run(&s, &e).is_err());
    }
}
