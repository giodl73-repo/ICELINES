//! Read complete, explicitly regular-season NHL report assemblies for stat feats.
use icelines_core::view_model::rinkcraft_achievements::{Input, PlayerInput};
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Deserialize)]
struct Reports {
    season: u32,
    source_urls: Vec<String>,
    skaters: Value,
    realtime: Value,
    goalies: Value,
    #[serde(default)]
    penalties: Option<Value>,
}
/// Complete official goalie-only assembly for opt-in studies, without invented skater reports.
pub fn goalie_input(bytes:&[u8],season:u32,source_urls:Vec<String>)->Result<Input,String>{
    if bytes.len()>4*1024*1024 || source_urls.is_empty(){return Err("Missing/bounded goalie report".into());}
    for source in &source_urls {
        let u=url::Url::parse(source).map_err(|e|e.to_string())?;
        let q=u.query_pairs().collect::<BTreeMap<_,_>>();
        let filter=q.get("cayenneExp").ok_or("Missing goalie source window")?;
        let tokens=filter.split(" and ").collect::<Vec<_>>();
        if u.scheme()!="https"||u.host_str()!=Some("api.nhle.com")||u.path()!="/stats/rest/en/goalie/summary"
            || !tokens.contains(&format!("seasonId={season}").as_str())||!tokens.contains(&"gameTypeId=2")
            || q.get("isAggregate").map(|s|s.as_ref())!=Some("false")||q.get("isGame").map(|s|s.as_ref())!=Some("false") {
            return Err("Wrong goalie report source/window".into());
        }
    }
    let v:Value=serde_json::from_slice(bytes).map_err(|e|e.to_string())?;
    let mut players=vec![];
    for (id,row) in rows(&v,season)? {
        let n=|key|count(row,key)?.ok_or_else(||format!("Missing goalie {key}"));
        let (games,starts,shots,saves,ga)=(n("gamesPlayed")?,n("gamesStarted")?,n("shotsAgainst")?,n("saves")?,n("goalsAgainst")?);
        if id==0||starts>games||saves>shots{return Err("Incoherent goalie summary counts".into());}
        players.push(PlayerInput{player_id:id,name:row["goalieFullName"].as_str().filter(|s|!s.trim().is_empty()).ok_or("Missing goalie name")?.into(),group:"Goalies".into(),games,
            counts:BTreeMap::from([("starts".into(),starts as f64),("shutouts".into(),n("shutouts")? as f64),("shots_against".into(),shots as f64),("saves".into(),saves as f64),("goals_against".into(),ga as f64)])});
    }
    Ok(Input{season,players,source_urls,source_sha256:format!("{:x}",Sha256::digest(bytes))})
}
fn rows(report: &Value, season: u32) -> Result<BTreeMap<u32, &Value>, String> {
    let data = report["data"].as_array().ok_or("missing report data")?;
    if report["total"].as_u64() != Some(data.len() as u64) {
        return Err("incomplete report assembly".into());
    }
    let mut out = BTreeMap::new();
    for row in data {
        let id = count(row, "playerId")?.ok_or("missing actor")?;
        if count(row, "seasonId")? != Some(season) || out.insert(id, row).is_some() {
            return Err("mixed season or duplicate actor".into());
        }
    }
    Ok(out)
}
fn count(row: &Value, key: &str) -> Result<Option<u32>, String> {
    match row.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .map(Some)
            .ok_or_else(|| format!("invalid count {key}")),
    }
}
pub fn parse(bytes: &[u8]) -> Result<Input, String> {
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("achievement reports exceed 4 MiB".into());
    }
    let r: Reports = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    let mut paths = BTreeSet::new();
    for source in &r.source_urls {
        let u = url::Url::parse(source).map_err(|e| e.to_string())?;
        let exp = u
            .query_pairs()
            .find(|(k, _)| k == "cayenneExp")
            .map(|(_, v)| v.into_owned())
            .ok_or("missing source window")?;
        let filters: Vec<_> = exp.split(" and ").collect();
        if u.scheme() != "https"
            || u.host_str() != Some("api.nhle.com")
            || !filters.contains(&format!("seasonId={}", r.season).as_str())
            || !filters.contains(&"gameTypeId=2")
            || !matches!(
                u.path(),
                "/stats/rest/en/skater/summary"
                    | "/stats/rest/en/skater/realtime"
                    | "/stats/rest/en/goalie/summary"
                    | "/stats/rest/en/skater/penalties"
            )
        {
            return Err("wrong achievement report source/window".into());
        }
        paths.insert(u.path().to_owned());
    }
    if ![
        "/stats/rest/en/skater/summary",
        "/stats/rest/en/skater/realtime",
        "/stats/rest/en/goalie/summary",
    ]
    .iter()
    .all(|p| paths.contains(*p))
        || (r.penalties.is_some() && !paths.contains("/stats/rest/en/skater/penalties"))
    {
        return Err("all three report sources required".into());
    }
    let sk = rows(&r.skaters, r.season)?;
    let rt = rows(&r.realtime, r.season)?;
    let goalies = rows(&r.goalies, r.season)?;
    let penalties = r
        .penalties
        .as_ref()
        .map(|p| rows(p, r.season))
        .transpose()?;
    let mut players = Vec::new();
    for (id, row) in sk {
        let gp = count(row, "gamesPlayed")?.ok_or("missing GP")?;
        let group = match row["positionCode"].as_str() {
            Some("D") => "Defense",
            Some("C" | "L" | "R") => "Forwards",
            _ => return Err("unknown skater group".into()),
        };
        let mut counts = BTreeMap::new();
        for key in ["points", "goals", "assists", "shots"] {
            if let Some(v) = count(row, key)? {
                counts.insert(key.into(), v as f64);
            }
        }
        for (dest, src) in [
            ("pp_goals", "ppGoals"),
            ("pp_points", "ppPoints"),
            ("sh_goals", "shGoals"),
            ("sh_points", "shPoints"),
        ] {
            if let Some(v) = count(row, src)? {
                counts.insert(dest.into(), v as f64);
            }
        }
        if let Some((points, goals)) = count(row, "ppPoints")?.zip(count(row, "ppGoals")?) {
            let assists = points
                .checked_sub(goals)
                .ok_or("incoherent PP point counts")?;
            counts.insert("pp_assists".into(), assists as f64);
        }
        if let Some(v) = count(row, "penaltyMinutes")? {
            counts.insert("pim".into(), v as f64);
        }
        if let Some(pen) = penalties.as_ref().and_then(|p| p.get(&id)) {
            if count(pen, "gamesPlayed")? != Some(gp) {
                return Err("penalty GP mismatch".into());
            }
            for (dest, src) in [("drawn", "penaltiesDrawn"), ("taken", "penalties")] {
                if let Some(v) = count(pen, src)? {
                    counts.insert(dest.into(), v as f64);
                }
            }
        }
        if let Some(real) = rt.get(&id) {
            if count(real, "gamesPlayed")? != Some(gp) {
                return Err("summary/realtime GP mismatch".into());
            }
            for (dest, src) in [
                ("hits", "hits"),
                ("blocks", "blockedShots"),
                ("takeaways", "takeaways"),
                ("giveaways", "giveaways"),
            ] {
                if let Some(v) = count(real, src)? {
                    counts.insert(dest.into(), v as f64);
                }
            }
        }
        players.push(PlayerInput {
            player_id: id,
            name: row["skaterFullName"]
                .as_str()
                .ok_or("missing skater name")?
                .into(),
            group: group.into(),
            games: gp,
            counts,
        });
    }
    for (id, row) in goalies {
        let mut counts = BTreeMap::new();
        for (dest, src) in [
            ("starts", "gamesStarted"),
            ("shutouts", "shutouts"),
            ("shots_against", "shotsAgainst"),
            ("saves", "saves"),
        ] {
            if let Some(v) = count(row, src)? {
                counts.insert(dest.into(), v as f64);
            }
        }
        let games = count(row, "gamesPlayed")?.ok_or("missing goalie GP")?;
        if counts.get("starts").is_some_and(|s| *s > games as f64)
            || counts
                .get("saves")
                .zip(counts.get("shots_against"))
                .is_some_and(|(s, a)| s > a)
        {
            return Err("incoherent goalie counts".into());
        }
        players.push(PlayerInput {
            player_id: id,
            name: row["goalieFullName"]
                .as_str()
                .ok_or("missing goalie name")?
                .into(),
            group: "Goalies".into(),
            games,
            counts,
        });
    }
    Ok(Input {
        season: r.season,
        players,
        source_urls: r.source_urls,
        source_sha256: format!("{:x}", Sha256::digest(bytes)),
    })
}
