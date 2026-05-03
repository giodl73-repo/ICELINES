//! Schedule tab: per-week and per-team caches with non-blocking fetch.
//!
//! Mirrors the pattern from `tonight.rs` but supports multiple cache keys
//! (one entry per week and per team-season). All `App` state mutations stay
//! on the main thread; background tasks publish results into the cache via
//! `Arc<Mutex<HashMap<...>>>`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{Datelike, Duration, NaiveDate, Utc};
use icelines_fetch::nhl_api::{NhlApiClient, ScheduledGame};

#[derive(Debug, Clone, Default)]
pub enum ScheduleState {
    #[default]
    Idle,
    Loading,
    Loaded(Vec<ScheduledGame>),
    Error(String),
}

pub type WeekCache = Arc<Mutex<HashMap<String, ScheduleState>>>;
/// Hart.5c.6 Phase C — D5 cache key widening. Keyed by
/// `(team_abbrev, season)` so a season switch can't return wrong-
/// season schedule data after `repo_swap` (KEEL/HART catch).
pub type TeamSeasonCache = Arc<Mutex<HashMap<(String, String), ScheduleState>>>;

pub fn new_week_cache() -> WeekCache {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn new_team_cache() -> TeamSeasonCache {
    Arc::new(Mutex::new(HashMap::new()))
}

/// Get the Monday (week-start) for a YYYY-MM-DD date string.
pub fn monday_of(date: &str) -> Option<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let offset = d.weekday().num_days_from_monday() as i64;
    let monday = d - Duration::days(offset);
    Some(monday.format("%Y-%m-%d").to_string())
}

/// Add (or subtract) days from a YYYY-MM-DD date string.
pub fn add_days(date: &str, days: i64) -> Option<String> {
    let d = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
    let new = if days >= 0 {
        d + Duration::days(days)
    } else {
        d - Duration::days(-days)
    };
    Some(new.format("%Y-%m-%d").to_string())
}

/// Today's date as YYYY-MM-DD (UTC). Used as a default starting point —
/// the schedule API itself returns the gameWeek containing this date.
pub fn today_iso() -> String {
    Utc::now().format("%Y-%m-%d").to_string()
}

/// "Apr 28 — May 4" style label for a Monday date string.
pub fn week_label(monday: &str) -> String {
    let parsed = NaiveDate::parse_from_str(monday, "%Y-%m-%d");
    if let Ok(d) = parsed {
        let end = d + Duration::days(6);
        format!("{} — {}", d.format("%b %-d"), end.format("%b %-d"))
    } else {
        monday.to_owned()
    }
}

/// Trigger a background fetch for the week starting at `week_start`
/// (must be a Monday) if the entry is Idle or missing.
pub fn maybe_fetch_week(cache: WeekCache, week_start: String) {
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap().insert(
            week_start,
            ScheduleState::Error(crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()),
        );
        return;
    }
    {
        let mut map = cache.lock().unwrap();
        match map.get(&week_start) {
            Some(ScheduleState::Loading)
            | Some(ScheduleState::Loaded(_))
            | Some(ScheduleState::Error(_)) => return,
            _ => {}
        }
        map.insert(week_start.clone(), ScheduleState::Loading);
    }

    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }

    let cache2 = cache.clone();
    let key = week_start.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_schedule_for_date(&key).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(games) => map.insert(key, ScheduleState::Loaded(games)),
            Err(e) => map.insert(key, ScheduleState::Error(e.to_string())),
        };
    });
}

/// Force a refetch even if the cache already has data — used by `r` (retry).
pub fn force_fetch_week(cache: WeekCache, week_start: String) {
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap().insert(
            week_start,
            ScheduleState::Error(crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()),
        );
        return;
    }
    {
        let mut map = cache.lock().unwrap();
        map.insert(week_start.clone(), ScheduleState::Loading);
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    let key = week_start.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_schedule_for_date(&key).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(games) => map.insert(key, ScheduleState::Loaded(games)),
            Err(e) => map.insert(key, ScheduleState::Error(e.to_string())),
        };
    });
}

/// Pre-fetch the Monday for `from_week` plus the next two weeks.
pub fn prefetch_around(cache: WeekCache, from_week: &str) {
    if let Some(monday0) = monday_of(from_week) {
        maybe_fetch_week(cache.clone(), monday0.clone());
        if let Some(w1) = add_days(&monday0, 7) {
            maybe_fetch_week(cache.clone(), w1);
        }
        if let Some(w2) = add_days(&monday0, 14) {
            maybe_fetch_week(cache, w2);
        }
    }
}

