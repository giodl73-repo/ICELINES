use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use icelines_core::freshness::{FetchSource, Freshness, Ttl};
use icelines_core::identity::GameId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use serde::{Deserialize, Serialize};

use crate::atomic_write::write_bytes_atomic;
use crate::datastore::DataStore;
use crate::manifest::{DataKey, DataKind, ManifestEntry};
use crate::nhl_api::{NhlApiClient, ScheduledGame};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum GameCacheArtifact {
    Boxscore,
    PlayByPlay,
}

impl GameCacheArtifact {
    pub fn parse_list(value: &str) -> Result<Vec<Self>, String> {
        let mut out = Vec::new();
        for raw in value.split(',') {
            let token = raw.trim().to_ascii_lowercase();
            let artifact = match token.as_str() {
                "" => continue,
                "boxscore" | "boxscores" | "game-lines" => Self::Boxscore,
                "play-by-play" | "play_by_play" | "pbp" => Self::PlayByPlay,
                other => {
                    return Err(format!(
                        "unknown game-cache artifact '{other}' - valid: boxscore, play-by-play"
                    ));
                }
            };
            if !out.contains(&artifact) {
                out.push(artifact);
            }
        }
        if out.is_empty() {
            return Err("at least one game-cache artifact is required".to_string());
        }
        Ok(out)
    }

    fn data_kind(self) -> DataKind {
        match self {
            Self::Boxscore => DataKind::Boxscore,
            Self::PlayByPlay => DataKind::PlayByPlay,
        }
    }

