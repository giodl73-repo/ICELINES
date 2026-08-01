//! Compatibility facade for team catalogs and provider normalization.

pub use icelines_sources::teams::{espn_to_nhl_abbrev, nhl_teams_for_season, ALL_NHL_TEAMS};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn l0_all_nhl_teams_has_thirty_two_franchises() {
        assert_eq!(
            ALL_NHL_TEAMS.len(),
            32,
            "NHL has 32 franchises; list drift will break joins. Got {} entries.",
            ALL_NHL_TEAMS.len(),
        );
    }

    #[test]
    fn l0_all_nhl_teams_has_no_duplicates() {
        let set: HashSet<&str> = ALL_NHL_TEAMS.iter().copied().collect();
        assert_eq!(
            set.len(),
            ALL_NHL_TEAMS.len(),
            "ALL_NHL_TEAMS must contain unique abbrevs",
        );
    }

    #[test]
    fn l0_all_nhl_teams_sorted_alphabetically() {
        let mut sorted = ALL_NHL_TEAMS.to_vec();
        sorted.sort();
        assert_eq!(
            sorted.as_slice(),
            ALL_NHL_TEAMS,
            "list must stay sorted — drift makes diffs unreviewable",
        );
    }

    #[test]
    fn l0_season_membership_tracks_expansion_and_relocation() {
        let pre_seattle = nhl_teams_for_season("20202021");
        assert_eq!(pre_seattle.len(), 31);
        assert!(pre_seattle.contains(&"ARI"));
        assert!(!pre_seattle.contains(&"SEA"));
        assert!(!pre_seattle.contains(&"UTA"));

        let coyotes = nhl_teams_for_season("20232024");
        assert_eq!(coyotes.len(), 32);
        assert!(coyotes.contains(&"ARI"));
        assert!(coyotes.contains(&"SEA"));
        assert!(!coyotes.contains(&"UTA"));

        let utah = nhl_teams_for_season("20242025");
        assert_eq!(utah.len(), 32);
        assert!(utah.contains(&"UTA"));
        assert!(!utah.contains(&"ARI"));
    }

    #[test]
    fn l0_all_nhl_teams_uses_nhl_api_long_form() {
        // Concrete invariants around the two abbrevs that diverged
        // historically (TBL vs TB, SJS vs SJ). The NHL API uses the
        // 3-letter form for these — anything else breaks roster fetches.
        assert!(
            ALL_NHL_TEAMS.contains(&"TBL"),
            "Tampa Bay must be 'TBL', not 'TB'"
        );
        assert!(
            ALL_NHL_TEAMS.contains(&"SJS"),
            "San Jose must be 'SJS', not 'SJ'"
        );
        assert!(
            !ALL_NHL_TEAMS.contains(&"TB"),
            "'TB' is not a valid NHL API abbrev"
        );
        assert!(
            !ALL_NHL_TEAMS.contains(&"SJ"),
            "'SJ' is not a valid NHL API abbrev"
        );
    }

    #[test]
    fn l1_all_nhl_teams_matches_newest_bundled_bios() {
        // Every team in the canonical list must produce ≥ 1 player in
        // the newest completed-season bundled bios. A zero count means the
        // canonical abbrev disagrees with what the NHL emits and any
        // Home → Team navigation for that team will look empty.
        let newest_bundle = crate::bundled::BUNDLED_SEASONS[0];
        let bios = crate::bundled::get_bios(newest_bundle)
            .expect("newest completed-season bios must be bundled");
        let teams_in_bios: HashSet<String> = bios
            .iter()
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
    fn l1_all_nhl_teams_matches_newest_bundled_goalie_stats() {
        // Same coverage, goalie side — catches the case where bios and
        // goalie data drift apart. Mid-season trades are split with
        // commas (e.g. "EDM,PIT"), so we expand each entry.
        let newest_bundle = crate::bundled::BUNDLED_SEASONS[0];
        let goalies = crate::bundled::get_goalie_stats(newest_bundle)
            .expect("newest completed-season goalie stats must be bundled");
        let teams_in_goalies: HashSet<String> = goalies
            .iter()
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

    // ── ESPN → NHL abbrev mapper (Phase T.1) ──────────────────────────────────

    #[test]
    fn l0_espn_to_nhl_two_letter_shorthand() {
        // The four ESPN shorthand divergences (TB/SJ/NJ/LA) plus the
        // full-word UTAH form that surfaced during T.6 capture.
        assert_eq!(espn_to_nhl_abbrev("TB", "20252026"), Some("TBL"));
        assert_eq!(espn_to_nhl_abbrev("SJ", "20252026"), Some("SJS"));
        assert_eq!(espn_to_nhl_abbrev("NJ", "20252026"), Some("NJD"));
        assert_eq!(espn_to_nhl_abbrev("LA", "20252026"), Some("LAK"));
        assert_eq!(espn_to_nhl_abbrev("UTAH", "20252026"), Some("UTA"));
    }

    #[test]
    fn l0_espn_to_nhl_canonical_passthrough() {
        // Already-canonical codes round-trip unchanged.
        for &t in ALL_NHL_TEAMS {
            assert_eq!(
                espn_to_nhl_abbrev(t, "20252026"),
                Some(t),
                "canonical '{t}' must passthrough"
            );
        }
    }

    #[test]
    fn l0_espn_to_nhl_unknown_returns_none() {
        // BENCH-mandated: never silently accept an unmapped code.
        assert_eq!(espn_to_nhl_abbrev("BOGUS", "20252026"), None);
        assert_eq!(espn_to_nhl_abbrev("ZZZ", "20252026"), None);
        assert_eq!(espn_to_nhl_abbrev("", "20252026"), None);
    }

    #[test]
    fn l0_espn_to_nhl_case_insensitive_input() {
        // ESPN sometimes lowercases (rare but real).
        assert_eq!(espn_to_nhl_abbrev("edm", "20252026"), Some("EDM"));
        assert_eq!(espn_to_nhl_abbrev("tb", "20252026"), Some("TBL"));
    }

    #[test]
    fn l0_espn_to_nhl_ari_pre_relocation_preserved() {
        // 2023-24 — Coyotes still in Arizona.
        assert_eq!(espn_to_nhl_abbrev("ARI", "20232024"), Some("ARI"));
        assert_eq!(
            espn_to_nhl_abbrev("PHX", "20232024"),
            Some("ARI"),
            "legacy PHX must normalize to ARI in pre-relocation seasons"
        );
    }

    #[test]
    fn l0_espn_to_nhl_ari_post_relocation_maps_to_uta() {
        // 2024-25 onward — Coyotes are now Utah HC.
        assert_eq!(espn_to_nhl_abbrev("ARI", "20242025"), Some("UTA"));
        assert_eq!(espn_to_nhl_abbrev("ARI", "20252026"), Some("UTA"));
        assert_eq!(espn_to_nhl_abbrev("PHX", "20242025"), Some("UTA"));
    }

    #[test]
    fn l0_espn_to_nhl_atl_thrasher_era_preserved() {
        // 2010-11 — Thrashers' last season before relocating to Winnipeg.
        // We preserve ATL rather than mapping to WPG (different franchise
        // history; the post-2011 WPG is the new Jets, not the 1996 ones).
        assert_eq!(espn_to_nhl_abbrev("ATL", "20102011"), Some("ATL"));
    }

    #[test]
    fn l0_espn_to_nhl_uta_in_post_relocation_season() {
        // ESPN may emit UTA directly in 24-25 / 25-26 — must round-trip.
        assert_eq!(espn_to_nhl_abbrev("UTA", "20242025"), Some("UTA"));
        assert_eq!(espn_to_nhl_abbrev("UTA", "20252026"), Some("UTA"));
    }
}
