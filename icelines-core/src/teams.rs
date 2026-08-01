use crate::{IcelinesError, TeamAbbr};

/// All current NHL franchise abbreviations in NHL API form, sorted
/// alphabetically. Provider adapters and product consumers share this list.
pub const ALL_NHL_TEAM_ABBREVIATIONS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

/// All 32 NHL team abbreviations (NHL API format) paired with full names.
pub const CANONICAL_TEAMS: &[(&str, &str)] = &[
    ("ANA", "Anaheim Ducks"),
    ("BOS", "Boston Bruins"),
    ("BUF", "Buffalo Sabres"),
    ("CAR", "Carolina Hurricanes"),
    ("CBJ", "Columbus Blue Jackets"),
    ("CGY", "Calgary Flames"),
    ("CHI", "Chicago Blackhawks"),
    ("COL", "Colorado Avalanche"),
    ("DAL", "Dallas Stars"),
    ("DET", "Detroit Red Wings"),
    ("EDM", "Edmonton Oilers"),
    ("FLA", "Florida Panthers"),
    ("LAK", "Los Angeles Kings"),
    ("MIN", "Minnesota Wild"),
    ("MTL", "Montréal Canadiens"),
    ("NJD", "New Jersey Devils"),
    ("NSH", "Nashville Predators"),
    ("NYI", "New York Islanders"),
    ("NYR", "New York Rangers"),
    ("OTT", "Ottawa Senators"),
    ("PHI", "Philadelphia Flyers"),
    ("PIT", "Pittsburgh Penguins"),
    ("SEA", "Seattle Kraken"),
    ("SJS", "San Jose Sharks"),
    ("STL", "St. Louis Blues"),
    ("TBL", "Tampa Bay Lightning"),
    ("TOR", "Toronto Maple Leafs"),
    ("UTA", "Utah Hockey Club"),
    ("VAN", "Vancouver Canucks"),
    ("VGK", "Vegas Golden Knights"),
    ("WPG", "Winnipeg Jets"),
    ("WSH", "Washington Capitals"),
];

/// Yahoo abbreviations that differ from NHL API abbreviations.
const YAHOO_TO_NHL: &[(&str, &str)] = &[("LA", "LAK"), ("NJ", "NJD"), ("TB", "TBL"), ("SJ", "SJS")];

impl TeamAbbr {
    /// Parse a team abbreviation — accepts both NHL API and Yahoo formats.
    /// Returns Err for unknown abbreviations.
    pub fn parse(s: &str) -> Result<Self, IcelinesError> {
        let upper = s.trim().to_uppercase();
        // Normalize Yahoo abbreviation to NHL format
        let normalized = YAHOO_TO_NHL
            .iter()
            .find(|(yahoo, _)| *yahoo == upper.as_str())
            .map(|(_, nhl)| *nhl)
            .unwrap_or(upper.as_str());
        if CANONICAL_TEAMS.iter().any(|(abbr, _)| *abbr == normalized) {
            Ok(Self(normalized.to_owned()))
        } else {
            Err(IcelinesError::UnknownTeam(s.to_owned()))
        }
    }

    pub fn full_name(&self) -> Option<&'static str> {
        CANONICAL_TEAMS
            .iter()
            .find(|(abbr, _)| *abbr == self.0.as_str())
            .map(|(_, name)| *name)
    }

    pub fn all() -> impl Iterator<Item = TeamAbbr> {
        CANONICAL_TEAMS
            .iter()
            .map(|(abbr, _)| TeamAbbr(abbr.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_team_parse_sea_valid() {
        assert!(TeamAbbr::parse("SEA").is_ok());
    }

    #[test]
    fn l0_team_parse_xyz_invalid() {
        assert!(TeamAbbr::parse("XYZ").is_err());
    }

    #[test]
    fn l0_team_parse_yahoo_la_normalizes_to_lak() {
        let t = TeamAbbr::parse("LA").unwrap();
        assert_eq!(t.as_str(), "LAK");
    }

    #[test]
    fn l0_team_parse_yahoo_nj_normalizes_to_njd() {
        let t = TeamAbbr::parse("NJ").unwrap();
        assert_eq!(t.as_str(), "NJD");
    }

    #[test]
    fn l0_team_all_returns_32_teams() {
        assert_eq!(TeamAbbr::all().count(), 32);
    }

    #[test]
    fn l0_abbreviation_catalog_matches_named_team_catalog() {
        let named = CANONICAL_TEAMS
            .iter()
            .map(|(abbreviation, _)| *abbreviation)
            .collect::<Vec<_>>();
        assert_eq!(named, ALL_NHL_TEAM_ABBREVIATIONS);
    }

    #[test]
    fn l0_team_full_name_sea() {
        let t = TeamAbbr::parse("SEA").unwrap();
        assert_eq!(t.full_name(), Some("Seattle Kraken"));
    }
}
