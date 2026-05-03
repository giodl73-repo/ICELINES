//! Search helpers — pure functions, no I/O, deterministic.
//!
//! Two surfaces:
//! - [`description_matches_query`]: case-insensitive, NFD-stripped substring
//!   match used by the TUI `/` search bar and the CLI `--player` filter.
//! - [`transactions_for_player`]: the player-card join — given a player's
//!   full name, returns every transaction whose sanitized description
//!   contains the player's last name (NFD-stripped, case-insensitive).
//!
//! Why last-name match instead of full-name fuzzy: ESPN abbreviates first
//! names erratically ("Mike" vs "Michael", "JT" vs "J.T."), but last
//! names are stable. False positives are bounded — if two players on the
//! same league share a last name (Sebastian Aho the F vs the D, the
//! Hughes brothers, etc.), the caller can disambiguate by team.

use crate::name::normalize_name;
use crate::Transaction;

/// Case-insensitive substring match. Both the description and the query
/// are NFD-stripped via `normalize_name`, so "Hörnqvist" matches a query
/// of "hornqvist" and vice versa.
pub fn description_matches_query(description: &str, query: &str) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    let nd = normalize_name(description);
    let nq = normalize_name(query);
    nd.contains(&nq)
}

/// Return every transaction whose description references the given
/// player's last name. `full_name` is anything the caller has —
/// "Connor McDavid", "C. McDavid", "McDavid" all work because we always
/// match on the last whitespace-separated token.
///
/// Optional `team_filter` reduces false positives for shared last names
/// — when set, only rows with `team == Some(team_filter)` OR with no
/// team field at all are returned.
pub fn transactions_for_player<'a>(
    transactions: &'a [Transaction],
    full_name: &str,
    team_filter: Option<&str>,
) -> Vec<&'a Transaction> {
    let last = match last_token(full_name) {
        Some(s) => s,
        None => return Vec::new(),
    };
    let last_norm = normalize_name(last);
    if last_norm.is_empty() {
        return Vec::new();
    }

    transactions
        .iter()
        .filter(|tx| {
            // Team disambiguation. Rows with no team are always considered
            // (league-wide notes shouldn't drop on team filter), but rows
            // with a different team are filtered out when we have one.
            if let Some(want) = team_filter {
                if let Some(t) = &tx.team {
                    if !t.0.eq_ignore_ascii_case(want) {
                        return false;
                    }
                }
            }
            let desc_norm = normalize_name(&tx.description);
            desc_norm.contains(&last_norm)
        })
        .collect()
}