    fn directory(self) -> &'static str {
        match self {
            Self::Boxscore => "boxscores",
            Self::PlayByPlay => "play_by_play",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameCacheLoadRequest {
    pub season: Season,
    pub season_type: SeasonType,
    pub teams: Vec<String>,
    pub artifacts: Vec<GameCacheArtifact>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameCacheLoadSummary {
    pub season: u32,
    pub season_type: String,
    pub teams: Vec<String>,
    pub scheduled_games: usize,
    pub final_games: usize,
    pub cached_artifacts: usize,
    pub fetched_artifacts: usize,
    pub failed_artifacts: usize,
    pub errors: Vec<String>,
}

pub async fn ensure_team_game_cache(
    data_root: impl AsRef<Path>,
    request: GameCacheLoadRequest,
) -> Result<GameCacheLoadSummary> {
    let data_root = data_root.as_ref();
    let store = DataStore::open(data_root).context("open DataStore")?;
    let client = NhlApiClient::production();
    ensure_team_game_cache_with_client(data_root, &store, &client, request).await
}

async fn ensure_team_game_cache_with_client(
    data_root: &Path,
    store: &DataStore,
    client: &NhlApiClient,
    request: GameCacheLoadRequest,
) -> Result<GameCacheLoadSummary> {
    let teams = normalize_teams(&request.teams);
    let mut summary = GameCacheLoadSummary {
        season: request.season.0,
        season_type: request.season_type.label().to_string(),
        teams: teams.clone(),
        ..GameCacheLoadSummary::default()
    };
    if teams.is_empty() {
        return Err(anyhow::anyhow!("at least one team is required"));
    }

    let mut games_by_id: BTreeMap<u64, ScheduledGame> = BTreeMap::new();
    let season = request.season.as_str();
    for team in &teams {
        match client.fetch_team_season_schedule(team, &season).await {
            Ok(games) => {
                for game in games {
                    games_by_id.entry(game.game_id).or_insert(game);
                }
            }
            Err(err) => summary
                .errors
                .push(format!("schedule fetch failed for {team}: {err}")),
        }
    }

    summary.scheduled_games = games_by_id.len();
    let final_games: Vec<ScheduledGame> = games_by_id
        .into_values()
        .filter(|game| game.game_type == game_type_for(request.season_type) && game.is_final())
        .collect();
    summary.final_games = final_games.len();

    for game in &final_games {
        for artifact in &request.artifacts {
            if store
                .manifest()
                .get(artifact.data_kind(), &DataKey::Game(GameId(game.game_id)))
                .is_some()
            {
                summary.cached_artifacts += 1;
                continue;
            }
            match persist_artifact(data_root, store, client, game, *artifact).await {
                Ok(()) => summary.fetched_artifacts += 1,
                Err(err) => {
                    summary.failed_artifacts += 1;
                    summary
                        .errors
                        .push(format!("game {} {:?}: {err}", game.game_id, artifact));
                }
            }
        }
    }

    Ok(summary)
}

async fn persist_artifact(
    data_root: &Path,
    store: &DataStore,
    client: &NhlApiClient,
    game: &ScheduledGame,
    artifact: GameCacheArtifact,
) -> Result<()> {
    let raw = match artifact {
        GameCacheArtifact::Boxscore => client
            .fetch_boxscore_with_raw(game.game_id)
            .await
            .map(|(_parsed, raw)| raw)
            .with_context(|| format!("fetch boxscore {}", game.game_id))?,
        GameCacheArtifact::PlayByPlay => client
            .fetch_play_by_play_with_raw(game.game_id)
            .await
            .map(|(_parsed, raw)| raw)
            .with_context(|| format!("fetch play-by-play {}", game.game_id))?,
    };

    let path = artifact_path(data_root, artifact, &game.date, game.game_id);
    let bytes = serde_json::to_vec(&raw).context("serialize game artifact")?;
    write_bytes_atomic(&path, &bytes).with_context(|| format!("write {}", path.display()))?;
    store.manifest().upsert(
        artifact.data_kind(),
        ManifestEntry {
            key: DataKey::Game(GameId(game.game_id)),
            path,
            freshness: Freshness {
                fetched_at: chrono::Utc::now(),
                source: FetchSource::Live,
                ttl: Ttl::Static,
            },
        },
    )?;
    Ok(())
}

fn artifact_path(
    data_root: &Path,
    artifact: GameCacheArtifact,
    date: &str,
    game_id: u64,
) -> PathBuf {
    data_root
        .join(artifact.directory())
        .join(date)
        .join(format!("{game_id}.json"))
}

fn game_type_for(season_type: SeasonType) -> u8 {
    match season_type {
        SeasonType::Regular => 2,
        SeasonType::Playoff => 3,
    }
}

pub fn normalize_teams(teams: &[String]) -> Vec<String> {
    teams
        .iter()
        .flat_map(|team| team.split(','))
        .map(|team| team.trim().to_ascii_uppercase())
        .filter(|team| !team.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_game_cache_artifacts_parse_aliases_and_deduplicate() {
        let artifacts = GameCacheArtifact::parse_list("boxscores,pbp,boxscore").unwrap();
        assert_eq!(
            artifacts,
            vec![GameCacheArtifact::Boxscore, GameCacheArtifact::PlayByPlay]
        );
    }

    #[test]
    fn l0_game_cache_rejects_unknown_artifact() {
        let err = GameCacheArtifact::parse_list("shifts").unwrap_err();
        assert!(err.contains("unknown game-cache artifact"));
    }

    #[test]
    fn l0_game_cache_normalizes_teams() {
        let teams = normalize_teams(&["edm, BOS".to_string(), "EDM".to_string()]);
        assert_eq!(teams, vec!["BOS".to_string(), "EDM".to_string()]);
    }

    #[test]
    fn l0_game_cache_artifact_paths_match_manifest_layout() {
        let root = PathBuf::from("data");
        let path = artifact_path(
            &root,
            GameCacheArtifact::PlayByPlay,
            "2026-01-01",
            2025020001,
        );
        assert_eq!(
            path,
            PathBuf::from("data")
                .join("play_by_play")
                .join("2026-01-01")
                .join("2025020001.json")
        );
    }
}
