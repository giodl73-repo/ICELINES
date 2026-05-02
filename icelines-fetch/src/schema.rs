use serde::{Deserialize, Serialize};

// ── Contract data (from NHL landing API /v1/player/{id}/landing) ──────────────
//
// NOTE: As of 2026-04-26, the NHL public landing API does not expose contract
// fields. The API returns player bios, career stats, awards, and roster data
// but no salary, expiry type, or expiry year. These fields are present in the
// struct for forward-compatibility and are always None when fetched from the
// current NHL API. Future NHL API versions or a third-party source may populate
// them. The `player_id` is always populated.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlayerContract {
    pub player_id: u32,
    /// Year the contract expires (e.g. 2027). None — not available in current NHL API.
    pub expiry_year: Option<u16>,
    /// Contract type: "UFA", "RFA", "ELC", etc. None — not available in current NHL API.
    pub expiry_type: Option<String>,
    /// Current-season cap hit / salary in dollars. None — not available in current NHL API.
    pub salary: Option<u64>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PagedResponse<T> {
    pub data: Vec<T>,
    pub total: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkaterBio {
    pub player_id: u32,
    pub skater_full_name: String,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub current_team_abbrev: Option<String>,  // null for retired/unsigned players
    pub position_code: String,
    pub birth_date: Option<String>,
    pub birth_country: Option<String>,
    pub nationality_code: Option<String>,
    pub shoots_catches: Option<String>,
    pub draft_year: Option<u32>,
    pub draft_round: Option<u32>,
    pub draft_overall: Option<u32>,
    pub birth_city: Option<String>,
    pub birth_state_province_code: Option<String>,
    pub height: Option<u32>,
    pub weight: Option<u32>,
    pub first_season_for_game_type: Option<u32>,
    pub is_in_hall_of_fame_yn: Option<String>,
    pub last_name: String,
    pub points: u32,
    /// `seasonId` from the NHL API (e.g. `20242025`). The live API
    /// includes this in every row; bundled bios pre-Hart.6 lack it.
    /// Loader rejects rows where `season_id != requested_season` per
    /// Hart.6.4 — when `None`, the check is skipped (bundled trust).
    #[serde(default)]
    pub season_id: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkaterStats {
    pub player_id: u32,
    pub games_played: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub points_per_game: f32,
    pub pp_goals: u32,
    pub pp_points: u32,
    pub sh_goals: u32,
    pub sh_points: u32,
    pub game_winning_goals: u32,
    pub ot_goals: u32,
    pub shots: u32,
    #[serde(rename = "shootingPct", alias = "shootingPctg")]
    pub shooting_pctg: Option<f32>,   // null for players with 0 shots
    pub plus_minus: i32,
    pub time_on_ice_per_game: Option<f32>,
    pub faceoff_win_pct: Option<f32>, // null for non-centers
    /// `seasonId` from the NHL API (e.g. `20242025`). Every bundled
    /// stats.json row carries it (verified 2026-05-01); the live API
    /// returns it on every row. Loader rejects mismatched rows per
    /// Hart.6.4. `Option` (with `serde(default)`) only as a safety
    /// net for hand-edited fixtures.
    #[serde(default)]
    pub season_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkaterRealtime {
    pub player_id: u32,
    pub hits: u32,
    #[serde(rename = "blockedShots")]
    pub blocked_shots: u32,
    pub missed_shots: u32,
    pub giveaways: u32,
    pub takeaways: u32,
    #[serde(rename = "pim")]
    pub pim: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RosterResponse {
    pub forwards: Vec<RosterPlayer>,
    pub defensemen: Vec<RosterPlayer>,
    pub goalies: Vec<RosterPlayer>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RosterPlayer {
    pub id: u32,
    pub first_name: LocalizedString,
    pub last_name: LocalizedString,
    pub sweater_number: Option<u32>,
    pub position_code: String,
    pub shoots_catches: Option<String>,
    pub birth_date: Option<String>,
    pub birth_country: Option<String>,
    pub height_in_inches: Option<u32>,
    pub weight_in_pounds: Option<u32>,
    pub headshot: Option<String>,
    pub birth_city: Option<LocalizedString>,
    pub birth_state_province: Option<LocalizedString>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum LocalizedString {
    Plain(String),
    Localized { default: String },
}

impl LocalizedString {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Plain(s) => s,
            Self::Localized { default } => default,
        }
    }
}

impl std::fmt::Display for LocalizedString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ── Goalie season stats (Phase G.1) ──────────────────────────────────────────
//
// Mirrors the JSON shape produced by NHL `/stats/rest/en/goalie/summary`
// and bundled at `data/seasons/{S}/goalie-stats.json`. Era-typical nulls:
// `ties` is null for 2005-06 and later (NHL eliminated ties in the lockout
// rules); `ot_losses` is null pre-2005 (no shootout era).

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalieStats {
    pub player_id:             u32,
    pub goalie_full_name:      String,
    pub last_name:             String,
    /// Comma-separated abbrevs when traded mid-season (e.g. "BOS,OTT").
    pub team_abbrevs:          String,
    pub season_id:             u32,
    pub shoots_catches:        Option<String>,
    pub games_played:          u32,
    pub games_started:         u32,
    pub wins:                  u32,
    pub losses:                u32,
    pub ot_losses:             Option<u32>,
    pub ties:                  Option<u32>,
    pub shots_against:         u32,
    pub goals_against:         u32,
    pub saves:                 u32,
    /// Save percentage as a 0.0..=1.0 decimal. Null when shots_against=0.
    #[serde(rename = "savePct", alias = "savePctg")]
    pub save_pct:              Option<f32>,
    /// Goals-against average. Null when games_played=0.
    pub goals_against_average: Option<f32>,
    pub shutouts:              u32,
    pub time_on_ice:           u32,  // seconds
    /// Goalie scoring (rare but real — empty-net assists, occasional goals).
    pub goals:                 u32,
    pub assists:               u32,
    pub points:                u32,
    pub penalty_minutes:       u32,
}

impl GoalieStats {
    /// True iff this goalie meets the league's typical "qualifying"
    /// threshold for SV% / GAA leaderboards. Default 15 GP — matches
    /// the NHL.com convention for the Vezina-eligibility filter.
    pub fn qualified(&self, min_gp: u32) -> bool {
        self.games_played >= min_gp
    }

    /// Primary team abbrev — for traded goalies, returns the first one
    /// listed in `team_abbrevs` (chronologically the earlier stop).
    pub fn primary_team(&self) -> &str {
        self.team_abbrevs
            .split(',')
            .next()
            .unwrap_or("")
            .trim()
    }
}

// ── ESPN Transactions feed (Phase T.1) ──────────────────────────────────────────
//
// Captured from ESPN's site.api at:
//   https://site.api.espn.com/apis/site/v2/sports/hockey/nhl/transactions
//
// `deny_unknown_fields` is intentionally ON. ESPN sometimes adds metadata
// keys (e.g. `team.logos[]`); when that happens the fetcher falls back to
// a `serde_json::Value` extraction in `EspnSource` and logs the unknown
// field name — loud-on-change without losing the run. See spec
// "Schema validation" for the rationale.

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawTransaction {
    /// Raw timestamp from ESPN — typically date-only (`2026-04-29`),
    /// occasionally ISO 8601 with time (`2026-04-29T04:00:00Z`).
    /// Caller (T.3) is responsible for normalizing to America/New_York
    /// before bucketing into `Transaction.date`.
    pub date:        String,
    /// Free-form English description. Always populated.
    pub description: String,
    /// Team that initiated / received the move. Some rows (league-wide
    /// waiver windows, deadline-related notes) lack a team payload.
    #[serde(default)]
    pub team:        Option<RawTransactionTeam>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RawTransactionTeam {
    /// ESPN's internal numeric team id. Stringified in the response.
    pub id:           String,
    /// Short abbreviation as ESPN emits it. May need translation via
    /// `crate::teams::espn_to_nhl_abbrev` (e.g. `TB` → `TBL`, `ARI` →
    /// `UTA` post-2024-25).
    pub abbreviation: String,
    /// Full display name (e.g. "Edmonton Oilers"). Stored for the TUI
    /// detail pane; not used for matching.
    pub display_name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hart.6.1 — `season_id` rolls in as a `#[serde(default)]` field
    /// on `SkaterStats`. Live API rows always carry it; bundled rows
    /// always carry it; missing → `None` (defensive only).
    #[test]
    fn l0_hart6_1_skater_stats_seasonid_round_trips() {
        let json = r#"{
            "playerId": 8478402,
            "gamesPlayed": 76,
            "goals": 35,
            "assists": 65,
            "points": 100,
            "pointsPerGame": 1.31,
            "ppGoals": 10,
            "ppPoints": 35,
            "shGoals": 0,
            "shPoints": 0,
            "gameWinningGoals": 7,
            "otGoals": 1,
            "shots": 280,
            "shootingPctg": 0.125,
            "plusMinus": 22,
            "timeOnIcePerGame": 1240.5,
            "faceoffWinPct": 0.524,
            "seasonId": 20242025
        }"#;
        let s: SkaterStats = serde_json::from_str(json).expect("parse");
        assert_eq!(s.season_id, Some(20242025));
    }

    /// Hart.6.1 — `season_id` is optional via `#[serde(default)]`. When
    /// the API or a hand-edited fixture omits it, the row still parses.
    #[test]
    fn l0_hart6_1_skater_stats_seasonid_missing_defaults_to_none() {
        let json = r#"{
            "playerId": 8478402,
            "gamesPlayed": 1,
            "goals": 0,
            "assists": 0,
            "points": 0,
            "pointsPerGame": 0.0,
            "ppGoals": 0,
            "ppPoints": 0,
            "shGoals": 0,
            "shPoints": 0,
            "gameWinningGoals": 0,
            "otGoals": 0,
            "shots": 0,
            "shootingPctg": null,
            "plusMinus": 0,
            "timeOnIcePerGame": null,
            "faceoffWinPct": null
        }"#;
        let s: SkaterStats = serde_json::from_str(json).expect("parse without seasonId");
        assert_eq!(s.season_id, None);
    }

    /// Hart.6.1 — bundled bios pre-Hart.6 lack `seasonId`; the field
    /// must default to `None` so existing bundled JSON parses cleanly.
    #[test]
    fn l0_hart6_1_skater_bio_seasonid_optional_for_bundled_compat() {
        let json = r#"{
            "playerId": 8478402,
            "skaterFullName": "Connor McDavid",
            "lastName": "McDavid",
            "gamesPlayed": 76,
            "goals": 35,
            "assists": 65,
            "points": 100,
            "currentTeamAbbrev": "EDM",
            "positionCode": "C"
        }"#;
        let b: SkaterBio = serde_json::from_str(json).expect("bundled bio parses without seasonId");
        assert_eq!(b.season_id, None);
        assert_eq!(b.player_id, 8478402);
    }

    /// Hart.6.1 — when present, `seasonId` round-trips on `SkaterBio` too.
    /// Future-proofs the bundle authoring path which will inject seasonId
    /// per the Hart.6.3 procedure.
    #[test]
    fn l0_hart6_1_skater_bio_seasonid_round_trips_when_present() {
        let json = r#"{
            "playerId": 8478402,
            "skaterFullName": "Connor McDavid",
            "lastName": "McDavid",
            "gamesPlayed": 76,
            "goals": 35,
            "assists": 65,
            "points": 100,
            "currentTeamAbbrev": "EDM",
            "positionCode": "C",
            "seasonId": 20242025
        }"#;
        let b: SkaterBio = serde_json::from_str(json).expect("parse");
        assert_eq!(b.season_id, Some(20242025));
    }
}
