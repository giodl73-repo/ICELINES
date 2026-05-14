//! Phase Calder.1 — parser for `/v1/player/{id}/landing` `seasonTotals`.
//!
//! Pure: takes a `serde_json::Value` and returns `CareerHistory`. The
//! HTTP client lives in `nhl_api.rs::fetch_player_career_history` and
//! defers to this module for the schema work.
//!
//! Schema observations (from frozen fixtures captured 2026-05-06):
//! - `seasonTotals` is always an array; absent/empty → no career data.
//! - Every entry has `season` (u32, YYYYZZZZ), `gameTypeId` (u32,
//!   2 or 3), `leagueAbbrev` (string), `gamesPlayed` (u32). Other
//!   fields are league-dependent — junior leagues drop `avgToi`,
//!   college leagues drop `shootingPctg`, goalie stints drop
//!   `goals`/`assists`/`points` per-game.
//! - `sequence` is the per-(season, gameType) ordering when a player
//!   has multiple stints (e.g., an OHL season + a WJC tournament + a
//!   minor-hockey appearance all in 2014-15). We preserve it so
//!   renderers don't reshuffle.
//! - `teamName.default` is the natural display string (covers
//!   "Erie", "Edmonton Oilers", "Canada"). Falls back to
//!   `teamCommonName.default` when `teamName` is absent (rare).

use icelines_core::career_history::{CareerGameType, CareerHistory, CareerStint, LeagueAbbrev};
use icelines_core::model::Season;
use icelines_core::{PlayerAwardRow, PlayerAwardSeasonRow, PlayerAwardsView, ViewContext};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum CareerParseError {
    #[error("missing or invalid `seasonTotals` array")]
    MissingSeasonTotals,
}

