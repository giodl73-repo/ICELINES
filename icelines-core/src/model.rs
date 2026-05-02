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

// Hart.5c.7.10: the legacy flat `Player` and `Goalie` structs (and the
// model-local `GoalieSeasonStats` / `GoalieBio` blocks they composed) have
// been deleted. The post-Hart canonical types live in:
//   - `crate::stats_repository::PlayerView<'_>` for read access
//   - `crate::identity::PlayerIdentity` + `crate::season_stats::SeasonStats`
//     for owned data
//   - `crate::season_stats::GoalieSeasonStats` for goalie totals
// The CLI/TUI/site/HTTP surfaces all read through `PlayerView`.
//
// Most legacy `Player::*` helpers (gp / pp_assists / per_82 ratios /
// toi_mmss / is_rankable) ported to `PlayerView` and are covered in
// `stats_repository.rs`'s view test mod. A few legacy helpers with no
// post-Hart caller went away entirely — `takeaways_per_82`, `is_ufa`,
// `is_rfa`, `seasons_remaining`. Reintroduce them on `PlayerView` if a
// new caller emerges.

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // Hart.5c.7.10: `make_test_player` and 36 `l0_player_*` tests deleted
    // alongside the legacy `Player` struct. Equivalent helpers
    // (gp / pp_assists / per_82 ratios / is_rankable / TOI formatting / UFA
    // / RFA / seasons_remaining) now live on `PlayerView<'_>` and are
    // exercised in `stats_repository.rs` and `cross_team.rs` test mods.

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

/// One filled slot in a `DepthChart`. Owned, model-decoupled — the
/// builder copies the displayed fields out of each `PlayerView` so the
/// chart survives drops of the source `StatsRepository` (Hart.5c.1
/// design D1, Option B).
///
/// `team` is the destination team in `build_views_with_swap` and may
/// not match the underlying view's `team()` for the swap-in slot — see
/// `DepthChartBuilder::build_views_with_swap` rustdoc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthChartSlot {
    pub player_id: crate::identity::PlayerId,
    pub full_name: String,
    pub name_normalized: String,
    pub team: TeamAbbr,
    pub position: Position,
    pub pace_82: Option<f64>,
    pub goals_per_82: Option<f64>,
    pub gp: Option<u32>,
    pub headshot_canonical_url: Option<String>,
}

/// A depth chart for one NHL team.
/// forward_lines: 4 rows × 3 slots (LW=0, C=1, RW=2), None = unfilled.
/// defense_pairs: 3 rows × 2 slots, None = unfilled.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepthChart {
    pub team: TeamAbbr,
    pub season: Season,
    pub forward_lines: Vec<[Option<DepthChartSlot>; 3]>, // 4 rows
    pub defense_pairs: Vec<[Option<DepthChartSlot>; 2]>, // 3 rows
    pub unplaced: Vec<DepthChartSlot>,
    pub below_min_gp: Vec<DepthChartSlot>,
}
