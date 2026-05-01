use serde::{Deserialize, Serialize};

pub const MIN_GP: u32 = 10;

// ── Season ────────────────────────────────────────────────────────────────────

/// 8-digit YYYYZZZZ season identifier (e.g. Season(20252026)).
/// Newtype prevents silent confusion with 4-digit year u32 values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Season(pub u32);

impl Season {
    pub fn as_str(self) -> String {
        self.0.to_string()
    }
}

impl std::fmt::Display for Season {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Display as "2025-26" for readability
        let s = self.0.to_string();
        if s.len() == 8 {
            write!(f, "{}-{}", &s[..4], &s[6..])
        } else {
            write!(f, "{}", self.0)
        }
    }
}

// ── TeamAbbr ─────────────────────────────────────────────────────────────────

/// Newtype around a 3-letter NHL team abbreviation.
///
/// `#[serde(transparent)]` makes it serialize as the raw string ("TBL"),
/// not as a tuple struct (`["TBL"]`). This is the natural shape for JSON
/// snapshots and matches what the rest of the codebase already prints.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TeamAbbr(pub String);

impl TeamAbbr {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for TeamAbbr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ── Position ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Position {
    Center,
    LeftWing,
    RightWing,
    Defense,
    Goalie,
}

impl Position {
    pub fn is_forward(self) -> bool {
        matches!(self, Self::Center | Self::LeftWing | Self::RightWing)
    }

    pub fn is_defense(self) -> bool {
        matches!(self, Self::Defense)
    }

    /// Parse from NHL API positionCode (L, R, C, D, G)
    pub fn from_api_code(code: &str) -> Option<Self> {
        match code {
            "C" => Some(Self::Center),
            "L" => Some(Self::LeftWing),
            "R" => Some(Self::RightWing),
            "D" => Some(Self::Defense),
            "G" => Some(Self::Goalie),
            _ => None,
        }
    }

    pub fn abbreviation(self) -> &'static str {
        match self {
            Self::Center => "C",
            Self::LeftWing => "LW",
            Self::RightWing => "RW",
            Self::Defense => "D",
            Self::Goalie => "G",
        }
    }
}

// ── GpStatus ─────────────────────────────────────────────────────────────────

/// Games-played status for a player this season.
/// Using a proper enum prevents silent GP=0 and un-fetched states from
/// being treated the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpStatus {
    /// GP data has not been fetched yet.
    Unfetched,
    /// Player dressed for zero games (not on active roster, AHL, injury from day 1).
    Zero,
    /// Player dressed but below MIN_GP threshold — pace projection unreliable.
    BelowThreshold(u32),
    /// Player is eligible for pace ranking.
    Eligible(u32),
}

impl GpStatus {
    pub fn from_gp(gp: u32) -> Self {
        match gp {
            0 => Self::Zero,
            n if n < MIN_GP => Self::BelowThreshold(n),
            n => Self::Eligible(n),
        }
    }

    pub fn gp(self) -> Option<u32> {
        match self {
            Self::BelowThreshold(n) | Self::Eligible(n) => Some(n),
            _ => None,
        }
    }

    pub fn is_eligible(self) -> bool {
        matches!(self, Self::Eligible(_))
    }
}

// ── PaceScore ────────────────────────────────────────────────────────────────

/// Pace-projected stats for a skater.
/// All values are per-82-game projections based on current season rate.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PaceScore {
    /// Points per 82 games = (goals + assists) / gp * 82
    pub pace_82: f64,
    /// Goals per 82 games = goals / gp * 82 (tiebreaker)
    pub goals_per_82: f64,
    /// Raw points this season (before projection)
    pub raw_points: u32,
    /// Games played this season
    pub gp: u32,
}

impl PaceScore {
    /// Sorting key: pace_82 desc, goals_per_82 desc, encoded in one f64.
    /// Encoding: pace_82 + goals_per_82 * 0.001 (goals never exceed 100)
    pub fn sort_key(self) -> f64 {
        self.pace_82 + self.goals_per_82 * 0.001
    }
}

