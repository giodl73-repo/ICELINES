use serde::{Deserialize, Serialize};

pub const MIN_GP: u32 = 10;

// ── Season ────────────────────────────────────────────────────────────────────

/// 8-digit YYYYZZZZ season identifier (e.g. Season(20252026)).
/// Newtype prevents silent confusion with 4-digit year u32 values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Season(pub u32);

impl Season {
    pub fn as_str(self) -> String { self.0.to_string() }
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamAbbr(pub String);

impl TeamAbbr {
    pub fn as_str(&self) -> &str { &self.0 }
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

    pub fn is_defense(self) -> bool { matches!(self, Self::Defense) }

    /// Parse from NHL API positionCode (L, R, C, D, G)
    pub fn from_api_code(code: &str) -> Option<Self> {
        match code {
            "C"              => Some(Self::Center),
            "L"              => Some(Self::LeftWing),
            "R"              => Some(Self::RightWing),
            "D"              => Some(Self::Defense),
            "G"              => Some(Self::Goalie),
            _                => None,
        }
    }

    pub fn abbreviation(self) -> &'static str {
        match self {
            Self::Center    => "C",
            Self::LeftWing  => "LW",
            Self::RightWing => "RW",
            Self::Defense   => "D",
            Self::Goalie    => "G",
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

    pub fn is_eligible(self) -> bool { matches!(self, Self::Eligible(_)) }
}

// ── PaceScore ────────────────────────────────────────────────────────────────

/// Pace-projected stats for a skater.
/// All values are per-82-game projections based on current season rate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PaceScore {
    /// Points per 82 games = (goals + assists) / gp * 82
    pub pace_82:      f64,
    /// Goals per 82 games = goals / gp * 82 (tiebreaker)
    pub goals_per_82: f64,
    /// Raw points this season (before projection)
    pub raw_points:   u32,
    /// Games played this season
    pub gp:           u32,
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
            Self::Elite   => "Elite",
            Self::Solid   => "Solid",
            Self::Buried  => "Buried",
            Self::Stretch => "Stretch",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::Elite   => "★",
            Self::Solid   => "~",
            Self::Buried  => "↑",
            Self::Stretch => "↓",
        }
    }
}

// ── Player ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    /// NHL canonical player ID (u32 fits all current IDs ~6000–9000000)
    pub nhl_id:          Option<u32>,
    pub full_name:       String,
    /// Pre-normalized name for matching (lowercase, diacritics stripped)
    pub name_normalized: String,
    pub team:            TeamAbbr,
    /// Primary position from NHL API positionCode
    pub position:        Position,
    /// All positions eligible for (from Yahoo CSV or boxscore aggregation)
    pub eligible_pos:    Vec<Position>,
    pub gp_status:       GpStatus,
    /// Season statistics from NHL API SkaterStats
    pub season_goals:    u32,
    pub season_assists:  u32,
    pub season_points:   u32,
    /// Pace projection — None if GpStatus is not Eligible
    pub pace_score:      Option<PaceScore>,
    /// Headshot URL from NHL roster API
    pub headshot_url:    Option<String>,
}

impl Player {
    pub fn gp(&self) -> Option<u32> { self.gp_status.gp() }

    pub fn is_rankable(&self) -> bool { self.pace_score.is_some() }
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
    pub team:          TeamAbbr,
    pub season:        Season,
    pub forward_lines: Vec<[Option<Player>; 3]>,  // 4 rows
    pub defense_pairs: Vec<[Option<Player>; 2]>,  // 3 rows
    pub unplaced:      Vec<Player>,
    pub below_min_gp:  Vec<Player>,
}
