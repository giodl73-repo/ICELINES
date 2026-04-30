use crate::error::FetchError;
use crate::schema::{PagedResponse, PlayerContract, RosterResponse, SkaterBio, SkaterRealtime, SkaterStats};
use crate::teams::ALL_NHL_TEAMS as TEAMS;
use std::time::Duration;

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

    /// Fetch all goalie season stats for a season (paginated). Phase G.2.
    /// Returns the full league's goalies, including backups with low GP.
    /// Callers gate by `qualified()` for leaderboard rendering.
    pub async fn fetch_all_goalies(&self, season: &str) -> Result<Vec<crate::schema::GoalieStats>, FetchError> {
        let endpoint = format!(
            "{}/goalie/summary?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",
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
        self.fetch_schedule_url(&url).await
    }

    /// Fetch the gameWeek starting at a specific date (YYYY-MM-DD).
    /// Returns up to 7 days of games beginning from that date.
    pub async fn fetch_schedule_for_date(&self, date: &str) -> Result<Vec<ScheduledGame>, FetchError> {
        let url = format!("{}/schedule/{}", self.base_web, date);
        self.fetch_schedule_url(&url).await
    }

    /// Fetch the full season schedule for one team via
    /// `/v1/club-schedule-season/{team}/{season}`.
    pub async fn fetch_team_season_schedule(
        &self,
        team: &str,
        season: &str,
    ) -> Result<Vec<ScheduledGame>, FetchError> {
        let url = format!("{}/club-schedule-season/{team}/{season}", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        let mut games = Vec::new();
        if let Some(arr) = raw["games"].as_array() {
            for g in arr {
                if let Some(parsed) = parse_game(g, None) {
                    games.push(parsed);
                }
            }
        }
        Ok(games)
    }

    /// Internal helper: parse the gameWeek-shaped response at `url`.
    async fn fetch_schedule_url(&self, url: &str) -> Result<Vec<ScheduledGame>, FetchError> {
        let raw: serde_json::Value = self.get_json(url).await?;
        let mut games = Vec::new();
        if let Some(week) = raw["gameWeek"].as_array() {
            for day in week {
                let date = day["date"].as_str().map(str::to_owned);
                if let Some(day_games) = day["games"].as_array() {
                    for g in day_games {
                        if let Some(parsed) = parse_game(g, date.as_deref()) {
                            games.push(parsed);
                        }
                    }
                }
            }
        }
        Ok(games)
    }
}

/// Parse one game JSON object into a ScheduledGame.
/// `fallback_date` is used when the date isn't on the game itself (gameWeek nests
/// games under a date; club-schedule-season puts `gameDate` on the game).
fn parse_game(g: &serde_json::Value, fallback_date: Option<&str>) -> Option<ScheduledGame> {
    let game_id = g["id"].as_u64().unwrap_or(0);
    if game_id == 0 {
        return None;
    }
    let date = g["gameDate"].as_str()
        .map(str::to_owned)
        .or_else(|| fallback_date.map(str::to_owned))
        .unwrap_or_default();

    let away      = g["awayTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let away_name = g["awayTeam"]["placeName"]["default"].as_str().unwrap_or(&away).to_owned();
    let home      = g["homeTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let home_name = g["homeTeam"]["placeName"]["default"].as_str().unwrap_or(&home).to_owned();
    let start     = g["startTimeUTC"].as_str().unwrap_or("").to_owned();
    let game_type = g["gameType"].as_u64().unwrap_or(2) as u8;

    let away_score = g["awayTeam"]["score"].as_u64().map(|v| v as u8);
    let home_score = g["homeTeam"]["score"].as_u64().map(|v| v as u8);
    let game_state = g["gameState"].as_str().map(str::to_owned);
    let last_period = g["gameOutcome"]["lastPeriodType"].as_str().map(str::to_owned);

    // Series context for playoff games. The NHL API has used several
    // shapes over time and the schedule-now / club-schedule endpoints
    // serialise it differently:
    //   * seriesSummary.gameLabel / awayWins / homeWins      (historical)
    //   * seriesStatus.gameNumberOfSeven / topSeedWins / etc (current)
    //   * gameLabel / gameNumber may also live at top level
    // We try each path and pick the first hit. The label is stored as a
    // human-readable string ("Game 4"); the wins are taken from whichever
    // sub-object publishes them.
    let ss = &g["seriesSummary"];
    let st = &g["seriesStatus"];

    let series_game = ss["gameLabel"].as_str().map(str::to_owned)
        .or_else(|| st["gameLabel"].as_str().map(str::to_owned))
        .or_else(|| g["gameLabel"].as_str().map(str::to_owned))
        .or_else(|| {
            // Numeric "Game N" — the NHL API has used a few field names
            // over time:
            //   * `gameNumberOfSeries` (current as of 2026-04-29 — verified
            //     against /v1/schedule/now during a live round-1 series)
            //   * `gameNumberOfSeven` (older variant)
            //   * `gameNumber`         (some other endpoints)
            // Convert the first hit to a "Game N" label.
            let n = st["gameNumberOfSeries"].as_u64()
                .or_else(|| st["gameNumberOfSeven"].as_u64())
                .or_else(|| ss["gameNumber"].as_u64())
                .or_else(|| g["gameNumber"].as_u64())?;
            Some(format!("Game {n}"))
        });

    // Wins — top vs bottom in seriesStatus, away vs home in seriesSummary.
    // We map them to (away, home) by matching the team abbrevs since the
    // NHL doesn't always tell us which side is "top seed" in the schedule
    // payload.
    let away_wins = ss["awayWins"].as_u64()
        .or_else(|| {
            // seriesStatus uses topSeedWins/bottomSeedWins — pair by abbrev.
            let top_abbrev    = st["topSeedTeamAbbrev"].as_str().unwrap_or("");
            let bottom_abbrev = st["bottomSeedTeamAbbrev"].as_str().unwrap_or("");
            let top_wins      = st["topSeedWins"].as_u64();
            let bottom_wins   = st["bottomSeedWins"].as_u64();
            if away == top_abbrev { top_wins }
            else if away == bottom_abbrev { bottom_wins }
            else { None }
        })
        .map(|v| v as u8);
    let home_wins = ss["homeWins"].as_u64()
        .or_else(|| {
            let top_abbrev    = st["topSeedTeamAbbrev"].as_str().unwrap_or("");
            let bottom_abbrev = st["bottomSeedTeamAbbrev"].as_str().unwrap_or("");
            let top_wins      = st["topSeedWins"].as_u64();
            let bottom_wins   = st["bottomSeedWins"].as_u64();
            if home == top_abbrev { top_wins }
            else if home == bottom_abbrev { bottom_wins }
            else { None }
        })
        .map(|v| v as u8);

    Some(ScheduledGame {
        game_id,
        date,
        game_type,
        away_abbrev: away,
        away_name,
        home_abbrev: home,
        home_name,
        start_time_utc: start,
        away_score,
        home_score,
        game_state,
        last_period,
        series_game,
        away_wins,
        home_wins,
    })
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
    // Result fields — populated for completed/live games
    pub away_score:      Option<u8>,
    pub home_score:      Option<u8>,
    pub game_state:      Option<String>,  // "FUT","PRE","LIVE","CRIT","FINAL","OFF"
    pub last_period:     Option<String>,  // "REG","OT","SO" (when final)
    // Playoff series context (game_type == 3 only)
    pub series_game:     Option<String>,  // e.g. "Game 4"
    pub away_wins:       Option<u8>,
    pub home_wins:       Option<u8>,
}

impl ScheduledGame {
    pub fn is_playoff(&self) -> bool { self.game_type == 3 }

    /// True once the game has ended (FINAL or OFF).
    pub fn is_final(&self) -> bool {
        matches!(self.game_state.as_deref(), Some("FINAL") | Some("OFF"))
    }

    /// True if the game is in progress (LIVE or CRIT).
    pub fn is_live(&self) -> bool {
        matches!(self.game_state.as_deref(), Some("LIVE") | Some("CRIT"))
    }

    /// True if the game involves the given team (case-sensitive uppercase abbrev).
    pub fn involves(&self, abbrev: &str) -> bool {
        self.away_abbrev == abbrev || self.home_abbrev == abbrev
    }

    /// "EDM 2 – VGK 1 · Game 4" style label for playoffs.
    pub fn series_label(&self) -> Option<String> {
        let gm = self.series_game.as_deref()?;
        let aw = self.away_wins?;
        let hw = self.home_wins?;
        Some(format!("{} {aw}–{hw} {} · {gm}", self.away_abbrev, self.home_abbrev))
    }
}

// ── Boxscore types (Phase 7c gap-fix) ─────────────────────────────────────────

/// One goal scored in a game.
#[derive(Debug, Clone)]
pub struct Goal {
    pub period:        u8,        // 1, 2, 3, OT=4+
    pub period_type:   String,    // "REG" | "OT" | "SO"
    pub time_in_period:String,    // "MM:SS"
    pub scorer_name:   String,
    pub scorer_team:   String,    // home/away abbrev
    pub assist1_name:  Option<String>,
    pub assist2_name:  Option<String>,
    pub away_score:    u8,
    pub home_score:    u8,
}

/// Goalie line for one team's starting goalie in a game.
#[derive(Debug, Clone)]
pub struct GoalieLine {
    pub player_name: String,
    pub team_abbrev: String,
    pub saves:       u32,
    pub shots:       u32,
    pub decision:    Option<String>, // "W" | "L" | "OTL" | None
}

/// One skater's line in a single game's boxscore. Sourced from
/// `playerByGameStats.{home,away}Team.{forwards,defense}` on
/// `/v1/gamecenter/{id}/boxscore`. Used by the game-detail screen to
/// pick out per-team stat leaders (TOI, SOG, Hits, Blocks, Takeaways).
#[derive(Debug, Clone)]
pub struct SkaterLine {
    pub player_id:      u32,
    pub player_name:    String,
    pub team_abbrev:    String,
    pub position:       String,         // "C" | "L" | "R" | "D"
    /// Time on ice in seconds. Parsed from the API's "MM:SS" string.
    pub toi_seconds:    u32,
    pub goals:          u32,
    pub assists:        u32,
    pub plus_minus:     i32,
    pub sog:            u32,
    pub hits:           u32,
    pub blocked_shots:  u32,
    pub takeaways:      u32,
    pub giveaways:      u32,
    pub pim:            u32,
}

/// Detailed boxscore for one game.
#[derive(Debug, Clone)]
pub struct Boxscore {
    pub game_id:        u64,
    pub away_abbrev:    String,
    pub home_abbrev:    String,
    pub away_score:     u8,
    pub home_score:     u8,
    pub game_state:     Option<String>,
    pub last_period:    Option<String>,
    pub goals:          Vec<Goal>,
    pub goalies:        Vec<GoalieLine>,
    /// Per-team skater rows with full stat block. `away_skaters` first,
    /// `home_skaters` second. Empty when the boxscore endpoint
    /// pre-dates the `playerByGameStats` schema.
    pub away_skaters:   Vec<SkaterLine>,
    pub home_skaters:   Vec<SkaterLine>,
}

impl NhlApiClient {
    /// Fetch the boxscore for one game from `/v1/gamecenter/{id}/boxscore`.
    pub async fn fetch_boxscore(&self, game_id: u64) -> Result<Boxscore, FetchError> {
        let url = format!("{}/gamecenter/{game_id}/boxscore", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_boxscore(&raw, game_id))
    }
}

/// Defensive boxscore parser. NHL's boxscore endpoint shape varies — this
/// accepts the common forms and silently drops fields it doesn't recognize.
pub fn parse_boxscore(raw: &serde_json::Value, game_id: u64) -> Boxscore {
    let away_abbrev = raw["awayTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let home_abbrev = raw["homeTeam"]["abbrev"].as_str().unwrap_or("").to_owned();
    let away_score  = raw["awayTeam"]["score"].as_u64().unwrap_or(0) as u8;
    let home_score  = raw["homeTeam"]["score"].as_u64().unwrap_or(0) as u8;
    let game_state  = raw["gameState"].as_str().map(str::to_owned);
    let last_period = raw["gameOutcome"]["lastPeriodType"].as_str().map(str::to_owned);

    // Goals — try a few common nesting paths
    let mut goals = Vec::new();
    let goal_arrays: Vec<&serde_json::Value> = if let Some(arr) = raw["summary"]["scoring"].as_array() {
        // Newer endpoint: summary.scoring is array of period blocks; each has "goals"
        arr.iter().collect()
    } else if let Some(arr) = raw["scoring"].as_array() {
        arr.iter().collect()
    } else {
        Vec::new()
    };

    for period_block in goal_arrays {
        let period_num = period_block["periodDescriptor"]["number"].as_u64()
            .or_else(|| period_block["period"].as_u64())
            .unwrap_or(0) as u8;
        let period_type = period_block["periodDescriptor"]["periodType"].as_str()
            .or_else(|| period_block["periodType"].as_str())
            .unwrap_or("REG")
            .to_owned();

        if let Some(g_arr) = period_block["goals"].as_array() {
            for g in g_arr {
                let scorer_name = g["firstName"]["default"].as_str()
                    .map(|fn_| {
                        let ln = g["lastName"]["default"].as_str().unwrap_or("");
                        format!("{fn_} {ln}").trim().to_owned()
                    })
                    .or_else(|| g["name"]["default"].as_str().map(str::to_owned))
                    .or_else(|| g["scorer"].as_str().map(str::to_owned))
                    .unwrap_or_default();
                let scorer_team = g["teamAbbrev"]["default"].as_str()
                    .or_else(|| g["teamAbbrev"].as_str())
                    .unwrap_or("")
                    .to_owned();
                let time_in_period = g["timeInPeriod"].as_str()
                    .or_else(|| g["time"].as_str())
                    .unwrap_or("")
                    .to_owned();

                // Assists: prefer structured array
                let mut assists: Vec<String> = Vec::new();
                if let Some(arr) = g["assists"].as_array() {
                    for a in arr {
                        if let Some(name) = a["name"]["default"].as_str() {
                            assists.push(name.to_owned());
                        } else if let (Some(fnm), Some(lnm)) = (
                            a["firstName"]["default"].as_str(),
                            a["lastName"]["default"].as_str(),
                        ) {
                            assists.push(format!("{fnm} {lnm}"));
                        }
                    }
                }
                let assist1_name = assists.first().cloned();
                let assist2_name = assists.get(1).cloned();

                let aw_score = g["awayScore"].as_u64().unwrap_or(0) as u8;
                let hm_score = g["homeScore"].as_u64().unwrap_or(0) as u8;

                goals.push(Goal {
                    period:         period_num,
                    period_type:    period_type.clone(),
                    time_in_period,
                    scorer_name,
                    scorer_team,
                    assist1_name,
                    assist2_name,
                    away_score:     aw_score,
                    home_score:     hm_score,
                });
            }
        }
    }

    // Goalies — try common shapes: playerByGameStats.{home,away}Team.goalies / boxscore.goalies
    let mut goalies = Vec::new();
    let goalie_paths = [
        (&raw["playerByGameStats"]["awayTeam"]["goalies"], away_abbrev.as_str()),
        (&raw["playerByGameStats"]["homeTeam"]["goalies"], home_abbrev.as_str()),
        (&raw["boxscore"]["awayTeam"]["goalies"],          away_abbrev.as_str()),
        (&raw["boxscore"]["homeTeam"]["goalies"],          home_abbrev.as_str()),
    ];
    for (val, team) in goalie_paths {
        if let Some(arr) = val.as_array() {
            for g in arr {
                let player_name = g["name"]["default"].as_str()
                    .map(str::to_owned)
                    .or_else(|| {
                        let fnm = g["firstName"]["default"].as_str()?;
                        let lnm = g["lastName"]["default"].as_str().unwrap_or("");
                        Some(format!("{fnm} {lnm}").trim().to_owned())
                    })
                    .unwrap_or_default();
                let saves = g["saves"].as_u64().unwrap_or(0) as u32;
                let shots = g["shotsAgainst"].as_u64()
                    .or_else(|| g["shots"].as_u64())
                    .unwrap_or(0) as u32;
                let decision = g["decision"].as_str().map(str::to_owned);
                if !player_name.is_empty() {
                    goalies.push(GoalieLine {
                        player_name,
                        team_abbrev: team.to_owned(),
                        saves, shots, decision,
                    });
                }
            }
        }
    }

    // Per-team skater stats from `playerByGameStats.{home,away}Team.
    // {forwards,defense}`. Goalies live alongside but are already
    // pulled into the dedicated `goalies` array above.
    let pgs = &raw["playerByGameStats"];
    let away_skaters = parse_skater_lines(&pgs["awayTeam"], &away_abbrev);
    let home_skaters = parse_skater_lines(&pgs["homeTeam"], &home_abbrev);

    Boxscore {
        game_id, away_abbrev, home_abbrev,
        away_score, home_score, game_state, last_period,
        goals, goalies,
        away_skaters, home_skaters,
    }
}

/// Pull all forwards + defense out of one team's `playerByGameStats`
/// block. Goalies are intentionally excluded — they're handled by the
/// dedicated `goalies` parsing path above. Returns an empty Vec when
/// the `playerByGameStats` shape isn't present (older boxscore
/// endpoints; partial responses while a game is loading).
fn parse_skater_lines(team: &serde_json::Value, abbrev: &str) -> Vec<SkaterLine> {
    let mut out = Vec::new();
    for group in &["forwards", "defense"] {
        let Some(arr) = team[group].as_array() else { continue };
        for p in arr {
            let player_id   = p["playerId"].as_u64().unwrap_or(0) as u32;
            let player_name = p["name"]["default"].as_str()
                .or_else(|| p["name"].as_str())
                .unwrap_or("")
                .to_owned();
            let position = p["position"].as_str().unwrap_or("").to_owned();
            let toi_seconds = parse_mmss(p["toi"].as_str().unwrap_or("0:00"));
            out.push(SkaterLine {
                player_id,
                player_name,
                team_abbrev: abbrev.to_owned(),
                position,
                toi_seconds,
                goals:         p["goals"]         .as_u64().unwrap_or(0) as u32,
                assists:       p["assists"]       .as_u64().unwrap_or(0) as u32,
                plus_minus:    p["plusMinus"]     .as_i64().unwrap_or(0) as i32,
                sog:           p["sog"]           .as_u64().unwrap_or(0) as u32,
                hits:          p["hits"]          .as_u64().unwrap_or(0) as u32,
                blocked_shots: p["blockedShots"]  .as_u64().unwrap_or(0) as u32,
                takeaways:     p["takeaways"]     .as_u64().unwrap_or(0) as u32,
                giveaways:     p["giveaways"]     .as_u64().unwrap_or(0) as u32,
                pim:           p["pim"]           .as_u64().unwrap_or(0) as u32,
            });
        }
    }
    out
}

/// Parse "MM:SS" → seconds. Returns 0 on malformed input. Handles the
/// boxscore convention where TOI is published as a colon-separated
/// minutes-seconds string ("18:45") rather than a number.
fn parse_mmss(s: &str) -> u32 {
    let mut parts = s.splitn(2, ':');
    let m = parts.next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
    let s = parts.next().and_then(|p| p.parse::<u32>().ok()).unwrap_or(0);
    m * 60 + s
}

// ── Playoff bracket types (Phase 7e) ──────────────────────────────────────────

/// One series in a playoff round.
#[derive(Debug, Clone)]
pub struct PlayoffSeries {
    pub letter:             Option<String>, // e.g. "A" — used as a stable key
    pub top_seed_abbrev:    String,
    pub top_seed_name:      String,
    pub top_seed_wins:      u8,
    pub top_seed_rank:      Option<String>, // e.g. "A1", "WC1"
    pub bottom_seed_abbrev: String,
    pub bottom_seed_name:   String,
    pub bottom_seed_wins:   u8,
    pub bottom_seed_rank:   Option<String>,
    pub winner_abbrev:      Option<String>, // None until series concludes
    pub conference:         Option<String>, // "Eastern" | "Western" | None
    /// Per-game results for this series. Empty when the live API source
    /// does not include game logs; populated for historical bundles.
    /// Phase 8c.
    pub games:              Vec<PlayoffGameResult>,
}

/// One game inside a playoff series. Sourced from bundled `playoffs.json`
/// for historical seasons (Phase 8c). The live `/v1/playoff-bracket/{year}`
/// endpoint does not include per-game logs, so for current-season series
/// this vector is empty.
#[derive(Debug, Clone)]
pub struct PlayoffGameResult {
    pub date:         String,        // ISO 8601 (YYYY-MM-DD)
    pub home_abbrev:  String,
    pub away_abbrev:  String,
    pub home_score:   u8,
    pub away_score:   u8,
    pub series_after: String,        // e.g. "TBL 1-0", "tied 2-2"
    pub goals:        Vec<PlayoffGoal>,
}

/// One goal scored in a playoff game. v1 of the bundle records scorer name
/// and team abbrev only; assists and timestamps may be added in v2.
#[derive(Debug, Clone)]
pub struct PlayoffGoal {
    pub scorer: String,
    pub team:   String,
}

impl PlayoffSeries {
    /// True when one side has 4 wins and the other has fewer.
    pub fn is_complete(&self) -> bool {
        self.top_seed_wins == 4 || self.bottom_seed_wins == 4
    }

    /// Total number of games played so far in the series.
    pub fn games_played(&self) -> u8 {
        self.top_seed_wins + self.bottom_seed_wins
    }

    /// One-line summary like "FLA 4-2 TBL · FLA wins" (or "tied 2-2", "FLA leads 3-1").
    pub fn summary(&self) -> String {
        let (t, b) = (self.top_seed_wins, self.bottom_seed_wins);
        if let Some(w) = &self.winner_abbrev {
            format!("{} {t}-{b} {} · {w} wins", self.top_seed_abbrev, self.bottom_seed_abbrev)
        } else if t > b {
            format!("{} leads {t}-{b}", self.top_seed_abbrev)
        } else if b > t {
            format!("{} leads {b}-{t}", self.bottom_seed_abbrev)
        } else if t == 0 {
            format!("{} vs {} · series begins", self.top_seed_abbrev, self.bottom_seed_abbrev)
        } else {
            format!("Tied {t}-{b}")
        }
    }
}

/// One round of a playoff bracket.
#[derive(Debug, Clone)]
pub struct PlayoffRound {
    pub round_number: u8,    // 1..=4
    pub label:        String, // "First Round", "Second Round", "Conf Final", "Stanley Cup Final"
    pub series:       Vec<PlayoffSeries>,
}

/// Full playoff bracket for one season.
#[derive(Debug, Clone)]
pub struct PlayoffBracket {
    pub season:        String,
    pub current_round: Option<u8>,
    pub rounds:        Vec<PlayoffRound>,
}

impl PlayoffBracket {
    /// Find a series by its letter (e.g. "A").
    pub fn find_series(&self, letter: &str) -> Option<&PlayoffSeries> {
        for r in &self.rounds {
            for s in &r.series {
                if s.letter.as_deref() == Some(letter) {
                    return Some(s);
                }
            }
        }
        None
    }

    /// True if every round is empty (no series yet — pre-playoffs / off-season).
    pub fn is_empty(&self) -> bool {
        self.rounds.iter().all(|r| r.series.is_empty())
    }
}

impl NhlApiClient {
    /// Fetch the playoff bracket for a given playoff year (the second year of the
    /// season; e.g. for season 2025-26 the year is 2026).
    pub async fn fetch_playoff_bracket(&self, year: u16) -> Result<PlayoffBracket, FetchError> {
        let url = format!("{}/playoff-bracket/{year}", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_playoff_bracket(&raw))
    }
}

/// Parse a playoff-bracket JSON payload. Defensively accepts the shape NHL's
/// API has historically used (`series` list grouped by `playoffRounds`) and
/// extracts the fields we render. Unknown fields are silently dropped.
pub fn parse_playoff_bracket(raw: &serde_json::Value) -> PlayoffBracket {
    let season = raw["season"].as_str()
        .or_else(|| raw["seasonId"].as_str())
        .map(str::to_owned)
        .unwrap_or_default();
    let current_round = raw["currentRound"].as_u64()
        .or_else(|| raw["roundNumber"].as_u64())
        .map(|v| v as u8);

    let mut rounds: Vec<PlayoffRound> = Vec::new();

    // Shape A: legacy nested form — `playoffRounds: [{ roundNumber, series: [..] }]`.
    let round_arrays = raw["playoffRounds"].as_array()
        .or_else(|| raw["rounds"].as_array());
    if let Some(arr) = round_arrays {
        for r in arr {
            let round_number = r["roundNumber"].as_u64().unwrap_or(0) as u8;
            let label = r["roundLabel"].as_str()
                .or_else(|| r["roundName"].as_str())
                .map(str::to_owned)
                .unwrap_or_else(|| default_round_label(round_number));
            let mut series = Vec::new();
            if let Some(s_arr) = r["series"].as_array() {
                for s in s_arr {
                    series.push(parse_series(s));
                }
            }
            rounds.push(PlayoffRound { round_number, label, series });
        }
    }

    // Shape B: current API (verified 2026-04-29) — flat `series: [..]`
    // where each series carries its own `playoffRound`. Bucket by round.
    if rounds.is_empty() {
        if let Some(arr) = raw["series"].as_array() {
            use std::collections::BTreeMap;
            let mut by_round: BTreeMap<u8, Vec<PlayoffSeries>> = BTreeMap::new();
            let mut labels:   BTreeMap<u8, String>             = BTreeMap::new();
            for s in arr {
                let rn = s["playoffRound"].as_u64()
                    .or_else(|| s["roundNumber"].as_u64())
                    .unwrap_or(0) as u8;
                if let Some(t) = s["seriesTitle"].as_str() {
                    labels.entry(rn).or_insert_with(|| t.to_owned());
                }
                by_round.entry(rn).or_default().push(parse_series(s));
            }
            for (rn, ser) in by_round {
                let label = labels.get(&rn).cloned()
                    .unwrap_or_else(|| default_round_label(rn));
                rounds.push(PlayoffRound { round_number: rn, label, series: ser });
            }
        }
    }

    rounds.sort_by_key(|r| r.round_number);
    PlayoffBracket { season, current_round, rounds }
}

fn default_round_label(round_number: u8) -> String {
    match round_number {
        1 => "First Round".to_owned(),
        2 => "Second Round".to_owned(),
        3 => "Conference Final".to_owned(),
        4 => "Stanley Cup Final".to_owned(),
        _ => format!("Round {round_number}"),
    }
}

fn parse_series(s: &serde_json::Value) -> PlayoffSeries {
    let letter = s["seriesLetter"].as_str()
        .or_else(|| s["seriesAbbrev"].as_str())
        .map(str::to_owned);

    let top    = &s["topSeedTeam"];
    let bottom = &s["bottomSeedTeam"];

    let top_abbrev = top["abbrev"].as_str().unwrap_or("").to_owned();
    let top_name   = top["name"]["default"].as_str()
        .or_else(|| top["placeName"]["default"].as_str())
        .unwrap_or(&top_abbrev).to_owned();
    // Wins: legacy nested API put it on the team object; current API
    // (verified 2026-04-29 against /v1/playoff-bracket/2026) puts it at
    // the series level as `topSeedWins`/`bottomSeedWins`.
    let top_wins = top["wins"].as_u64()
        .or_else(|| s["topSeedWins"].as_u64())
        .unwrap_or(0) as u8;
    // Rank: prefer the abbreviated form ("D1", "WC1") when present —
    // matches what users see in the playoff bracket header.
    let top_rank = s["topSeedRankAbbrev"].as_str()
        .or_else(|| top["seed"].as_str())
        .or_else(|| s["topSeedRank"].as_str())
        .map(str::to_owned)
        // Numeric `topSeedRank` (1..=8) shows up as a u64 — fall through
        // to that and stringify so something usable lands in the UI.
        .or_else(|| s["topSeedRank"].as_u64().map(|n| n.to_string()));

    let bot_abbrev = bottom["abbrev"].as_str().unwrap_or("").to_owned();
    let bot_name   = bottom["name"]["default"].as_str()
        .or_else(|| bottom["placeName"]["default"].as_str())
        .unwrap_or(&bot_abbrev).to_owned();
    let bot_wins = bottom["wins"].as_u64()
        .or_else(|| s["bottomSeedWins"].as_u64())
        .unwrap_or(0) as u8;
    let bot_rank = s["bottomSeedRankAbbrev"].as_str()
        .or_else(|| bottom["seed"].as_str())
        .or_else(|| s["bottomSeedRank"].as_str())
        .map(str::to_owned)
        .or_else(|| s["bottomSeedRank"].as_u64().map(|n| n.to_string()));

    // Winner: explicit field or inferred from 4-win threshold
    let winner_abbrev = s["winningTeam"]["abbrev"].as_str()
        .map(str::to_owned)
        .or_else(|| {
            if top_wins == 4 { Some(top_abbrev.clone()) }
            else if bot_wins == 4 { Some(bot_abbrev.clone()) }
            else { None }
        });

    let conference = s["conferenceAbbrev"].as_str()
        .or_else(|| s["conference"].as_str())
        .map(|c| match c {
            "E" | "EAST" | "Eastern" => "Eastern".to_owned(),
            "W" | "WEST" | "Western" => "Western".to_owned(),
            other => other.to_owned(),
        });

    PlayoffSeries {
        letter,
        top_seed_abbrev:    top_abbrev,
        top_seed_name:      top_name,
        top_seed_wins:      top_wins,
        top_seed_rank:      top_rank,
        bottom_seed_abbrev: bot_abbrev,
        bottom_seed_name:   bot_name,
        bottom_seed_wins:   bot_wins,
        bottom_seed_rank:   bot_rank,
        winner_abbrev,
        conference,
        games: Vec::new(),
    }
}

#[cfg(test)]
mod parse_game_tests {
    //! Unit tests for `parse_game` — the field-name fallbacks for series
    //! context have been a source of empty `Game ?` placeholders in the
    //! TUI when the NHL API shape changed underneath us. Cover the three
    //! shapes we currently know about explicitly.

    use super::parse_game;
    use serde_json::json;

    fn base_playoff(extra: serde_json::Value) -> serde_json::Value {
        // The minimum game payload our parser requires, plus whatever the
        // caller layers on for series context.
        let mut v = json!({
            "id": 2025030101,
            "gameType": 3,
            "awayTeam": {"abbrev":"NYR","placeName":{"default":"New York"}},
            "homeTeam": {"abbrev":"WSH","placeName":{"default":"Washington"}},
            "startTimeUTC": "2026-04-28T23:05:00Z",
            "gameState": "FUT"
        });
        if let serde_json::Value::Object(extra_map) = extra {
            if let serde_json::Value::Object(base) = &mut v {
                for (k, val) in extra_map { base.insert(k, val); }
            }
        }
        v
    }

    #[test]
    fn l0_parse_game_reads_legacy_seriesSummary_gameLabel() {
        // Original API shape — seriesSummary.gameLabel + away/homeWins.
        let raw = base_playoff(json!({
            "seriesSummary": {"gameLabel": "Game 4", "awayWins": 2, "homeWins": 1}
        }));
        let g = parse_game(&raw, Some("2026-04-28")).expect("parses");
        assert_eq!(g.series_game.as_deref(), Some("Game 4"));
        assert_eq!(g.away_wins, Some(2));
        assert_eq!(g.home_wins, Some(1));
    }

    #[test]
    fn l0_parse_game_reads_seriesStatus_gameLabel() {
        // Newer endpoints publish series context under `seriesStatus`
        // with a `gameLabel` field. Our parser falls through to it when
        // `seriesSummary` is absent.
        let raw = base_playoff(json!({
            "seriesStatus": {
                "gameLabel": "Game 1",
                "topSeedTeamAbbrev": "WSH", "topSeedWins": 0,
                "bottomSeedTeamAbbrev": "NYR", "bottomSeedWins": 0
            }
        }));
        let g = parse_game(&raw, Some("2026-04-28")).expect("parses");
        assert_eq!(g.series_game.as_deref(), Some("Game 1"));
        // Wins map by abbrev: NYR is bottom seed, WSH is top seed.
        assert_eq!(g.away_wins, Some(0)); // NYR → bottom_wins
        assert_eq!(g.home_wins, Some(0)); // WSH → top_wins
    }

    #[test]
    fn l0_parse_game_reads_seriesStatus_gameNumberOfSeven() {
        // When only the numeric `gameNumberOfSeven` is present, the
        // parser synthesises a "Game N" label so the TUI never displays
        // "Game ?" for an otherwise valid playoff fixture. This is the
        // bug captured in 2026-04 — round 1 games returned only the
        // numeric form and the label fell back to a question mark.
        let raw = base_playoff(json!({
            "seriesStatus": {
                "gameNumberOfSeven": 3,
                "topSeedTeamAbbrev": "WSH", "topSeedWins": 1,
                "bottomSeedTeamAbbrev": "NYR", "bottomSeedWins": 1
            }
        }));
        let g = parse_game(&raw, Some("2026-04-28")).expect("parses");
        assert_eq!(g.series_game.as_deref(), Some("Game 3"),
            "numeric gameNumberOfSeven should synthesise a 'Game N' label");
        assert_eq!(g.away_wins, Some(1));
        assert_eq!(g.home_wins, Some(1));
    }

    #[test]
    fn l0_parse_game_reads_seriesStatus_gameNumberOfSeries_current() {
        // Current API (verified 2026-04-29 against /v1/schedule/now during
        // a live round-1 series): `gameNumberOfSeries` is the active
        // field name. Without this fallback we'd display "Game ?" — the
        // exact regression the user reported.
        let raw = base_playoff(json!({
            "seriesStatus": {
                "gameNumberOfSeries": 5,
                "round": 1,
                "seriesAbbrev": "R1",
                "seriesLetter": "B",
                "topSeedTeamAbbrev": "TBL", "topSeedWins": 2,
                "bottomSeedTeamAbbrev": "MTL", "bottomSeedWins": 3
            }
        }));
        let g = parse_game(&raw, Some("2026-04-29")).expect("parses");
        assert_eq!(g.series_game.as_deref(), Some("Game 5"),
            "current API uses gameNumberOfSeries");
        // away=NYR is bottom_seed; home=WSH is top_seed (from base_playoff).
        // Wait — base_playoff has NYR away, WSH home. seriesStatus has
        // TBL top, MTL bottom. The win-mapping should match by abbrev.
        // NYR doesn't match TBL or MTL → wins remain None.
        // For a meaningful assertion, build a fixture where abbrevs match.
        let raw2 = json!({
            "id": 2025030126,
            "gameType": 3,
            "awayTeam": {"abbrev":"MTL","placeName":{"default":"Montreal"}},
            "homeTeam": {"abbrev":"TBL","placeName":{"default":"Tampa Bay"}},
            "startTimeUTC": "2026-04-29T23:00:00Z",
            "gameState": "FUT",
            "seriesStatus": {
                "gameNumberOfSeries": 5,
                "topSeedTeamAbbrev": "TBL", "topSeedWins": 2,
                "bottomSeedTeamAbbrev": "MTL", "bottomSeedWins": 3
            }
        });
        let g2 = parse_game(&raw2, Some("2026-04-29")).expect("parses");
        assert_eq!(g2.series_game.as_deref(), Some("Game 5"));
        assert_eq!(g2.away_wins, Some(3), "MTL is bottom seed → 3 wins");
        assert_eq!(g2.home_wins, Some(2), "TBL is top seed → 2 wins");
    }

    #[test]
    fn l0_parse_game_reads_top_level_gameLabel_fallback() {
        // Some payloads put `gameLabel` at the top of the game object.
        let raw = base_playoff(json!({"gameLabel": "Game 7"}));
        let g = parse_game(&raw, Some("2026-04-28")).expect("parses");
        assert_eq!(g.series_game.as_deref(), Some("Game 7"));
    }

    #[test]
    fn l0_parse_game_no_series_context_leaves_label_none() {
        // Regular-season game with no series fields → series_game stays
        // None so the TUI renders without a "Game ?" suffix.
        let raw = json!({
            "id": 2025020100,
            "gameType": 2,
            "awayTeam": {"abbrev":"SEA","placeName":{"default":"Seattle"}},
            "homeTeam": {"abbrev":"VGK","placeName":{"default":"Vegas"}},
            "startTimeUTC": "2026-01-15T03:00:00Z",
            "gameState": "FUT"
        });
        let g = parse_game(&raw, Some("2026-01-14")).expect("parses");
        assert_eq!(g.series_game, None);
        assert_eq!(g.away_wins, None);
        assert_eq!(g.home_wins, None);
    }
}
