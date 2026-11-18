//! Strict selection of one seasonal workload row from a captured NHL report.
use icelines_core::{
    identity::PlayerId,
    model::Season,
    season_stats::SeasonType,
    view_model::rinkcraft_deployment::{self, DeploymentEvidence, UsageInput},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
pub fn parse(
    bytes: &[u8],
    player: u32,
    season: u32,
    game_type: u8,
    goalie: bool,
    source_url: &str,
) -> Result<DeploymentEvidence, String> {
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("usage report exceeds 2 MiB".into());
    }
    let season_type = match game_type {
        2 => SeasonType::Regular,
        3 => SeasonType::Playoff,
        _ => return Err("usage requires regular season or playoffs".into()),
    };
    if season / 10000 + 1 != season % 10000 || !(1900..=2200).contains(&(season / 10000)) {
        return Err("invalid usage season".into());
    }
    let url = url::Url::parse(source_url).map_err(|_| "invalid usage source URL")?;
    let path = if goalie {
        "/stats/rest/en/goalie/summary"
    } else {
        "/stats/rest/en/skater/timeonice"
    };
    let exp = url
        .query_pairs()
        .find(|(k, _)| k == "cayenneExp")
        .map(|(_, v)| v.into_owned())
        .ok_or("missing source window")?;
    let filters: Vec<_> = exp.split(" and ").collect();
    if url.scheme() != "https"
        || url.host_str() != Some("api.nhle.com")
        || url.path() != path
        || !filters.contains(&format!("seasonId={season}").as_str())
        || !filters.contains(&format!("gameTypeId={game_type}").as_str())
        || url
            .query_pairs()
            .any(|(k, v)| (k == "isAggregate" || k == "isGame") && v != "false")
    {
        return Err("usage source must name the exact NHL seasonal report and game type".into());
    }
    let root: Value = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let rows = root
        .get("data")
        .and_then(Value::as_array)
        .ok_or("missing usage rows")?;
    if root.get("total").and_then(Value::as_u64) != Some(rows.len() as u64) {
        return Err("usage capture is incomplete or paginated".into());
    }
    let n = |row: &Value, key: &str| -> Result<u32, String> {
        row.get(key)
            .and_then(Value::as_u64)
            .and_then(|n| u32::try_from(n).ok())
            .ok_or_else(|| format!("missing or invalid usage {key}"))
    };
    for row in rows {
        if n(row, "seasonId")? != season {
            return Err("mixed usage seasons".into());
        }
    }
    let selected: Vec<_> = rows
        .iter()
        .filter(|r| r.get("playerId").and_then(Value::as_u64) == Some(u64::from(player)))
        .collect();
    if selected.len() != 1 {
        return Err("usage requires exactly one row for requested actor".into());
    }
    let r = selected[0];
    let mut card = rinkcraft_deployment::build(UsageInput {
        player_id: PlayerId(player),
        season: Season(season),
        season_type,
        games: n(r, "gamesPlayed")?,
        total: n(r, "timeOnIce")?,
        skater_strengths: if goalie {
            None
        } else {
            Some([
                n(r, "evTimeOnIce")?,
                n(r, "ppTimeOnIce")?,
                n(r, "shTimeOnIce")?,
            ])
        },
        goalie_starts: if goalie {
            Some(n(r, "gamesStarted")?)
        } else {
            None
        },
    })?;
    card.source_url = Some(source_url.into());
    card.source_sha256 = Some(format!("{:x}", Sha256::digest(bytes)));
    Ok(card)
}
#[cfg(test)]
mod tests {
    use super::*;
    const URL:&str="https://api.nhle.com/stats/rest/en/skater/timeonice?isAggregate=false&isGame=false&cayenneExp=seasonId%3D20242025%20and%20gameTypeId%3D2";
    fn raw() -> Value {
        serde_json::json!({"total":1,"data":[{"playerId":1,"seasonId":20242025,"gamesPlayed":2,"timeOnIce":1200,"evTimeOnIce":1000,"ppTimeOnIce":200,"shTimeOnIce":0}]})
    }
    fn run(v: &Value) -> Result<DeploymentEvidence, String> {
        parse(&serde_json::to_vec(v).unwrap(), 1, 20242025, 2, false, URL)
    }
    #[test]
    fn captured_report_keeps_zero_and_source_hash() {
        let c = run(&raw()).unwrap();
        assert_eq!(c.rows[3].seconds, 0);
        assert_eq!(c.source_sha256.as_ref().unwrap().len(), 64);
        assert_eq!(c.source_url.as_deref(), Some(URL));
    }
    #[test]
    fn actor_window_duplicates_partial_and_missing_fields_reject() {
        let mut r = raw();
        r["data"][0]["playerId"] = 2.into();
        assert!(run(&r).is_err());
        let mut r = raw();
        r["data"][0]["seasonId"] = 20252026.into();
        assert!(run(&r).is_err());
        let mut r = raw();
        r["total"] = 2.into();
        assert!(run(&r).is_err());
        let mut r = raw();
        r["data"][0].as_object_mut().unwrap().remove("shTimeOnIce");
        assert!(run(&r).is_err());
        let mut r = raw();
        let a = r["data"][0].clone();
        r["data"].as_array_mut().unwrap().push(a);
        r["total"] = 2.into();
        assert!(run(&r).is_err());
        assert!(parse(
            &serde_json::to_vec(&raw()).unwrap(),
            1,
            20242025,
            3,
            false,
            URL
        )
        .is_err());
        assert!(parse(
            &serde_json::to_vec(&raw()).unwrap(),
            1,
            20242025,
            2,
            true,
            URL
        )
        .is_err());
    }
}
