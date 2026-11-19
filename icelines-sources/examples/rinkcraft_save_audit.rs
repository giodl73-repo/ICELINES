//! Offline source-consistency census for one explicitly selected goalie game.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
};
fn n(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64().ok_or_else(|| format!("missing count {k}"))
}
fn event_tally(pbp: &Value, actor: u64) -> Result<Value, String> {
    let mut saves = 0;
    let mut goals = 0;
    let mut shootout = 0;
    let mut periods: BTreeMap<u64, (u64, u64)> = BTreeMap::new();
    for p in pbp["plays"].as_array().ok_or("missing event ledger")? {
        if p["details"]["goalieInNetId"].as_u64() != Some(actor) {
            continue;
        }
        let kind = p["typeDescKey"].as_str().unwrap_or("");
        if !matches!(kind, "shot-on-goal" | "goal") {
            continue;
        }
        match p["periodDescriptor"]["periodType"].as_str() {
            Some("SO") => {
                shootout += 1;
                continue;
            }
            Some("REG" | "OT") => {}
            _ => return Err("unknown shot period type".into()),
        }
        let period = n(&p["periodDescriptor"], "number")?;
        let entry = periods.entry(period).or_default();
        if kind == "goal" {
            goals += 1;
            entry.1 += 1;
        } else {
            saves += 1;
            entry.0 += 1;
        }
    }
    Ok(
        json!({"shot_on_goal_events":saves,"goal_events":goals,"on_goal_labelled_events":saves+goals,"excluded_shootout_events":shootout,"periods":periods.iter().map(|(p,(s,g))|json!({"period":p,"shot_on_goal_events":s,"goal_events":g})).collect::<Vec<_>>(),"note":"Event-type census, not an independently corrected official shot ledger."}),
    )
}
fn audit(input: &Value) -> Result<Value, String> {
    let actor = n(input, "actor")?;
    let game = n(input, "game_id")?;
    let season = n(input, "season")?;
    // The HTML locator below was inspected for this case, not a generic parser.
    if actor != 8478048 || game != 2023021159 || season != 20232024 {
        return Err(
            "this investigation supports only the explicitly inspected Shesterkin case".into(),
        );
    }
    let mut sources = BTreeMap::new();
    let mut evidence = Vec::new();
    for source in input["sources"]
        .as_array()
        .ok_or("missing source manifest")?
    {
        let raw = fs::read(source["path"].as_str().ok_or("missing source path")?)
            .map_err(|e| e.to_string())?;
        let sha = format!("{:x}", Sha256::digest(&raw));
        if source["sha256"].as_str() != Some(sha.as_str()) {
            return Err("save audit source hash mismatch".into());
        }
        let key = source["kind"].as_str().ok_or("missing source kind")?;
        if sources.insert(key.to_owned(), raw).is_some() {
            return Err("duplicate source kind".into());
        }
        evidence.push(json!({"kind":key,"url":source["url"],"sha256":sha,"accessed_utc":source["accessed_utc"]}));
    }
    let parse = |key: &str| -> Result<Value, String> {
        serde_json::from_slice(sources.get(key).ok_or("missing source kind")?)
            .map_err(|e| e.to_string())
    };
    let stats = parse("game_stats")?;
    let boxscore = parse("boxscore")?;
    let pbp = parse("pbp")?;
    let note = parse("postgame_fact")?;
    let html = String::from_utf8(
        sources
            .get("html_summary")
            .ok_or("missing HTML summary")?
            .clone(),
    )
    .map_err(|e| e.to_string())?;
    if boxscore["id"].as_u64() != Some(game) || pbp["id"].as_u64() != Some(game) {
        return Err("save audit game identity mismatch".into());
    }
    let rows = stats["data"].as_array().ok_or("missing per-game stats")?;
    if stats["total"].as_u64() != Some(rows.len() as u64) {
        return Err("partial game ledger".into());
    }
    let mut ids = BTreeSet::new();
    let mut saves = 0;
    let mut shots = 0;
    let mut goals = 0;
    let mut bad = Vec::new();
    for row in rows {
        if n(row, "playerId")? != actor
            || n(row, "gameId")? / 1_000_000 != season / 10_000
            || (n(row, "gameId")? / 10_000) % 100 != 2
            || !ids.insert(n(row, "gameId")?)
        {
            return Err("mixed/duplicate season game actor".into());
        }
        let sv = n(row, "saves")?;
        let sa = n(row, "shotsAgainst")?;
        let ga = n(row, "goalsAgainst")?;
        saves += sv;
        shots += sa;
        goals += ga;
        if sv.checked_add(ga) != Some(sa) {
            bad.push(json!({"game_id":row["gameId"],"date":row["gameDate"],"saves":sv,"shots_against":sa,"goals_against":ga}));
        }
    }
    let row = rows
        .iter()
        .find(|r| r["gameId"].as_u64() == Some(game))
        .ok_or("selected game absent")?;
    let goalies: Vec<_> = ["awayTeam", "homeTeam"]
        .iter()
        .flat_map(|side| {
            boxscore["playerByGameStats"][side]["goalies"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .filter(|g| g["playerId"].as_u64() == Some(actor))
        .collect();
    if goalies.len() != 1 {
        return Err("ambiguous boxscore goalie".into());
    }
    if note["actor"].as_u64() != Some(actor)
        || note["game_id"].as_u64() != Some(game)
        || !html.contains("SHESTERKIN, IGOR")
        || !html.contains("2-40")
    {
        return Err("manual primary-source locator mismatch".into());
    }
    Ok(
        json!({"schema":"rinkcraft.save-source-audit.v1","actor":actor,"season":season,"game_id":game,"game_date":row["gameDate"],"state":"localized_unresolved","season_game_rows":rows.len(),"season_totals":{"reported_saves":saves,"shots_against":shots,"goals_against":goals},"inconsistent_games":bad,"selected_stats":{"saves":row["saves"],"shots_against":row["shotsAgainst"],"goals_against":row["goalsAgainst"]},"boxscore":{"saves":goalies[0]["saves"],"shots_against":goalies[0]["shotsAgainst"],"goals_against":goalies[0]["goalsAgainst"]},"event_tally":event_tally(&pbp,actor)?,"postgame_note":note,"html_summary":{"goals_against":2,"shots_against":40,"method":"Producer-read goalie total row (2-40); no separate save count asserted."},"sources":evidence,"conclusion":"Localized source disagreement, not a correction. Keep conflicted derived career saving value unavailable. Do not delete an event, alter a recorded save or infer a player mistake."}),
    )
}
fn main() {
    let a: Vec<_> = env::args().collect();
    let run = || -> Result<(), String> {
        if a.len() != 3 {
            return Err("Usage: SAVE_AUDIT_MANIFEST OUTPUT_JSON".into());
        }
        let input: Value = serde_json::from_slice(&fs::read(&a[1]).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        fs::write(
            &a[2],
            serde_json::to_vec(&audit(&input)?).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        println!("Save-count disagreement localized without changing the source or player record.");
        Ok(())
    };
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shootout_is_separate_from_game_save_tally() {
        let event = |kind, period| json!({"typeDescKey":kind,"periodDescriptor":{"periodType":period,"number":1},"details":{"goalieInNetId":1}});
        let p = json!({"plays":[event("shot-on-goal","REG"),event("goal","OT"),event("shot-on-goal","SO")]});
        let t = event_tally(&p, 1).unwrap();
        assert_eq!(t["on_goal_labelled_events"], 2);
        assert_eq!(t["excluded_shootout_events"], 1);
    }
    #[test]
    fn unknown_period_not_silently_regular() {
        assert!(event_tally(&json!({"plays":[{"typeDescKey":"goal","details":{"goalieInNetId":1},"periodDescriptor":{"periodType":"unknown"}}]}),1).is_err());
    }
}