// ── FitClass ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FitClass {
    /// Green — elite caliber for this line slot (avg ≤ own + 0.5)
    Elite,
    /// Yellow — solid, slightly above their level (avg ≤ own + 1.25)
    Solid,
    /// Blue — buried, better than their slot (avg < own - 0.75)
    Buried,
    /// Red — overextended, playing above talent (avg > own + 1.25)
    Stretch,
}

impl FitClass {
    pub fn label(self) -> &'static str {
        match self {
            Self::Elite => "Elite",
            Self::Solid => "Solid",
            Self::Buried => "Buried",
            Self::Stretch => "Stretch",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Elite => "★",
            Self::Solid => "~",
            Self::Buried => "↑",
            Self::Stretch => "↓",
        }
    }
}

// ── Region ────────────────────────────────────────────────────────────────────

/// Geographic region grouping for player nationality analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Region {
    NorthAmerica,  // CAN + USA
    Scandinavia,   // SWE + FIN + NOR + DEN
    CentralEurope, // CZE + SVK + AUT + SUI + DEU
    Russia,        // RUS
    Other,
}

impl Region {
    pub fn from_country(country: &str) -> Self {
        match country {
            "CAN" | "USA" => Self::NorthAmerica,
            "SWE" | "FIN" | "NOR" | "DEN" => Self::Scandinavia,
            "CZE" | "SVK" | "AUT" | "SUI" | "DEU" => Self::CentralEurope,
            "RUS" => Self::Russia,
            _ => Self::Other,
        }
    }
}

// ── Player ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    /// NHL canonical player ID (u32 fits all current IDs ~6000–9000000)
    pub nhl_id: Option<u32>,
    pub full_name: String,
    /// Pre-normalized name for matching (lowercase, diacritics stripped)
    pub name_normalized: String,
    pub team: TeamAbbr,
    /// Primary position from NHL API positionCode
    pub position: Position,
    /// All positions eligible for (from Yahoo CSV or boxscore aggregation)
    pub eligible_pos: Vec<Position>,
    pub gp_status: GpStatus,

    // ── All-situations scoring ────────────────────────────────────────────────
    pub season_goals: u32,
    pub season_assists: u32,
    pub season_points: u32,
    /// Pace projection — None if GpStatus is not Eligible
    pub pace_score: Option<PaceScore>,

    // ── Power play ────────────────────────────────────────────────────────────
    pub pp_goals: u32,
    pub pp_points: u32,   // includes pp_goals; pp_assists = pp_points - pp_goals

    // ── Shorthanded ──────────────────────────────────────────────────────────
    pub sh_goals: u32,
    pub sh_points: u32,

    // ── Other scoring ─────────────────────────────────────────────────────────
    pub gwg: u32,         // game-winning goals
    pub ot_goals: u32,    // overtime goals

    // ── Shot metrics ─────────────────────────────────────────────────────────
    pub shots: u32,
    pub shooting_pct: Option<f32>,  // null for 0 shots

    // ── Two-way / ice time ────────────────────────────────────────────────────
    pub plus_minus: i32,
    /// Average TOI per game in seconds (NHL API timeOnIcePerGame)
    pub toi_per_game_sec: Option<f32>,
    /// Faceoff win percentage — None for non-centers
    pub faceoff_win_pct: Option<f32>,

    // ── Physical / two-way stats (NHL realtime API) ───────────────────────────
    pub hits: u32,
    pub blocked_shots: u32,   // shots this player blocked
    pub missed_shots: u32,    // shots this player took that missed
    pub giveaways: u32,
    pub takeaways: u32,
    pub pim: u32,             // penalty minutes

    // ── MoneyPuck advanced metrics (None if not fetched with `icelines fetch moneypuck`) ──
    pub xg: Option<f32>,          // individual expected goals (all situations)
    pub xg_per_60: Option<f32>,   // ixG per 60 minutes of ice time
    pub cf_pct_5v5: Option<f32>,  // Corsi For % at 5v5 (0–100)
    pub ff_pct_5v5: Option<f32>,  // Fenwick For % at 5v5 (0–100)
    pub xgf_pct_5v5: Option<f32>, // Expected Goals For % at 5v5 (0–100)

    // ── Headshot / display ────────────────────────────────────────────────────
    pub headshot_url: Option<String>,
    pub sweater_number: Option<u32>,

    // ── Bio / demographics ────────────────────────────────────────────────────
    pub birth_date: Option<String>,              // "YYYY-MM-DD"
    pub birth_country: Option<String>,           // ISO-3166 alpha-3
    pub nationality_code: Option<String>,        // ISO alpha-3
    pub birth_city: Option<String>,
    pub birth_state_province: Option<String>,    // province/state code
    pub shoots_catches: Option<String>,          // "L" or "R"
    pub height_in_inches: Option<u32>,
    pub weight_lbs: Option<u32>,

    // ── Draft / career ────────────────────────────────────────────────────────
    pub draft_year: Option<u16>,
    pub draft_round: Option<u8>,
    pub draft_overall: Option<u16>,
    pub rookie_season: Option<u32>, // first NHL season (YYYYZZZZ format)

    // ── Contract (from NHL landing API) ───────────────────────────────────────
    /// Year the contract expires (e.g. 2027). None if not fetched or unavailable.
    pub contract_expiry_year: Option<u16>,
    /// Contract type: "UFA", "RFA", "ELC", etc. None if not fetched or unavailable.
    pub expiry_type: Option<String>,
    /// Current-season cap hit / salary in dollars. None if not fetched or unavailable.
    pub salary: Option<u64>,
}

