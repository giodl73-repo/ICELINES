use std::collections::{BTreeMap, BTreeSet, HashSet};
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
                "play-by-play" | "play_by_play" | "pbp" | "scoring" | "scoring-events"
                | "shot-events" | "shots" => Self::PlayByPlay,
                other => {
                    return Err(format!(
                        "unknown game-cache artifact '{other}' - valid: boxscore, play-by-play, scoring-events"
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

#[derive(Debug, Clone)]
pub struct FavoriteGameCacheLoadRequest {
    pub season: Season,
    pub season_type: SeasonType,
    pub player_ids: Vec<u32>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FavoriteGameCacheLoadSummary {
    pub season: u32,
    pub season_type: String,
    pub player_count: usize,
    pub team_count: usize,
    pub cache_requests: usize,
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

pub async fn ensure_favorites_game_cache(
    data_root: impl AsRef<Path>,
    request: FavoriteGameCacheLoadRequest,
) -> Result<FavoriteGameCacheLoadSummary> {
    let data_root = data_root.as_ref();
    let store = DataStore::open(data_root).context("open DataStore")?;
    let client = NhlApiClient::production();
    let requests = favorite_game_cache_requests(&request);
    let mut summary = FavoriteGameCacheLoadSummary {
        season: request.season.0,
        season_type: request.season_type.label().to_string(),
        player_count: request
            .player_ids
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len(),
        team_count: normalize_teams(&request.teams).len(),
        cache_requests: requests.len(),
        ..FavoriteGameCacheLoadSummary::default()
    };

    for request in requests {
        let result =
            ensure_team_game_cache_with_client(data_root, &store, &client, request).await?;
        summary.scheduled_games += result.scheduled_games;
        summary.final_games += result.final_games;
        summary.cached_artifacts += result.cached_artifacts;
        summary.fetched_artifacts += result.fetched_artifacts;
        summary.failed_artifacts += result.failed_artifacts;
        summary.errors.extend(result.errors);
    }

    Ok(summary)
}

pub fn favorite_game_cache_requests(
    request: &FavoriteGameCacheLoadRequest,
) -> Vec<GameCacheLoadRequest> {
    let mut grouped: BTreeMap<(Season, SeasonType), BTreeSet<String>> = BTreeMap::new();
    let active_teams = normalize_teams(&request.teams);
    if !active_teams.is_empty() {
        grouped
            .entry((request.season, request.season_type))
            .or_default()
            .extend(active_teams);
    }

    let player_ids: HashSet<u32> = request.player_ids.iter().copied().collect();
    if !player_ids.is_empty() {
        add_player_career_requests(&mut grouped, &player_ids);
    }

    grouped
        .into_iter()
        .map(|((season, season_type), teams)| GameCacheLoadRequest {
            season,
            season_type,
            teams: teams.into_iter().collect(),
            artifacts: request.artifacts.clone(),
        })
        .collect()
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

fn add_player_career_requests(
    grouped: &mut BTreeMap<(Season, SeasonType), BTreeSet<String>>,
    player_ids: &HashSet<u32>,
) {
    for season_id in crate::bundled::BUNDLED_SEASONS {
        let Ok(season_num) = season_id.parse::<u32>() else {
            continue;
        };
        let season = Season(season_num);

        if let Some(rows) = crate::bundled::get_stats(season_id) {
            for row in rows
                .iter()
                .filter(|row| player_ids.contains(&row.player_id) && row.games_played > 0)
            {
                if let Some(teams) = row.team_abbrevs.as_deref() {
                    add_teams(grouped, season, SeasonType::Regular, teams);
                }
            }
        }
        if let Some(rows) = crate::bundled::get_goalie_stats(season_id) {
            for row in rows
                .iter()
                .filter(|row| player_ids.contains(&row.player_id) && row.games_played > 0)
            {
                add_teams(
                    grouped,
                    season,
                    SeasonType::Regular,
                    row.team_abbrevs.as_str(),
                );
            }
        }

        let regular_fallback = regular_team_fallbacks(season_id, player_ids);
        if let Some(rows) = crate::bundled::get_playoff_stats(season_id) {
            for row in rows
                .iter()
                .filter(|row| player_ids.contains(&row.player_id) && row.games_played > 0)
            {
                if let Some(teams) = row.team_abbrevs.as_deref() {
                    add_teams(grouped, season, SeasonType::Playoff, teams);
                } else if let Some(team) = regular_fallback.get(&row.player_id) {
                    add_teams(grouped, season, SeasonType::Playoff, team);
                }
            }
        }
        if let Some(rows) = crate::bundled::get_playoff_goalie_stats(season_id) {
            for row in rows
                .iter()
                .filter(|row| player_ids.contains(&row.player_id) && row.games_played > 0)
            {
                add_teams(
                    grouped,
                    season,
                    SeasonType::Playoff,
                    row.team_abbrevs.as_str(),
                );
            }
        }
    }
}

fn regular_team_fallbacks(season_id: &str, player_ids: &HashSet<u32>) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    if let Some(rows) = crate::bundled::get_stats(season_id) {
        for row in rows
            .iter()
            .filter(|row| player_ids.contains(&row.player_id) && row.games_played > 0)
        {
            if let Some(teams) = row.team_abbrevs.as_deref() {
                if let Some(last) = split_team_abbrevs(teams).last() {
                    out.insert(row.player_id, last.clone());
                }
            }
        }
    }
    out
}

fn add_teams(
    grouped: &mut BTreeMap<(Season, SeasonType), BTreeSet<String>>,
    season: Season,
    season_type: SeasonType,
    teams: &str,
) {
    let teams = split_team_abbrevs(teams);
    if teams.is_empty() {
        return;
    }
    grouped
        .entry((season, season_type))
        .or_default()
        .extend(teams);
}

fn split_team_abbrevs(teams: &str) -> Vec<String> {
    teams
        .split(',')
        .map(|team| team.trim().to_ascii_uppercase())
        .filter(|team| !team.is_empty())
        .collect()
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
        let artifacts =
            GameCacheArtifact::parse_list("boxscores,pbp,boxscore,scoring-events,shots").unwrap();
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

    #[test]
    fn l0_favorite_game_cache_requests_include_player_career_and_team_year() {
        let request = FavoriteGameCacheLoadRequest {
            season: Season(20252026),
            season_type: SeasonType::Regular,
            player_ids: vec![8478402],
            teams: vec!["BOS".to_string()],
            artifacts: vec![GameCacheArtifact::Boxscore],
        };

        let requests = favorite_game_cache_requests(&request);
        assert!(
            requests.len() > 5,
            "Connor McDavid career should span multiple cached season requests"
        );
        let active = requests
            .iter()
            .find(|request| {
                request.season == Season(20252026)
                    && matches!(request.season_type, SeasonType::Regular)
            })
            .expect("active season request");
        assert!(active.teams.contains(&"EDM".to_string()));
        assert!(active.teams.contains(&"BOS".to_string()));
    }

    #[test]
    fn l0_favorite_game_cache_requests_include_goalie_careers() {
        let request = FavoriteGameCacheLoadRequest {
            season: Season(20252026),
            season_type: SeasonType::Regular,
            player_ids: vec![8476945],
            teams: Vec::new(),
            artifacts: vec![GameCacheArtifact::Boxscore],
        };

        let requests = favorite_game_cache_requests(&request);
        assert!(
            requests
                .iter()
                .any(|request| request.teams.contains(&"WPG".to_string())),
            "Connor Hellebuyck goalie career should plan WPG game-cache loads"
        );
    }
}