pub fn parse_career_history(
    player_id: u32,
    raw: &Value,
) -> Result<CareerHistory, CareerParseError> {
    let totals = raw
        .get("seasonTotals")
        .and_then(|v| v.as_array())
        .ok_or(CareerParseError::MissingSeasonTotals)?;

    // Per-entry resilience: a tournament we don't know how to bucket
    // (gameTypeId 6 / 7 / etc — preseason, exhibition) shouldn't
    // sink the whole player. Skip the bad entry and keep going.
    // Missing seasonTotals at the top level is still fatal.
    let mut stints = Vec::with_capacity(totals.len());
    for entry in totals.iter() {
        let Some(season) = entry.get("season").and_then(|v| v.as_u64()) else {
            continue;
        };
        let season = season as u32;

        let Some(game_type_id) = entry.get("gameTypeId").and_then(|v| v.as_u64()) else {
            continue;
        };
        let Some(game_type) = CareerGameType::from_api_id(game_type_id as u32) else {
            // Unknown gameTypeId (preseason, exhibition, etc) —
            // skip this entry, keep parsing the rest.
            continue;
        };

        let Some(league) = entry.get("leagueAbbrev").and_then(|v| v.as_str()) else {
            continue;
        };

        let Some(gp) = entry.get("gamesPlayed").and_then(|v| v.as_u64()) else {
            continue;
        };
        let gp = gp as u32;

        // Optional fields — best-effort. Team name resolves via the
        // localized `teamName.default`, falling back to common name
        // (rare but present for international entries).
        let team = entry
            .get("teamName")
            .and_then(|v| v.get("default"))
            .and_then(|v| v.as_str())
            .or_else(|| {
                entry
                    .get("teamCommonName")
                    .and_then(|v| v.get("default"))
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_owned();

        let sequence = entry
            .get("sequence")
            .and_then(|v| v.as_u64())
            .map(|n| n as u8)
            .unwrap_or(1);

        stints.push(CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team,
            game_type,
            sequence,
            gp,
            // Skater
            goals: u32_field(entry, "goals"),
            assists: u32_field(entry, "assists"),
            points: u32_field(entry, "points"),
            pim: u32_field(entry, "pim"),
            plus_minus: i32_field(entry, "plusMinus"),
            power_play_goals: u32_field(entry, "powerPlayGoals"),
            power_play_points: u32_field(entry, "powerPlayPoints"),
            shorthanded_goals: u32_field(entry, "shorthandedGoals"),
            shorthanded_points: u32_field(entry, "shorthandedPoints"),
            game_winning_goals: u32_field(entry, "gameWinningGoals"),
            ot_goals: u32_field(entry, "otGoals"),
            shots: u32_field(entry, "shots"),
            shooting_pct: f32_field(entry, "shootingPctg"),
            avg_toi_sec: toi_field(entry, "avgToi"),
            faceoff_win_pct: f32_field(entry, "faceoffWinningPctg"),
            // Goalie
            games_started: u32_field(entry, "gamesStarted"),
            wins: u32_field(entry, "wins"),
            losses: u32_field(entry, "losses"),
            ot_losses: u32_field(entry, "otLosses"),
            goals_against: u32_field(entry, "goalsAgainst"),
            goals_against_avg: f32_field(entry, "goalsAgainstAvg"),
            save_pct: f32_field(entry, "savePctg"),
            shots_against: u32_field(entry, "shotsAgainst"),
            shutouts: u32_field(entry, "shutouts"),
            time_on_ice_sec: toi_field(entry, "timeOnIce"),
        });
    }

    let mut history = CareerHistory { player_id, stints };
    history.sort_for_display();
    Ok(history)
}

fn u32_field(v: &Value, key: &str) -> Option<u32> {
    v.get(key).and_then(|n| n.as_u64()).map(|n| n as u32)
}

fn i32_field(v: &Value, key: &str) -> Option<i32> {
    v.get(key).and_then(|n| n.as_i64()).map(|n| n as i32)
}

fn f32_field(v: &Value, key: &str) -> Option<f32> {
    v.get(key).and_then(|n| n.as_f64()).map(|n| n as f32)
}

/// "MM:SS" → total seconds. Returns None on parse failure (rare but
/// the API does occasionally send malformed strings on minor rows).
fn toi_field(v: &Value, key: &str) -> Option<u32> {
    let raw = v.get(key)?.as_str()?;
    let mut parts = raw.splitn(2, ':');
    let m: u32 = parts.next()?.parse().ok()?;
    let s: u32 = parts.next()?.parse().ok()?;
    Some(m * 60 + s)
}

// ── Phase Profile.4 — awards parser + store ───────────────────────────

pub fn parse_player_awards(
    player_id: u32,
    player_name: impl Into<String>,
    context: ViewContext,
    raw: &Value,
) -> PlayerAwardsView {
    let awards = raw
        .get("awards")
        .and_then(|v| v.as_array())
        .map(|entries| {
            entries
                .iter()
                .filter_map(parse_award_row)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    PlayerAwardsView::new(context, player_id, player_name, awards)
}

fn parse_award_row(entry: &Value) -> Option<PlayerAwardRow> {
    let trophy = entry
        .get("trophy")
        .and_then(|v| v.get("default"))
        .and_then(|v| v.as_str())
        .filter(|name| !name.is_empty())?
        .to_string();
    let seasons = entry
        .get("seasons")
        .and_then(|v| v.as_array())
        .map(|rows| rows.iter().filter_map(parse_award_season).collect())
        .unwrap_or_default();
    Some(PlayerAwardRow { trophy, seasons })
}

fn parse_award_season(entry: &Value) -> Option<PlayerAwardSeasonRow> {
    let season = entry
        .get("seasonId")
        .or_else(|| entry.get("season"))
        .and_then(|v| v.as_u64())? as u32;
    let game_type_id = entry
        .get("gameTypeId")
        .and_then(|v| v.as_u64())
        .map(|v| v as u8)
        .unwrap_or(2);
    Some(PlayerAwardSeasonRow {
        season: Season(season),
        game_type_id,
        games_played: u32_field(entry, "gamesPlayed"),
        goals: u32_field(entry, "goals"),
        assists: u32_field(entry, "assists"),
        points: u32_field(entry, "points"),
        plus_minus: i32_field(entry, "plusMinus"),
        pim: u32_field(entry, "pim"),
        hits: u32_field(entry, "hits"),
        blocked_shots: u32_field(entry, "blockedShots"),
    })
}

// ── Phase Calder.2 — store ───────────────────────────────────────────

/// On-disk format for the global career-history blob.
///
/// Lives at `~/.icelines/career_history.json`. Single global file
/// (not per-season) because a player's pre-NHL career doesn't change
/// with the active season — McDavid's OHL years are the same fact
/// regardless of which NHL season the user is browsing.
///
/// Map shape rather than vec so the writer can do partial updates
/// without rewriting unrelated entries, and so the loader can
/// `O(1)`-lookup by pid.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CareerHistoryStore {
    pub schema_version: u32,
    /// RFC-3339 — when this blob was last refreshed. The CLI uses
    /// this to decide whether `fetch career` needs to re-run.
    pub fetched_at: Option<String>,
    /// Keyed by stringified pid so JSON object keys (which must be
    /// strings) round-trip cleanly.
    pub histories: HashMap<String, CareerHistory>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerAwardsStore {
    pub schema_version: u32,
    pub fetched_at: Option<String>,
    pub players: HashMap<String, PlayerAwardsView>,
}

impl PlayerAwardsStore {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            fetched_at: None,
            players: HashMap::new(),
        }
    }

    pub fn load(path: &Path) -> std::io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("invalid player_awards.json: {e}"),
                )
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(e),
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut tmp = path.to_path_buf();
        let mut name = tmp.file_name().map(|n| n.to_owned()).unwrap_or_default();
        name.push(".tmp");
        tmp.set_file_name(name);
        let bytes = serde_json::to_vec(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn upsert(&mut self, view: PlayerAwardsView) {
        self.players.insert(view.player_id.to_string(), view);
    }

    pub fn get(&self, player_id: u32) -> Option<&PlayerAwardsView> {
        self.players.get(&player_id.to_string())
    }

    pub fn stamp_now(&mut self) {
        self.fetched_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

impl CareerHistoryStore {
    pub fn new() -> Self {
        Self {
            schema_version: 1,
            fetched_at: None,
            histories: HashMap::new(),
        }
    }

    /// Read the blob from disk. Returns an empty store if the path
    /// doesn't exist — first-run is not an error.
    pub fn load(path: &Path) -> std::io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => {
                let store: Self = serde_json::from_slice(&bytes).map_err(|e| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("invalid career_history.json: {e}"),
                    )
                })?;
                Ok(store)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::new()),
            Err(e) => Err(e),
        }
    }

    /// Persist atomically: write to `<path>.tmp` then rename, so a
    /// crash mid-write never leaves a corrupt blob in place.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut tmp = path.to_path_buf();
        let mut name = tmp.file_name().map(|n| n.to_owned()).unwrap_or_default();
        name.push(".tmp");
        tmp.set_file_name(name);
        // Compact (`to_vec` not `to_vec_pretty`): the blob is 30+ MB
        // when populated for the active 5-season roster; pretty
        // formatting wastes ~50% on indentation we never look at.
        let bytes = serde_json::to_vec(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
        std::fs::write(&tmp, &bytes)?;
        std::fs::rename(&tmp, path)?;
        Ok(())
    }

    pub fn upsert(&mut self, history: CareerHistory) {
        let key = history.player_id.to_string();
        self.histories.insert(key, history);
    }

    pub fn get(&self, player_id: u32) -> Option<&CareerHistory> {
        self.histories.get(&player_id.to_string())
    }

    pub fn len(&self) -> usize {
        self.histories.len()
    }

    pub fn is_empty(&self) -> bool {
        self.histories.is_empty()
    }

    /// Stamp the fetched_at timestamp using the current UTC time.
    pub fn stamp_now(&mut self) {
        self.fetched_at = Some(chrono::Utc::now().to_rfc3339());
    }
}

