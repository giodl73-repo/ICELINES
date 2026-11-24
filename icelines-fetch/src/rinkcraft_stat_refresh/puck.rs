//! Source-complete ledger over selected finalized games. No capability scoring.
use super::{Capture, Store};
use crate::{nhl_api::NhlApiClient, teams::nhl_teams_for_season};
use icelines_sources::nhl::goalie_puck;
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub struct Request {
    pub season: u32,
    /// Empty means all teams; otherwise collect every finalized game of these teams.
    pub teams: Vec<String>,
    pub cache_dir: PathBuf,
    pub output_dir: PathBuf,
    pub offline: bool,
}
#[derive(Default, Serialize)]
struct Summary {
    player_id: u64,
    name: String,
    /// Roster membership includes backups. Never use this as GP or an exposure denominator.
    roster_games: u32,
    counts: BTreeMap<String, u32>,
    giveaways_followed_by_opponent_attempt_within_10s: u32,
    penalties_by_description: BTreeMap<String, u32>,
}
pub(super) async fn capture_games<T, F>(
    store: Store,
    season: u32,
    teams: BTreeSet<String>,
    parse: F,
) -> Result<(Vec<T>, Vec<Capture>), String>
where
    T: Send + 'static,
    F: Fn(&[u8], u64, u32) -> Result<T, String> + Copy + Send + 'static,
{
    let year = season / 10000;
    let mut captures: Vec<Capture> = vec![];
    let mut ids = BTreeSet::new();
    for team in &teams {
        let url = format!(
            "https://api-web.nhle.com/v1/club-schedule-season/{team}/{}",
            season
        );
        let (body, c) = store.get(&url, false, false).await?;
        let v: Value =
            serde_json::from_slice(&body.ok_or("Missing schedule")?).map_err(|e| e.to_string())?;
        for game in v["games"].as_array().ok_or("Missing schedule games")? {
            if game["gameType"].as_u64() == Some(2)
                && matches!(game["gameState"].as_str(), Some("OFF" | "FINAL"))
            {
                let id = game["id"]
                    .as_u64()
                    .ok_or("Missing scheduled game identity")?;
                if id / 1_000_000 != year as u64 {
                    return Err("Schedule contains wrong season".into());
                }
                ids.insert(id);
            }
        }
        captures.push(c);
    }
    if ids.is_empty() {
        return Err("No finalized regular-season games in selected schedules".into());
    }
    let ids: Vec<_> = ids.into_iter().collect();
    let mut games = vec![];
    for chunk in ids.chunks(4) {
        let mut jobs = vec![];
        for &id in chunk {
            let s = store.clone();
            let season = season;
            jobs.push(tokio::spawn(async move {
                let url = format!("https://api-web.nhle.com/v1/gamecenter/{id}/play-by-play");
                let (body, c) = s.get(&url, true, false).await?;
                let game = parse(&body.ok_or("Missing game")?, id, season)?;
                Ok::<_, String>((game, c))
            }));
        }
        for job in jobs {
            let (game, c) = job.await.map_err(|e| e.to_string())??;
            games.push(game);
            captures.push(c);
        }
    }
    Ok((games, captures))
}

