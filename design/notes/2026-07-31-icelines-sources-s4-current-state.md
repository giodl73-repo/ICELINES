# IceLines Sources S4 — Identity and Current State

**Date:** 2026-07-31
**Status:** Complete locally

## Implemented

- `reconcile_staged_player_assertions` is the provider-neutral bridge from a
  reviewed identity proposal and its staged hockey assertion to a canonical
  player fact. Duplicate decisions, missing proposals, and rejected identities
  fail closed or remain explicit exclusions.
- `ReplayCutoffs` separates effective hockey time from evidence/review
  knowledge time. Default `AsKnown` replay excludes identity decisions made
  after the knowledge cutoff.
- Optional `ReconstructedIdentity` replay may use a later review to identify an
  earlier staged row. The canonical result retains only the earlier staged
  hockey fact and evidence, so later performance, assignment, transaction, or
  organization facts cannot leak into the historical state.
- `current-player-state.v1` resolves current rights, assignment, and
  participation independently from canonical facts. Its output carries both
  cutoffs, policy version, the complete accepted fact-ID set, reasons, and
  disclosures.
- Rights are one of `Supported`, `Expired`, `Transferred`, `Unknown`, or
  `Conflicted`. Draft and camp facts never establish current control. Contract,
  explicit transfer, release, and expiry facts drive rights state; a broken
  transfer/release chain fails closed as a conflict.
- Assignment is resolved only from assignment, roster, recall, and loan facts.
  Same-time observations naming different clubs conflict. Participation stays
  visible in a separate participation-only ledger.
- Superseded and retracted facts are removed before current-state resolution.
  Facts after the effective cutoff or learned after the knowledge cutoff are
  omitted with a historical-cutoff disclosure.

No CBA rights-duration inference is introduced in S4. Expiry must be an
explicit fact until a separately versioned and historically testable rights
policy is implemented.

## Validation

```text
cargo test -p icelines-sources -- --test-threads=1
cargo clippy -p icelines-sources --all-targets -- -D warnings
```

The full source suite passes, including five current-state and dual-cutoff
tests. Strict source-crate lint passes. A broader core `--all-targets` lint also
reports unrelated existing example/test lints outside the source contracts;
those are retained as repository-wide validation debt rather than hidden in
this checkpoint.
