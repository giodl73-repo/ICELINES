//! Phase Calder — multi-league career history.
//!
//! The NHL `/v1/player/{id}/landing` endpoint returns a `seasonTotals`
//! array covering every league a player has touched: NHL, AHL,
//! OHL/QMJHL/WHL, NCAA, KHL, SHL, Liiga, junior leagues like USHL /
//! NAHL / CSSHL / GTHL, plus international tournaments (WJC, WC, OG).
//!
//! This module owns the types that carry that data through the rest of
//! the codebase. Distinct from `history::SeasonLine` (NHL-only career
//! summary used by `scouting`/`career`) — the schema here has to span
//! pro / junior / college / international shapes, so most fields are
//! `Option<T>`.

use crate::model::Season;
use serde::{Deserialize, Serialize};

/// League code as it appears in the NHL landing response.
///
/// Kept as a freeform string newtype because the value space is huge
/// (NHL, AHL, OHL, WHL, QMJHL, NCAA, H-East, ECAC, NAHL, USHL,
/// J20 Nationell, KHL, SHL, Liiga, MHL, WJC-A, WJC-18, WC, OG,
/// GTHL U16, CSSHL U18, "Other", …). An enum would need either an
/// `Other(String)` escape hatch — defeating the purpose — or constant
/// churn as new tournaments appear.
///
/// Use `tier()` to bucket into Pro / Junior / College / International
/// when filtering or grouping.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LeagueAbbrev(pub String);

impl LeagueAbbrev {
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Bucket the league into a coarse tier for filtering.
    ///
    /// Returns `LeagueTier::Other` for anything we don't explicitly
    /// recognize — better to under-classify a brand-new league than to
    /// silently drop it.
    pub fn tier(&self) -> LeagueTier {
        let u = self.0.to_ascii_uppercase();
        // International tournaments first — many include hyphens that
        // would otherwise match other prefixes.
        if u.starts_with("WJC")
            || u.starts_with("WC")
            || u == "OG"
            || u.starts_with("OGC")
            || u == "WHC-17"
            || u == "WJ18-A"
            || u == "4 NATIONS"
            || u == "INTERNATIONAL"
        {
            return LeagueTier::International;
        }
        match u.as_str() {
            "NHL" | "AHL" | "ECHL" | "KHL" | "SHL" | "LIIGA" | "DEL" | "NL" | "EXTRA"
            | "EXTRALIGA" | "CZECHIA" | "SLOVAKIA" | "MESTIS" | "ALLSVENSKAN" => LeagueTier::Pro,
            "OHL" | "QMJHL" | "WHL" | "USHL" | "NAHL" | "MHL" | "J20 NATIONELL" => {
                LeagueTier::Junior
            }
            // NCAA + its conferences (H-East, ECAC, Big Ten, NCHC,
            // CCHA, Atlantic Hockey, USHS-MI for prep).
            "NCAA" | "H-EAST" | "ECAC" | "BIG TEN" | "NCHC" | "CCHA" | "ATLANTIC HOCKEY"
            | "USHS-MI" => LeagueTier::College,
            _ => LeagueTier::Other,
        }
    }
}

impl std::fmt::Display for LeagueAbbrev {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Coarse classification used for filtering and grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LeagueTier {
    /// NHL / AHL / KHL / SHL / Liiga / DEL etc.
    Pro,
    /// Major junior (CHL — OHL/WHL/QMJHL), USHL, NAHL, MHL, J20.
    Junior,
    /// NCAA conferences.
    College,
    /// IIHF events — WJC, WC, OG, 4 Nations.
    International,
    /// AAA/U16/U18 minor + anything we haven't classified yet.
    Other,
}

/// Game type — matches the NHL API `gameTypeId` (2 = regular, 3 =
/// playoff). Re-using `season_stats::SeasonType` would couple
/// pre-NHL data to the NHL stats axis; this stays separate so a
/// tournament with `gameTypeId=2` (group stage) doesn't get treated
/// as an NHL "regular season".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CareerGameType {
    Regular,
    Playoff,
}

impl CareerGameType {
    pub fn from_api_id(id: u32) -> Option<Self> {
        match id {
            2 => Some(Self::Regular),
            3 => Some(Self::Playoff),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Playoff => "playoff",
        }
    }
}

