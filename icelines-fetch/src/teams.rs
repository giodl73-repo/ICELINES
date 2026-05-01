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

// ── ESPN → NHL abbrev mapping (Phase T.1) ─────────────────────────────────────
//
// ESPN's transactions feed and our NHL bios feed do not always agree on
// short team codes. The two diverge in three classes:
//
// 1. **Two-letter shorthand** — `TB` / `SJ` from ESPN, `TBL` / `SJS` from NHL.
// 2. **Relocations** — Coyotes `ARI` (and earlier `PHX`) became `UTA` for
//    seasons ≥ 2024-25. ESPN may emit either depending on the row's date.
// 3. **Defunct franchises** — Atlanta `ATL` (Thrashers, pre-2011) appears
//    in any deep historical pull. We preserve it verbatim — it is NOT
//    `WPG` (which post-2011 refers to the new Jets, not the 1996 ones).
//
// The mapper is **season-aware**: an `ARI` row for 2023-24 stays as `ARI`,
// but an `ARI` row for 2024-25 maps to `UTA`. Tests cover the boundary.

/// Convert an ESPN-emitted team abbrev into our canonical NHL form for the
/// given season. Returns `None` for unknown abbrevs — callers should
/// surface the row as teamless (`LEAGUE` bucket) rather than dropping it,
/// and emit a WARN so we discover new ESPN codes early.
pub fn espn_to_nhl_abbrev(abbrev: &str, season: &str) -> Option<&'static str> {
    let upper = abbrev.to_ascii_uppercase();

    // Two-letter shorthand and other ESPN-side variants — always map
    // to the canonical NHL form regardless of season. Each entry here
    // surfaced as a real "unmapped abbrev" warning during the T.6
    // historical capture.
    if let Some(canonical) = match upper.as_str() {
        "TB"   => Some("TBL"),
        "SJ"   => Some("SJS"),
        "NJ"   => Some("NJD"),  // ESPN emits the two-letter form for NJ Devils
        "LA"   => Some("LAK"),  // ESPN emits the two-letter form for LA Kings
        "UTAH" => Some("UTA"),  // ESPN's full-word form for Utah HC
        _     => None,
    } {
        return Some(canonical);
    }

    // Coyotes / Utah relocation. ESPN may emit either name for either
    // season; we map by row season:
    //   - Pre-2024-25: ARI (and legacy PHX) preserved as ARI.
    //   - 2024-25 onward: ARI / PHX → UTA.
    let season_int = season.parse::<u32>().unwrap_or(0);
    let is_post_relocation = season_int >= 20242025;

    match upper.as_str() {
        "ARI" | "PHX" => {
            return Some(if is_post_relocation { "UTA" } else { "ARI" });
        }
        // Atlanta Thrashers (pre-2011-12). We don't carry ATL in the
        // canonical 32, but historical rows must round-trip.
        "ATL" => return Some("ATL"),
        _ => {}
    }

    // Whitelist passthrough: only when the abbrev already matches a
    // canonical entry. Refuses to silently accept anything ESPN invents.
    // We look up the &'static str rather than returning a fresh allocation.
    ALL_NHL_TEAMS.iter().find(|t| **t == upper.as_str()).copied()
}

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

    // ── ESPN → NHL abbrev mapper (Phase T.1) ──────────────────────────────────

    #[test]
    fn l0_espn_to_nhl_two_letter_shorthand() {
        // The four ESPN shorthand divergences (TB/SJ/NJ/LA) plus the
        // full-word UTAH form that surfaced during T.6 capture.
        assert_eq!(espn_to_nhl_abbrev("TB",   "20252026"), Some("TBL"));
        assert_eq!(espn_to_nhl_abbrev("SJ",   "20252026"), Some("SJS"));
        assert_eq!(espn_to_nhl_abbrev("NJ",   "20252026"), Some("NJD"));
        assert_eq!(espn_to_nhl_abbrev("LA",   "20252026"), Some("LAK"));
        assert_eq!(espn_to_nhl_abbrev("UTAH", "20252026"), Some("UTA"));
    }

    #[test]
    fn l0_espn_to_nhl_canonical_passthrough() {
        // Already-canonical codes round-trip unchanged.
        for &t in ALL_NHL_TEAMS {
            assert_eq!(espn_to_nhl_abbrev(t, "20252026"), Some(t),
                "canonical '{t}' must passthrough");
        }
    }

    #[test]
    fn l0_espn_to_nhl_unknown_returns_none() {
        // BENCH-mandated: never silently accept an unmapped code.
        assert_eq!(espn_to_nhl_abbrev("BOGUS", "20252026"), None);
        assert_eq!(espn_to_nhl_abbrev("ZZZ",   "20252026"), None);
        assert_eq!(espn_to_nhl_abbrev("",      "20252026"), None);
    }

    #[test]
    fn l0_espn_to_nhl_case_insensitive_input() {
        // ESPN sometimes lowercases (rare but real).
        assert_eq!(espn_to_nhl_abbrev("edm", "20252026"), Some("EDM"));
        assert_eq!(espn_to_nhl_abbrev("tb",  "20252026"), Some("TBL"));
    }

    #[test]
    fn l0_espn_to_nhl_ari_pre_relocation_preserved() {
        // 2023-24 — Coyotes still in Arizona.
        assert_eq!(espn_to_nhl_abbrev("ARI", "20232024"), Some("ARI"));
        assert_eq!(espn_to_nhl_abbrev("PHX", "20232024"), Some("ARI"),
            "legacy PHX must normalize to ARI in pre-relocation seasons");
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