/// Phase Calder.3 — load the local store from `~/.icelines/career_history.json`.
///
/// Returns an empty store if the file doesn't exist (first-run case)
/// or fails to parse. Available to both CLI and Web surfaces so they
/// don't both have to reimplement the path-finding ceremony.
pub fn load_local_store() -> CareerHistoryStore {
    let Some(path) = local_store_path() else {
        return CareerHistoryStore::new();
    };
    CareerHistoryStore::load(&path).unwrap_or_else(|_| CareerHistoryStore::new())
}

pub fn load_local_awards_store() -> PlayerAwardsStore {
    let Some(path) = local_awards_store_path() else {
        return PlayerAwardsStore::new();
    };
    PlayerAwardsStore::load(&path).unwrap_or_else(|_| PlayerAwardsStore::new())
}

pub fn save_local_awards_view(view: PlayerAwardsView) -> std::io::Result<()> {
    let Some(path) = local_awards_store_path() else {
        return Ok(());
    };
    let mut store = PlayerAwardsStore::load(&path).unwrap_or_else(|_| PlayerAwardsStore::new());
    store.upsert(view);
    store.stamp_now();
    store.save(&path)
}

/// Phase Foster +6 (SCOUT M7) — opportunistic career-history pull
/// for a newly-favorited player. Resolves the normalized name to a
/// PlayerId via the bundled bios index, fetches the landing endpoint
/// (when `live_feeds=true`), and persists to the local store. All
/// failure paths are silent except for a one-line stderr nudge so
/// callers don't have to wrap in their own error swallowing.
///
/// Called from both CLI (`icelines group add Favorites <name>`) and
/// the web `POST /favorites/add` so favoriting from either surface
/// gets the same pre-NHL arc populated for the dashboard.
pub async fn augment_career_history_for_player(
    display_name: &str,
    normalized: &str,
    live_feeds: bool,
) {
    let Some(pid) = crate::stats_loader::resolve_player_id_by_name(normalized) else {
        return; // Brand-new rookie not in bundled bios — defer to lazy fetch.
    };

    let mut store = load_local_store();
    if store.histories.contains_key(&pid.to_string()) {
        return; // Already cached.
    }

    if !live_feeds {
        return; // Honor offline mode.
    }

    let client = crate::nhl_api::NhlApiClient::production();
    match client.fetch_player_career_history(pid).await {
        Ok(history) => {
            store.upsert(history);
            store.fetched_at = Some(chrono::Utc::now().to_rfc3339());
            if let Some(path) = local_store_path() {
                match store.save(&path) {
                    Ok(()) => {
                        eprintln!("note: pre-NHL career added for {display_name}");
                    }
                    Err(e) => {
                        eprintln!("note: career-history augment write failed: {e}");
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("note: career-history augment skipped for {display_name}: {e}");
        }
    }
}

/// Phase Calder.3 — pure filter for "pre-NHL development tier"
/// stints (Pro / Junior / College, regular season only). Used by
/// every player-card surface (CLI scouting + query player, TUI
/// player screen, Web /player/:id HTML + JSON twin) so they all
/// agree on what "pre-NHL career" means.
///
/// Drops:
/// - NHL stints (those land in the existing NHL career section)
/// - International tournaments (WJC/WC/OG/4 Nations) — interesting
///   but not part of the "road to the NHL" arc
/// - Other / youth (GTHL/CSSHL/Brick — too noisy for the card)
/// - Playoff stints (development arc focuses on regular-season pace)
pub fn extract_pre_nhl_stints(
    history: &icelines_core::career_history::CareerHistory,
) -> Vec<icelines_core::career_history::CareerStint> {
    use icelines_core::career_history::{CareerGameType, LeagueTier};
    history
        .stints
        .iter()
        .filter(|s| {
            s.league.0 != "NHL"
                && matches!(
                    s.league.tier(),
                    LeagueTier::Pro | LeagueTier::Junior | LeagueTier::College
                )
                && matches!(s.game_type, CareerGameType::Regular)
        })
        .cloned()
        .collect()
}

/// `$HOME/.icelines/career_history.json` (Unix) or
/// `%USERPROFILE%\.icelines\career_history.json` (Windows). Returns
/// None when neither env var is set.
fn local_store_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        std::path::PathBuf::from(home)
            .join(".icelines")
            .join("career_history.json"),
    )
}

fn local_awards_store_path() -> Option<std::path::PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(
        std::path::PathBuf::from(home)
            .join(".icelines")
            .join("player_awards.json"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::career_history::LeagueTier;
    use icelines_core::season_stats::SeasonType;
    use icelines_core::{ViewContext, ViewWindow};

    fn load_fixture(name: &str) -> Value {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("landing")
            .join(name);
        let raw = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        serde_json::from_str(&raw).expect("fixture is valid json")
    }

    /// Calder.1 / l0_parse_mcdavid_includes_ohl_and_nhl
    /// — McDavid's career touches GTHL minor → OHL Erie 2012-15 →
    ///   NHL Edmonton 2015-now, plus WJ18-A and WJC-A. The parser
    ///   has to keep all of them and order them oldest first.
    #[test]
    fn l0_parse_mcdavid_includes_ohl_and_nhl() {
        let raw = load_fixture("mcdavid_8478402.json");
        let h = parse_career_history(8478402, &raw).expect("parse ok");
        assert!(h.stints.len() >= 12, "fewer stints than expected");
        // First league chronologically should be a youth league
        // (GTHL or "QC Int PW"), not NHL.
        let first = h.stints.first().expect("non-empty");
        assert_ne!(first.league.0, "NHL");
        // OHL Erie 2014-15 regular season has 47 GP and 120 points.
        let ohl_2015 = h
            .stints
            .iter()
            .find(|s| {
                s.season.0 == 20142015
                    && s.league.0 == "OHL"
                    && s.game_type == CareerGameType::Regular
            })
            .expect("McDavid 2014-15 OHL regular missing");
        assert_eq!(ohl_2015.gp, 47);
        assert_eq!(ohl_2015.points, Some(120));
        assert_eq!(ohl_2015.team, "Erie");
        assert_eq!(ohl_2015.league.tier(), LeagueTier::Junior);
        // First NHL season is 2015-16.
        let nhl_first = h
            .stints
            .iter()
            .find(|s| s.league.0 == "NHL" && s.game_type == CareerGameType::Regular)
            .expect("first NHL stint missing");
        assert_eq!(nhl_first.season.0, 20152016);
        assert_eq!(nhl_first.team, "Edmonton Oilers");
    }

    #[test]
    fn l0_parse_mcdavid_awards_includes_trophy_case() {
        let raw = load_fixture("mcdavid_8478402.json");
        let context = ViewContext::new(ViewWindow::new(Season(20252026), SeasonType::Regular));
        let view = parse_player_awards(8478402, "Connor McDavid", context, &raw);
        assert!(view.trophy_count() >= 5);
        let art_ross = view
            .awards
            .iter()
            .find(|award| award.trophy == "Art Ross Trophy")
            .expect("Art Ross Trophy present");
        assert!(art_ross.seasons.iter().any(|row| row.points == Some(100)));
    }

    /// Calder.1 / l0_parse_bedard_includes_whl_and_european_junior
    /// — Bedard's path includes Brick Invitational, CSSHL, then
    ///   J20 Nationell (Sweden) loan, then WHL Regina, then NHL.
    #[test]
    fn l0_parse_bedard_includes_whl_and_european_junior() {
        let raw = load_fixture("bedard_8484144.json");
        let h = parse_career_history(8484144, &raw).expect("parse ok");
        let leagues: Vec<&str> = h.leagues_in_order().iter().map(|l| l.0.as_str()).collect();
        assert!(leagues.contains(&"WHL"), "missing WHL: {leagues:?}");
        assert!(leagues.contains(&"NHL"), "missing NHL: {leagues:?}");
        assert!(
            leagues.contains(&"J20 Nationell"),
            "missing J20: {leagues:?}"
        );
    }

    /// Calder.1 / l0_parse_drysdale_includes_ushl_and_ncaa
    /// — Drysdale: GTHL U16 → USHL → NCAA → NHL. Tier classifier
    ///   should split them cleanly.
    #[test]
    fn l0_parse_drysdale_includes_ushl_and_ncaa() {
        let raw = load_fixture("drysdale_8482671.json");
        let h = parse_career_history(8482671, &raw).expect("parse ok");
        let junior_leagues: Vec<&str> = h
            .by_tier(LeagueTier::Junior)
            .map(|s| s.league.0.as_str())
            .collect();
        assert!(
            junior_leagues.contains(&"USHL"),
            "USHL missing from junior tier: {junior_leagues:?}"
        );
        let college_leagues: Vec<&str> = h
            .by_tier(LeagueTier::College)
            .map(|s| s.league.0.as_str())
            .collect();
        assert!(
            college_leagues.contains(&"NCAA"),
            "NCAA missing: {college_leagues:?}"
        );
    }

    /// Calder.1 / l0_parse_hellebuyck_carries_goalie_fields
    /// — Hellebuyck (G): NCAA UMass-Lowell H-East → AHL → NHL. The
    ///   parser must populate goalie-specific fields (wins, GAA, sv%)
    ///   and leave skater-only fields like avg_toi at None for the
    ///   junior/NCAA stints (which only carry GP/W/L for goalies).
    #[test]
    fn l0_parse_hellebuyck_carries_goalie_fields() {
        let raw = load_fixture("hellebuyck_8476945.json");
        let h = parse_career_history(8476945, &raw).expect("parse ok");
        // First NHL regular-season stint must have goalie fields populated.
        let nhl_first = h
            .stints
            .iter()
            .find(|s| s.league.0 == "NHL" && s.game_type == CareerGameType::Regular)
            .expect("first NHL stint");
        assert!(nhl_first.wins.is_some(), "wins must be present");
        assert!(nhl_first.save_pct.is_some(), "save_pct must be present");
        assert!(nhl_first.goals_against_avg.is_some(), "GAA must be present");
        // AHL stint should be populated too.
        let ahl = h
            .stints
            .iter()
            .find(|s| s.league.0 == "AHL")
            .expect("Hellebuyck spent time in AHL");
        assert!(ahl.wins.is_some());
    }

    /// Calder.1 / l0_parse_handles_missing_seasontotals
    /// — When the API returns an object without `seasonTotals` (e.g.,
    ///   a brand-new player or a malformed response), we surface a
    ///   typed error instead of panicking.
    #[test]
    fn l0_parse_handles_missing_seasontotals() {
        let raw = serde_json::json!({"playerId": 1});
        let err = parse_career_history(1, &raw).unwrap_err();
        assert!(matches!(err, CareerParseError::MissingSeasonTotals));
    }

    /// Calder.2 / l0_parse_skips_unknown_game_type
    /// — gameTypeId 6 (preseason) and 7 (exhibition) appear in real
    ///   data. We skip the entry rather than fail the whole player
    ///   so a single tournament row doesn't drop a 20-year career.
    ///   Recovers the 3 players we lost in the first live fetch.
    #[test]
    fn l0_parse_skips_unknown_game_type() {
        let raw = serde_json::json!({
            "seasonTotals": [
                {
                    "season": 20152016,
                    "gameTypeId": 6,
                    "leagueAbbrev": "PRE",
                    "gamesPlayed": 1,
                },
                {
                    "season": 20162017,
                    "gameTypeId": 2,
                    "leagueAbbrev": "NHL",
                    "gamesPlayed": 82,
                    "teamName": {"default": "Edmonton"},
                }
            ]
        });
        let h = parse_career_history(1, &raw).expect("parse must succeed");
        assert_eq!(
            h.stints.len(),
            1,
            "preseason entry should be skipped, NHL kept"
        );
        assert_eq!(h.stints[0].league.0, "NHL");
    }

    /// Calder.2 / l0_parse_skips_entry_missing_required_field
    /// — same resilience for missing `gamesPlayed` (saw 1 player
    ///   skipped in live fetch for this exact case).
    #[test]
    fn l0_parse_skips_entry_missing_required_field() {
        let raw = serde_json::json!({
            "seasonTotals": [
                {
                    "season": 20152016,
                    "gameTypeId": 2,
                    "leagueAbbrev": "NHL",
                    // gamesPlayed deliberately absent
                },
                {
                    "season": 20162017,
                    "gameTypeId": 2,
                    "leagueAbbrev": "NHL",
                    "gamesPlayed": 82,
                }
            ]
        });
        let h = parse_career_history(1, &raw).expect("parse must succeed");
        assert_eq!(h.stints.len(), 1, "incomplete entry skipped, complete kept");
    }

    /// Calder.1 / l0_toi_parser_round_trips
    #[test]
    fn l0_toi_parser_round_trips() {
        let v = serde_json::json!({"avgToi": "21:52"});
        assert_eq!(toi_field(&v, "avgToi"), Some(21 * 60 + 52));
        let v = serde_json::json!({"avgToi": "0:00"});
        assert_eq!(toi_field(&v, "avgToi"), Some(0));
        let v = serde_json::json!({"avgToi": "garbage"});
        assert_eq!(toi_field(&v, "avgToi"), None);
        let v = serde_json::json!({});
        assert_eq!(toi_field(&v, "avgToi"), None);
    }

    /// Calder.2 / l0_store_round_trips_through_disk
    /// — write a store, read it back, get-by-pid still works.
    #[test]
    fn l0_store_round_trips_through_disk() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("career_history.json");
        let mut store = CareerHistoryStore::new();
        let mcdavid = parse_career_history(8478402, &load_fixture("mcdavid_8478402.json"))
            .expect("mcdavid parses");
        let original_count = mcdavid.stints.len();
        store.upsert(mcdavid);
        store.stamp_now();
        store.save(&path).expect("save ok");

        let loaded = CareerHistoryStore::load(&path).expect("load ok");
        assert_eq!(loaded.schema_version, 1);
        assert!(loaded.fetched_at.is_some(), "fetched_at must round-trip");
        assert_eq!(loaded.len(), 1);
        let h = loaded.get(8478402).expect("McDavid present");
        assert_eq!(h.stints.len(), original_count);
        // OHL stint still resolves after round-trip.
        assert!(h.stints.iter().any(|s| s.league.0 == "OHL"));
    }

    /// Calder.2 / l0_store_load_missing_file_returns_empty
    /// — first run: the blob doesn't exist yet, load() must NOT
    ///   error — the caller proceeds with an empty store.
    #[test]
    fn l0_store_load_missing_file_returns_empty() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("nope.json");
        let store = CareerHistoryStore::load(&path).expect("missing file is not an error");
        assert!(store.is_empty());
        assert_eq!(store.schema_version, 1);
    }

    /// Calder.2 / l0_store_load_corrupt_file_errors_clearly
    /// — a malformed JSON body must surface as InvalidData, not
    ///   panic. Operator runs `fetch career` to regenerate.
    #[test]
    fn l0_store_load_corrupt_file_errors_clearly() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("bad.json");
        std::fs::write(&path, b"{not valid json").unwrap();
        let err = CareerHistoryStore::load(&path).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData);
    }

    /// Calder.2 / l0_store_atomic_write_does_not_leave_tmp
    /// — after a successful save, no `.tmp` sidecar is left in the
    ///   directory.
    #[test]
    fn l0_store_atomic_write_does_not_leave_tmp() {
        let dir = tempfile::tempdir().expect("tmpdir");
        let path = dir.path().join("career_history.json");
        let store = CareerHistoryStore::new();
        store.save(&path).unwrap();
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            !entries.iter().any(|n| n.ends_with(".tmp")),
            "tmp sidecar leaked: {entries:?}"
        );
        assert!(entries.iter().any(|n| n == "career_history.json"));
    }

    /// Calder.2 / l0_store_upsert_replaces_existing
    #[test]
    fn l0_store_upsert_replaces_existing() {
        let mut store = CareerHistoryStore::new();
        let h1 = CareerHistory {
            player_id: 1,
            stints: vec![],
        };
        store.upsert(h1);
        assert_eq!(store.len(), 1);

        let mut h2 = CareerHistory {
            player_id: 1,
            stints: vec![],
        };
        h2.stints.push(CareerStint {
            season: Season(20242025),
            league: LeagueAbbrev::new("NHL"),
            team: "EDM".into(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp: 82,
            goals: Some(48),
            assists: None,
            points: None,
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        });
        store.upsert(h2);
        assert_eq!(store.len(), 1, "upsert must REPLACE, not duplicate");
        assert_eq!(store.get(1).unwrap().stints.len(), 1);
    }

    /// Build a minimal CareerStint with all stat-fields blank — local
    /// helper for the extract_pre_nhl_stints L0s below.
    fn stint(season: u32, league: &str, _seq: u8, gt: CareerGameType) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: "T".into(),
            game_type: gt,
            sequence: 1,
            gp: 1,
            goals: None,
            assists: None,
            points: None,
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        }
    }

    /// Calder.3 / l0_extract_pre_nhl_stints_filters_correctly
    /// — Junior + College + Pro kept (regular season). NHL, playoff,
    ///   international, and youth (Other) all dropped.
    #[test]
    fn l0_extract_pre_nhl_stints_filters_correctly() {
        let history = icelines_core::career_history::CareerHistory {
            player_id: 8478402,
            stints: vec![
                stint(20122013, "OHL", 1, CareerGameType::Regular), // junior  — keep
                stint(20132014, "OHL", 1, CareerGameType::Regular), // junior  — keep
                stint(20132014, "OHL", 1, CareerGameType::Playoff), // junior  — drop (playoff)
                stint(20142015, "WJC-A", 2, CareerGameType::Regular), // intl  — drop
                stint(20142015, "GTHL", 1, CareerGameType::Regular), // youth — drop (Other tier)
                stint(20132014, "NCAA", 1, CareerGameType::Regular), // college — keep
                stint(20142015, "AHL", 1, CareerGameType::Regular), // pro    — keep
                stint(20152016, "NHL", 1, CareerGameType::Regular), // NHL    — drop
            ],
        };
        let pre = extract_pre_nhl_stints(&history);
        assert_eq!(pre.len(), 4, "OHL×2 + NCAA + AHL — got {}", pre.len());
        let leagues: Vec<&str> = pre.iter().map(|s| s.league.0.as_str()).collect();
        assert!(leagues.contains(&"OHL") && leagues.contains(&"NCAA") && leagues.contains(&"AHL"));
        assert!(!leagues.contains(&"NHL"));
        assert!(!leagues.contains(&"WJC-A"));
        assert!(!leagues.contains(&"GTHL"));
    }

    /// Calder.3 / l0_extract_pre_nhl_stints_empty_history
    #[test]
    fn l0_extract_pre_nhl_stints_empty_history() {
        let h = icelines_core::career_history::CareerHistory {
            player_id: 1,
            stints: vec![],
        };
        assert!(extract_pre_nhl_stints(&h).is_empty());
    }

    /// Calder.3 / l0_extract_pre_nhl_stints_only_nhl_returns_empty
    /// — A career-NHL'er yields empty; renderers skip the section.
    #[test]
    fn l0_extract_pre_nhl_stints_only_nhl_returns_empty() {
        let h = icelines_core::career_history::CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20152016, "NHL", 1, CareerGameType::Regular),
                stint(20162017, "NHL", 1, CareerGameType::Regular),
            ],
        };
        assert!(extract_pre_nhl_stints(&h).is_empty());
    }

    /// Calder.1 / l0_sort_for_display_keeps_seq_order
    /// — Bedard's 2017-18 has Brick Invitational + CSSHL U13 + WSI
    ///   etc. all in the same season. The parser preserves API
    ///   sequence ordering after sort_for_display.
    #[test]
    fn l0_sort_for_display_keeps_seq_order() {
        let raw = load_fixture("bedard_8484144.json");
        let h = parse_career_history(8484144, &raw).expect("parse ok");
        // For each (season, game_type) tuple, sequence must be
        // monotonic in the rendered order.
        let mut prev_key: Option<(u32, CareerGameType)> = None;
        let mut prev_seq: u8 = 0;
        for s in &h.stints {
            let key = (s.season.0, s.game_type);
            if Some(key) == prev_key {
                assert!(
                    s.sequence >= prev_seq,
                    "seq regressed within {key:?}: {prev_seq} → {}",
                    s.sequence
                );
            }
            prev_key = Some(key);
            prev_seq = s.sequence;
        }
    }
}
