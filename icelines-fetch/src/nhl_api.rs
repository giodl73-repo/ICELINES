use crate::error::FetchError;
use crate::schema::{PagedResponse, PlayerContract, RosterResponse, SkaterBio, SkaterRealtime, SkaterStats};
use std::time::Duration;

const TEAMS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

/// Async NHL API client.
/// `base_url_stats` and `base_url_web` are configurable to allow mocking in tests.
pub struct NhlApiClient {
    client: reqwest::Client,
    base_stats: String, // https://api.nhle.com/stats/rest/en
    base_web: String,   // https://api-web.nhle.com/v1
    max_retries: u32,
    retry_base_ms: u64,
}

impl NhlApiClient {
    pub fn new(base_stats: impl Into<String>, base_web: impl Into<String>) -> Self {
        let client = reqwest::Client::builder()
            .user_agent("icelines/0.1")
            .timeout(Duration::from_secs(30))
            .build()
            .expect("reqwest client construction is infallible with these options");
        Self {
            client,
            base_stats: base_stats.into(),
            base_web: base_web.into(),
            max_retries: 3,
            retry_base_ms: 1000,
        }
    }

    /// Production constructor using the real NHL API endpoints.
    pub fn production() -> Self {
        Self::new(
            "https://api.nhle.com/stats/rest/en",
            "https://api-web.nhle.com/v1",
        )
    }