// ── Goalie (Phase G.1) ────────────────────────────────────────────────────────
//
// Separate type from `Player` per the goalies spec: the schema doesn't
// share enough with skaters to justify polymorphism, and the type
// system prevents skater-only ops on goalies.

/// One goalie's row in the rendered TUI / CLI views. Stats are
/// optional — a roster goalie who hasn't played yet has `stats: None`
/// and just shows their bio block.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goalie {
    pub nhl_id:           u32,
    pub full_name:        String,
    pub name_normalized:  String,
    pub team:             TeamAbbr,
    pub stats:            Option<GoalieSeasonStats>,
    pub bio:              GoalieBio,
    pub headshot_url:     Option<String>,
    pub sweater_number:   Option<u32>,
}

/// Per-season counting + rate stats for one goalie. Mirrors
/// `icelines_fetch::schema::GoalieStats` minus the metadata fields
/// (player_id, name, team, season_id) that live on the `Goalie` itself.
/// Era-typical nulls preserved as `Option`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieSeasonStats {
    pub games_played:         u32,
    pub games_started:        u32,
    pub wins:                 u32,
    pub losses:               u32,
    pub ot_losses:            Option<u32>,
    pub ties:                 Option<u32>,
    pub shots_against:        u32,
    pub goals_against:        u32,
    pub saves:                u32,
    pub save_pct:             Option<f32>,
    pub goals_against_average: Option<f32>,
    pub shutouts:             u32,
    /// Time on ice in seconds.
    pub time_on_ice:          u32,
}

/// Demographic / draft / bio data for one goalie. Same fields as the
/// skater `Player` bio block where they overlap; `catches` replaces
/// `shoots_catches` since goalies' handedness is glove-side.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalieBio {
    pub birth_date:        Option<String>,
    pub birth_country:     Option<String>,
    pub nationality_code:  Option<String>,
    pub catches:           Option<String>,        // "L" | "R"
    pub height_in_inches:  Option<u32>,
    pub weight_lbs:        Option<u32>,
    pub draft_year:        Option<u16>,
    pub draft_round:       Option<u8>,
    pub draft_overall:     Option<u16>,
    pub rookie_season:     Option<u32>,
}

