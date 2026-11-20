//! Strict, stateless parsing of a captured regular-season NHL EDGE detail.
use icelines_core::view_model::rinkcraft_edge::{
    EdgeEvidence, EdgeLocation, EdgeMeasure, EdgeSource,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
fn number(v: &Value) -> Result<Option<f64>, String> {
    if v.is_null() {
        return Ok(None);
    }
    v.as_f64()
        .filter(|n| n.is_finite() && *n >= 0.)
        .map(Some)
        .ok_or("invalid EDGE number".into())
}
fn fraction(v: &Value) -> Result<Option<f64>, String> {
    let n = number(v)?;
    if n.is_some_and(|n| n > 1.) {
        return Err("EDGE fraction outside 0..1".into());
    }
    Ok(n)
}
fn count(v: &Value) -> Result<Option<u32>, String> {
    if v.is_null() {
        return Ok(None);
    }
    v.as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .map(Some)
        .ok_or("invalid EDGE count".into())
}
fn push(
    out: &mut Vec<EdgeMeasure>,
    v: &Value,
    key: &str,
    label: &str,
    unit: &str,
    path: &str,
    attribute: Option<&str>,
    component: Option<&str>,
    meaning: &str,
    physical: bool,
) -> Result<(), String> {
    let raw = if physical {
        &v["imperial"]
    } else {
        &v["value"]
    };
    let Some(value) = number(raw)? else {
        return Ok(());
    };
    if physical && value == 0. {
        return Ok(());
    }
    if unit == "Count" && value.fract() != 0. {
        return Err("noninteger EDGE count".into());
    }
    if physical {
        if let Some(metric) = number(&v["metric"])? {
            // EDGE may use rounded conversion factors for distance totals.
            // This is a consistency check, not a claim about sensor precision.
            if (metric - value * 1.609344).abs() > 0.02_f64.max(metric * 0.001) {
                return Err("EDGE imperial/metric unit mismatch".into());
            }
        }
    }
    let average = number(if physical {
        &v["leagueAvg"]["imperial"]
    } else {
        &v["leagueAvg"]["value"]
    })?;
    out.push(EdgeMeasure {
        key: key.into(),
        label: label.into(),
        value,
        unit: unit.into(),
        provider_percentile: fraction(&v["percentile"])?,
        provider_average: average,
        source_field: path.into(),
        attribute_id: attribute.map(str::to_owned),
        component_id: component.map(str::to_owned),
        interpretation: meaning.into(),
    });
    Ok(())
}
#[allow(clippy::too_many_arguments)]
pub fn parse(
    bytes: &[u8],
    player_id: u32,
    season: u32,
    group: &str,
    games: u32,
    url: &str,
    captured_at: &str,
) -> Result<EdgeEvidence, String> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("EDGE detail exceeds 2 MiB".into());
    }
    if player_id == 0
        || season / 10000 + 1 != season % 10000
        || season < 20212022
        || !matches!(group, "Forwards" | "Defense" | "Goalies")
        || games == 0
    {
        return Err("invalid EDGE target scope".into());
    }
    chrono::DateTime::parse_from_rfc3339(captured_at)
        .map_err(|_| "invalid EDGE capture timestamp")?;
    let goalie = group == "Goalies";
    let expected = format!(
        "https://api-web.nhle.com/v1/edge/{}/{player_id}/{season}/2",
        if goalie {
            "goalie-detail"
        } else {
            "skater-detail"
        }
    );
    if url != expected {
        return Err("EDGE source must name the exact actor, season, competition and report".into());
    }
    let root: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if !root["seasonsWithEdgeStats"].as_array().is_some_and(|rows| {
        rows.iter().any(|r| {
            r["id"].as_u64() == Some(season as u64)
                && r["gameTypes"]
                    .as_array()
                    .is_some_and(|types| types.contains(&Value::from(2)))
        })
    }) {
        return Err("requested EDGE regular-season coverage is absent".into());
    }
    if count(&root["player"]["id"])? != Some(player_id)
        || count(&root["player"]["gamesPlayed"])? != Some(games)
    {
        return Err("EDGE actor/appearance count differs from selected profile".into());
    }
    // The goalie detail schema omits position; its goalie-specific source route
    // and the supplied NHL summary identity bind that group instead.
    let position = root["player"]["position"]
        .as_str()
        .or(if goalie { Some("G") } else { None })
        .ok_or("EDGE position absent")?;
    let actual = if position == "G" {
        "Goalies"
    } else if position == "D" {
        "Defense"
    } else if matches!(position, "C" | "L" | "R" | "LW" | "RW") {
        "Forwards"
    } else {
        return Err("unknown EDGE position".into());
    };
    if actual != group {
        return Err("EDGE position group mismatch".into());
    }
    let mut measures = Vec::new();
    let mut locations = Vec::new();
    if !goalie {
        push(&mut measures,&root["skatingSpeed"]["speedMax"],"max-skating-speed","Season maximum skating speed","Mph","skatingSpeed.speedMax",Some("CAP-SKATING"),Some("SKA-SPEED"),"Observed maximum, all strengths. It does not establish repeatable speed, acceleration, agility or puck-carrying speed.",true)?;
        push(&mut measures,&root["skatingSpeed"]["burstsOver20"],"speed-bursts-over-20","Bursts above 20 mph","Count","skatingSpeed.burstsOver20",Some("CAP-SKATING"),Some("SKA-SPEED"),"Recorded speed-threshold crossings; affected by exposure and deployment. Not an acceleration test.",false)?;
        push(&mut measures,&root["totalDistanceSkated"],"skating-distance","Season skating distance","Miles","totalDistanceSkated",Some("CAP-SKATING"),None,"Distance includes movement without the puck and depends on ice time; not transport success or endurance ability.",true)?;
        push(&mut measures,&root["topShotSpeed"],"max-shot-speed","Season maximum shot speed","Mph","topShotSpeed",Some("CAP-SHOOTING"),Some("SHO-VELOCITY"),"Observed maximum across tracked shots; not accuracy, release timing or finishing ability.",true)?;
        for (key, label, field) in [
            (
                "offensive-zone-puck-time",
                "Offensive-zone puck time",
                "offensiveZonePctg",
            ),
            (
                "neutral-zone-puck-time",
                "Neutral-zone puck time",
                "neutralZonePctg",
            ),
            (
                "defensive-zone-puck-time",
                "Defensive-zone puck time",
                "defensiveZonePctg",
            ),
        ] {
            if let Some(value) = fraction(&root["zoneTimeDetails"][field])? {
                measures.push(EdgeMeasure{key:key.into(),label:label.into(),value,unit:"Pct".into(),provider_percentile:None,provider_average:None,source_field:format!("zoneTimeDetails.{field}"),attribute_id:None,component_id:None,interpretation:"Time the puck is in this zone while the player is on ice, not the player's own location or individual defensive effectiveness.".into()});
            }
        }
    }
    let rows = if goalie {
        &root["shotLocationSummary"]
    } else {
        &root["sogSummary"]
    };
    if let Some(rows) = rows.as_array() {
        let mut codes = std::collections::BTreeSet::new();
        for row in rows {
            let code = row["locationCode"]
                .as_str()
                .ok_or("EDGE location code absent")?;
            if !codes.insert(code) {
                return Err("duplicate EDGE location summary".into());
            }
            let shots = count(&row["shots"])?;
            let goals = count(&row["goals"])?;
            let saves = count(&row["saves"])?;
            let against = count(&row["goalsAgainst"])?;
            if goals.zip(shots).is_some_and(|(g, s)| g > s) {
                return Err("EDGE goals exceed shots".into());
            }
            let denom = if goalie {
                saves.zip(against).map(|(s, a)| u64::from(s) + u64::from(a))
            } else {
                shots.map(u64::from)
            };
            let frac = if denom == Some(0) {
                None
            } else {
                fraction(&row[if goalie { "savePctg" } else { "shootingPctg" }])?
            };
            locations.push(EdgeLocation {
                code: code.into(),
                shots,
                goals,
                saves,
                goals_against: against,
                fraction: frac,
                provider_percentile: if frac.is_some() {
                    fraction(
                        &row[if goalie {
                            "savePctgPercentile"
                        } else {
                            "shootingPctgPercentile"
                        }],
                    )?
                } else {
                    None
                },
            });
        }
    }
    Ok(EdgeEvidence{schema:"icelines.rinkcraft-edge.v1".into(),player_id,season,season_type:"regular".into(),group:group.into(),games,captured_at:captured_at.into(),strength_scope:"all strengths".into(),source:EdgeSource{url:url.into(),sha256:format!("{:x}",Sha256::digest(bytes)),bytes:bytes.len()},measures,locations,notes:vec!["NHL EDGE provider percentiles use provider-defined league references. Their eligibility and reference counts are not supplied in this response; they are not IceLines F/D/G midranks.".into(),"Maximum speeds and burst/distance counts are observations, not complete capability estimates. Do not combine missing facets or assign a 1–20 capability from them.".into(),"Location codes and fractions retain the provider's definitions. The location subgroups are not assumed to partition the all-location total.".into(),if goalie{"Saving by shot location is outcome context; it does not measure Tracking, Positioning, Movement, Rebounds, Recovery or Puck Play technique.".into()}else{"On-ice zone time describes puck location, not player routes or defensive responsibility.".into()}]})
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    const URL: &str = "https://api-web.nhle.com/v1/edge/skater-detail/1/20242025/2";
    fn raw() -> Value {
        json!({"player":{"id":1,"gamesPlayed":50,"position":"C"},"seasonsWithEdgeStats":[{"id":20242025,"gameTypes":[2]}],"skatingSpeed":{"speedMax":{"imperial":24.,"metric":38.624256,"percentile":0.99},"burstsOver20":{"value":0,"percentile":0.1}},"sogSummary":[{"locationCode":"all","shots":0,"goals":0,"shootingPctg":0,"shootingPctgPercentile":0.2}]})
    }
    fn run(v: Value) -> Result<EdgeEvidence, String> {
        parse(
            &serde_json::to_vec(&v).unwrap(),
            1,
            20242025,
            "Forwards",
            50,
            URL,
            "2026-11-19T23:00:00Z",
        )
    }
    #[test]
    fn observations_not_capability_estimates() {
        let p = run(raw()).unwrap();
        assert_eq!(p.measures.len(), 2);
        assert_eq!(p.measures[0].component_id.as_deref(), Some("SKA-SPEED"));
        assert_eq!(p.measures[1].value, 0.);
        assert!(p.locations[0].fraction.is_none());
    }
    #[test]
    fn scope_identity_units_and_fractions_are_strict() {
        for (key, value) in [
            ("id", json!(2)),
            ("gamesPlayed", json!(51)),
            ("position", json!("G")),
        ] {
            let mut v = raw();
            v["player"][key] = value;
            assert!(run(v).is_err());
        }
        let mut v = raw();
        v["skatingSpeed"]["speedMax"]["percentile"] = json!(99);
        assert!(run(v).is_err());
        let mut v = raw();
        v["skatingSpeed"]["speedMax"]["metric"] = json!(24);
        assert!(run(v).is_err());
        assert!(parse(
            b"{}",
            1,
            20242025,
            "Forwards",
            50,
            &URL.replace("/2", "/3"),
            "2026-11-19T23:00:00Z"
        )
        .is_err());
    }
    #[test]
    fn missing_physical_values_are_not_zero_speed() {
        let mut v = raw();
        v["skatingSpeed"]["speedMax"] = json!({"imperial":0,"metric":0});
        let p = run(v).unwrap();
        assert_eq!(p.measures.len(), 1);
        assert_eq!(p.measures[0].key, "speed-bursts-over-20");
    }
    #[test]
    fn unsupported_seasons_and_goalie_schema_are_explicit() {
        let mut v = raw();
        v["seasonsWithEdgeStats"] = Value::from(Vec::<Value>::new());
        assert!(run(v).is_err());
        let v = serde_json::json!({"player":{"id":1,"gamesPlayed":50},"seasonsWithEdgeStats":[{"id":20242025,"gameTypes":[2]}],"shotLocationSummary":[{"locationCode":"all","saves":900,"goalsAgainst":100,"savePctg":0.9,"savePctgPercentile":0.5}]});
        let p = parse(
            &serde_json::to_vec(&v).unwrap(),
            1,
            20242025,
            "Goalies",
            50,
            "https://api-web.nhle.com/v1/edge/goalie-detail/1/20242025/2",
            "2026-11-19T23:00:00Z",
        )
        .unwrap();
        assert_eq!(p.locations[0].fraction, Some(0.9));
        assert!(p.measures.is_empty());
    }
}