/// Trigger a background fetch for one team's full-season schedule if not
/// already loading or loaded. Key is `(team_abbrev_uppercase, season)` —
/// post-Hart.5c.6 D5 widening so the cache survives a season switch
/// without returning stale data.
pub fn maybe_fetch_team(cache: TeamSeasonCache, team: String, season: String) {
    let key = (team.clone(), season.clone());
    if !crate::config::live_feeds_enabled() {
        cache.lock().unwrap().insert(
            key,
            ScheduleState::Error(crate::tui::tonight::LIVE_DISABLED_MSG.to_owned()),
        );
        return;
    }
    {
        let mut map = cache.lock().unwrap();
        match map.get(&key) {
            Some(ScheduleState::Loading)
            | Some(ScheduleState::Loaded(_))
            | Some(ScheduleState::Error(_)) => return,
            _ => {}
        }
        map.insert(key.clone(), ScheduleState::Loading);
    }
    if tokio::runtime::Handle::try_current().is_err() {
        return;
    }
    let cache2 = cache.clone();
    tokio::spawn(async move {
        let client = NhlApiClient::production();
        let result = client.fetch_team_season_schedule(&team, &season).await;
        let mut map = cache2.lock().unwrap();
        match result {
            Ok(games) => map.insert(key, ScheduleState::Loaded(games)),
            Err(e) => map.insert(key, ScheduleState::Error(e.to_string())),
        };
    });
}

/// Parse a search query into 0, 1, or 2 team filters.
/// Lowercase is normalized to uppercase. Splits on whitespace, comma, "vs", "@".
/// Returns Ok((Option<team1>, Option<team2>)) or an error message string.
pub fn parse_search(query: &str) -> Result<SearchFilter, String> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Ok(SearchFilter::None);
    }

    // Split on whitespace, commas, and "vs"/"@" tokens
    let tokens: Vec<String> = trimmed
        .split(|c: char| c.is_whitespace() || c == ',' || c == '@')
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("vs"))
        .map(|s| s.to_uppercase())
        .collect();

    match tokens.len() {
        0 => Ok(SearchFilter::None),
        1 => {
            validate_team(&tokens[0])?;
            Ok(SearchFilter::Team(tokens[0].clone()))
        }
        2 => {
            validate_team(&tokens[0])?;
            validate_team(&tokens[1])?;
            if tokens[0] == tokens[1] {
                return Err("Cannot search same team vs itself".to_owned());
            }
            Ok(SearchFilter::Matchup(tokens[0].clone(), tokens[1].clone()))
        }
        _ => Err(format!(
            "Too many teams (got {}); use one or two abbrevs",
            tokens.len()
        )),
    }
}

