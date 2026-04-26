//! BoxscoreClient — fetches game logs and boxscores from the NHL web API,
//! then aggregates per-player position appearances into `PositionProfile`s.

use std::collections::HashMap;
use std::time::Duration;

use icelines_core::{Position, PositionProfile};
use serde::Deserialize;

use crate::error::FetchError;

// ── Internal response shapes ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GameLogResponse {
    #[serde(rename = "gameLog")]
    game_log: Vec<GameLogEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameLogEntry {
    game_id: u64,
}

/// Minimal boxscore shape — we only need the skaters section.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BoxscoreResponse {
    player_by_game_stats: PlayerByGameStats,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PlayerByGameStats {
    home_team: TeamStats,
    away_team: TeamStats,
}

#[derive(Debug, Deserialize)]
struct TeamStats {
    forwards: Vec<SkaterEntry>,
    defense: Vec<SkaterEntry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkaterEntry {
    player_id: u32,
    position: String, // "C", "L", "R", "D"
}

// ── BoxscoreClient ────────────────────────────────────────────────────────────

/// HTTP client that fetches game logs and boxscores from the NHL web API.
///
/// `base_web` is configurable so tests can point at a mock server.
pub struct BoxscoreClient {
    pub base_web: String,
    client: reqwest::Client,
}

impl BoxscoreClient {
    /// Create a client with a custom base URL (useful for mocking in tests).
    pub fn new(base_web: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("icelines/0.1")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client construction is infallible with these options");
        Self {
            base_web: base_web.into(),
            client,
        }
    }

    /// Production client using the real NHL API endpoint.
    pub fn production() -> Self {
        Self::new("https://api-web.nhle.com/v1")
    }

    // ── Internal HTTP helper ─────────────────────────────────────────────────

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, FetchError> {
        let resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| FetchError::Http {
                status: 0,
                url: format!("{url}: {e}"),
            })?;

        let status = resp.status().as_u16();
        match status {
            200 => resp
                .json::<T>()
                .await
                .map_err(|e| FetchError::SchemaChanged {
                    detail: format!("{url}: {e}"),
                }),
            s => Err(FetchError::Http {
                status: s,
                url: url.to_owned(),
            }),
        }
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Fetch the list of game IDs this player appeared in during the regular
    /// season (`gameTypeId=2`) for `season` (e.g. "20252026").
    ///
    /// Endpoint: `GET {base_web}/v1/player/{player_id}/game-log/{season}/2`
    pub async fn fetch_game_log(
        &self,
        player_id: u32,
        season: &str,
    ) -> Result<Vec<u64>, FetchError> {
        let url = format!("{}/player/{player_id}/game-log/{season}/2", self.base_web);
        let resp: GameLogResponse = self.get_json(&url).await?;
        Ok(resp.game_log.iter().map(|e| e.game_id).collect())
    }

    /// Fetch a single boxscore and return a map of `player_id → position string`.
    ///
    /// Position strings are the raw NHL API codes: "C", "L", "R", "D".
    ///
    /// Endpoint: `GET {base_web}/v1/gamecenter/{game_id}/boxscore`
    pub async fn fetch_boxscore_positions(
        &self,
        game_id: u64,
    ) -> Result<HashMap<u32, String>, FetchError> {
        let url = format!("{}/gamecenter/{game_id}/boxscore", self.base_web);
        let resp: BoxscoreResponse = self.get_json(&url).await?;

        let mut positions: HashMap<u32, String> = HashMap::new();

        let all_teams = [
            resp.player_by_game_stats.home_team.forwards.as_slice(),
            resp.player_by_game_stats.home_team.defense.as_slice(),
            resp.player_by_game_stats.away_team.forwards.as_slice(),
            resp.player_by_game_stats.away_team.defense.as_slice(),
        ];

        for team_skaters in &all_teams {
            for skater in *team_skaters {
                positions.insert(skater.player_id, skater.position.clone());
            }
        }

        Ok(positions)
    }
}

// ── aggregate_profiles ────────────────────────────────────────────────────────

/// For each player ID, fetch their game log, then fetch each boxscore and
/// tally how many games they appeared at each position.  Build a
/// `PositionProfile` for each player that has at least one appearance.
///
/// Players with zero appearances (empty game log or not found in any boxscore)
/// are silently skipped — `PositionProfile::build` returns `None` for them.
pub async fn aggregate_profiles(
    player_ids: &[u32],
    season: &str,
    client: &BoxscoreClient,
) -> Vec<PositionProfile> {
    let mut profiles = Vec::new();

    for &player_id in player_ids {
        // Fetch game log — skip this player on error (best-effort)
        let game_ids = match client.fetch_game_log(player_id, season).await {
            Ok(ids) => ids,
            Err(_) => continue,
        };

        let mut appearances: HashMap<Position, u32> = HashMap::new();

        for game_id in game_ids {
            // Fetch boxscore — skip individual games on error
            let pos_map = match client.fetch_boxscore_positions(game_id).await {
                Ok(m) => m,
                Err(_) => continue,
            };

            if let Some(pos_str) = pos_map.get(&player_id) {
                if let Some(pos) = Position::from_api_code(pos_str) {
                    *appearances.entry(pos).or_insert(0) += 1;
                }
            }
        }

        if let Some(profile) = PositionProfile::build(player_id, season.to_owned(), appearances) {
            profiles.push(profile);
        }
    }

    profiles
}