/// One stint = one (player, season, league, team, game_type) row.
///
/// A player can have multiple stints in one season (e.g., minor
/// hockey + a trial assignment + a youth tournament). The
/// `sequence` field preserves the API's display order so we can
/// render stints stably.
///
/// All stat fields are `Option<u32>` / `Option<f32>` because pre-NHL
/// leagues drop fields the NHL exposes (no `avg_toi` in OHL, no
/// shooting % in GTHL) and goalie stints overlap heavily with skater
/// stints in field set. The renderer decides what to show based on
/// the player's position and the league's typical schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerStint {
    pub season: Season,
    pub league: LeagueAbbrev,
    pub team: String,
    pub game_type: CareerGameType,
    pub sequence: u8,
    pub gp: u32,

    // ── Skater fields ────────────────────────────────────────────────
    pub goals: Option<u32>,
    pub assists: Option<u32>,
    pub points: Option<u32>,
    pub pim: Option<u32>,
    pub plus_minus: Option<i32>,
    pub power_play_goals: Option<u32>,
    pub power_play_points: Option<u32>,
    pub shorthanded_goals: Option<u32>,
    pub shorthanded_points: Option<u32>,
    pub game_winning_goals: Option<u32>,
    pub ot_goals: Option<u32>,
    pub shots: Option<u32>,
    pub shooting_pct: Option<f32>,
    pub avg_toi_sec: Option<u32>,
    pub faceoff_win_pct: Option<f32>,

    // ── Goalie fields ────────────────────────────────────────────────
    pub games_started: Option<u32>,
    pub wins: Option<u32>,
    pub losses: Option<u32>,
    pub ot_losses: Option<u32>,
    pub goals_against: Option<u32>,
    pub goals_against_avg: Option<f32>,
    pub save_pct: Option<f32>,
    pub shots_against: Option<u32>,
    pub shutouts: Option<u32>,
    pub time_on_ice_sec: Option<u32>,
}

impl CareerStint {
    pub fn points_per_game(&self) -> Option<f32> {
        let p = self.points?;
        if self.gp == 0 {
            None
        } else {
            Some(p as f32 / self.gp as f32)
        }
    }
}

/// Everything the landing endpoint tells us about one player's
/// career across every league. Sorted oldest → newest by season,
/// then by sequence.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CareerHistory {
    pub player_id: u32,
    pub stints: Vec<CareerStint>,
}

impl CareerHistory {
    /// Sort stints in display order (oldest first, then sequence
    /// ascending). Idempotent.
    pub fn sort_for_display(&mut self) {
        self.stints
            .sort_by_key(|s| (s.season.0, s.sequence, s.game_type as u8));
    }

    /// Stints filtered to one tier — handy for the "pre-NHL" arc on
    /// the player card, or "show me only the OHL years".
    pub fn by_tier(&self, tier: LeagueTier) -> impl Iterator<Item = &CareerStint> {
        self.stints.iter().filter(move |s| s.league.tier() == tier)
    }