impl Goalie {
    /// True iff this goalie has played enough games to qualify for the
    /// position-rank section of the TUI panel. Default 15 GP (NHL.com
    /// leaderboard convention).
    pub fn qualified(&self, min_gp: u32) -> bool {
        self.stats.as_ref().map(|s| s.games_played >= min_gp).unwrap_or(false)
    }
}

impl Player {
    pub fn gp(&self) -> Option<u32> {
        self.gp_status.gp()
    }

    pub fn is_rankable(&self) -> bool {
        self.pace_score.is_some()
    }

    pub fn pp_assists(&self) -> u32 {
        self.pp_points.saturating_sub(self.pp_goals)
    }

    pub fn pp_points_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.pp_points as f64 / gp * 82.0)
    }

    pub fn pp_goals_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.pp_goals as f64 / gp * 82.0)
    }

    pub fn sh_goals_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.sh_goals as f64 / gp * 82.0)
    }

    pub fn gwg_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.gwg as f64 / gp * 82.0)
    }

    pub fn shots_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.shots as f64 / gp * 82.0)
    }

    /// TOI per game formatted as "MM:SS", or None if unavailable.
    pub fn toi_mmss(&self) -> Option<String> {
        let sec = self.toi_per_game_sec? as u32;
        Some(format!("{:02}:{:02}", sec / 60, sec % 60))
    }

    pub fn hits_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.hits as f64 / gp * 82.0)
    }

    pub fn blocked_shots_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.blocked_shots as f64 / gp * 82.0)
    }

    pub fn takeaways_per_82(&self) -> Option<f64> {
        let gp = self.gp()? as f64;
        Some(self.takeaways as f64 / gp * 82.0)
    }

    /// Returns true if this player is on an unrestricted free-agent contract.
    pub fn is_ufa(&self) -> bool {
        self.expiry_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("UFA"))
            .unwrap_or(false)
    }

    /// Returns true if this player is on a restricted free-agent contract.
    pub fn is_rfa(&self) -> bool {
        self.expiry_type
            .as_deref()
            .map(|t| t.eq_ignore_ascii_case("RFA"))
            .unwrap_or(false)
    }

    /// Returns the number of contract seasons remaining relative to `current_season_end_year`.
    /// Returns None if expiry year is not known.
    pub fn seasons_remaining(&self, current_season_end_year: u16) -> Option<i32> {
        self.contract_expiry_year
            .map(|exp| exp as i32 - current_season_end_year as i32)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a Player with ALL current fields populated so tests can mutate specific fields.
    fn make_test_player() -> Player {
        Player {
            nhl_id: Some(8478402),
            full_name: "Test Player".to_owned(),
            name_normalized: "test player".to_owned(),
            team: TeamAbbr("EDM".to_owned()),
            position: Position::Center,
            eligible_pos: vec![Position::Center],
            gp_status: GpStatus::Eligible(60),
            season_goals: 20,
            season_assists: 30,
            season_points: 50,
            pace_score: Some(PaceScore {
                pace_82: (50.0 / 60.0) * 82.0,
                goals_per_82: (20.0 / 60.0) * 82.0,
                raw_points: 50,
                gp: 60,
            }),
            pp_goals: 10,
            pp_points: 18,  // 10 goals + 8 assists
            sh_goals: 2,
            sh_points: 3,
            gwg: 4,
            ot_goals: 1,
            shots: 180,
            shooting_pct: Some(0.111),
            plus_minus: 12,
            toi_per_game_sec: Some(1220.0),  // 20:20
            faceoff_win_pct: Some(0.52),
            hits: 100,
            blocked_shots: 50,
            missed_shots: 30,
            giveaways: 20,
            takeaways: 40,
            pim: 24,
            xg: Some(18.5),
            xg_per_60: Some(2.3),
            cf_pct_5v5: Some(55.0),
            ff_pct_5v5: Some(54.5),
            xgf_pct_5v5: Some(57.0),
            headshot_url: None,
            sweater_number: Some(97),
            birth_date: Some("1997-01-13".to_owned()),
            birth_country: Some("CAN".to_owned()),
            nationality_code: Some("CAN".to_owned()),
            birth_city: Some("Edmonton".to_owned()),
            birth_state_province: Some("AB".to_owned()),
            shoots_catches: Some("L".to_owned()),
            height_in_inches: Some(73),
            weight_lbs: Some(193),
            draft_year: Some(2015),
            draft_round: Some(1),
            draft_overall: Some(1),
            rookie_season: Some(20152016),
            contract_expiry_year: None,
            expiry_type: None,
            salary: None,
        }
    }

    // ── GpStatus ──────────────────────────────────────────────────────────────

    #[test]
    fn l0_player_gp_returns_some_when_eligible() {
        let p = make_test_player();
        assert_eq!(p.gp(), Some(60));
    }

    #[test]
    fn l0_player_gp_returns_none_when_unfetched() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.gp(), None);
    }

    #[test]
    fn l0_player_gp_returns_none_when_zero() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Zero;
        assert_eq!(p.gp(), None);
    }

    #[test]
    fn l0_player_gp_returns_some_when_below_threshold() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::BelowThreshold(5);
        assert_eq!(p.gp(), Some(5));
    }

    // ── is_ufa / is_rfa ───────────────────────────────────────────────────────

    #[test]
    fn l0_player_is_ufa_true() {
        let mut p = make_test_player();
        p.expiry_type = Some("UFA".to_owned());
        assert!(p.is_ufa());
    }

    #[test]
    fn l0_player_is_ufa_case_insensitive() {
        let mut p = make_test_player();
        p.expiry_type = Some("ufa".to_owned());
        assert!(p.is_ufa());
    }

    #[test]
    fn l0_player_is_ufa_false_when_rfa() {
        let mut p = make_test_player();
        p.expiry_type = Some("RFA".to_owned());
        assert!(!p.is_ufa());
    }

    #[test]
    fn l0_player_is_ufa_false_when_none() {
        let p = make_test_player();
        assert!(!p.is_ufa());
    }

    #[test]
    fn l0_player_is_rfa_true() {
        let mut p = make_test_player();
        p.expiry_type = Some("RFA".to_owned());
        assert!(p.is_rfa());
    }

    #[test]
    fn l0_player_is_rfa_case_insensitive() {
        let mut p = make_test_player();
        p.expiry_type = Some("rfa".to_owned());
        assert!(p.is_rfa());
    }

    #[test]
    fn l0_player_is_rfa_false_when_ufa() {
        let mut p = make_test_player();
        p.expiry_type = Some("UFA".to_owned());
        assert!(!p.is_rfa());
    }

    #[test]
    fn l0_player_is_rfa_false_when_none() {
        let p = make_test_player();
        assert!(!p.is_rfa());
    }

    // ── seasons_remaining ─────────────────────────────────────────────────────

    #[test]
    fn l0_player_seasons_remaining_positive() {
        let mut p = make_test_player();
        p.contract_expiry_year = Some(2028);
        assert_eq!(p.seasons_remaining(2026), Some(2));
    }

    #[test]
    fn l0_player_seasons_remaining_zero() {
        let mut p = make_test_player();
        p.contract_expiry_year = Some(2026);
        assert_eq!(p.seasons_remaining(2026), Some(0));
    }

    #[test]
    fn l0_player_seasons_remaining_negative() {
        let mut p = make_test_player();
        p.contract_expiry_year = Some(2024);
        assert_eq!(p.seasons_remaining(2026), Some(-2));
    }

    #[test]
    fn l0_player_seasons_remaining_none_when_no_expiry() {
        let p = make_test_player();
        assert_eq!(p.seasons_remaining(2026), None);
    }

    // ── toi_mmss ──────────────────────────────────────────────────────────────

    #[test]
    fn l0_player_toi_mmss_rounds_correctly() {
        let p = make_test_player();
        // 1220s = 20 min 20 sec
        assert_eq!(p.toi_mmss(), Some("20:20".to_owned()));
    }

    #[test]
    fn l0_player_toi_mmss_none_when_no_toi() {
        let mut p = make_test_player();
        p.toi_per_game_sec = None;
        assert_eq!(p.toi_mmss(), None);
    }

    #[test]
    fn l0_player_toi_mmss_zero() {
        let mut p = make_test_player();
        p.toi_per_game_sec = Some(0.0);
        assert_eq!(p.toi_mmss(), Some("00:00".to_owned()));
    }

    #[test]
    fn l0_player_toi_mmss_exactly_one_minute() {
        let mut p = make_test_player();
        p.toi_per_game_sec = Some(60.0);
        assert_eq!(p.toi_mmss(), Some("01:00".to_owned()));
    }

    // ── pp_assists ────────────────────────────────────────────────────────────

    #[test]
    fn l0_player_pp_assists_subtracts_pp_goals() {
        let p = make_test_player();
        // pp_points=18, pp_goals=10 → pp_assists=8
        assert_eq!(p.pp_assists(), 8);
    }

    #[test]
    fn l0_player_pp_assists_zero_when_no_pp_points() {
        let mut p = make_test_player();
        p.pp_goals = 0;
        p.pp_points = 0;
        assert_eq!(p.pp_assists(), 0);
    }

    #[test]
    fn l0_player_pp_assists_saturating_sub() {
        // If pp_goals > pp_points (shouldn't happen but must not panic)
        let mut p = make_test_player();
        p.pp_goals = 5;
        p.pp_points = 3;
        assert_eq!(p.pp_assists(), 0); // saturating_sub
    }

    // ── hits_per_82 ───────────────────────────────────────────────────────────

    #[test]
    fn l0_player_hits_per_82_with_gp() {
        let p = make_test_player();
        // hits=100, gp=60 → 100/60*82 ≈ 136.67
        let expected = 100.0f64 / 60.0 * 82.0;
        let actual = p.hits_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01, "expected {expected:.2}, got {actual:.2}");
    }

    #[test]
    fn l0_player_hits_per_82_none_when_gp_zero() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Zero;
        assert_eq!(p.hits_per_82(), None);
    }

    #[test]
    fn l0_player_hits_per_82_none_when_unfetched() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.hits_per_82(), None);
    }

    // ── blocked_shots_per_82 ─────────────────────────────────────────────────

    #[test]
    fn l0_player_blocked_shots_per_82_with_gp() {
        let p = make_test_player();
        // blocked_shots=50, gp=60 → 50/60*82 ≈ 68.33
        let expected = 50.0f64 / 60.0 * 82.0;
        let actual = p.blocked_shots_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01);
    }

    #[test]
    fn l0_player_blocked_shots_per_82_none_when_no_gp() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.blocked_shots_per_82(), None);
    }

    // ── takeaways_per_82 ─────────────────────────────────────────────────────

    #[test]
    fn l0_player_takeaways_per_82_with_gp() {
        let p = make_test_player();
        // takeaways=40, gp=60 → 40/60*82 ≈ 54.67
        let expected = 40.0f64 / 60.0 * 82.0;
        let actual = p.takeaways_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01);
    }

    #[test]
    fn l0_player_takeaways_per_82_none_when_no_gp() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.takeaways_per_82(), None);
    }

    // ── pp_points_per_82 ─────────────────────────────────────────────────────

    #[test]
    fn l0_player_pp_points_per_82() {
        let p = make_test_player();
        // pp_points=18, gp=60 → 18/60*82 = 24.6
        let expected = 18.0f64 / 60.0 * 82.0;
        let actual = p.pp_points_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01, "expected {expected:.2}, got {actual:.2}");
    }

    #[test]
    fn l0_player_pp_points_per_82_none_when_no_gp() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.pp_points_per_82(), None);
    }

    // ── sh_goals_per_82 ──────────────────────────────────────────────────────

    #[test]
    fn l0_player_sh_goals_per_82() {
        let p = make_test_player();
        // sh_goals=2, gp=60 → 2/60*82 ≈ 2.73
        let expected = 2.0f64 / 60.0 * 82.0;
        let actual = p.sh_goals_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01);
    }

    #[test]
    fn l0_player_sh_goals_per_82_none_when_no_gp() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.sh_goals_per_82(), None);
    }

    // ── gwg_per_82 ───────────────────────────────────────────────────────────

    #[test]
    fn l0_player_gwg_per_82() {
        let p = make_test_player();
        // gwg=4, gp=60 → 4/60*82 ≈ 5.47
        let expected = 4.0f64 / 60.0 * 82.0;
        let actual = p.gwg_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01);
    }

    #[test]
    fn l0_player_gwg_per_82_none_when_no_gp() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.gwg_per_82(), None);
    }

    // ── shots_per_82 ─────────────────────────────────────────────────────────

    #[test]
    fn l0_player_shots_per_82() {
        let p = make_test_player();
        // shots=180, gp=60 → 180/60*82 = 246.0
        let expected = 180.0f64 / 60.0 * 82.0;
        let actual = p.shots_per_82().unwrap();
        assert!((actual - expected).abs() < 0.01);
    }

    #[test]
    fn l0_player_shots_per_82_none_when_no_gp() {
        let mut p = make_test_player();
        p.gp_status = GpStatus::Unfetched;
        assert_eq!(p.shots_per_82(), None);
    }

    // ── is_rankable ───────────────────────────────────────────────────────────

    #[test]
    fn l0_player_is_rankable_true_with_pace_score() {
        let p = make_test_player();
        assert!(p.is_rankable());
    }

    #[test]
    fn l0_player_is_rankable_false_without_pace_score() {
        let mut p = make_test_player();
        p.pace_score = None;
        assert!(!p.is_rankable());
    }

    // ── GpStatus helpers ──────────────────────────────────────────────────────

    #[test]
    fn l0_gp_status_from_gp_zero() {
        assert_eq!(GpStatus::from_gp(0), GpStatus::Zero);
    }

    #[test]
    fn l0_gp_status_from_gp_below_threshold() {
        assert_eq!(GpStatus::from_gp(5), GpStatus::BelowThreshold(5));
    }

    #[test]
    fn l0_gp_status_from_gp_eligible() {
        assert_eq!(GpStatus::from_gp(10), GpStatus::Eligible(10));
        assert_eq!(GpStatus::from_gp(82), GpStatus::Eligible(82));
    }

    #[test]
    fn l0_gp_status_is_eligible() {
        assert!(GpStatus::Eligible(60).is_eligible());
        assert!(!GpStatus::BelowThreshold(5).is_eligible());
        assert!(!GpStatus::Zero.is_eligible());
        assert!(!GpStatus::Unfetched.is_eligible());
    }

    // ── Region ────────────────────────────────────────────────────────────────

    #[test]
    fn l0_region_north_america() {
        assert_eq!(Region::from_country("CAN"), Region::NorthAmerica);
        assert_eq!(Region::from_country("USA"), Region::NorthAmerica);
    }

    #[test]
    fn l0_region_scandinavia() {
        assert_eq!(Region::from_country("SWE"), Region::Scandinavia);
        assert_eq!(Region::from_country("FIN"), Region::Scandinavia);
    }

    #[test]
    fn l0_region_russia() {
        assert_eq!(Region::from_country("RUS"), Region::Russia);
    }

    #[test]
    fn l0_region_other() {
        assert_eq!(Region::from_country("AUS"), Region::Other);
    }

    // ── Position ──────────────────────────────────────────────────────────────

    #[test]
    fn l0_position_is_forward() {
        assert!(Position::Center.is_forward());
        assert!(Position::LeftWing.is_forward());
        assert!(Position::RightWing.is_forward());
        assert!(!Position::Defense.is_forward());
        assert!(!Position::Goalie.is_forward());
    }

    #[test]
    fn l0_position_is_defense() {
        assert!(Position::Defense.is_defense());
        assert!(!Position::Center.is_defense());
    }

    #[test]
    fn l0_position_from_api_code() {
        assert_eq!(Position::from_api_code("C"), Some(Position::Center));
        assert_eq!(Position::from_api_code("L"), Some(Position::LeftWing));
        assert_eq!(Position::from_api_code("R"), Some(Position::RightWing));
        assert_eq!(Position::from_api_code("D"), Some(Position::Defense));
        assert_eq!(Position::from_api_code("G"), Some(Position::Goalie));
        assert_eq!(Position::from_api_code("X"), None);
    }

    #[test]
    fn l0_position_abbreviation() {
        assert_eq!(Position::Center.abbreviation(), "C");
        assert_eq!(Position::LeftWing.abbreviation(), "LW");
        assert_eq!(Position::Defense.abbreviation(), "D");
    }

    // ── PaceScore sort_key ────────────────────────────────────────────────────

    #[test]
    fn l0_pace_score_sort_key_orders_by_pace_then_goals() {
        let a = PaceScore { pace_82: 100.0, goals_per_82: 50.0, raw_points: 100, gp: 82 };
        let b = PaceScore { pace_82: 100.0, goals_per_82: 40.0, raw_points: 100, gp: 82 };
        // Same pace, higher goals → higher sort_key
        assert!(a.sort_key() > b.sort_key());
    }

    #[test]
    fn l0_pace_score_sort_key_higher_pace_wins() {
        let a = PaceScore { pace_82: 120.0, goals_per_82: 10.0, raw_points: 120, gp: 82 };
        let b = PaceScore { pace_82: 100.0, goals_per_82: 50.0, raw_points: 100, gp: 82 };
        // Higher pace wins even with fewer goals
        assert!(a.sort_key() > b.sort_key());
    }

    // ── Season display ────────────────────────────────────────────────────────

    #[test]
    fn l0_season_display_formats_correctly() {
        let s = Season(20252026);
        assert_eq!(s.to_string(), "2025-26");
    }

    #[test]
    fn l0_season_as_str() {
        let s = Season(20252026);
        assert_eq!(s.as_str(), "20252026");
    }

    /// Phase Hart.0 verification: serde derives on a one-field tuple
    /// struct (`Season(pub u32)`) emit the inner value bare — same as
    /// `#[serde(transparent)]`. This test pins the current behavior;
    /// when Hart.1 adds the explicit attribute it should be a no-op.
    /// If this test ever fails, that's the canary that something
    /// non-trivial about serde-derive behavior changed.
    #[test]
    fn l0_hart0_season_serde_emits_bare_number() {
        let s = Season(20252026);
        assert_eq!(
            serde_json::to_string(&s).unwrap(),
            "20252026",
            "Season serializes as a bare number; Hart.1 will lock this \
             with `#[serde(transparent)]` as a no-op stamp.",
        );
    }

    // ── FitClass ──────────────────────────────────────────────────────────────

    #[test]
    fn l0_fit_class_labels() {
        assert_eq!(FitClass::Elite.label(), "Elite");
        assert_eq!(FitClass::Solid.label(), "Solid");
        assert_eq!(FitClass::Buried.label(), "Buried");
        assert_eq!(FitClass::Stretch.label(), "Stretch");
    }
}

// ── DepthChart ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineAssignment {
    pub line: u8,
    pub slot: Slot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Slot {
    LeftWing,
    Center,
    RightWing,
    Defense,
}

/// A depth chart for one NHL team.
/// forward_lines: 4 rows × 3 slots (LW=0, C=1, RW=2), None = unfilled.
/// defense_pairs: 3 rows × 2 slots, None = unfilled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthChart {
    pub team: TeamAbbr,
    pub season: Season,
    pub forward_lines: Vec<[Option<Player>; 3]>, // 4 rows
    pub defense_pairs: Vec<[Option<Player>; 2]>, // 3 rows
    pub unplaced: Vec<Player>,
    pub below_min_gp: Vec<Player>,
}
