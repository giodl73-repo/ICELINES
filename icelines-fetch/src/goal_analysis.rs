//! One-goal observation service. Published tracking remains separate from
//! inferred possession, active personnel, coaching duties and player grades.
use crate::{goal_visualizer as gv, nhl_api::NhlApiClient};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
#[path = "goal_analysis_motion.rs"]
mod motion;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GoalTarget {
    pub game_id: u64,
    pub event_id: u32,
}
impl GoalTarget {
    pub fn new(game_id: u64, event_id: u32) -> Result<Self> {
        anyhow::ensure!(
            (2_000_000_000..=2_099_999_999).contains(&game_id) && event_id > 0,
            "Use a ten-digit NHL game ID and a positive event ID"
        );
        Ok(Self { game_id, event_id })
    }
    pub fn parse(input: &str) -> Result<Self> {
        let value = input.trim();
        let parts: Vec<String> = if value.starts_with("https://") {
            let u = reqwest::Url::parse(value)?;
            anyhow::ensure!(
                u.username().is_empty()
                    && u.password().is_none()
                    && u.port_or_known_default() == Some(443),
                "Use an official NHL goal link"
            );
            let p: Vec<_> = u
                .path_segments()
                .context("Goal URL has no path")?
                .filter(|s| !s.is_empty())
                .collect();
            match u.host_str(){
    Some("www.nhl.com"|"nhl.com") if p.len()==4&&p[0]=="ppt-replay"&&p[1]=="goal"=>vec![p[2].into(),p[3].into()],
    Some("wsr.nhle.com") if p.len()==4&&p[0]=="sprites"=>{
     let event=p[3].strip_prefix("ev").and_then(|s|s.strip_suffix(".json")).context("Invalid NHL tracking event URL")?;
     let season=p[2].get(..4).and_then(|s|s.parse::<u32>().ok()).context("Invalid NHL game ID")?;
     anyhow::ensure!(p[1]==format!("{season}{}",season+1),"Tracking URL season does not match game");vec![p[2].into(),event.into()]
    },
    _=>bail!("Paste an NHL Goal Visualizer link or GAME_ID/EVENT_ID; video and game-page links do not identify one goal")
   }
        } else {
            value.split('/').map(str::to_owned).collect()
        };
        anyhow::ensure!(
            parts.len() == 2,
            "Use GAME_ID/EVENT_ID or an NHL Goal Visualizer URL"
        );
        Self::new(
            parts[0].parse().context("Invalid game ID")?,
            parts[1].parse().context("Invalid event ID")?,
        )
    }
    pub fn key(self) -> String {
        format!("{}/{}", self.game_id, self.event_id)
    }
}
pub fn default_data_root() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".icelines/data")
}
fn read(p: &Path) -> Result<Value> {
    Ok(serde_json::from_slice(&std::fs::read(p).with_context(
        || format!("Source capture unavailable: {}", p.display()),
    )?)?)
}
fn source_path(dir: &Path, game: u64, kind: &str) -> PathBuf {
    dir.join(format!("{game}.{kind}.json"))
}
pub fn analyze_sources(
    target: GoalTarget,
    bundle: &gv::GoalVisualizerBundle,
    landing: &Value,
    boxscore: &Value,
    pbp: &Value,
) -> Result<Value> {
    anyhow::ensure!(
        bundle.game_id == target.game_id,
        "Tracking bundle game mismatch"
    );
    GoalTarget::new(target.game_id, target.event_id)?;
    let mut one = bundle.clone();
    one.events.retain(|e| e.goal.event_id == target.event_id);
    anyhow::ensure!(
        one.events.len() == 1,
        "Selected goal has no captured tracking sequence"
    );
    let e = &one.events[0];
    gv::validate_replay_url(&e.source_url)?;
    anyhow::ensure!(
        GoalTarget::parse(&e.source_url)? == target,
        "Tracking URL does not match selected goal"
    );
    anyhow::ensure!(
        e.frame_count == e.frames.len(),
        "Tracking frame count mismatch"
    );
    let audit = crate::goal_sequence_audit::audit_bundle(&one, landing, boxscore, pbp)?;
    let event = &audit["events"][0];
    let scorer = e.goal.scorer_id.context("Unknown goal scorer")?;
    anyhow::ensure!(
        event["official_goal_details"]["scoringPlayerId"].as_u64() == Some(u64::from(scorer)),
        "Scorer attribution conflicts across sources"
    );
    let official_goal = landing["summary"]["scoring"]
        .as_array()
        .context("Missing goal summary")?
        .iter()
        .flat_map(|p| p["goals"].as_array().into_iter().flatten())
        .find(|g| g["eventId"].as_u64() == Some(u64::from(target.event_id)))
        .context("Selected event is not an official goal")?;
    anyhow::ensure!(
        official_goal["playerId"].as_u64() == Some(u64::from(scorer)),
        "Landing scorer conflicts with tracking metadata"
    );
    let mut runs: Vec<Value> = vec![];
    for f in event["frames"].as_array().context("No frames")? {
        let id = f["nearest_puck_observation"]["player_id"].as_u64();
        if let Some(last) = runs.last_mut().filter(|r| r["player_id"].as_u64() == id) {
            last["last_frame"] = f["frame"].clone();
            last["last_native_tick"] = f["elapsed_native_ticks"].clone();
        } else {
            let name = event["players"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["player_id"].as_u64() == id)
                .map(|p| p["roster"]["name"].clone())
                .unwrap_or(Value::Null);
            runs.push(json!({"player_id":id,"name":name,"first_frame":f["frame"],"last_frame":f["frame"],"first_native_tick":f["elapsed_native_ticks"],"last_native_tick":f["elapsed_native_ticks"],"interpretation":"nearest observed point; not possession or an inferred pass"}));
        }
    }
    let empty = event["goal_modifier"] == "empty-net";
    let goalie = if empty {
        "empty_net_recorded"
    } else if event["official_goal_details"]["goalieInNetId"].is_number() {
        "goalie_recorded"
    } else {
        "unknown"
    };
    let mut questions = vec![
        "Which observed puck/player movements created the opportunity?",
        "What changed in defensive geometry, and which alternatives remained?",
        "What called coverage or handoff is actually documented for this unit?",
    ];
    if empty {
        questions.push("How did goalkeeper absence and the extra attacker change pursuit, release and return responsibilities?");
    }
    if e.goal.strength.as_deref() == Some("sh") {
        questions.push("What control/transition cue created the shorthanded advancing threat, and how did the power-play unit recover?");
    }
    if e.goal.shot_type.as_deref() == Some("tip-in") {
        questions.push("Which stick/contact evidence distinguishes delivery, receiver movement and redirection?");
    }
    questions.push("What do video and shift records add about contact, active personnel, release/goal timing and goalie vision?");
    Ok(
        json!({"schema":"icelines.goal-analysis.v0.1","target":target,"audit":audit,
  "summary":{"game_date":landing["gameDate"],"away_team":landing["awayTeam"]["abbrev"],"home_team":landing["homeTeam"]["abbrev"],"goal":e.goal,"goal_modifier":event["goal_modifier"],"goalkeeper_context":goalie,"frame_count":e.frames.len(),"unique_observed_players":e.tracked_player_ids.len(),"attribution_status":"official scorer/assists; no called duties inferred"},
  "geometry_changes":motion::comparisons(event),
  "nearest_observation_runs":runs,"questions":questions,"assignment_assessment":{"status":"unresolved","reason":"Tracking geometry alone does not establish called duties, possession or execution fault."},
  "source_links":{"visualizer":event["visualizer_url"],"tracking":e.source_url,"video":official_goal["highlightClipSharingUrl"],"landing":format!("https://api-web.nhle.com/v1/gamecenter/{}/landing",target.game_id)},
  "evidence_status":{"game_context":"official_source","native_geometry":"computed_from_published_frames","possession":"unverified","active_ice_membership":"unverified","goal_frame_alignment":"unverified","coordinate_time_scale":"unverified","called_assignments":"unverified"}}),
    )
}
async fn lock(path: PathBuf) -> Result<crate::fetch_lock::FetchLockGuard> {
    Ok(tokio::task::spawn_blocking(move || {
        crate::fetch_lock::acquire(&path, std::time::Duration::from_secs(30))
    })
    .await??)
}
async fn store_discovery(game: u64, root: &Path, landing: &Value) -> Result<()> {
    let _guard = lock(
        root.join("goal_analysis/locks/discovery")
            .join(game.to_string()),
    )
    .await?;
    crate::atomic_write::write_json_atomic(
        &root
            .join("goal_analysis/discovery")
            .join(format!("{game}.landing.json")),
        landing,
    )?;
    Ok(())
}
pub async fn discover_game(game: u64, data_root: &Path, allow_live: bool) -> Result<Value> {
    GoalTarget::new(game, 1)?;
    let path = data_root
        .join("goal_analysis/discovery")
        .join(format!("{game}.landing.json"));
    if path.exists() {
        return read(&path);
    }
    anyhow::ensure!(
        allow_live,
        "No stored goal list; live NHL access is disabled"
    );
    let _guard = lock(
        data_root
            .join("goal_analysis/locks/discovery")
            .join(game.to_string()),
    )
    .await?;
    if path.exists() {
        return read(&path);
    }
    let landing = NhlApiClient::production()
        .fetch_game_landing_raw(game)
        .await?;
    crate::atomic_write::write_json_atomic(&path, &landing)?;
    Ok(landing)
}
pub async fn prepare(
    target: GoalTarget,
    data_root: &Path,
    allow_live: bool,
    offline: Option<&Path>,
) -> Result<Value> {
    let cache = data_root
        .join("goal_analysis")
        .join(target.game_id.to_string())
        .join(target.event_id.to_string());
    GoalTarget::new(target.game_id, target.event_id)?;
    let _guard = lock(
        data_root
            .join("goal_analysis/locks/goals")
            .join(target.game_id.to_string())
            .join(target.event_id.to_string()),
    )
    .await?;
    let source = offline.unwrap_or(&cache);
    let captured = ["bundle", "landing", "boxscore", "play-by-play"]
        .iter()
        .all(|k| source_path(source, target.game_id, k).exists());
    let (bundle, landing, boxscore, pbp) = if captured {
        (
            gv::load_bundle(&source_path(source, target.game_id, "bundle"))?,
            read(&source_path(source, target.game_id, "landing"))?,
            read(&source_path(source, target.game_id, "boxscore"))?,
            read(&source_path(source, target.game_id, "play-by-play"))?,
        )
    } else {
        anyhow::ensure!(offline.is_none(),"Offline capture needs GAME.bundle.json, GAME.landing.json, GAME.boxscore.json and GAME.play-by-play.json");
        anyhow::ensure!(
            allow_live,
            "Goal sources are not stored; live NHL access is disabled"
        );
        let client = NhlApiClient::production();
        let landing = discover_game(target.game_id, data_root, true).await?;
        let goal = gv::discover_goals(&landing)?
            .into_iter()
            .find(|g| g.event_id == target.event_id)
            .context("Selected event is not a goal in the official game summary")?;
        let url = goal.replay_url.as_deref().context(
            "NHL tracking is unavailable for this goal; no positional analysis can be generated",
        )?;
        anyhow::ensure!(
            GoalTarget::parse(url)? == target,
            "Published tracking URL goal mismatch"
        );
        let bytes = gv::fetch_replay_bytes(
            target.game_id,
            target.event_id,
            url,
            data_root.join(".fletch"),
            false,
        )
        .await?;
        let raw: Value = serde_json::from_slice(&bytes)?;
        let ((_, boxscore), (_, pbp)) = tokio::try_join!(
            client.fetch_boxscore_with_raw(target.game_id),
            client.fetch_play_by_play_with_raw(target.game_id)
        )?;
        let mut bundle = gv::GoalVisualizerBundle::new(target.game_id, chrono::Utc::now());
        bundle.upsert_event(gv::build_event(goal, &raw, gv::sha256_hex(&bytes))?);
        (bundle, landing, boxscore, pbp)
    };
    let mut result = analyze_sources(target, &bundle, &landing, &boxscore, &pbp)?;
    let records = [
        ("bundle", serde_json::to_value(&bundle)?),
        ("landing", landing),
        ("boxscore", boxscore),
        ("play-by-play", pbp),
    ];
    let mut hashes = vec![];
    for (kind, value) in records {
        let path = source_path(&cache, target.game_id, kind);
        let hash = gv::sha256_hex(&serde_json::to_vec(&value)?);
        crate::atomic_write::write_json_atomic(&path, &value)?;
        hashes.push(json!({"kind":kind,"sha256":hash}));
        if kind == "landing" {
            store_discovery(target.game_id, data_root, &value).await?;
        }
    }
    result["source_capture"] = json!({"origin":if offline.is_some(){"explicit_offline_capture"}else if captured{"stored_capture"}else{"official_live_fetch"},"inputs":hashes,"tracking_source_sha256":bundle.events.iter().find(|e|e.goal.event_id==target.event_id).unwrap().source_sha256});
    crate::atomic_write::write_json_atomic(&cache.join("analysis.json"), &result)?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selectors_identify_one_goal_and_reject_ambiguous_or_external_targets() {
        let t = GoalTarget::new(2025021167, 996).unwrap();
        for s in [
            "2025021167/996",
            "https://www.nhl.com/ppt-replay/goal/2025021167/996?foo=1",
            "https://wsr.nhle.com/sprites/20252026/2025021167/ev996.json",
        ] {
            assert_eq!(GoalTarget::parse(s).unwrap(), t);
        }
        for s in [
            "2025021167",
            "2025021167/0",
            "https://evil.test/ppt-replay/goal/2025021167/996",
            "https://www.nhl.com@evil.test/ppt-replay/goal/2025021167/996",
            "https://www.nhl.com:444/ppt-replay/goal/2025021167/996",
            "https://wsr.nhle.com/sprites/20242025/2025021167/ev996.json",
            "https://www.nhl.com/video/example",
        ] {
            assert!(GoalTarget::parse(s).is_err(), "{s}");
        }
    }
    #[tokio::test]
    async fn offline_and_no_live_do_not_silently_fetch_or_invent_analysis() {
        let dir = tempfile::tempdir().unwrap();
        let t = GoalTarget::new(2025021167, 996).unwrap();
        assert!(prepare(t, dir.path(), false, None)
            .await
            .unwrap_err()
            .to_string()
            .contains("disabled"));
        assert!(prepare(t, dir.path(), true, Some(dir.path()))
            .await
            .unwrap_err()
            .to_string()
            .contains("Offline capture"));
    }
}

#[cfg(test)]
#[path = "goal_analysis_tests.rs"]
mod source_tests;
