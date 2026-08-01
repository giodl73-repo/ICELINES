//! Provider-team normalization over the core franchise catalog.

pub use icelines_core::teams::ALL_NHL_TEAM_ABBREVIATIONS as ALL_NHL_TEAMS;

/// NHL franchise abbreviations that participated in a season.
/// Seattle joined in 2021-22 and Utah replaced Arizona in 2024-25.
pub fn nhl_teams_for_season(season: &str) -> Vec<&'static str> {
    let season = season.parse::<u32>().unwrap_or(u32::MAX);
    let mut teams = ALL_NHL_TEAMS
        .iter()
        .copied()
        .filter(|team| season >= 20212022 || *team != "SEA")
        .map(|team| {
            if season < 20242025 && team == "UTA" {
                "ARI"
            } else {
                team
            }
        })
        .collect::<Vec<_>>();
    teams.sort_unstable();
    teams
}

/// Convert an ESPN-emitted team abbreviation into canonical NHL API form for
/// the given season. Unknown values remain explicit as `None`.
pub fn espn_to_nhl_abbrev(abbrev: &str, season: &str) -> Option<&'static str> {
    let upper = abbrev.to_ascii_uppercase();
    if let Some(canonical) = match upper.as_str() {
        "TB" => Some("TBL"),
        "SJ" => Some("SJS"),
        "NJ" => Some("NJD"),
        "LA" => Some("LAK"),
        "UTAH" => Some("UTA"),
        _ => None,
    } {
        return Some(canonical);
    }

    let season_int = season.parse::<u32>().unwrap_or(0);
    match upper.as_str() {
        "ARI" | "PHX" => {
            return Some(if season_int >= 20242025 { "UTA" } else { "ARI" });
        }
        "ATL" => return Some("ATL"),
        _ => {}
    }

    ALL_NHL_TEAMS
        .iter()
        .find(|team| **team == upper.as_str())
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_membership_tracks_expansion_and_relocation() {
        assert_eq!(nhl_teams_for_season("20202021").len(), 31);
        assert!(nhl_teams_for_season("20232024").contains(&"ARI"));
        assert!(nhl_teams_for_season("20242025").contains(&"UTA"));
    }

    #[test]
    fn espn_mapping_is_season_aware_and_fail_closed() {
        assert_eq!(espn_to_nhl_abbrev("TB", "20252026"), Some("TBL"));
        assert_eq!(espn_to_nhl_abbrev("PHX", "20232024"), Some("ARI"));
        assert_eq!(espn_to_nhl_abbrev("PHX", "20242025"), Some("UTA"));
        assert_eq!(espn_to_nhl_abbrev("BOGUS", "20252026"), None);
    }
}
