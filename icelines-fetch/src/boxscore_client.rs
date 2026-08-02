//! BoxscoreClient — fetches game logs and boxscores from the NHL web API,
//! then aggregates per-player position appearances into `PositionProfile`s
//! and linemate `ShiftProfile`s.

use std::collections::HashMap;
use std::time::Duration;

use icelines_core::{Position, PositionProfile};
use icelines_sources::nhl::position_boxscore::{BoxscoreResponse, GameLogResponse, SkaterEntry};

use crate::error::FetchError;
use crate::shift_profile::{
    build_profile_from_boxscores, parse_toi_mmss, BoxscoreData, BoxscorePlayerEntry, ShiftProfile,
};

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
            (
                "home",
                resp.player_by_game_stats.home_team.forwards.as_slice(),
            ),
            (
                "home",
                resp.player_by_game_stats.home_team.defense.as_slice(),
            ),
            (
                "away",
                resp.player_by_game_stats.away_team.forwards.as_slice(),
            ),
            (
                "away",
                resp.player_by_game_stats.away_team.defense.as_slice(),
            ),
        ];

        for (_side, team_skaters) in &all_teams {
            for skater in *team_skaters {
                positions.insert(skater.player_id, skater.position.clone());
            }
        }

        Ok(positions)
    }

    /// Fetch a boxscore and map it into a `BoxscoreData` for linemate analysis.
    ///
    /// TOI strings ("MM:SS") are parsed to integer seconds; malformed strings
    /// produce 0 without panicking.
    ///
    /// Endpoint: `GET {base_web}/v1/gamecenter/{game_id}/boxscore`
    pub async fn fetch_boxscore_data(&self, game_id: u64) -> Result<BoxscoreData, FetchError> {
        let url = format!("{}/gamecenter/{game_id}/boxscore", self.base_web);
        let resp: BoxscoreResponse = self.get_json(&url).await?;

        // We need to know which team abbreviation belongs to which side.
        // The NHL API doesn't embed the abbrev in playerByGameStats, so we
        // use "HOME" / "AWAY" as synthetic team tokens — sufficient for the
        // same-team co-occurrence logic in build_profile_from_boxscores.
        let home_abbrev = "HOME".to_owned();
        let away_abbrev = "AWAY".to_owned();

        let mut players: Vec<BoxscorePlayerEntry> = Vec::new();

        let teams: [(&str, &[SkaterEntry], &[SkaterEntry]); 2] = [
            (
                &home_abbrev,
                &resp.player_by_game_stats.home_team.forwards,
                &resp.player_by_game_stats.home_team.defense,
            ),
            (
                &away_abbrev,
                &resp.player_by_game_stats.away_team.forwards,
                &resp.player_by_game_stats.away_team.defense,
            ),
        ];

        for (team_token, forwards, defense) in &teams {
            for skater in forwards.iter().chain(defense.iter()) {
                let toi_secs = skater.toi.as_deref().map(parse_toi_mmss).unwrap_or(0);
                players.push(BoxscorePlayerEntry {
                    player_id: skater.player_id,
                    team: team_token.to_string(),
                    position: skater.position.clone(),
                    toi_secs,
                    shifts: skater.shifts.unwrap_or(0),
                });
            }
        }

        Ok(BoxscoreData {
            game_id,
            home_team: home_abbrev,
            away_team: away_abbrev,
            players,
        })
    }
}

// ── aggregate_shift_profiles ──────────────────────────────────────────────────

/// For each player ID, fetch their game log, fetch each boxscore, and build a
/// `ShiftProfile` capturing linemate co-occurrence.
///
/// Players with zero appearances are silently skipped.
pub async fn aggregate_shift_profiles(
    player_ids: &[u32],
    season: &str,
    client: &BoxscoreClient,
) -> Vec<ShiftProfile> {
    let mut profiles = Vec::new();

    for &player_id in player_ids {
        let game_ids = match client.fetch_game_log(player_id, season).await {
            Ok(ids) => ids,
            Err(_) => continue,
        };

        let mut boxscores: Vec<BoxscoreData> = Vec::new();
        for game_id in game_ids {
            match client.fetch_boxscore_data(game_id).await {
                Ok(bs) => boxscores.push(bs),
                Err(_) => continue, // skip individual game errors
            }
        }

        if let Some(profile) = build_profile_from_boxscores(player_id, &boxscores) {
            profiles.push(profile);
        }
    }

    profiles
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