    // ── Internal HTTP helper ─────────────────────────────────────────────────

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, FetchError> {
        let mut attempt = 0u32;
        loop {
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
                200 => {
                    return resp
                        .json::<T>()
                        .await
                        .map_err(|e| FetchError::SchemaChanged {
                            detail: format!("{url}: {e}"),
                        });
                }
                429 => {
                    if attempt >= self.max_retries {
                        return Err(FetchError::RateLimited {
                            url: url.to_owned(),
                        });
                    }
                    let delay = self.retry_base_ms * (1 << attempt); // 1s, 2s, 4s
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
                }
                503 => {
                    return Err(FetchError::ServiceUnavailable {
                        url: url.to_owned(),
                    });
                }
                s => {
                    return Err(FetchError::Http {
                        status: s,
                        url: url.to_owned(),
                    });
                }
            }
        }
    }

    // ── Paginated bulk fetch helper ──────────────────────────────────────────

    async fn fetch_all_paged<T>(&self, endpoint: &str) -> Result<Vec<T>, FetchError>
    where
        T: serde::de::DeserializeOwned,
    {
        let mut all: Vec<T> = Vec::new();
        let mut start = 0usize;
        let limit = 100usize;
        loop {
            let url = format!("{endpoint}&limit={limit}&start={start}");
            let page: PagedResponse<T> = self.get_json(&url).await?;
            let page_len = page.data.len();
            all.extend(page.data);
            if all.len() >= page.total as usize || page_len == 0 {
                break;
            }
            start += limit;
        }
        Ok(all)
    }

    // ── Public API ───────────────────────────────────────────────────────────

    /// Fetch all skater bios for a season (paginated).
    pub async fn fetch_all_bios(&self, season: &str) -> Result<Vec<SkaterBio>, FetchError> {
        let endpoint = format!(
            "{}/skater/bios?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch all skater season stats for a season (paginated).
    pub async fn fetch_all_stats(&self, season: &str) -> Result<Vec<SkaterStats>, FetchError> {
        let endpoint = format!(
            "{}/skater/summary?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch the roster for one team.
    pub async fn fetch_team_roster(
        &self,
        team: &str,
        season: &str,
    ) -> Result<RosterResponse, FetchError> {
        let url = format!("{}/roster/{team}/{season}", self.base_web);
        self.get_json(&url).await
    }

    /// Fetch rosters for all 32 NHL teams.
    pub async fn fetch_all_rosters(
        &self,
        season: &str,
    ) -> Result<Vec<(String, RosterResponse)>, FetchError> {
        let mut results = Vec::new();
        for team in TEAMS {
            let roster = self.fetch_team_roster(team, season).await?;
            results.push((team.to_string(), roster));
        }
        Ok(results)
    }

    /// Fetch all skater realtime stats for a season (paginated).
    pub async fn fetch_all_realtime(&self, season: &str) -> Result<Vec<SkaterRealtime>, FetchError> {
        let endpoint = format!(
            "{}/skater/realtime?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch a URL as raw text (used for CSV downloads such as MoneyPuck).
    pub async fn fetch_text(&self, url: &str) -> Result<String, FetchError> {
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
        if status != 200 {
            return Err(FetchError::Http {
                status,
                url: url.to_owned(),
            });
        }
        resp.text().await.map_err(|e| FetchError::SchemaChanged {
            detail: format!("{url}: {e}"),
        })
    }

    /// Fetch contract/landing data for a single player.
    ///
    /// NOTE: As of 2026-04-26, the NHL landing API does not return contract fields
    /// (expiry_year, expiry_type, salary). The returned `PlayerContract` will have
    /// `player_id` populated and all other fields as `None`.
    pub async fn fetch_player_landing_contract(
        &self,
        player_id: u32,
    ) -> Result<PlayerContract, FetchError> {
        let url = format!("{}/player/{player_id}/landing", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;

        // Extract contract fields if/when the NHL API exposes them.
        // Current API response has no contract keys — all fields will be None.
        let expiry_year = raw["currentContract"]["expiryYear"]
            .as_u64()
            .map(|y| y as u16)
            .or_else(|| raw["expiryYear"].as_u64().map(|y| y as u16));

        let expiry_type = raw["currentContract"]["expiryType"]
            .as_str()
            .map(|s| s.to_owned())
            .or_else(|| raw["expiryType"].as_str().map(|s| s.to_owned()));

        let salary = raw["currentContract"]["capHit"]
            .as_u64()
            .or_else(|| raw["currentContract"]["salary"].as_u64())
            .or_else(|| raw["capHit"].as_u64());

        Ok(PlayerContract {
            player_id,
            expiry_year,
            expiry_type,
            salary,
        })
    }

    /// Batch-fetch contracts for all player IDs.
    /// Skips errors (logs them), returns what succeeded.
    /// Uses a 50ms delay between requests to avoid rate-limiting.
    pub async fn fetch_all_contracts(&self, player_ids: &[u32]) -> Vec<PlayerContract> {
        let mut results = Vec::with_capacity(player_ids.len());
        for &id in player_ids {
            match self.fetch_player_landing_contract(id).await {
                Ok(c) => results.push(c),
                Err(e) => eprintln!("  contracts: skipping player {id}: {e}"),
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        results
    }

    /// Fetch today's NHL schedule from /v1/schedule/now.
    /// Fetch the current game week schedule (up to 7 days from today).
    /// Returns all games with their calendar date attached.
    pub async fn fetch_today_schedule(&self) -> Result<Vec<ScheduledGame>, FetchError> {
        let url = format!("{}/schedule/now", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;

        let mut games = Vec::new();
        if let Some(week) = raw["gameWeek"].as_array() {
            for day in week {
                let date = day["date"].as_str().unwrap_or("").to_owned();
                if let Some(day_games) = day["games"].as_array() {
                    for g in day_games {
                        let game_id = g["id"].as_u64().unwrap_or(0);
                        let away = g["awayTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
                        let away_name = g["awayTeam"]["placeName"]["default"]
                            .as_str().unwrap_or(&away).to_owned();
                        let home = g["homeTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
                        let home_name = g["homeTeam"]["placeName"]["default"]
                            .as_str().unwrap_or(&home).to_owned();
                        let start     = g["startTimeUTC"].as_str().unwrap_or("").to_owned();
                        let game_type = g["gameType"].as_u64().unwrap_or(2) as u8;
                        let ss        = &g["seriesSummary"];
                        let series_game = ss["gameLabel"].as_str().map(str::to_owned);
                        let away_wins   = ss["awayWins"].as_u64().map(|v| v as u8);
                        let home_wins   = ss["homeWins"].as_u64().map(|v| v as u8);
                        if game_id > 0 {
                            games.push(ScheduledGame {
                                game_id,
                                date: date.clone(),
                                game_type,
                                away_abbrev: away,
                                away_name,
                                home_abbrev: home,
                                home_name,
                                start_time_utc: start,
                                series_game,
                                away_wins,
                                home_wins,
                            });
                        }
                    }
                }
            }
        }
        Ok(games)
    }
}

#[derive(Debug, Clone)]
pub struct ScheduledGame {
    pub game_id:         u64,
    pub date:            String,  // "YYYY-MM-DD"
    pub game_type:       u8,      // 1=preseason 2=regular 3=playoff
    pub away_abbrev:     String,
    pub away_name:       String,
    pub home_abbrev:     String,
    pub home_name:       String,
    pub start_time_utc:  String,
    // Playoff series context (game_type == 3 only)
    pub series_game:     Option<String>,  // e.g. "Game 4"
    pub away_wins:       Option<u8>,
    pub home_wins:       Option<u8>,
}

impl ScheduledGame {
    pub fn is_playoff(&self) -> bool { self.game_type == 3 }

    /// "EDM 2 – VGK 1 · Game 4" style label for playoffs.
    pub fn series_label(&self) -> Option<String> {
        let gm = self.series_game.as_deref()?;
        let aw = self.away_wins?;
        let hw = self.home_wins?;
        Some(format!("{} {aw}–{hw} {} · {gm}", self.away_abbrev, self.home_abbrev))
    }
}
