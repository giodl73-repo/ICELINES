//! Anchored-regex classifier for ESPN transaction descriptions.
//!
//! The substring approach considered in spec v0.1 was rejected by TAPE
//! because real ESPN prose has too many overlapping phrases:
//! - "Signed F X to a PTO"           — substring "Signed" → would mis-classify
//! - "Acquired the rights to RFA F X" — substring "Acquired" → would mis-classify
//! - "Loaned G X to Sweden for IIHF"  — substring "loaned" → would mis-classify
//!
//! Instead, every rule is an anchored case-insensitive regex evaluated in
//! priority order. **Order matters**: the `Other`-promoting negative rules
//! (PTO, rights-acquisition, international loan) are checked BEFORE the
//! broader `Signing` / `Trade` / `Reassignment` rules so a PTO never
//! classifies as Signing.
//!
//! The fallback `Other` is a real outcome — never bail when a description
//! doesn't match. Observability: T.4 will assert `other_rate < 5%` against
//! the fixture file so a future ESPN prose change ("Acquired" → "Obtained")
//! that drops trades into `Other` fails CI before it ships.

use once_cell::sync::Lazy;
use regex::Regex;

use super::TransactionKind;

/// Bumped on every change to the regex set or priority order.
/// `load_transactions_with_fallback` re-runs `classify()` on any persisted
/// row whose `classifier_version < CURRENT_CLASSIFIER_VERSION`, so bundled
/// snapshots and live data never disagree on `kind` after a rule update.
///
/// Version history:
/// - v1: initial regex set (T.1)
/// - v2: added "Agreed to terms" → Signing, "Waived X" / "on unconditional
///       waivers" → WaiverPlacement, "Singed" typo → Signing (T.6 capture
///       surfaced these in real 21-22 ESPN prose).
/// - v3: rule order changed — Trade and IR now outrank Waivers in
///       compound rows ("Placed X on IR. Designated Y for waivers."
///       is primarily an IR move; the Marchment regression). Also
///       added "Assigned X to AHL", "Reinstated X", "Sent X to AHL"
///       (current-season AHL movement verbs).
pub const CURRENT_CLASSIFIER_VERSION: u16 = 3;

/// One rule = one regex + the kind to return on a match. Compiled once
/// per process via `Lazy<Regex>` inside the rules accessor.
struct Rule {
    pattern: &'static Lazy<Regex>,
    kind:    TransactionKind,
}

// ── Compiled-once regexes ──────────────────────────────────────────────
//
// The `(?i)` prefix is case-insensitive at compile time so per-call cost
// is just a state-machine walk.
//
// Why individual `Lazy<Regex>` statics rather than a single `Lazy<Vec<Rule>>`:
// `static` items disallow interior-mutable temporaries in slice literals,
// so we can't write `static RULES: &[Rule] = &[Rule { pattern: Lazy::new(...) }, ...]`.
// One Lazy per regex is the cleanest path that keeps the rule list visibly
// ordered + allocation-free.

static RE_PTO: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\bsigned\b.*\bto an?\s+(PTO|professional[- ]?tryout|amateur[- ]?tryout|ATO)\b"
).expect("PTO regex must compile"));

static RE_RIGHTS: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\bacquired\b.*\b(the\s+)?(negotiating\s+)?rights\b"
).expect("rights regex must compile"));

static RE_INTL_LOAN: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\bloaned\b.*\b(IIHF|Olympic|World\s+(Junior|Championship)|Spengler)\b"
).expect("intl loan regex must compile"));

static RE_WAIVER_CLAIM: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\bclaimed\b.*\boff\s+waivers\b"
).expect("waiver claim regex must compile"));

static RE_WAIVER_CLEAR: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\bcleared\s+waivers\b"
).expect("waiver clear regex must compile"));

static RE_WAIVER_PLACEMENT: Lazy<Regex> = Lazy::new(|| Regex::new(
    // ESPN historically writes "Placed C X on (unconditional )?waivers"
    // and the bare "Waived RW X" verb. Both are placements.
    r"(?i)(\bplaced\b.*\bwaivers\b|\bwaived\b)"
).expect("waiver placement regex must compile"));

static RE_TRADE: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\b(traded|acquired)\b"
).expect("trade regex must compile"));

