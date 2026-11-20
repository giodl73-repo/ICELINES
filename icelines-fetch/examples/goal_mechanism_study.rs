//! Reproducible exploratory cohort acquisition through the existing goal service.
//! Usage: goal_mechanism_study PROTOCOL_JSON OUTPUT_DIRECTORY [acquire]
use anyhow::{Context, Result};
use icelines_fetch::{atomic_write::write_json_atomic, goal_analysis, goal_visualizer as gv};
use serde_json::{json, Value};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

fn spread_indices(n: usize, count: usize) -> Result<Vec<usize>> {
    anyhow::ensure!(
        count > 0 && n >= count,
        "Insufficient scoring games for the declared sample"
    );
    Ok((0..count)
        .map(|k| {
            if count == 1 {
                0
            } else {
                k * (n - 1) / (count - 1)
            }
        })
        .collect())
}
async fn capture(client: &reqwest::Client, url: &str, path: &Path) -> Result<Value> {
    if path.exists() {
        return Ok(serde_json::from_slice(&std::fs::read(path)?)?);
    }
    let value: Value = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    write_json_atomic(path, &value)?;
    Ok(value)
}
fn goal_order(g: &gv::GoalVisualizerGoal) -> (u8, String, u32) {
    (g.period, g.time_in_period.clone(), g.event_id)
}
fn player_scoring_goals(landing: &Value, player: u64) -> Result<Vec<gv::GoalVisualizerGoal>> {
    let mut goals = gv::discover_goals(landing)?;
    // Landing includes shootout conversions; player game-log goals exclude them.
    goals.retain(|g| g.scorer_id.map(u64::from) == Some(player) && g.period_type != "SO");
    goals.sort_by_key(goal_order);
    Ok(goals)
}
#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    anyhow::ensure!(
        args.len() == 2 || (args.len() == 3 && args[2] == "acquire"),
        "usage: goal_mechanism_study PROTOCOL_JSON OUTPUT_DIRECTORY [acquire]"
    );
    let protocol_bytes = std::fs::read(&args[0])?;
    let protocol: Value = serde_json::from_slice(&protocol_bytes)?;
    anyhow::ensure!(
        protocol["schema"] == "rinkcraft.goal-mechanism-study.v0.1",
        "Unsupported study protocol"
    );
    let season = protocol["season"].as_u64().context("season")?;
    anyhow::ensure!(season / 10000 + 1 == season % 10000, "Invalid season");
    let game_type = protocol["game_type"].as_u64().context("game_type")?;
    anyhow::ensure!(game_type == 2, "This pilot requires one regular season");
    let count = protocol["goals_per_player"]
        .as_u64()
        .context("goals_per_player")? as usize;
    let players = protocol["players"].as_array().context("players")?;
    anyhow::ensure!(
        !players.is_empty() && players.len() <= 32 && (1..=25).contains(&count),
        "Study exceeds bounded acquisition limits"
    );
    let mut ids = BTreeSet::new();
    for p in players {
        anyhow::ensure!(
            ids.insert(p["player_id"].as_u64().context("player_id")?),
            "Duplicate cohort player"
        );
    }
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(out.join("sources"))?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(45))
        .build()?;
    let mut selections = vec![];
    let mut population = vec![];
    for p in players {
        let id = p["player_id"].as_u64().unwrap();
        let identity_url = format!("https://api-web.nhle.com/v1/player/{id}/landing");
        let identity_path = out.join(format!("sources/{id}.identity.json"));
        let identity = capture(&client, &identity_url, &identity_path).await?;
        let official_name = format!(
            "{} {}",
            identity["firstName"]["default"]
                .as_str()
                .context("firstName")?,
            identity["lastName"]["default"]
                .as_str()
                .context("lastName")?
        );
        anyhow::ensure!(
            identity["playerId"].as_u64() == Some(id)
                && p["name"].as_str() == Some(official_name.as_str()),
            "Player identity/name mismatch for {id}: {official_name}"
        );
        let url = format!("https://api-web.nhle.com/v1/player/{id}/game-log/{season}/{game_type}");
        let path = out.join(format!("sources/{id}.game-log.json"));
        let log = capture(&client, &url, &path).await?;
        anyhow::ensure!(
            log["seasonId"].as_u64() == Some(season)
                && log["gameTypeId"].as_u64() == Some(game_type),
            "Game log context mismatch"
        );
        let mut games: Vec<_> = log["gameLog"]
            .as_array()
            .context("gameLog")?
            .iter()
            .filter(|g| g["goals"].as_u64().unwrap_or(0) > 0)
            .collect();
        games.sort_by_key(|g| {
            (
                g["gameDate"].as_str().unwrap_or(""),
                g["gameId"].as_u64().unwrap_or(0),
            )
        });
        let unique: BTreeSet<_> = games.iter().filter_map(|g| g["gameId"].as_u64()).collect();
        anyhow::ensure!(unique.len() == games.len(), "Duplicate or missing game ID");
        let indices = spread_indices(games.len(), count)?;
        population.push(json!({"player":p,"identity_source_url":identity_url,"identity_sha256":gv::sha256_hex(&std::fs::read(&identity_path)?),"scoring_games":games.len(),"season_goals":games.iter().map(|g|g["goals"].as_u64().unwrap()).sum::<u64>(),"source_url":url,"source_sha256":gv::sha256_hex(&std::fs::read(&path)?),"selected_indices":indices}));
        for (slot, index) in indices.iter().enumerate() {
            let game = games[*index];
            selections.push(json!({"player":p,"slot":slot,"scoring_game_rank":index,"game_id":game["gameId"],"game_date":game["gameDate"],"goals_in_game":game["goals"]}));
        }
        println!(
            "{}: {} scoring games, {} selected",
            p["name"],
            games.len(),
            count
        );
    }
    let plan = json!({"schema":"icelines.goal-study-plan.v0.1","protocol_sha256":gv::sha256_hex(&protocol_bytes),"protocol":protocol,"population":population,"selections":selections});
    let plan_path = out.join("plan.json");
    if plan_path.exists() {
        anyhow::ensure!(
            serde_json::from_slice::<Value>(&std::fs::read(&plan_path)?)? == plan,
            "Stored plan changed; use a separate output directory"
        );
    }
    write_json_atomic(&plan_path, &plan)?;
    if args.len() == 2 {
        return Ok(());
    }
    let mut records = vec![];
    let root = goal_analysis::default_data_root();
    for selection in plan["selections"].as_array().unwrap() {
        let game = selection["game_id"].as_u64().unwrap();
        let id = selection["player"]["player_id"].as_u64().unwrap();
        let mut identified_goal = None;
        let attempt:Result<Value>=async {
            let landing=goal_analysis::discover_game(game,&root,true).await?;
            let goals=player_scoring_goals(&landing,id)?;
            anyhow::ensure!(Some(goals.len() as u64)==selection["goals_in_game"].as_u64(),"Game-log and landing goal totals disagree");
            let eligible:Vec<_>=goals.iter().filter(|g|g.replay_url.is_some()).collect();
            let goal=eligible.first().context("No published tracking for this scorer in the selected game")?;
            identified_goal=Some((*goal).clone());
            let target=goal_analysis::GoalTarget::new(game,goal.event_id)?;
            let report=goal_analysis::prepare(target,&root,true,None).await?;
            let file=out.join(format!("goals/{game}-{}.json",goal.event_id));
            write_json_atomic(&file,&report)?;
            let e=&report["audit"]["events"][0];
            let mut observation_ids=BTreeSet::new();
            for p in e["players"].as_array().unwrap() { if let Some(id)=p["player_id"].as_u64(){observation_ids.insert(id);} }
            let questions=report["questions"].clone();
            Ok(json!({"selection":selection,"status":"captured","target":target,"report_file":file.file_name().unwrap().to_string_lossy(),
                "report_sha256":gv::sha256_hex(&std::fs::read(&file)?),"summary":report["summary"],"source_links":report["source_links"],
                "published_player_goals_in_game":goals.len(),"tracked_player_goals_in_game":eligible.len(),
                "observed_players":observation_ids,"annotation":{"status":"official-context-and-observation-pass","observed_sequence":null,
                    "candidate_mechanism":null,"supporting_players":null,"defensive_response":null,"goalie_execution":null,
                    "alternative_explanations":[],"questions":questions,"called_assignment":"unknown","video_reviewed":false,
                    "release_goal_aligned":false,"independently_reviewed":false}}))
        }.await;
        match attempt {
            Ok(record) => {
                println!(
                    "captured {}/{} · {}",
                    game, record["target"]["event_id"], selection["player"]["name"]
                );
                records.push(record);
            }
            Err(error) => {
                eprintln!("unavailable {game}: {error:#}");
                records.push(json!({"selection":selection,"status":"unavailable","identified_goal":identified_goal,"reason":format!("{error:#}")}));
            }
        }
        write_json_atomic(
            &out.join("records.json"),
            &json!({"schema":"icelines.goal-study-records.v0.1","protocol_sha256":plan["protocol_sha256"],"records":records}),
        )?;
    }
    println!(
        "Captured {} of {} planned cases",
        records.iter().filter(|r| r["status"] == "captured").count(),
        records.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spread_covers_endpoints_without_replacement() {
        assert_eq!(spread_indices(12, 4).unwrap(), vec![0, 3, 7, 11]);
        assert_eq!(spread_indices(10, 10).unwrap(), (0..10).collect::<Vec<_>>());
        assert_eq!(spread_indices(20, 1).unwrap(), vec![0]);
        assert!(spread_indices(9, 10).is_err());
        assert!(spread_indices(10, 0).is_err());
    }
    #[test]
    fn chronological_goal_selection_ignores_event_id_order() {
        let goal = |event_id, period, time: &str| gv::GoalVisualizerGoal {
            event_id,
            period,
            time_in_period: time.into(),
            period_type: "REG".into(),
            situation_code: None,
            strength: None,
            scorer_id: Some(1),
            scorer_name: None,
            team_abbrev: None,
            shot_type: None,
            home_team_defending_side: None,
            assists: vec![],
            replay_url: None,
        };
        let mut goals = vec![
            goal(2, 3, "01:01"),
            goal(900, 1, "15:00"),
            goal(800, 1, "02:00"),
        ];
        goals.sort_by_key(goal_order);
        assert_eq!(
            goals.iter().map(|g| g.event_id).collect::<Vec<_>>(),
            vec![800, 900, 2]
        );
    }
    #[test]
    fn shootout_conversions_do_not_count_as_player_goals() {
        let landing = json!({"summary":{"scoring":[
            {"periodDescriptor":{"number":3,"periodType":"REG"},"goals":[{"eventId":10,"playerId":1,"timeInPeriod":"01:00"}]},
            {"periodDescriptor":{"number":4,"periodType":"OT"},"goals":[{"eventId":20,"playerId":1,"timeInPeriod":"02:00"}]},
            {"periodDescriptor":{"number":5,"periodType":"SO"},"goals":[{"eventId":30,"playerId":1,"timeInPeriod":"00:00"}]}
        ]}});
        let goals = player_scoring_goals(&landing, 1).unwrap();
        assert_eq!(
            goals.iter().map(|g| g.event_id).collect::<Vec<_>>(),
            vec![10, 20]
        );
    }
}
