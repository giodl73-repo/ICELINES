//! CSV-export dialects for `icelines scheme from-csv` (Phase 8f.7).
//!
//! Different fantasy services export rosters / scoring summaries with
//! different column conventions. This module captures the four largest
//! services in pure data:
//!
//! - **Yahoo**: `G (P)`, `A (P)`, `(P)` and `(G)` suffixes per stat
//! - **ESPN**: bare `G`, `A`, `+/-`, `PIM` columns; signature is the
//!   `RANK,PLAYER,TEAM,POS` quad up front
//! - **Sleeper**: `G`, `A`, `PTS`, `+/-`, `PPP`, `SHP`, `SOG`, etc.;
//!   signature column `Roster ID` or `On Roster Of`
//! - **Fantrax**: `Fantrax-style` columns — `G`, `A`, `Pts`, `PPG`, `PPA`
//!   plus `Tm`, `Pos`, `Status` quad
//!
//! Detection strategy: every dialect declares a list of "signature" column
//! tokens. The detector picks the dialect with the most signature matches
//! against the header line; ties break in declaration order (Yahoo first
//! for back-compat with the original Phase 5 implementation).
//!
//! When detection is forced via `--platform`, the matching dialect is used
//! verbatim regardless of header content.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Yahoo,
    Espn,
    Sleeper,
    Fantrax,
}

impl Platform {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "yahoo"   => Some(Self::Yahoo),
            "espn"    => Some(Self::Espn),
            "sleeper" => Some(Self::Sleeper),
            "fantrax" => Some(Self::Fantrax),
            _         => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Yahoo   => "Yahoo",
            Self::Espn    => "ESPN",
            Self::Sleeper => "Sleeper",
            Self::Fantrax => "Fantrax",
        }
    }
}

/// One platform's CSV dialect — signature tokens for auto-detection plus the
/// column → normalized-stat-key mapping that drives template generation.
pub struct Dialect {
    pub platform:    Platform,
    /// Columns whose presence in the header strongly indicates this platform.
    /// Detection counts how many of these appear and picks the leader.
    pub signatures:  &'static [&'static str],
    /// `(column_name, normalized_stat_key)` pairs. The normalized keys match
    /// `icelines_core::scheme::SkaterWeights` / `GoalieWeights` field names
    /// so a generated template lines up with the Scheme struct.
    pub stat_cols:   &'static [(&'static str, &'static str)],
}

/// All known dialects, in detection priority order.
pub const ALL_DIALECTS: &[Dialect] = &[
    // Yahoo — original supported format. Distinctive `(P)` / `(G)` suffixes.
    Dialect {
        platform:   Platform::Yahoo,
        signatures: &["(P)", "(G)", "Owner", "Fan Pts"],
        stat_cols: &[
            ("G (P)",   "goals"),
            ("A (P)",   "assists"),
            ("PPG (P)", "pp_goals"),
            ("PPA (P)", "pp_assists"),
            ("SHG (P)", "sh_goals"),
            ("SHA (P)", "sh_assists"),
            ("GWG (P)", "gwg"),
            ("HIT (P)", "hits"),
            ("BLK (P)", "blocks"),
            ("SOG (P)", "shots_on_goal"),
            ("+/- (P)", "plus_minus"),
            ("PIM (P)", "pim"),
            ("FOW (P)", "faceoff_wins"),
            // Goalie
            ("W (G)",   "goalie_wins"),
            ("L (G)",   "goalie_losses"),
            ("GA (G)",  "goalie_ga"),
            ("SV (G)",  "goalie_saves"),
            ("SHO (G)", "goalie_shutouts"),
            ("SV% (G)", "goalie_save_pct"),
        ],
    },
    // ESPN — bare column names; signature is the canonical RANK-PLAYER-TEAM-POS
    // quad followed by the season totals.
    Dialect {
        platform:   Platform::Espn,
        signatures: &["RANK", "OWNER", "PLAYER", "TYPE", "STATUS"],
        stat_cols: &[
            ("G",     "goals"),
            ("A",     "assists"),
            ("PPG",   "pp_goals"),
            ("PPA",   "pp_assists"),
            ("SHG",   "sh_goals"),
            ("GWG",   "gwg"),
            ("SOG",   "shots_on_goal"),
            ("+/-",   "plus_minus"),
            ("PIM",   "pim"),
            ("HIT",   "hits"),
            ("BLK",   "blocks"),
            ("FW",    "faceoff_wins"),
            // Goalie
            ("W",     "goalie_wins"),
            ("L",     "goalie_losses"),
            ("GAA",   "goalie_gaa"),
            ("SV",    "goalie_saves"),
            ("SO",    "goalie_shutouts"),
            ("SV%",   "goalie_save_pct"),
        ],
    },
    // Sleeper — `Roster ID` / `On Roster Of` are unique identifiers.
    Dialect {
        platform:   Platform::Sleeper,
        signatures: &["Roster ID", "On Roster Of", "Sleeper"],
        stat_cols: &[
            ("G",   "goals"),
            ("A",   "assists"),
            ("PPP", "pp_points"),
            ("SHP", "sh_points"),
            ("PTS", "points"),
            ("+/-", "plus_minus"),
            ("SOG", "shots_on_goal"),
            ("PIM", "pim"),
            ("HIT", "hits"),
            ("BLK", "blocks"),
            ("FOW", "faceoff_wins"),
            // Goalie
            ("W",   "goalie_wins"),
            ("L",   "goalie_losses"),
            ("GA",  "goalie_ga"),
            ("SV",  "goalie_saves"),
            ("SO",  "goalie_shutouts"),
        ],
    },
    // Fantrax — `Fantrax`, `FPts`, `FP/G` are distinctive.
    Dialect {
        platform:   Platform::Fantrax,
        signatures: &["Fantrax", "FPts", "FP/G", "Status"],
        stat_cols: &[
            ("G",   "goals"),
            ("A",   "assists"),
            ("Pts", "points"),
            ("PPG", "pp_goals"),
            ("PPA", "pp_assists"),
            ("SHG", "sh_goals"),
            ("SHA", "sh_assists"),
            ("GWG", "gwg"),
            ("OTG", "ot_goals"),
            ("SOG", "shots_on_goal"),
            ("HT",  "hits"),
            ("BLK", "blocks"),
            ("PIM", "pim"),
            ("FW",  "faceoff_wins"),
            // Goalie
            ("W",   "goalie_wins"),
            ("L",   "goalie_losses"),
            ("GA",  "goalie_ga"),
            ("SV",  "goalie_saves"),
            ("SHO", "goalie_shutouts"),
        ],
    },
];