static RE_IR: Lazy<Regex> = Lazy::new(|| Regex::new(
    // "Activated F X from long-term injured reserve" — handles the
    // hyphenated long-term form between `from` and `injured reserve`.
    // "Reinstated F X" / "Reinstated F X from conditioning" — current-
    // season verb for IR / conditioning returns.
    r"(?i)(\bplaced\b.*\bon\s+(injured\s+reserve|IR\b|LTIR|long[- ]term)|\bactivated\b.*\b(injured\s+reserve|IR|LTIR)|\breinstated\b)"
).expect("IR regex must compile"));

static RE_RECALL: Lazy<Regex> = Lazy::new(|| Regex::new(
    r"(?i)\b(recalled|called\s+up)\b"
).expect("recall regex must compile"));

static RE_REASSIGN: Lazy<Regex> = Lazy::new(|| Regex::new(
    // ESPN's daily AHL movement uses bare "Assigned X to Y (AHL)" in
    // current-season feeds. "Sent X to AHL" without "down" also recurs.
    // International-loan rule fires before this so non-AHL "loaned" /
    // "sent" are pre-empted.
    r"(?i)\b(reassigned|returned|sent(\s+down)?|optioned|loaned|assigned)\b"
).expect("reassignment regex must compile"));

static RE_SIGNING: Lazy<Regex> = Lazy::new(|| Regex::new(
    // "Agreed to terms with F X on a 1-year contract" — ESPN's most
    // common prose for off-season signings; was missed entirely in v0.
    // "Singed" is a real ESPN typo that recurs ~11×/season; accept it
    // rather than lose those rows to Other.
    r"(?i)(\b(signed|re-signed|singed|extended)\b|\bagree(d)?\s+to\s+terms\b)"
).expect("signing regex must compile"));

/// Ordered rule list. Earlier rules win; `Other`-promoting negatives
/// (PTO, rights-acquisition, international loan) come first.
///
/// Compound-row resolution: ESPN often packs multiple events into one
/// description ("Placed X on injured reserve. Designated Y for waivers.
/// Recalled Z."). The order below resolves these to the primary event:
/// - IR before Waivers — Marchment-style "placed on IR + designate for
///   waivers" rows are fundamentally IR moves.
/// - Trade before everything substantive — trades with riders ("Acquired
///   D X. Recalled F Y.") are trades, period.
/// - Recall before Reassignment before Signing.
fn rules() -> [Rule; 11] {
    [
        // Negative rules (promote to Other before broader catches fire).
        Rule { pattern: &RE_PTO,        kind: TransactionKind::Other },
        Rule { pattern: &RE_RIGHTS,     kind: TransactionKind::Other },
        Rule { pattern: &RE_INTL_LOAN,  kind: TransactionKind::Other },
        // Trade outranks all daily-movement rules — a deadline trade
        // can mention recalls/IR/waivers as riders, but the primary event
        // is the trade.
        Rule { pattern: &RE_TRADE,      kind: TransactionKind::Trade },
        // IR before Waivers — "Placed X on IR. Designated Y for waivers."
        // is primarily an IR move. Real-world Marchment row.
        Rule { pattern: &RE_IR,         kind: TransactionKind::InjuryReserve },
        // Waiver kinds.
        Rule { pattern: &RE_WAIVER_CLAIM,     kind: TransactionKind::WaiverClaim },
        Rule { pattern: &RE_WAIVER_CLEAR,     kind: TransactionKind::WaiverClear },
        Rule { pattern: &RE_WAIVER_PLACEMENT, kind: TransactionKind::WaiverPlacement },
        // Daily AHL movement.
        Rule { pattern: &RE_RECALL,     kind: TransactionKind::Recall },
        Rule { pattern: &RE_REASSIGN,   kind: TransactionKind::Reassignment },
        // Signing last among substantive rules — PTO/rights are pre-empted
        // by the negative rules above.
        Rule { pattern: &RE_SIGNING,    kind: TransactionKind::Signing },
    ]
}

/// Classify an ESPN transaction description. Returns `Other` for any
/// string that doesn't match a known pattern — never panics, never bails.
pub fn classify(description: &str) -> TransactionKind {
    for rule in rules() {
        if rule.pattern.is_match(description) {
            return rule.kind;
        }
    }
    TransactionKind::Other
}

