//! Trade-mirror grouping.
//!
//! ESPN reports trades twice — once per team, with the team field and the
//! prose flipped:
//!   `(2026-04-29, "TBL", "Acquired D Ryan McDonagh from NSH for D Philippe Myers")`
//!   `(2026-04-29, "NSH", "Traded D Ryan McDonagh to TBL for D Philippe Myers")`
//!
//! Different `(date, team, description)` → different dedup hash → both
//! rows correctly persisted (the data is the data). The TUI / CLI then
//! group rows that share a `trade_group_id` and collapse the mirror
//! under one display row with a "(+1 mirror)" suffix.
//!
//! The id is permutation-invariant: the order in which the teams or
//! players are mentioned does not affect the hash. The same trade always
//! groups the same way no matter which mirror the loop sees first.

use sha2::{Digest, Sha256};

/// Compute a stable group id over (date, sorted teams, sorted normalized
/// players). Two mirror rows of the same trade compute identical ids.
///
/// `players` are expected to be already-normalized via
/// `icelines_core::name::normalize_name` so spelling differences across
/// rows ("Hörnqvist" vs "Hornqvist") do not split the group.
pub fn trade_group_id(date: &str, teams: &[String], players: &[String]) -> String {
    let mut t: Vec<&str> = teams.iter().map(String::as_str).collect();
    t.sort();
    let mut p: Vec<&str> = players.iter().map(String::as_str).collect();
    p.sort();

    let mut hasher = Sha256::new();
    hasher.update(date.as_bytes());
    hasher.update(b"|");
    hasher.update(t.join(",").as_bytes());
    hasher.update(b"|");
    hasher.update(p.join(",").as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn l0_trade_group_id_permutation_invariant_on_teams() {
        let date = "2026-04-29";
        let players = vec!["mcdonagh".to_owned(), "myers".to_owned()];

        let a = trade_group_id(date, &["TBL".to_owned(), "NSH".to_owned()], &players);
        let b = trade_group_id(date, &["NSH".to_owned(), "TBL".to_owned()], &players);
        assert_eq!(a, b, "team order must not affect group id");
    }

    #[test]
    fn l0_trade_group_id_permutation_invariant_on_players() {
        let date = "2026-04-29";
        let teams = vec!["TBL".to_owned(), "NSH".to_owned()];

        let a = trade_group_id(date, &teams, &["mcdonagh".to_owned(), "myers".to_owned()]);
        let b = trade_group_id(date, &teams, &["myers".to_owned(), "mcdonagh".to_owned()]);
        assert_eq!(a, b, "player order must not affect group id");
    }

    #[test]
    fn l0_trade_group_id_changes_with_date() {
        let teams = vec!["TBL".to_owned(), "NSH".to_owned()];
        let players = vec!["mcdonagh".to_owned()];

        let a = trade_group_id("2026-04-29", &teams, &players);
        let b = trade_group_id("2026-04-30", &teams, &players);
        assert_ne!(a, b, "different dates must produce different ids");
    }

    #[test]
    fn l0_trade_group_id_changes_with_players() {
        let date = "2026-04-29";
        let teams = vec!["TBL".to_owned(), "NSH".to_owned()];

        let a = trade_group_id(date, &teams, &["mcdonagh".to_owned()]);
        let b = trade_group_id(date, &teams, &["bedard".to_owned()]);
        assert_ne!(a, b, "different players must produce different ids");
    }

    #[test]
    fn l0_trade_group_id_empty_inputs_safe() {
        // Doesn't panic; returns a deterministic hash.
        let a = trade_group_id("2026-04-29", &[], &[]);
        let b = trade_group_id("2026-04-29", &[], &[]);
        assert_eq!(a, b);
    }

    proptest! {
        #[test]
        fn l0_trade_group_id_proptest_team_permutation_invariant(
            date in "[0-9]{4}-[0-9]{2}-[0-9]{2}",
            mut teams in prop::collection::vec("[A-Z]{2,4}", 2..6),
            mut players in prop::collection::vec("[a-z]+", 1..6),
        ) {
            let a = trade_group_id(&date, &teams, &players);
            // Reverse both — id must be the same.
            teams.reverse();
            players.reverse();
            let b = trade_group_id(&date, &teams, &players);
            prop_assert_eq!(a, b);
        }

        #[test]
        fn l0_trade_group_id_proptest_no_panic(
            date in "\\PC{0,30}",
            teams in prop::collection::vec("\\PC{0,10}", 0..6),
            players in prop::collection::vec("\\PC{0,30}", 0..6),
        ) {
            let _ = trade_group_id(&date, &teams, &players);
        }
    }
}