/// Pick the dialect whose signature columns best match `header`. Ties break
/// in declaration order (Yahoo wins ties — preserves existing behavior on
/// known-Yahoo CSVs that happen to share generic columns with other dialects).
///
/// Returns `None` only when no signature matches at all — meaning the header
/// has no recognizable platform marker. The caller should treat that as
/// "unknown format" and either error or fall back to a manual `--platform`.
pub fn detect_platform(header: &str) -> Option<&'static Dialect> {
    let mut best: Option<(&Dialect, usize)> = None;
    for d in ALL_DIALECTS {
        let hits = d.signatures.iter()
            .filter(|s| header_contains_token(header, s))
            .count();
        if hits == 0 { continue; }
        match best {
            Some((_, prev)) if prev >= hits => {} // earlier dialect wins ties
            _ => best = Some((d, hits)),
        }
    }
    best.map(|(d, _)| d)
}

/// Look up a dialect by explicit platform selector. Distinct from `detect_*`
/// so callers can short-circuit auto-detection when `--platform` is set.
pub fn dialect_for(platform: Platform) -> &'static Dialect {
    ALL_DIALECTS.iter()
        .find(|d| d.platform == platform)
        .expect("every Platform variant has a corresponding dialect entry")
}

/// Apply a dialect to a header, returning the list of `(column, stat_key)`
/// pairs whose `column` is present. The order matches the dialect's `stat_cols`
/// declaration (which is curated to be the most user-friendly listing).
pub fn matched_stats<'d>(d: &'d Dialect, header: &str) -> Vec<&'d (&'static str, &'static str)> {
    d.stat_cols.iter()
        .filter(|(col, _)| header_contains_token(header, col))
        .collect()
}