/// Compute the "other rate" (fraction of inputs that classify as `Other`)
/// over a slice of descriptions. Used by the observability test in T.1
/// (against bundled fixtures) and by T.4 (against the live captured payload).
pub fn other_rate<'a, I: IntoIterator<Item = &'a str>>(descriptions: I) -> f64 {
    let mut total = 0usize;
    let mut other = 0usize;
    for d in descriptions {
        total += 1;
        if classify(d) == TransactionKind::Other {
            other += 1;
        }
    }
    if total == 0 { 0.0 } else { other as f64 / total as f64 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use TransactionKind::*;

    // ── Example tests, one per kind, drawn from real ESPN prose ────────

    #[test]
    fn l0_classify_acquired_is_trade() {
        assert_eq!(classify("Acquired D Ryan McDonagh from NSH for D Philippe Myers"), Trade);
    }

    #[test]
    fn l0_classify_traded_is_trade() {
        assert_eq!(classify("Traded D Ryan McDonagh to TBL for D Philippe Myers"), Trade);
    }

    #[test]
    fn l0_classify_signed_to_contract_is_signing() {
        assert_eq!(classify("Signed F Connor Bedard to a 8-year, $11.4M extension"), Signing);
    }

    #[test]
    fn l0_classify_re_signed_is_signing() {
        assert_eq!(classify("Re-signed G Connor Hellebuyck to a 7-year contract"), Signing);
    }

    #[test]
    fn l0_classify_pto_is_other_not_signing() {
        // The critical TAPE-flagged miss: PTO is NOT a roster signing.
        assert_eq!(classify("Signed F Vladimir Sobotka to a PTO"), Other);
        assert_eq!(classify("Signed F X to a professional tryout"), Other);
        assert_eq!(classify("Signed F X to an amateur tryout"), Other);
    }

    #[test]
    fn l0_classify_rights_acquisition_is_other_not_trade() {
        // TAPE-flagged miss: acquiring negotiating rights is NOT a roster trade.
        assert_eq!(classify("Acquired the rights to RFA F X from BOS"), Other);
        assert_eq!(classify("Acquired negotiating rights to UFA G Y from MTL"), Other);
    }

    #[test]
    fn l0_classify_intl_loan_is_other_not_reassignment() {
        // TAPE-flagged miss: international loans are not AHL demotions.
        assert_eq!(classify("Loaned G X to Sweden for the IIHF World Championship"), Other);
        assert_eq!(classify("Loaned F Y to Czechia for the World Junior Championship"), Other);
    }

    #[test]
    fn l0_classify_claimed_off_waivers_is_claim() {
        assert_eq!(classify("Claimed F X off waivers from BOS"), WaiverClaim);
    }

    #[test]
    fn l0_classify_cleared_waivers_is_clear() {
        assert_eq!(classify("Cleared waivers; assigned to AHL"), WaiverClear);
    }

    #[test]
    fn l0_classify_placed_on_waivers_is_placement() {
        assert_eq!(classify("Placed F X on waivers"), WaiverPlacement);
    }

    #[test]
    fn l0_classify_recalled_is_recall() {
        assert_eq!(classify("Recalled F Vasily Podkolzin from Bakersfield (AHL)"), Recall);
    }

    #[test]
    fn l0_classify_emergency_recall_is_recall() {
        // EDGE-flagged: ESPN sometimes writes "emergency conditions".
        assert_eq!(classify("Recalled F X under emergency conditions"), Recall);
    }

    #[test]
    fn l0_classify_called_up_is_recall() {
        assert_eq!(classify("Called up G X from the AHL"), Recall);
    }

    #[test]
    fn l0_classify_reassigned_is_reassignment() {
        assert_eq!(classify("Reassigned F X to AHL"), Reassignment);
    }

    #[test]
    fn l0_classify_returned_to_ahl_is_reassignment() {
        // TAPE-flagged: "Returned" is in ESPN's vocabulary.
        assert_eq!(classify("Returned F X to Bakersfield"), Reassignment);
    }

    #[test]
    fn l0_classify_optioned_is_reassignment() {
        assert_eq!(classify("Optioned G X to AHL"), Reassignment);
    }

    #[test]
    fn l0_classify_sent_down_is_reassignment() {
        assert_eq!(classify("Sent down F X to AHL"), Reassignment);
    }

    #[test]
    fn l0_classify_loaned_to_ahl_is_reassignment() {
        // Plain "loaned" without IIHF / World qualifier → AHL loan.
        assert_eq!(classify("Loaned F X to Bakersfield"), Reassignment);
    }

    #[test]
    fn l0_classify_placed_on_ir_is_ir() {
        assert_eq!(classify("Placed F Sam Reinhart on IR (lower body)"), InjuryReserve);
    }

    #[test]
    fn l0_classify_placed_on_ltir_is_ir() {
        assert_eq!(classify("Placed F X on LTIR retroactive to Oct 12"), InjuryReserve);
    }

    #[test]
    fn l0_classify_placed_on_long_term_is_ir() {
        assert_eq!(classify("Placed F X on long-term injured reserve"), InjuryReserve);
    }

    #[test]
    fn l0_classify_activated_from_ir_is_ir() {
        assert_eq!(classify("Activated F X from injured reserve"), InjuryReserve);
    }

    #[test]
    fn l0_classify_unknown_pattern_is_other() {
        assert_eq!(classify("Some new ESPN prose pattern we have not seen yet"), Other);
        assert_eq!(classify(""), Other);
    }

    // ── Order-of-rules sanity ──────────────────────────────────────────

    #[test]
    fn l0_classify_pto_with_signed_prefix_is_other_not_signing() {
        // Order matters — the PTO rule must run before the broad Signing rule.
        let d = "Signed F X to a PTO and recalled to NHL roster";
        // First-rule-wins: PTO rule matches first → Other.
        assert_eq!(classify(d), Other);
    }

    #[test]
    fn l0_classify_classifier_version_constant_is_set() {
        // Compile-time invariant — Lazy::new of zero would mean we shipped
        // something we hadn't classified, which is a bug. Static_assert
        // pattern keeps the intent visible without producing the constant-
        // assertion clippy warning.
        const _: () = assert!(CURRENT_CLASSIFIER_VERSION >= 1);
    }

    // ── Property tests ─────────────────────────────────────────────────

    proptest! {
        #[test]
        fn l0_classifier_property_pto_always_other(prefix in "[a-zA-Z ]{0,40}", suffix in "[a-zA-Z ]{0,40}") {
            // BENCH-mandated: any string containing "PTO" or "professional
            // tryout" classifies Other, never Signing.
            let d = format!("{prefix} Signed F X to a PTO {suffix}");
            prop_assert_ne!(classify(&d), Signing,
                "PTO must never classify as Signing: '{}'", d);
            let d2 = format!("{prefix} Signed F Y to a professional tryout {suffix}");
            prop_assert_ne!(classify(&d2), Signing,
                "professional tryout must never classify as Signing: '{}'", d2);
        }

        #[test]
        fn l0_classifier_property_rights_never_trade(prefix in "[a-zA-Z ]{0,40}") {
            // BENCH-mandated: rights acquisition never classifies as Trade.
            let d = format!("{prefix} Acquired the rights to RFA F X from BOS");
            prop_assert_ne!(classify(&d), Trade,
                "rights acquisition must never classify as Trade: '{}'", d);
            let d2 = format!("{prefix} Acquired negotiating rights to UFA G Y");
            prop_assert_ne!(classify(&d2), Trade,
                "negotiating rights must never classify as Trade: '{}'", d2);
        }

        #[test]
        fn l0_classifier_property_intl_loan_never_reassign(
            country in "(IIHF|World Championship|World Junior|Olympic|Spengler)"
        ) {
            // BENCH-mandated: international loan never classifies as Reassignment.
            let d = format!("Loaned F X to Sweden for the {country}");
            prop_assert_ne!(classify(&d), Reassignment,
                "international loan must never classify as Reassignment: '{}'", d);
        }

        #[test]
        fn l0_classifier_never_panics(s in "\\PC{0,200}") {
            // Any input — no panic, returns some kind.
            let _ = classify(&s);
        }
    }

    #[test]
    fn l0_other_rate_zero_when_all_match() {
        let inputs = vec![
            "Acquired F X from BOS",
            "Signed F Y to a 1-year contract",
            "Recalled G Z from AHL",
        ];
        assert!(other_rate(inputs.into_iter()) < 0.01,
            "all-match input should yield other_rate ≈ 0");
    }

    #[test]
    fn l0_other_rate_one_when_none_match() {
        let inputs = vec!["nonsense", "more nonsense"];
        assert_eq!(other_rate(inputs.into_iter()), 1.0);
    }

    #[test]
    fn l0_other_rate_empty_input_is_zero() {
        let inputs: Vec<&str> = vec![];
        assert_eq!(other_rate(inputs.into_iter()), 0.0);
    }
}