pub async fn refresh(r: Request) -> Result<Value, String> {
    let year = r.season / 10000;
    if !(2008..=2200).contains(&year) || r.season % 10000 != year + 1 {
        return Err("Use a YYYYZZZZ season since 2008-09".into());
    }
    if r.output_dir.exists() {
        return Err("Choose a new output directory".into());
    }
    let valid = nhl_teams_for_season(&r.season.to_string());
    let teams: BTreeSet<String> = if r.teams.is_empty() {
        valid.iter().map(|s| s.to_string()).collect()
    } else {
        r.teams.iter().map(|s| s.to_ascii_uppercase()).collect()
    };
    if teams.iter().any(|t| !valid.contains(&t.as_str())) {
        return Err("Unknown team for season".into());
    }
    let store = Store {
        root: r.cache_dir.join(r.season.to_string()),
        offline: r.offline,
        client: NhlApiClient::production(),
    };
    let (games, captures) =
        capture_games(store, r.season, teams.clone(), goalie_puck::parse).await?;
    let captured_games = games.len();
    let mut summaries: BTreeMap<u64, Summary> = BTreeMap::new();
    for game in &games {
        for g in &game.goalies_on_roster {
            let row = summaries.entry(g.player_id).or_insert_with(|| Summary {
                player_id: g.player_id,
                name: g.name.clone(),
                counts: [
                    "goal",
                    "primary_assist",
                    "secondary_assist",
                    "giveaway",
                    "takeaway",
                    "penalty",
                ]
                .into_iter()
                .map(|s| (s.into(), 0))
                .collect(),
                ..Default::default()
            });
            row.roster_games += 1;
        }
        for e in &game.events {
            let row = summaries
                .get_mut(&e.player_id)
                .ok_or("Goalie event absent from roster")?;
            *row.counts.entry(e.kind.clone()).or_default() += 1;
            if e.following_opponent_attempt.is_some() {
                row.giveaways_followed_by_opponent_attempt_within_10s += 1;
            }
            if e.kind == "penalty" {
                *row.penalties_by_description
                    .entry(
                        e.penalty_description
                            .clone()
                            .unwrap_or_else(|| "unspecified".into()),
                    )
                    .or_default() += 1;
            }
        }
    }
    let event_count: usize = games.iter().map(|g| g.events.len()).sum();
    let report = json!({"schema_version":1,"season":r.season,"season_type":"regular",
        "teams":teams,"expected_games":captured_games,"captured_games":games.len(),
        "event_count":event_count,"goalies":summaries.into_values().collect::<Vec<_>>(),
        "limits":["Counts are credited events in captured logs, not all puck actions.",
          "Roster games include backups and are not appearances or puck-touch opportunities.",
          "No pass-completion rate or capability rating is inferred.",
          "Following attempts describe timing before a stoppage, not possession or causation.",
          "All goalie penalties are retained by description; not all are puck-handling errors."],
        "games":games,"captures":captures});
    fs::create_dir_all(&r.output_dir).map_err(|e| e.to_string())?;
    fs::write(
        r.output_dir.join("goalie-puck-events.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(
        json!({"season":r.season,"captured_games":captured_games,"event_count":event_count,
        "output":r.output_dir.join("goalie-puck-events.json")}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    fn cache(root: &std::path::Path, url: &str, value: Value) {
        fs::create_dir_all(root).unwrap();
        let body = serde_json::to_vec(&value).unwrap();
        let key = format!("{:x}", Sha256::digest(url.as_bytes()));
        fs::write(root.join(format!("{key}.bin")), &body).unwrap();
        fs::write(
            root.join(format!("{key}.json")),
            serde_json::to_vec(&Capture {
                url: url.into(),
                sha256: format!("{:x}", Sha256::digest(&body)),
                captured_at: "2026-11-23T00:00:00Z".into(),
                status: 200,
            })
            .unwrap(),
        )
        .unwrap();
    }
    #[tokio::test]
    async fn offline_coverage_deduplicates_schedules_without_fabricating_appearances() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("cache/20262027");
        let schedule = json!({"games":[
          {"id":2026020001_u64,"gameType":2,"gameState":"OFF"},
          {"id":2026020001_u64,"gameType":2,"gameState":"OFF"},
          {"id":2026020002_u64,"gameType":2,"gameState":"FUT"},
          {"id":2026030001_u64,"gameType":3,"gameState":"OFF"}]});
        for team in ["NYR", "SEA"] {
            cache(
                &root,
                &format!("https://api-web.nhle.com/v1/club-schedule-season/{team}/20262027"),
                schedule.clone(),
            );
        }
        cache(
            &root,
            "https://api-web.nhle.com/v1/gamecenter/2026020001/play-by-play",
            json!({
          "id":2026020001_u64,"season":20262027,"gameType":2,"gameState":"OFF",
          "rosterSpots":[{"playerId":1,"teamId":10,"positionCode":"G"}],
          "plays":[{"eventId":1,"sortOrder":1,"typeDescKey":"period-start"}]}),
        );
        let request = || Request {
            season: 20262027,
            teams: vec!["NYR".into(), "SEA".into()],
            cache_dir: tmp.path().join("cache"),
            output_dir: tmp.path().join("output"),
            offline: true,
        };
        refresh(request()).await.unwrap();
        let report: Value = serde_json::from_slice(
            &fs::read(tmp.path().join("output/goalie-puck-events.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(report["captured_games"], 1);
        assert_eq!(report["expected_games"], 1);
        assert_eq!(report["event_count"], 0);
        assert_eq!(report["goalies"][0]["roster_games"], 1);
        assert!(report["goalies"][0].get("games_played").is_none());
        assert!(report["goalies"][0].get("rating").is_none());
        assert!(refresh(request()).await.is_err());
        fs::remove_file(root.join(format!(
            "{:x}.bin",
            Sha256::digest(b"https://api-web.nhle.com/v1/gamecenter/2026020001/play-by-play")
        )))
        .unwrap();
        let mut missing = request();
        missing.output_dir = tmp.path().join("incomplete");
        assert!(refresh(missing).await.is_err());
        assert!(!tmp.path().join("incomplete").exists());
    }
}