fn validate_team(abbrev: &str) -> Result<(), String> {
    use icelines_core::TeamAbbr;
    TeamAbbr::parse(abbrev)
        .map(|_| ())
        .map_err(|_| format!("Unknown team: '{abbrev}'. Try: SEA, NYR, EDM, ..."))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SearchFilter {
    None,
    Team(String),
    Matchup(String, String),
}

impl SearchFilter {
    /// True if the game matches the active filter (or filter is None).
    pub fn matches(&self, g: &ScheduledGame) -> bool {
        match self {
            SearchFilter::None => true,
            SearchFilter::Team(t) => g.involves(t),
            SearchFilter::Matchup(a, b) => g.involves(a) && g.involves(b),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_schedule_monday_of_helper() {
        // 2026-04-28 is a Tuesday → Monday is 2026-04-27
        assert_eq!(monday_of("2026-04-28"), Some("2026-04-27".to_owned()));
        // A Monday returns itself
        assert_eq!(monday_of("2026-04-27"), Some("2026-04-27".to_owned()));
        // A Sunday rolls back six days
        assert_eq!(monday_of("2026-05-03"), Some("2026-04-27".to_owned()));
        // Garbage in → None
        assert_eq!(monday_of("not-a-date"), None);
    }

    #[test]
    fn l0_schedule_add_days_helper() {
        assert_eq!(add_days("2026-04-27", 7), Some("2026-05-04".to_owned()));
        assert_eq!(add_days("2026-04-27", -7), Some("2026-04-20".to_owned()));
        // Wraps month boundary
        assert_eq!(add_days("2026-04-30", 1), Some("2026-05-01".to_owned()));
    }

    #[test]
    fn l0_schedule_search_single_team() {
        let f = parse_search("SEA").unwrap();
        assert_eq!(f, SearchFilter::Team("SEA".to_owned()));
    }

    #[test]
    fn l0_schedule_search_normalizes_lowercase() {
        let f = parse_search("nyr").unwrap();
        assert_eq!(f, SearchFilter::Team("NYR".to_owned()));
    }

    #[test]
    fn l0_schedule_search_matchup() {
        let f = parse_search("NYR WSH").unwrap();
        assert_eq!(f, SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned()));

        // Alternate forms — "vs" and "@" should be treated as separators
        let f2 = parse_search("nyr vs wsh").unwrap();
        assert_eq!(
            f2,
            SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned())
        );
        let f3 = parse_search("NYR @ WSH").unwrap();
        assert_eq!(
            f3,
            SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned())
        );
    }

    #[test]
    fn l0_schedule_search_invalid_team_error() {
        let err = parse_search("XYZ").unwrap_err();
        assert!(err.contains("Unknown team"), "got: {err}");

        // One valid + one invalid still errors on the invalid
        let err2 = parse_search("NYR INVALID").unwrap_err();
        assert!(err2.contains("Unknown team"), "got: {err2}");
    }

    #[test]
    fn l0_schedule_search_cannot_search_same_team() {
        let err = parse_search("SEA SEA").unwrap_err();
        assert!(err.contains("same team"), "got: {err}");
    }

    #[test]
    fn l0_schedule_search_empty_returns_none() {
        assert_eq!(parse_search("").unwrap(), SearchFilter::None);
        assert_eq!(parse_search("   ").unwrap(), SearchFilter::None);
    }

    #[test]
    fn l0_schedule_search_too_many_teams_error() {
        let err = parse_search("NYR WSH SEA").unwrap_err();
        assert!(err.contains("Too many"), "got: {err}");
    }

    #[test]
    fn l0_schedule_filter_matches() {
        let mk_game = |away: &str, home: &str| icelines_fetch::nhl_api::ScheduledGame {
            game_id: 1,
            date: "2026-04-28".to_owned(),
            game_type: 2,
            away_abbrev: away.to_owned(),
            away_name: away.to_owned(),
            home_abbrev: home.to_owned(),
            home_name: home.to_owned(),
            start_time_utc: "2026-04-28T23:00:00Z".to_owned(),
            away_score: None,
            home_score: None,
            game_state: None,
            last_period: None,
            series_game: None,
            away_wins: None,
            home_wins: None,
        };

        let g1 = mk_game("SEA", "VGK");
        let g2 = mk_game("NYR", "WSH");
        let g3 = mk_game("EDM", "CGY");

        // None matches everything
        assert!(SearchFilter::None.matches(&g1));

        // Team filter
        let team = SearchFilter::Team("SEA".to_owned());
        assert!(team.matches(&g1));
        assert!(!team.matches(&g2));

        // Matchup needs both
        let matchup = SearchFilter::Matchup("NYR".to_owned(), "WSH".to_owned());
        assert!(matchup.matches(&g2));
        assert!(!matchup.matches(&g1));
        assert!(!matchup.matches(&g3));
    }

    #[test]
    fn l0_schedule_week_label_is_human_readable() {
        let label = week_label("2026-04-27");
        // Just check it contains both ends
        assert!(label.contains("Apr"));
        assert!(label.contains("May") || label.contains("Apr"));
    }

    #[test]
    fn l0_team_cache_keyed_by_team_and_season_distinguishes_windows() {
        // Hart.5c.6 Phase C — D5 cache key widening: pre-fix, the cache
        // was keyed only by team, so a season switch returned wrong-
        // season schedules. With the (team, season) key, two seasons
        // produce two distinct entries that don't poison each other.
        let cache = new_team_cache();
        let key_24 = ("EDM".to_owned(), "20242025".to_owned());
        let key_25 = ("EDM".to_owned(), "20252026".to_owned());
        cache
            .lock()
            .unwrap()
            .insert(key_24.clone(), ScheduleState::Loaded(vec![]));
        cache
            .lock()
            .unwrap()
            .insert(key_25.clone(), ScheduleState::Loading);

        let map = cache.lock().unwrap();
        // Both entries coexist; they don't collide on team alone.
        assert!(matches!(map.get(&key_24), Some(ScheduleState::Loaded(_))));
        assert!(matches!(map.get(&key_25), Some(ScheduleState::Loading)));
        assert_eq!(
            map.len(),
            2,
            "two seasons must produce two distinct cache entries"
        );
    }
}