    /// Stints filtered to one league literal.
    pub fn by_league(&self, league: &str) -> impl Iterator<Item = &CareerStint> + '_ {
        let needle = league.to_owned();
        self.stints.iter().filter(move |s| s.league.0 == needle)
    }

    /// Convenience: every distinct league the player has appeared in,
    /// in chronological order of first appearance.
    pub fn leagues_in_order(&self) -> Vec<&LeagueAbbrev> {
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut out: Vec<&LeagueAbbrev> = Vec::new();
        for s in &self.stints {
            if seen.insert(s.league.0.as_str()) {
                out.push(&s.league);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_tier_classifies_known_pro_leagues() {
        assert_eq!(LeagueAbbrev::new("NHL").tier(), LeagueTier::Pro);
        assert_eq!(LeagueAbbrev::new("AHL").tier(), LeagueTier::Pro);
        assert_eq!(LeagueAbbrev::new("KHL").tier(), LeagueTier::Pro);
        assert_eq!(LeagueAbbrev::new("SHL").tier(), LeagueTier::Pro);
        assert_eq!(LeagueAbbrev::new("Liiga").tier(), LeagueTier::Pro);
    }

    #[test]
    fn l0_tier_classifies_junior() {
        assert_eq!(LeagueAbbrev::new("OHL").tier(), LeagueTier::Junior);
        assert_eq!(LeagueAbbrev::new("WHL").tier(), LeagueTier::Junior);
        assert_eq!(LeagueAbbrev::new("QMJHL").tier(), LeagueTier::Junior);
        assert_eq!(LeagueAbbrev::new("USHL").tier(), LeagueTier::Junior);
        assert_eq!(
            LeagueAbbrev::new("J20 Nationell").tier(),
            LeagueTier::Junior
        );
    }

    #[test]
    fn l0_tier_classifies_college() {
        assert_eq!(LeagueAbbrev::new("NCAA").tier(), LeagueTier::College);
        assert_eq!(LeagueAbbrev::new("H-East").tier(), LeagueTier::College);
        assert_eq!(LeagueAbbrev::new("ECAC").tier(), LeagueTier::College);
    }

    #[test]
    fn l0_tier_classifies_international() {
        assert_eq!(LeagueAbbrev::new("WJC-A").tier(), LeagueTier::International);
        assert_eq!(
            LeagueAbbrev::new("WJC-20").tier(),
            LeagueTier::International
        );
        assert_eq!(LeagueAbbrev::new("WC").tier(), LeagueTier::International);
        assert_eq!(LeagueAbbrev::new("OG").tier(), LeagueTier::International);
        assert_eq!(
            LeagueAbbrev::new("4 Nations").tier(),
            LeagueTier::International
        );
    }

    #[test]
    fn l0_tier_classifies_youth_as_other() {
        assert_eq!(LeagueAbbrev::new("GTHL").tier(), LeagueTier::Other);
        assert_eq!(LeagueAbbrev::new("CSSHL U18").tier(), LeagueTier::Other);
        assert_eq!(
            LeagueAbbrev::new("Brick Invitational").tier(),
            LeagueTier::Other
        );
    }

    #[test]
    fn l0_career_game_type_round_trip() {
        assert_eq!(
            CareerGameType::from_api_id(2),
            Some(CareerGameType::Regular)
        );
        assert_eq!(
            CareerGameType::from_api_id(3),
            Some(CareerGameType::Playoff)
        );
        assert_eq!(CareerGameType::from_api_id(1), None);
        assert_eq!(CareerGameType::from_api_id(99), None);
    }

    #[test]
    fn l0_career_history_sorts_oldest_first() {
        let mut h = CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20162017, "NHL", 1, CareerGameType::Regular),
                stint(20122013, "OHL", 1, CareerGameType::Regular),
                stint(20122013, "OHL", 1, CareerGameType::Playoff),
                stint(20142015, "OHL", 2, CareerGameType::Regular),
                stint(20142015, "OHL", 1, CareerGameType::Regular),
            ],
        };
        h.sort_for_display();
        let years: Vec<_> = h
            .stints
            .iter()
            .map(|s| (s.season.0, s.sequence, s.game_type))
            .collect();
        assert_eq!(
            years,
            vec![
                (20122013, 1, CareerGameType::Regular),
                (20122013, 1, CareerGameType::Playoff),
                (20142015, 1, CareerGameType::Regular),
                (20142015, 2, CareerGameType::Regular),
                (20162017, 1, CareerGameType::Regular),
            ]
        );
    }

    #[test]
    fn l0_career_history_by_tier_filters() {
        let h = CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20122013, "OHL", 1, CareerGameType::Regular),
                stint(20142015, "WJC-A", 1, CareerGameType::Regular),
                stint(20162017, "NHL", 1, CareerGameType::Regular),
            ],
        };
        let pro: Vec<_> = h.by_tier(LeagueTier::Pro).collect();
        assert_eq!(pro.len(), 1);
        assert_eq!(pro[0].league.0, "NHL");
        let junior: Vec<_> = h.by_tier(LeagueTier::Junior).collect();
        assert_eq!(junior.len(), 1);
        assert_eq!(junior[0].league.0, "OHL");
    }

    #[test]
    fn l0_leagues_in_order_dedupes_chronologically() {
        let h = CareerHistory {
            player_id: 1,
            stints: vec![
                stint(20122013, "OHL", 1, CareerGameType::Regular),
                stint(20122013, "OHL", 1, CareerGameType::Playoff),
                stint(20142015, "WJC-A", 1, CareerGameType::Regular),
                stint(20142015, "OHL", 1, CareerGameType::Regular),
                stint(20162017, "NHL", 1, CareerGameType::Regular),
            ],
        };
        let leagues: Vec<&str> = h.leagues_in_order().iter().map(|l| l.0.as_str()).collect();
        assert_eq!(leagues, vec!["OHL", "WJC-A", "NHL"]);
    }

    fn stint(season: u32, league: &str, sequence: u8, game_type: CareerGameType) -> CareerStint {
        CareerStint {
            season: Season(season),
            league: LeagueAbbrev::new(league),
            team: "Test".into(),
            game_type,
            sequence,
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
}