/// Last whitespace-separated token of `name`, or None when input is empty.
fn last_token(name: &str) -> Option<&str> {
    name.split_whitespace().next_back()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::TeamAbbr;
    use crate::TransactionKind;

    fn fixture(team: Option<&str>, description: &str) -> Transaction {
        Transaction {
            date: "2026-04-29".to_owned(),
            team: team.map(|t| TeamAbbr(t.to_owned())),
            kind: TransactionKind::Trade,
            description: description.to_owned(),
            id: "id".to_owned(),
            trade_group_id: None,
            classifier_version: 1,
        }
    }

    // ── description_matches_query ────────────────────────────────────

    #[test]
    fn l0_search_empty_query_matches_everything() {
        assert!(description_matches_query("anything", ""));
        assert!(description_matches_query("anything", "   "));
    }

    #[test]
    fn l0_search_substring_match_case_insensitive() {
        assert!(description_matches_query(
            "Acquired D Ryan McDonagh from NSH",
            "mcdonagh"
        ));
        assert!(description_matches_query(
            "Acquired D Ryan McDonagh from NSH",
            "MCDONAGH"
        ));
        assert!(description_matches_query(
            "Acquired D Ryan McDonagh from NSH",
            "ryan mcd"
        ));
    }

    #[test]
    fn l0_search_no_match_returns_false() {
        assert!(!description_matches_query(
            "Recalled F X from AHL",
            "tarasenko"
        ));
    }

    #[test]
    fn l0_search_diacritic_stripped_both_sides() {
        // ESPN sometimes drops diacritics, sometimes preserves. The
        // query must match either spelling.
        assert!(description_matches_query(
            "Signed F Patric Hörnqvist",
            "hornqvist"
        ));
        assert!(description_matches_query(
            "Signed F Patric Hornqvist",
            "hörnqvist"
        ));
    }

    // ── transactions_for_player ─────────────────────────────────────

    #[test]
    fn l0_player_match_finds_by_last_name() {
        let txs = vec![
            fixture(Some("EDM"), "Recalled F Vasily Podkolzin from Bakersfield"),
            fixture(Some("CHI"), "Signed F Connor Bedard to an extension"),
            fixture(Some("EDM"), "Acquired D X from NSH"),
        ];
        let hits = transactions_for_player(&txs, "Vasily Podkolzin", None);
        assert_eq!(hits.len(), 1);
        assert!(hits[0].description.contains("Podkolzin"));
    }

    #[test]
    fn l0_player_match_works_with_only_last_name() {
        let txs = vec![fixture(
            Some("CHI"),
            "Signed F Connor Bedard to an extension",
        )];
        let hits = transactions_for_player(&txs, "Bedard", None);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn l0_player_match_team_filter_drops_other_teams() {
        // Two players named Aho: F on CAR, D on NYI. Without team filter
        // both rows return — caller must pass team to disambiguate.
        let txs = vec![
            fixture(Some("CAR"), "Recalled F Sebastian Aho from AHL"),
            fixture(Some("NYI"), "Acquired D Sebastian Aho from CAR"),
        ];
        let car_only = transactions_for_player(&txs, "Sebastian Aho", Some("CAR"));
        assert_eq!(car_only.len(), 1);
        assert_eq!(car_only[0].team.as_ref().unwrap().0, "CAR");
    }

    #[test]
    fn l0_player_match_team_filter_passes_teamless_rows() {
        // League-wide rows (team=None) shouldn't be dropped by team
        // filtering — they're broadcast and any player linked to one
        // legitimately surfaces.
        let txs = vec![
            fixture(Some("EDM"), "Signed F McDavid to an extension"),
            fixture(None, "League-wide: McDavid named to All-Star Team"),
            fixture(Some("CHI"), "Acquired F McDavid from BOS"), // hypothetical
        ];
        let edm_only = transactions_for_player(&txs, "Connor McDavid", Some("EDM"));
        // EDM row + league-wide row, NOT the CHI row.
        assert_eq!(edm_only.len(), 2);
    }

    #[test]
    fn l0_player_match_empty_name_returns_empty() {
        let txs = vec![fixture(Some("EDM"), "Anything")];
        assert!(transactions_for_player(&txs, "", None).is_empty());
        assert!(transactions_for_player(&txs, "   ", None).is_empty());
    }

    #[test]
    fn l0_player_match_diacritic_normalized_both_sides() {
        // ESPN: "Hornqvist". Player struct: "Hörnqvist". Match must work.
        let txs = vec![fixture(
            Some("NSH"),
            "Signed F Patric Hornqvist to a 1-year deal",
        )];
        let hits = transactions_for_player(&txs, "Patric Hörnqvist", None);
        assert_eq!(hits.len(), 1);
    }

    #[test]
    fn l0_player_match_no_match_returns_empty() {
        let txs = vec![fixture(Some("EDM"), "Recalled F X from AHL")];
        let hits = transactions_for_player(&txs, "Tarasenko", None);
        assert!(hits.is_empty());
    }

    #[test]
    fn l0_last_token_handles_single_name() {
        assert_eq!(last_token("Bedard"), Some("Bedard"));
        assert_eq!(last_token("Connor McDavid"), Some("McDavid"));
        assert_eq!(last_token("J.T. Compher"), Some("Compher"));
        assert_eq!(last_token(""), None);
        assert_eq!(last_token("   "), None);
    }
}
