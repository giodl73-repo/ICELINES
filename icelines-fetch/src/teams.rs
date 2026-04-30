//! Single source of truth for the NHL team abbreviation set.
//!
//! Multiple modules used to keep their own copy of the 32-team list
//! (`repository.rs`, `goalie_repository.rs`, `nhl_api.rs`). Drift between
//! them — or between any of those and the bundled bios/goalie data — is
//! invisible at compile time and produces silent gaps at runtime
//! (e.g. an entire team appearing empty on the Home → Team flow). This
//! module owns the canonical list and is referenced by every consumer
//! so a missed update fails one place, not several.
//!
//! The list uses NHL API form (`SJS` not `SJ`, `TBL` not `TB`) — that
//! is what the live `roster/{team}/{season}` and `skater/bios` endpoints
//! emit, so it must match exactly for joins to succeed.

/// All 32 NHL franchise abbreviations in NHL API form, sorted alphabetically.
/// Use this everywhere a team-abbrev list is needed; never duplicate locally.
pub const ALL_NHL_TEAMS: &[&str] = &[
    "ANA", "BOS", "BUF", "CAR", "CBJ", "CGY", "CHI", "COL", "DAL", "DET", "EDM", "FLA", "LAK",
    "MIN", "MTL", "NJD", "NSH", "NYI", "NYR", "OTT", "PHI", "PIT", "SEA", "SJS", "STL", "TBL",
    "TOR", "UTA", "VAN", "VGK", "WPG", "WSH",
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn l0_all_nhl_teams_has_thirty_two_franchises() {
        assert_eq!(
            ALL_NHL_TEAMS.len(), 32,
            "NHL has 32 franchises; list drift will break joins. Got {} entries.",
            ALL_NHL_TEAMS.len(),
        );
    }

    #[test]
    fn l0_all_nhl_teams_has_no_duplicates() {
        let set: HashSet<&str> = ALL_NHL_TEAMS.iter().copied().collect();
        assert_eq!(
            set.len(), ALL_NHL_TEAMS.len(),
            "ALL_NHL_TEAMS must contain unique abbrevs",
        );
    }

    #[test]
    fn l0_all_nhl_teams_sorted_alphabetically() {
        let mut sorted = ALL_NHL_TEAMS.to_vec();
        sorted.sort();
        assert_eq!(
            sorted.as_slice(), ALL_NHL_TEAMS,
            "list must stay sorted — drift makes diffs unreviewable",
        );
    }

    #[test]
    fn l0_all_nhl_teams_uses_nhl_api_long_form() {
        // Concrete invariants around the two abbrevs that diverged
        // historically (TBL vs TB, SJS vs SJ). The NHL API uses the
        // 3-letter form for these — anything else breaks roster fetches.
        assert!(ALL_NHL_TEAMS.contains(&"TBL"), "Tampa Bay must be 'TBL', not 'TB'");
        assert!(ALL_NHL_TEAMS.contains(&"SJS"), "San Jose must be 'SJS', not 'SJ'");
        assert!(!ALL_NHL_TEAMS.contains(&"TB"),  "'TB' is not a valid NHL API abbrev");
        assert!(!ALL_NHL_TEAMS.contains(&"SJ"),  "'SJ' is not a valid NHL API abbrev");
    }

    #[test]
    fn l1_all_nhl_teams_matches_bundled_bios_25_26() {
        // Every team in the canonical list must produce ≥ 1 player in
        // the current-season bundled bios. A zero count means the
        // canonical abbrev disagrees with what the NHL emits and any
        // Home → Team navigation for that team will look empty.
        let bios = crate::bundled::get_bios(icelines_core::CURRENT_SEASON_STR)
            .expect("25-26 bios must be bundled");
        let teams_in_bios: HashSet<String> = bios.iter()
            .filter_map(|b| b.current_team_abbrev.clone())
            .collect();

        for &t in ALL_NHL_TEAMS {
            assert!(
                teams_in_bios.contains(t),
                "team '{t}' is in ALL_NHL_TEAMS but no bios row uses that abbrev — \
                 check the canonical list against the live NHL feed",
            );
        }

        // Also flag bios abbrevs we don't know about — catches a new
        // expansion team or a rename we haven't yet integrated.
        let canonical: HashSet<&str> = ALL_NHL_TEAMS.iter().copied().collect();
        for t in &teams_in_bios {
            assert!(
                canonical.contains(t.as_str()),
                "bios contain abbrev '{t}' which is NOT in ALL_NHL_TEAMS — \
                 the canonical list is out of date",
            );
        }
    }

    #[test]
    fn l1_all_nhl_teams_matches_bundled_goalie_stats_25_26() {
        // Same coverage, goalie side — catches the case where bios and
        // goalie data drift apart. Mid-season trades are split with
        // commas (e.g. "EDM,PIT"), so we expand each entry.
        let goalies = crate::bundled::get_goalie_stats(icelines_core::CURRENT_SEASON_STR)
            .expect("25-26 goalie-stats must be bundled");
        let teams_in_goalies: HashSet<String> = goalies.iter()
            .flat_map(|g| g.team_abbrevs.split(',').map(|s| s.trim().to_owned()))
            .filter(|s| !s.is_empty())
            .collect();

        let canonical: HashSet<&str> = ALL_NHL_TEAMS.iter().copied().collect();
        for t in &teams_in_goalies {
            assert!(
                canonical.contains(t.as_str()),
                "goalie data contains abbrev '{t}' which is NOT in \
                 ALL_NHL_TEAMS — canonical list out of date",
            );
        }
    }
}
