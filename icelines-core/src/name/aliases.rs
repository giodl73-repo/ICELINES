//! Bidirectional first-name alias table for player linking.
//!
//! ESPN transactions feed and the NHL bios feed do not always agree on a
//! player's display name — ESPN tends to use the everyday name (`Mike
//! Matheson`, `Tom Wilson`, `Alex Ovechkin`) while NHL bios use the legal
//! name (`Michael Matheson`, `Thomas Wilson`, `Alexander Ovechkin`). A
//! string match between the two without alias resolution silently fails to
//! link real moves to real players.
//!
//! The table is one source of truth: each entry lists the canonical form
//! plus its known variants. Lookups go both ways via [`canonical_for`]:
//! - `canonical_for("Mike")     == "Michael"`
//! - `canonical_for("Michael")  == "Michael"`
//! - `canonical_for("Tyler")    == "Tyler"`   (passthrough — not in table)
//!
//! Add to the table when a real linker miss surfaces; do not pre-emptively
//! add aliases without evidence (the table is small for a reason — every
//! entry is a chance to wrong-link a name with the same prefix).

use std::collections::HashMap;
use std::sync::OnceLock;

/// Canonical → variants. Single source of truth; the variant→canonical
/// HashMap is built lazily from this table.
const ALIAS_TABLE: &[(&str, &[&str])] = &[
    ("Michael", &["Mike", "Mick", "Mikey"]),
    ("Thomas", &["Tom", "Tommy"]),
    ("Alexander", &["Alex", "Aleksander", "Aleksandr", "Sasha"]),
    ("Matthew", &["Matt", "Mat"]),
    ("Samuel", &["Sam", "Sammy"]),
    ("Nicholas", &["Nick", "Nico"]),
    ("Anthony", &["Tony"]),
    ("Daniel", &["Dan", "Danny"]),
    ("David", &["Dave"]),
    ("Robert", &["Rob", "Bob", "Bobby"]),
    ("William", &["Will", "Bill", "Billy"]),
    ("Joseph", &["Joe", "Joey"]),
    ("Christopher", &["Chris"]),
    ("Patrick", &["Pat"]),
    ("Andrew", &["Andy", "Drew"]),
    ("Jonathan", &["Jon", "Jonny"]),
    ("Benjamin", &["Ben", "Benny"]),
    ("Theodore", &["Teddy", "Ted"]),
    ("Edward", &["Ed", "Eddie"]),
    ("J.T.", &["JT"]),
    ("T.J.", &["TJ"]),
    ("J.J.", &["JJ"]),
    // Cyrillic-origin transliterations the NHL feeds normalize differently.
    ("Evgeny", &["Evgeni", "Yevgeni"]),
    ("Kirill", &["Kiril"]),
    ("Andrei", &["Andrey"]),
    ("Sergei", &["Sergey"]),
    ("Dmitri", &["Dmitry", "Dmitrii"]),
    ("Nikita", &["Nikitia"]),
    ("Yegor", &["Egor"]),
    ("Iurii", &["Yuri", "Yury"]),
    ("Aleksei", &["Alexei", "Alexey"]),
];

static VARIANT_TO_CANONICAL: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();

fn variant_to_canonical() -> &'static HashMap<&'static str, &'static str> {
    VARIANT_TO_CANONICAL.get_or_init(|| {
        let mut m = HashMap::new();
        for (canonical, variants) in ALIAS_TABLE {
            // Canonical points to itself so `canonical_for("Michael") == "Michael"`.
            m.insert(*canonical, *canonical);
            for v in *variants {
                m.insert(*v, *canonical);
            }
        }
        m
    })
}

/// Resolve a first-name variant to its canonical form. Unknown names pass
/// through unchanged — the table is intentionally narrow, and a missing
/// entry just means we link by exact match (after diacritic stripping).
pub fn canonical_for(name: &str) -> &str {
    variant_to_canonical().get(name).copied().unwrap_or(name)
}

/// Two names are alias-equivalent if they normalize to the same canonical.
/// Diacritic stripping is the caller's responsibility — pass already-normalized
/// inputs in.
pub fn equivalent(a: &str, b: &str) -> bool {
    canonical_for(a) == canonical_for(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_alias_mike_matches_michael() {
        assert_eq!(canonical_for("Mike"), "Michael");
        assert_eq!(canonical_for("Michael"), "Michael");
    }

    #[test]
    fn l0_alias_alex_matches_aleksander_transliteration() {
        // ESPN spells him "Alex Ovechkin"; NHL bios spell him "Alexander
        // Ovechkin"; some Russian-language sources spell him "Aleksander".
        assert_eq!(canonical_for("Alex"), "Alexander");
        assert_eq!(canonical_for("Aleksander"), "Alexander");
        assert_eq!(canonical_for("Alexander"), "Alexander");
        assert!(
            equivalent("Alex", "Aleksander"),
            "Alex and Aleksander must be alias-equivalent"
        );
    }

    #[test]
    fn l0_alias_tom_matches_thomas() {
        assert!(equivalent("Tom", "Thomas"));
        assert!(equivalent("Tommy", "Thomas"));
    }

    #[test]
    fn l0_alias_evgeni_matches_evgeny() {
        // Malkin appears as both "Evgeni" and "Evgeny" across feeds.
        assert!(equivalent("Evgeni", "Evgeny"));
    }

    #[test]
    fn l0_alias_unknown_passthrough() {
        assert_eq!(canonical_for("Tyler"), "Tyler");
        assert_eq!(canonical_for("Connor"), "Connor");
        assert_eq!(canonical_for(""), "");
    }

    #[test]
    fn l0_alias_jt_matches_dotted_form() {
        // Some feeds write "JT Compher", others "J.T. Compher". The alias
        // table normalizes to the dotted (NHL bios) form.
        assert_eq!(canonical_for("JT"), "J.T.");
        assert!(equivalent("JT", "J.T."));
    }

    #[test]
    fn l0_alias_no_collision_between_canonicals() {
        // No variant maps to two canonicals (the table cannot have a name
        // listed under two parents — would be ambiguous).
        let mut seen = std::collections::HashMap::new();
        for (canonical, variants) in ALIAS_TABLE {
            for v in *variants {
                if let Some(prior) = seen.insert(*v, *canonical) {
                    panic!("variant '{v}' is listed under both '{prior}' and '{canonical}'");
                }
            }
        }
    }
}
