//! Phase T.1 — fixture-driven classifier verification.
//!
//! Loads `tests/fixtures/espn_descriptions.json` and asserts:
//!  - every fixture description classifies to its expected kind, and
//!  - the overall `other_rate` over the fixture is below 5%.
//!
//! These tests are the regression anchor for `CURRENT_CLASSIFIER_VERSION = 1`.
//! When a future PR bumps the classifier, this file stays as-is and the
//! pre-bump kind expectations stay locked. Add new patterns to the
//! fixture as ESPN reveals them; do not replace existing entries.

use icelines_core::transactions::{classify, other_rate, TransactionKind};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Fixture {
    items: Vec<FixtureItem>,
}

#[derive(Debug, Deserialize)]
struct FixtureItem {
    kind:        String,
    description: String,
}

fn load_fixture() -> Fixture {
    let raw = include_str!("fixtures/espn_descriptions.json");
    serde_json::from_str(raw).expect("fixture must parse")
}

fn parse_kind(label: &str) -> TransactionKind {
    match label {
        "Trade"            => TransactionKind::Trade,
        "Signing"          => TransactionKind::Signing,
        "Recall"           => TransactionKind::Recall,
        "Reassignment"     => TransactionKind::Reassignment,
        "WaiverPlacement"  => TransactionKind::WaiverPlacement,
        "WaiverClear"      => TransactionKind::WaiverClear,
        "WaiverClaim"      => TransactionKind::WaiverClaim,
        "InjuryReserve"    => TransactionKind::InjuryReserve,
        "Other"            => TransactionKind::Other,
        bad => panic!("fixture has unknown kind label: {bad}"),
    }
}

#[test]
fn l1_fixture_every_description_classifies_to_expected_kind() {
    let fixture = load_fixture();
    assert!(fixture.items.len() >= 30,
        "fixture must carry ≥30 strings (we have {}); add real ESPN strings, not synthetic", fixture.items.len());

    let mut failures: Vec<String> = Vec::new();
    for item in &fixture.items {
        let expected = parse_kind(&item.kind);
        let actual   = classify(&item.description);
        if actual != expected {
            failures.push(format!(
                "  description: {:?}\n    expected: {:?}\n    actual:   {:?}",
                item.description, expected, actual,
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} fixture descriptions classified incorrectly:\n{}",
        failures.len(), fixture.items.len(), failures.join("\n"),
    );
}

#[test]
fn l1_fixture_other_rate_is_under_5_percent_excluding_intentional_other_rows() {
    // The fixture intentionally includes `Other`-labeled negative cases
    // (PTO, rights, intl loan) that *should* classify Other. The
    // observability rule applies only to rows that aren't intentional
    // Others — those represent "real" transactions where Other is a
    // misclassification. If real-row other_rate creeps above 5%, ESPN
    // has changed prose and the regex set needs an update.
    let fixture = load_fixture();
    let real_descriptions: Vec<&str> = fixture.items.iter()
        .filter(|item| item.kind != "Other")
        .map(|item| item.description.as_str())
        .collect();

    let rate = other_rate(real_descriptions.iter().copied());
    assert!(
        rate < 0.05,
        "other_rate over real rows is {:.2}% (>5% threshold) — \
         ESPN prose has likely drifted; add fixtures for the new patterns \
         or update the classifier regex set",
        rate * 100.0,
    );
}

#[test]
fn l1_fixture_intentional_other_rows_actually_classify_other() {
    // Symmetric check: every fixture row labeled `Other` must classify
    // Other. If a future regex change accidentally captures one of the
    // negative cases (PTO / rights / intl loan), this fails before ship.
    let fixture = load_fixture();
    for item in &fixture.items {
        if item.kind == "Other" {
            assert_eq!(
                classify(&item.description),
                TransactionKind::Other,
                "intentional Other case classified as something else: {:?}",
                item.description,
            );
        }
    }
}

#[test]
fn l1_fixture_covers_every_kind() {
    // Every TransactionKind must appear at least once. Ensures we don't
    // ship a kind variant that never has fixture coverage.
    use std::collections::HashSet;
    let fixture = load_fixture();
    let labels: HashSet<&str> = fixture.items.iter().map(|i| i.kind.as_str()).collect();
    for k in TransactionKind::ALL {
        let label = match k {
            TransactionKind::Trade            => "Trade",
            TransactionKind::Signing          => "Signing",
            TransactionKind::Recall           => "Recall",
            TransactionKind::Reassignment     => "Reassignment",
            TransactionKind::WaiverPlacement  => "WaiverPlacement",
            TransactionKind::WaiverClear      => "WaiverClear",
            TransactionKind::WaiverClaim      => "WaiverClaim",
            TransactionKind::InjuryReserve    => "InjuryReserve",
            TransactionKind::Other            => "Other",
        };
        assert!(labels.contains(label),
            "fixture has no rows for kind {label}; add at least one");
    }
}