/// Substring match against a CSV header — but bounded to whole-token hits so
/// `"G"` doesn't match `"GAA"` or `"GP"`. Tokens are delimited by `,` or
/// whitespace; this is good enough for fantasy CSV headers (which never
/// quote stat columns).
fn header_contains_token(header: &str, token: &str) -> bool {
    // Multi-character tokens that are themselves substrings of the header
    // (e.g. `(P)`, `Roster ID`) only need plain contains.
    if token.contains(' ') || token.contains('(') || token.len() >= 4 {
        return header.contains(token);
    }
    // For short alphanumeric tokens, walk comma/whitespace boundaries.
    header.split(|c: char| c == ',' || c.is_whitespace())
        .any(|tok| tok == token)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Yahoo ─────────────────────────────────────────────────────────────────

    const YAHOO_HEADER: &str =
        "Player,Owner,GP,G (P),A (P),PPG (P),PPA (P),HIT (P),BLK (P),Fan Pts";

    #[test]
    fn l0_detect_yahoo_from_signature_columns() {
        let d = detect_platform(YAHOO_HEADER).expect("yahoo detected");
        assert_eq!(d.platform, Platform::Yahoo);
    }

    #[test]
    fn l0_yahoo_matched_stats_includes_goals_assists_hits() {
        let d = dialect_for(Platform::Yahoo);
        let stats = matched_stats(d, YAHOO_HEADER);
        let keys: Vec<&str> = stats.iter().map(|(_, k)| *k).collect();
        for expected in ["goals", "assists", "pp_goals", "hits", "blocks"] {
            assert!(keys.contains(&expected),
                "yahoo header should match {expected}, got: {keys:?}");
        }
    }

    // ── ESPN ──────────────────────────────────────────────────────────────────

    const ESPN_HEADER: &str =
        "RANK,PLAYER,TEAM,POS,STATUS,OWNER,G,A,+/-,PIM,SOG,HIT,BLK,FW";

    #[test]
    fn l0_detect_espn_from_signature_columns() {
        let d = detect_platform(ESPN_HEADER).expect("espn detected");
        assert_eq!(d.platform, Platform::Espn);
    }

    #[test]
    fn l0_espn_matched_stats_handles_bare_column_names() {
        let d = dialect_for(Platform::Espn);
        let stats = matched_stats(d, ESPN_HEADER);
        let keys: Vec<&str> = stats.iter().map(|(_, k)| *k).collect();
        for expected in ["goals", "assists", "plus_minus", "pim", "shots_on_goal", "hits", "blocks"] {
            assert!(keys.contains(&expected),
                "espn header should match {expected}, got: {keys:?}");
        }
    }

    // ── Sleeper ───────────────────────────────────────────────────────────────

    const SLEEPER_HEADER: &str =
        "Player,Pos,Team,Roster ID,G,A,PTS,PPP,SHP,SOG,PIM,HIT,BLK";

    #[test]
    fn l0_detect_sleeper_from_roster_id_signature() {
        let d = detect_platform(SLEEPER_HEADER).expect("sleeper detected");
        assert_eq!(d.platform, Platform::Sleeper);
    }

    #[test]
    fn l0_sleeper_matched_stats_includes_pp_points() {
        let d = dialect_for(Platform::Sleeper);
        let stats = matched_stats(d, SLEEPER_HEADER);
        let keys: Vec<&str> = stats.iter().map(|(_, k)| *k).collect();
        assert!(keys.contains(&"pp_points"),
            "PPP should map to pp_points, got: {keys:?}");
    }

    // ── Fantrax ───────────────────────────────────────────────────────────────

    const FANTRAX_HEADER: &str =
        "Player,Tm,Pos,Status,Fantrax ID,FPts,FP/G,G,A,PPG,PPA,SHG,GWG,SOG,HT,BLK";

    #[test]
    fn l0_detect_fantrax_from_fpts_and_status() {
        let d = detect_platform(FANTRAX_HEADER).expect("fantrax detected");
        assert_eq!(d.platform, Platform::Fantrax);
    }

    #[test]
    fn l0_fantrax_matched_stats_uses_ht_for_hits() {
        let d = dialect_for(Platform::Fantrax);
        let stats = matched_stats(d, FANTRAX_HEADER);
        let keys: Vec<&str> = stats.iter().map(|(_, k)| *k).collect();
        // Fantrax uses `HT` not `HIT` for hits — the dialect must respect that.
        assert!(keys.contains(&"hits"),
            "fantrax HT should map to hits, got: {keys:?}");
    }

    // ── Detection edge cases ─────────────────────────────────────────────────

    #[test]
    fn l0_detect_unknown_format_returns_none() {
        let header = "Name,Score,Date,Result"; // not a fantasy export
        assert!(detect_platform(header).is_none());
    }

    #[test]
    fn l0_token_match_does_not_confuse_g_with_gaa() {
        // A header with GAA should NOT match the bare "G" signature.
        let header_gaa_only = "GAA,SV%,SO";
        // None of the dialects have only those signatures — but we want to
        // verify the underlying token boundary check.
        assert!(!header_contains_token(header_gaa_only, "G"));
        assert!(header_contains_token(header_gaa_only, "GAA"));
    }

    #[test]
    fn l0_platform_parse_case_insensitive() {
        assert_eq!(Platform::parse("yahoo"),   Some(Platform::Yahoo));
        assert_eq!(Platform::parse("ESPN"),    Some(Platform::Espn));
        assert_eq!(Platform::parse("Sleeper"), Some(Platform::Sleeper));
        assert_eq!(Platform::parse("FaNtRaX"), Some(Platform::Fantrax));
        assert_eq!(Platform::parse("draftkings"), None);
    }
}
