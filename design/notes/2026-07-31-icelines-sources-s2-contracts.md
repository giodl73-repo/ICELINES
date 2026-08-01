# IceLines Sources S2 — Canonical Facts and Source Package

**Date:** 2026-07-31
**Plan:** [`../plans/2026-07-31-icelines-sources.md`](../plans/2026-07-31-icelines-sources.md)
**Status:** Complete locally

## Delivered

`icelines-core::source_facts` now owns the source-neutral contract:

- validated source, provider, adapter, policy, fact, proposal, decision,
  organization, club, league, package, URL, and SHA-256 identities;
- private, validated `FactAssertion<T>` records with semantic keys, typed
  subjects, fact-domain time, fact-family authority, non-empty evidence,
  supersession, and retraction links;
- explicit organization events carrying organization or from/to parties;
- separate participation facts and participation authority;
- immutable identity proposals plus separately authored review decisions;
- replay-safe staged player assertions that retain the fact awaiting identity
  review and must reference a proposal in the same sealed package;
- requested-scope run manifests that distinguish acquired-empty,
  not-applicable, failed, quarantined, and incomplete-pagination states;
- deterministic freshness evaluated at the package knowledge cutoff;
- typed conflicts, exclusions, coverage buckets, and stable disclosures; and
- canonical, order-invariant `icelines_source_package.v1` fingerprints.

The evidence ledger remains independent of `StatsRepository`. A package cannot
silently update canonical identity, season statistics, organization state, or a
product score.

## Temporal contract

Two cutoffs are mandatory:

- `effective_cutoff` bounds when the hockey event occurred; and
- `knowledge_cutoff` bounds when evidence was captured or a review decision was
  made.

Tests independently reject a later event and a later capture. Freshness must be
evaluated against the package knowledge cutoff rather than the wall clock.

## Compatibility lowering

`icelines-sources::compat::prospect_population_v1` reads and round-trips the
existing `prospect_population_overlay.v1` shape.

- Legacy rights/contract/assignment labels lower only to a visibly
  compatibility-scoped relationship fact.
- Development-camp attendance lowers to participation with unknown control
  authority.
- Free-agent invites remain participation-only.
- Unknown relationships become typed exclusions.

The bridge therefore preserves old artifacts without manufacturing new legal
control facts.

## Schema and verification

The wire contract is published at
[`../schemas/icelines_source_package.v1.schema.json`](../schemas/icelines_source_package.v1.schema.json)
and embedded as `SOURCE_PACKAGE_JSON_SCHEMA`.

```text
cargo test -p icelines-core source_facts::tests --lib
6 passed; 0 failed

cargo test -p icelines-sources
9 passed; 0 failed

cargo clippy -p icelines-core --lib -- -D warnings
passed

cargo clippy -p icelines-sources --all-targets -- -D warnings
passed
```

S3 is next: populate these contracts from official draft, club camp,
contract/transaction, and current assignment payloads, beginning with frozen
provider fixtures and a complete requested-scope manifest.
