use crate::error::FetchError;
use crate::schema::{
    GoalieBio, PagedResponse, PlayerContract, RosterResponse, SkaterBio, SkaterRealtime,
    SkaterStats,
};
use crate::shift_chart::{OfficialShiftChartResponse, OfficialShiftChartRow};
use crate::teams::nhl_teams_for_season;
use icelines_core::season_stats::SeasonType;
pub use icelines_sources::nhl::player_landing::parse_player_landing_contract;
pub use icelines_sources::nhl::schedule::ScheduledGame;
pub use icelines_sources::nhl::standings::{parse_standings, NhlStandingsRow};
use std::time::Duration;

/// Map `SeasonType` to the NHL API's `gameTypeId` query parameter.
/// 2 = regular season, 3 = playoffs (cayenneExp filter values).
fn game_type_id(season_type: SeasonType) -> u8 {
    match season_type {
        SeasonType::Regular => 2,
        SeasonType::Playoff => 3,
    }
}

/// Async NHL API client.
/// `base_url_stats` and `base_url_web` are configurable to allow mocking in tests.
#[derive(Clone)]
pub struct NhlApiClient {
    client: reqwest::Client,
    base_stats: String, // https://api.nhle.com/stats/rest/en
    base_web: String,   // https://api-web.nhle.com/v1
    /// Retry policy — Phase Lindsay L.1.5 (TAPE-R3 rate-limit policy):
    /// exponential backoff, base 500ms, capped at 30s, max 5 retries.
    /// Retries fire on 429 (rate-limited) AND 5xx (server-side transient).
    /// Pre-Lindsay: 3 retries × 1000ms base × 429-only.
    max_retries: u32,
    retry_base_ms: u64,
    /// Backoff cap (Lindsay L.1.5). Without this, attempt 5 at base
    /// 500ms would wait 16 seconds — fine. With base 1000ms, attempt 5
    /// would be 32s — over the spec's 30s cap. We cap at 30s
    /// regardless of base*2^attempt.
    retry_cap_ms: u64,
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
            // Lindsay L.1.5: bumped from 3 → 5 retries, base 1000 → 500
            // ms, retry surface widened from {429} → {429, 5xx}, with a
            // 30s ceiling on the per-attempt sleep.
            max_retries: 5,
            retry_base_ms: 500,
            retry_cap_ms: 30_000,
        }
    }

    /// Production constructor using the real NHL API endpoints.
    pub fn production() -> Self {
        Self::new(
            "https://api.nhle.com/stats/rest/en",
            "https://api-web.nhle.com/v1",
        )
    }

    /// Override the retry policy. Used by L1 tests to keep retry waits
    /// at millisecond scale (production base is 500ms × 2^attempt — too
    /// slow for tests). Production callers should NOT use this — the
    /// defaults from `Self::new` reflect the L.1.5 spec contract.
    pub fn with_retry_params(
        mut self,
        max_retries: u32,
        retry_base_ms: u64,
        retry_cap_ms: u64,
    ) -> Self {
        self.max_retries = max_retries;
        self.retry_base_ms = retry_base_ms;
        self.retry_cap_ms = retry_cap_ms;
        self
    }

    // ── Internal HTTP helper ─────────────────────────────────────────────────

    async fn get_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        // Phase Lindsay L.1.5 (TAPE-R3 rate-limit policy):
        //   - Retry surface: 429 (rate-limited) + 5xx (server-side transient).
        //   - Backoff: exponential, base `retry_base_ms`, capped at
        //     `retry_cap_ms`. Pre-Lindsay only retried 429 with no cap.
        //   - Max retries: `max_retries` (5 in production).
        // Other 4xx (auth, bad request, etc.) fail-fast — they won't
        // succeed on retry. The `documented-broken` 500 endpoints
        // (skater/advanced, several goalie/* per probe artifact) ARE
        // 5xx — they'll retry uselessly. Mitigated by `ReportKind::tier`
        // dispatch in L.1.6 (we never call broken endpoints) so the
        // wasted retries should never fire in practice.
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
                    return resp.bytes().await.map(|bytes| bytes.to_vec()).map_err(|e| {
                        FetchError::SchemaChanged {
                            detail: format!("{url}: {e}"),
                        }
                    });
                }
                // Retryable: 429 (rate-limit) + 5xx (server-side transient).
                429 | 500..=599 => {
                    if attempt >= self.max_retries {
                        // Choice of error: 429 → RateLimited; 5xx → Http
                        // (carrying the actual status). Preserves the
                        // pre-Lindsay error surface for 429-exhaust;
                        // adds a clear "exhausted retries on 5xx" path.
                        return Err(if status == 429 {
                            FetchError::RateLimited {
                                url: url.to_owned(),
                            }
                        } else if status == 503 {
                            FetchError::ServiceUnavailable {
                                url: url.to_owned(),
                            }
                        } else {
                            FetchError::Http {
                                status,
                                url: url.to_owned(),
                            }
                        });
                    }
                    // Exponential backoff with cap.
                    let raw_delay = self.retry_base_ms.saturating_mul(1 << attempt);
                    let delay = raw_delay.min(self.retry_cap_ms);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                    attempt += 1;
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

    async fn get_json<T: serde::de::DeserializeOwned>(&self, url: &str) -> Result<T, FetchError> {
        let bytes = self.get_bytes(url).await?;
        serde_json::from_slice(&bytes).map_err(|error| FetchError::SchemaChanged {
            detail: format!("{url}: {error}"),
        })
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

    /// Fetch all skater bios for a season + season-type (paginated).
    pub async fn fetch_all_bios(
        &self,
        season: &str,
        season_type: SeasonType,
    ) -> Result<Vec<SkaterBio>, FetchError> {
        let gt = game_type_id(season_type);
        let endpoint = format!(
            "{}/skater/bios?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D{gt}",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch all skater season stats for a season + season-type (paginated).
    pub async fn fetch_all_stats(
        &self,
        season: &str,
        season_type: SeasonType,
    ) -> Result<Vec<SkaterStats>, FetchError> {
        let gt = game_type_id(season_type);
        let endpoint = format!(
            "{}/skater/summary?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D{gt}",
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
        let window = if season == icelines_core::CURRENT_SEASON_STR {
            "current"
        } else {
            season
        };
        let url = format!("{}/roster/{team}/{window}", self.base_web);
        self.get_json(&url).await
    }

    /// Fetch rosters for every franchise participating in the season.
    pub async fn fetch_all_rosters(
        &self,
        season: &str,
    ) -> Result<Vec<(String, RosterResponse)>, FetchError> {
        let mut results = Vec::new();
        for team in nhl_teams_for_season(season) {
            let roster = self.fetch_team_roster(team, season).await?;
            results.push((team.to_string(), roster));
        }
        Ok(results)
    }

    /// Phase Lindsay L.1.6 — generic Tier-1/Tier-2 fetcher.
    ///
    /// Builds the URL from `kind.url_path()` + season-aware cayenneExp,
    /// paginates via `fetch_all_paged`, and returns the rows as raw
    /// `serde_json::Value`s. Callers that want typed deserialization
    /// (Tier-1) can `serde_json::from_value::<R>` over the slice.
    ///
    /// Refuses to dispatch known-broken endpoints: per the probe artifact
    /// 8 documented-broken URLs return 500 — we never call them. The
    /// CLI gates on `kind.is_known_working()` BEFORE invoking this
    /// helper so the L.1.5 5-retry-with-backoff policy doesn't burn
    /// 2.5 minutes on a known-dead URL.
    pub async fn fetch_report_paged(
        &self,
        kind: icelines_core::stats_catalog::ReportKind,
        season: &str,
        season_type: SeasonType,
    ) -> Result<Vec<serde_json::Value>, FetchError> {
        let gt = game_type_id(season_type);
        let endpoint = format!(
            "{}/{}?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D{gt}",
            self.base_stats,
            kind.url_path(),
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch all skater realtime stats for a season (paginated).
    /// Realtime is regular-season only; the live game feed updates
    /// realtime entries during playoffs through the same endpoint, so
    /// no `season_type` parameter is exposed (per Hart.6 D6 / Risk #5).
    pub async fn fetch_all_realtime(
        &self,
        season: &str,
    ) -> Result<Vec<SkaterRealtime>, FetchError> {
        let endpoint = format!(
            "{}/skater/realtime?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D2",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch all goalie season stats for a season + season-type (paginated).
    /// Returns the full league's goalies, including backups with low GP.
    /// Callers gate by `qualified()` for leaderboard rendering.
    pub async fn fetch_all_goalies(
        &self,
        season: &str,
        season_type: SeasonType,
    ) -> Result<Vec<crate::schema::GoalieStats>, FetchError> {
        let gt = game_type_id(season_type);
        let endpoint = format!(
            "{}/goalie/summary?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D{gt}",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch all goalie bios for a season. Unlike `/goalie/summary`, this
    /// endpoint carries draft year/round/overall and can therefore join
    /// goalie appearances to a complete draft population without names.
    pub async fn fetch_all_goalie_bios(
        &self,
        season: &str,
        season_type: SeasonType,
    ) -> Result<Vec<GoalieBio>, FetchError> {
        let gt = game_type_id(season_type);
        let endpoint = format!(
            "{}/goalie/bios?cayenneExp=seasonId%3D{season}%20and%20gameTypeId%3D{gt}",
            self.base_stats
        );
        self.fetch_all_paged(&endpoint).await
    }

    /// Fetch a URL as raw text (used for CSV downloads such as MoneyPuck).
    pub async fn fetch_text(&self, url: &str) -> Result<String, FetchError> {
        String::from_utf8(self.get_bytes(url).await?).map_err(|error| FetchError::SchemaChanged {
            detail: format!("{url}: {error}"),
        })
    }

    /// Fetch provider bytes without decoding them. Source adapters receive
    /// these exact bytes after the fetch layer stores them by content hash.
    pub async fn fetch_source_bytes(&self, url: &str) -> Result<Vec<u8>, FetchError> {
        self.get_bytes(url).await
    }

    /// Fetch contract/landing data for a single player.
    ///
    /// NOTE: As of 2026-04-26, the NHL landing API does not return contract fields
    /// (expiry_year, expiry_type, salary). The returned `PlayerContract` will have
    /// `player_id` populated and all other fields as `None`.
    /// Phase Calder.1 — fetch a player's full career history from
    /// `/v1/player/{id}/landing.seasonTotals`. Returns every league
    /// stint (NHL, AHL, OHL, NCAA, KHL, junior, international, …)
    /// the API knows about.
    pub async fn fetch_player_career_history(
        &self,
        player_id: u32,
    ) -> Result<icelines_core::career_history::CareerHistory, FetchError> {
        let url = format!("{}/player/{player_id}/landing", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        crate::career_landing::parse_career_history(player_id, &raw).map_err(|e| {
            FetchError::SchemaChanged {
                detail: format!("career history (pid {player_id}): {e}"),
            }
        })
    }

    pub async fn fetch_player_awards(
        &self,
        player_id: u32,
        player_name: &str,
        context: icelines_core::ViewContext,
    ) -> Result<icelines_core::PlayerAwardsView, FetchError> {
        let url = format!("{}/player/{player_id}/landing", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(crate::career_landing::parse_player_awards(
            player_id,
            player_name,
            context,
            &raw,
        ))
    }

    /// Phase Calder.2 — batch career-history fetch.
    ///
    /// Sequential with a small per-request delay (50ms — matches
    /// `fetch_all_contracts`). Skip-and-log on individual failures
    /// rather than fail the batch: 700 players × 99% success is much
    /// more useful than 0 players because one returned 404.
    ///
    /// Returns (histories, skipped_pids). Caller decides whether
    /// `skipped > 0` is acceptable (typically yes for a one-shot
    /// refresh; surface-level callers can re-try the skipped pids
    /// later).
    pub async fn fetch_all_career_histories(
        &self,
        player_ids: &[u32],
    ) -> (
        Vec<icelines_core::career_history::CareerHistory>,
        Vec<(u32, String)>,
    ) {
        let mut histories = Vec::with_capacity(player_ids.len());
        let mut skipped: Vec<(u32, String)> = Vec::new();
        for &pid in player_ids {
            match self.fetch_player_career_history(pid).await {
                Ok(h) => histories.push(h),
                Err(e) => skipped.push((pid, e.to_string())),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        (histories, skipped)
    }

    pub async fn fetch_player_landing_contract(
        &self,
        player_id: u32,
    ) -> Result<PlayerContract, FetchError> {
        let url = format!("{}/player/{player_id}/landing", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_player_landing_contract(player_id, &raw))
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
    pub async fn fetch_schedule_for_date(
        &self,
        date: &str,
    ) -> Result<Vec<ScheduledGame>, FetchError> {
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

    /// Fetch official per-player shift intervals for one NHL game.
    pub async fn fetch_shift_chart(
        &self,
        game_id: u64,
    ) -> Result<Vec<OfficialShiftChartRow>, FetchError> {
        let url = format!(
            "{}/shiftcharts?cayenneExp=gameId={game_id}",
            self.base_stats
        );
        let response: OfficialShiftChartResponse = self.get_json(&url).await?;
        if response.total != response.data.len() {
            return Err(FetchError::SchemaChanged {
                detail: format!(
                    "{url}: total {} does not match {} interval rows",
                    response.total,
                    response.data.len()
                ),
            });
        }
        Ok(response.data)
    }

    /// Fetch current NHL standings from `/v1/standings/now`.
    pub async fn fetch_standings_now(&self) -> Result<Vec<NhlStandingsRow>, FetchError> {
        let url = format!("{}/standings/now", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_standings(&raw))
    }

    /// Fetch standings snapshot for a date (`YYYY-MM-DD`) from `/v1/standings/{date}`.
    pub async fn fetch_standings_for_date(
        &self,
        date: &str,
    ) -> Result<Vec<NhlStandingsRow>, FetchError> {
        let url = format!("{}/standings/{date}", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_standings(&raw))
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

/// Compatibility facade for the source-owned NHL schedule parser.
pub(crate) fn parse_game(
    game: &serde_json::Value,
    fallback_date: Option<&str>,
) -> Option<ScheduledGame> {
    icelines_sources::nhl::schedule::parse_game(game, fallback_date)
}

pub use icelines_sources::nhl::gamecenter::{
    parse_boxscore, parse_play_by_play, Boxscore, Goal, GoalieLine, PlayByPlay, PlayByPlayGoal,
    PlayByPlayPenalty, SkaterLine,
};

impl NhlApiClient {
    /// Fetch the boxscore for one game from `/v1/gamecenter/{id}/boxscore`.
    pub async fn fetch_boxscore(&self, game_id: u64) -> Result<Boxscore, FetchError> {
        let url = format!("{}/gamecenter/{game_id}/boxscore", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_boxscore(&raw, game_id))
    }

    /// Phase Foster +3 — return the raw JSON body alongside the
    /// parsed `Boxscore` so callers can persist the source-of-truth
    /// to disk (data/boxscores/&lt;date&gt;/&lt;game_id&gt;.json) and re-parse
    /// later for favorited-line population (Foster +4).
    pub async fn fetch_boxscore_with_raw(
        &self,
        game_id: u64,
    ) -> Result<(Boxscore, serde_json::Value), FetchError> {
        let url = format!("{}/gamecenter/{game_id}/boxscore", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        let parsed = parse_boxscore(&raw, game_id);
        Ok((parsed, raw))
    }

    /// Fetch event-level play-by-play for one game from
    /// `/v1/gamecenter/{id}/play-by-play`.
    pub async fn fetch_play_by_play(&self, game_id: u64) -> Result<PlayByPlay, FetchError> {
        let url = format!("{}/gamecenter/{game_id}/play-by-play", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_play_by_play(&raw, game_id))
    }

    /// Return the raw play-by-play JSON body alongside the typed projection so
    /// callers can persist the source before deriving records.
    pub async fn fetch_play_by_play_with_raw(
        &self,
        game_id: u64,
    ) -> Result<(PlayByPlay, serde_json::Value), FetchError> {
        let url = format!("{}/gamecenter/{game_id}/play-by-play", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        let parsed = parse_play_by_play(&raw, game_id);
        Ok((parsed, raw))
    }
}

pub use icelines_sources::nhl::playoff_bracket::{
    parse_playoff_bracket, PlayoffBracket, PlayoffGameResult, PlayoffGoal, PlayoffRound,
    PlayoffSeries,
};

impl NhlApiClient {
    /// Fetch the playoff bracket for a given playoff year (the second year of the
    /// season; e.g. for season 2025-26 the year is 2026).
    pub async fn fetch_playoff_bracket(&self, year: u16) -> Result<PlayoffBracket, FetchError> {
        let url = format!("{}/playoff-bracket/{year}", self.base_web);
        let raw: serde_json::Value = self.get_json(&url).await?;
        Ok(parse_playoff_bracket(&raw))
    }
}

#[cfg(test)]
mod boxscore_tests {
    use super::{parse_boxscore, parse_play_by_play};
    use icelines_core::{ShotEventKind, TeamSide};
    use serde_json::json;

    #[test]
    fn l0_parse_boxscore_reads_goal_scorer_id() {
        let raw = json!({
            "awayTeam": {"abbrev": "SEA", "score": 1},
            "homeTeam": {"abbrev": "EDM", "score": 0},
            "summary": {
                "scoring": [
                    {
                        "periodDescriptor": {"number": 1, "periodType": "REG"},
                        "goals": [
                            {
                                "playerId": 8477444,
                                "firstName": {"default": "Andre"},
                                "lastName": {"default": "Burakovsky"},
                                "teamAbbrev": {"default": "SEA"},
                                "timeInPeriod": "04:12",
                                "awayScore": 1,
                                "homeScore": 0
                            }
                        ]
                    }
                ]
            }
        });

        let parsed = parse_boxscore(&raw, 2025020001);

        assert_eq!(parsed.goals.len(), 1);
        assert_eq!(parsed.goals[0].scorer_id, Some(8477444));
        assert_eq!(parsed.goals[0].scorer_team, "SEA");
    }

    #[test]
    fn l0_parse_play_by_play_reads_goalie_in_net_and_empty_net_gap() {
        let raw = json!({
            "id": 2023020001,
            "gameDate": "2023-10-10",
            "awayTeam": {"abbrev": "NSH"},
            "homeTeam": {"abbrev": "TBL"},
            "plays": [
                {
                    "eventId": 154,
                    "periodDescriptor": {"number": 1, "periodType": "REG"},
                    "timeInPeriod": "09:48",
                    "situationCode": "1551",
                    "typeDescKey": "goal",
                    "details": {
                        "scoringPlayerId": 8476453,
                        "eventOwnerTeamId": 14,
                        "goalieInNetId": 8477424
                    }
                },
                {
                    "eventId": 179,
                    "periodDescriptor": {"number": 3, "periodType": "REG"},
                    "timeInPeriod": "19:58",
                    "situationCode": "0651",
                    "typeDescKey": "goal",
                    "details": {
                        "scoringPlayerId": 8476453,
                        "eventOwnerTeamId": 14
                    }
                }
            ]
        });

        let parsed = parse_play_by_play(&raw, 0);

        assert_eq!(parsed.game_id, 2023020001);
        assert_eq!(parsed.goals.len(), 2);
        assert_eq!(parsed.goals[0].scoring_player_id, Some(8476453));
        assert_eq!(parsed.goals[0].goalie_in_net_id, Some(8477424));
        assert_eq!(parsed.goals[1].goalie_in_net_id, None);
        assert_eq!(parsed.scoring_events.len(), 2);
        assert_eq!(parsed.scoring_events[0].kind, ShotEventKind::Goal);
        assert_eq!(parsed.scoring_events[0].scoring_player_id, Some(8476453));
        assert_eq!(parsed.scoring_events[0].shooting_player_id, Some(8476453));
        assert_eq!(parsed.scoring_events[0].goalie_in_net_id, Some(8477424));
    }

    #[test]
    fn l0_parse_play_by_play_reads_shot_attempt_families() {
        let raw = json!({
            "id": 2025020001u64,
            "gameDate": "2025-10-07",
            "awayTeam": {"id": 16, "abbrev": "CHI"},
            "homeTeam": {"id": 24, "abbrev": "LAK"},
            "plays": [
                {
                    "eventId": 21,
                    "periodDescriptor": {"number": 1, "periodType": "REG"},
                    "timeInPeriod": "03:10",
                    "situationCode": "1551",
                    "homeTeamDefendingSide": "right",
                    "typeDescKey": "shot-on-goal",
                    "details": {
                        "eventOwnerTeamId": 16,
                        "shootingPlayerId": 8483493,
                        "goalieInNetId": 8475683,
                        "xCoord": 66,
                        "yCoord": -1,
                        "zoneCode": "O",
                        "shotType": "snap"
                    }
                },
                {
                    "eventId": 42,
                    "periodDescriptor": {"number": 1, "periodType": "REG"},
                    "timeInPeriod": "06:40",
                    "situationCode": "1551",
                    "typeDescKey": "missed-shot",
                    "details": {
                        "eventOwnerTeamId": 24,
                        "shootingPlayerId": 8471685,
                        "xCoord": -74,
                        "yCoord": 14,
                        "zoneCode": "O",
                        "reason": "wide-of-net",
                        "shotType": "wrist"
                    }
                },
                {
                    "eventId": 53,
                    "periodDescriptor": {"number": 2, "periodType": "REG"},
                    "timeInPeriod": "02:01",
                    "situationCode": "1551",
                    "typeDescKey": "blocked-shot",
                    "details": {
                        "eventOwnerTeamId": 16,
                        "shootingPlayerId": 8483493,
                        "blockingPlayerId": 8476457,
                        "xCoord": 52,
                        "yCoord": -8,
                        "zoneCode": "O",
                        "reason": "blocked"
                    }
                }
            ]
        });

        let parsed = parse_play_by_play(&raw, 0);

        assert_eq!(parsed.scoring_events.len(), 3);
        assert_eq!(parsed.scoring_events[0].kind, ShotEventKind::ShotOnGoal);
        assert_eq!(parsed.scoring_events[0].date.as_deref(), Some("2025-10-07"));
        assert_eq!(parsed.scoring_events[0].event_owner_team_id, Some(16));
        assert_eq!(
            parsed.scoring_events[0].event_owner_team_abbrev.as_deref(),
            Some("CHI")
        );
        assert_eq!(
            parsed.scoring_events[0].event_owner_side,
            Some(TeamSide::Away)
        );
        assert_eq!(parsed.scoring_events[0].shooting_player_id, Some(8483493));
        assert_eq!(parsed.scoring_events[0].goalie_in_net_id, Some(8475683));
        assert_eq!(parsed.scoring_events[0].location.x_coord, Some(66));
        assert_eq!(parsed.scoring_events[0].location.y_coord, Some(-1));
        assert_eq!(
            parsed.scoring_events[0].location.zone_code.as_deref(),
            Some("O")
        );
        assert_eq!(parsed.scoring_events[0].shot_type.as_deref(), Some("snap"));
        assert_eq!(
            parsed.scoring_events[0].home_team_defending_side.as_deref(),
            Some("right")
        );

        assert_eq!(parsed.scoring_events[1].kind, ShotEventKind::MissedShot);
        assert_eq!(
            parsed.scoring_events[1].event_owner_team_abbrev.as_deref(),
            Some("LAK")
        );
        assert_eq!(
            parsed.scoring_events[1].event_owner_side,
            Some(TeamSide::Home)
        );
        assert_eq!(
            parsed.scoring_events[1].reason.as_deref(),
            Some("wide-of-net")
        );

        assert_eq!(parsed.scoring_events[2].kind, ShotEventKind::BlockedShot);
        assert_eq!(parsed.scoring_events[2].blocking_player_id, Some(8476457));
    }

    #[test]
    fn l0_parse_play_by_play_preserves_missing_shot_coordinates() {
        let raw = json!({
            "id": 2025020002u64,
            "awayTeam": {"id": 16, "abbrev": "CHI"},
            "homeTeam": {"id": 24, "abbrev": "LAK"},
            "plays": [
                {
                    "eventId": 99,
                    "periodDescriptor": {"number": 1, "periodType": "REG"},
                    "timeInPeriod": "01:00",
                    "typeDescKey": "shot-on-goal",
                    "details": {
                        "eventOwnerTeamId": 16,
                        "shootingPlayerId": 8483493
                    }
                }
            ]
        });

        let parsed = parse_play_by_play(&raw, 0);

        assert_eq!(parsed.scoring_events.len(), 1);
        assert_eq!(parsed.scoring_events[0].location.x_coord, None);
        assert_eq!(parsed.scoring_events[0].location.y_coord, None);
        assert_eq!(parsed.scoring_events[0].location.zone_code, None);
        assert_eq!(parsed.scoring_events[0].goalie_in_net_id, None);
    }

    #[test]
    fn l0_parse_play_by_play_reads_fighting_participants() {
        let raw = json!({
            "id": 2023020005,
            "plays": [
                {
                    "eventId": 372,
                    "periodDescriptor": {"number": 1, "periodType": "REG"},
                    "timeInPeriod": "10:20",
                    "situationCode": "1551",
                    "typeDescKey": "penalty",
                    "details": {
                        "typeCode": "MAJ",
                        "descKey": "fighting",
                        "duration": 5,
                        "committedByPlayerId": 8471817,
                        "drawnByPlayerId": 8482964,
                        "eventOwnerTeamId": 10
                    }
                }
            ]
        });

        let parsed = parse_play_by_play(&raw, 0);

        assert_eq!(parsed.penalties.len(), 1);
        let penalty = &parsed.penalties[0];
        assert_eq!(penalty.penalty_type.as_deref(), Some("MAJ"));
        assert_eq!(penalty.desc_key.as_deref(), Some("fighting"));
        assert_eq!(penalty.duration, Some(5));
        assert_eq!(penalty.committed_by_player_id, Some(8471817));
        assert_eq!(penalty.drawn_by_player_id, Some(8482964));
    }
}

#[cfg(test)]
mod standings_tests {
    use super::parse_standings;
    use serde_json::json;

    #[test]
    fn l0_parse_standings_projects_team_rows_for_core_input() {
        let raw = json!({
            "standings": [
                {
                    "teamAbbrev": { "default": "SEA" },
                    "conferenceName": "Western",
                    "divisionName": "Pacific",
                    "gamesPlayed": 40,
                    "wins": 22,
                    "losses": 13,
                    "otLosses": 5,
                    "points": 49,
                    "pointPctg": 0.613,
                    "regulationWins": 19,
                    "goalDifferential": 14,
                    "leagueSequence": 9,
                    "conferenceSequence": 5,
                    "divisionSequence": 3,
                    "wildcardSequence": 1
                }
            ]
        });

        let rows = parse_standings(&raw);
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row.team, "SEA");
        assert_eq!(row.conference.as_deref(), Some("Western"));
        assert_eq!(row.division.as_deref(), Some("Pacific"));
        assert_eq!(row.games_played, 40);
        assert_eq!(row.points, 49);
        assert_eq!(row.points_percentage, 0.613);
        assert_eq!(row.wild_card_rank, Some(1));

        let input = row.to_team_standing_input();
        assert_eq!(input.team, "SEA");
        assert_eq!(input.regulation_wins, Some(19));
    }

    #[test]
    fn l0_parse_standings_computes_points_percentage_when_missing() {
        let raw = json!({
            "standings": [
                {
                    "teamAbbrev": "EDM",
                    "conferenceAbbrev": "W",
                    "gamesPlayed": 10,
                    "wins": 6,
                    "losses": 3,
                    "otLosses": 1,
                    "points": 13
                }
            ]
        });

        let rows = parse_standings(&raw);
        assert_eq!(rows[0].team, "EDM");
        assert_eq!(rows[0].conference.as_deref(), Some("Western"));
        assert!((rows[0].points_percentage - 0.65).abs() < 0.001);
    }
}

#[cfg(test)]
#[allow(non_snake_case)] // test names encode literal NHL API field names (camelCase): seriesSummary, gameLabel, etc.
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
                for (k, val) in extra_map {
                    base.insert(k, val);
                }
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
        assert_eq!(
            g.series_game.as_deref(),
            Some("Game 3"),
            "numeric gameNumberOfSeven should synthesise a 'Game N' label"
        );
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
        assert_eq!(
            g.series_game.as_deref(),
            Some("Game 5"),
            "current API uses gameNumberOfSeries"
        );
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
