---
skill: roles-check
topic: league-aware-daily-decisions-implementation
date: 2026-09-05
roles_used: 8
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# League-aware daily decisions — implementation role review

Artifact type: Rust implementation, versioned JSON contracts, CLI/TUI/Web
adapters, tests, consumer documentation, and performance evidence.

Selected roles: HART for season/player axes; KEEL for four-surface convergence;
FORGE for Rust and `!Send` boundaries; PACE for bounded scoring and latency;
BENCH for contract and integration proof; EDGE for time, GP=0, and stale-state
failures; WIRE for cached evidence and schema evolution; broadcast for the Web
experience.

## HART

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Stats are loaded for an explicit regular-season axis and borrowed `PlayerView` values do not escape assembly. | P3 | service | Preserve the request season/type axis when adding playoff support. |
| 2 | Current-team joins originally fell back silently when one roster cache was absent; the implementation now marks the entire fallback provisional with recovery. | P3 | current rosters | Keep historical stat-team fallback out of ready decisions. |
| 3 | `fantasy_today.v2` wraps the complete v1 shape and changes only the schema label; `v1_projection()` restores the exact v1 contract. | P3 | core contract | Continue additive versioning rather than changing v1 semantics. |

## KEEL

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | CLI, TUI, and Web now invoke one `icelines-fetch` assembly owner; subprocess and `OnceLock` adapters are gone. | P3 | surfaces | Keep renderers calculation-free. |
| 2 | Initial TUI refresh was first attached to a snapshot helper rather than the event loop; fixed in `run_loop`, command entry, MDI entry, and `r`. | P3 | TUI lifecycle | Retain entry/refresh regression coverage. |
| 3 | v1 JSON remains independently projected while default CLI, TUI, HTML, and v2 JSON consume v2. | P3 | compatibility | Remove v1 only through a separately reviewed deprecation. |

## FORGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Web creates and drops all non-`Send` repository/database state inside `spawn_blocking` and returns only an owned ViewModel; workspace compilation proves the boundary. | P3 | Web assembler | Never move `StatsRepository` into returned or shared state. |
| 2 | The service exposes a `thiserror` boundary, but several inner errors are classified from message prefixes. | P3 | errors | Replace prefix classification with typed internal resolution errors when the service grows. |
| 3 | TUI refresh is synchronous and may visibly pause on a cold cache even though warm release p95 is below 400 ms. | P3 | TUI | Consider a dedicated local worker only if field measurements show user-visible cold pauses. |

## PACE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Exhaustive add/drop lineup rebuilding measured near 10 seconds and was rejected for Today. | P3 | candidate design | Keep exhaustive work behind `fantasy pickups --top 5`. |
| 2 | The replacement uses the canonical pickup score with a position-aware remaining-week matching projection; displaced-starter value is reconciled into the net delta. | P3 | candidate math | Preserve the reconciliation test when score components change. |
| 3 | The supported 12-candidate/250-ms ceiling measured 3 ms release and 20 ms dev; warm release command p95 measured 388.9 ms. | P3 | performance | Re-measure on material loader or roster-size changes. |

## BENCH

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Independent v1 and v2 deterministic goldens cover schema, primary firmness, candidate state, and projection compatibility. | P3 | L0 | Extend the v2 golden when new stable decision fields ship. |
| 2 | Snapshot tests cover future, wrong-team, and reverse-orientation axes; missing database tests prove typed failure without file creation. | P3 | L1 | Add a fully seeded successful service fixture when compact sealed schedule/stats fixtures become available. |
| 3 | Web tests cover v1, v2, and semantic no-script missing-state degradation. | P3 | Web L1 | Add ready-state mobile HTML assertions alongside the future seeded service fixture. |

## EDGE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | GP=0 roster players were initially exposed as zero-value drops; they are now protected from bounded recommendations. | P3 | candidate legality | Keep unknown/zero-sample players out of drop advice. |
| 2 | Started games were initially present in remaining-week value and locked drops; evaluation-time schedule filtering now removes both. | P3 | lock time | Add provider-specific lock rules only as explicit league configuration. |
| 3 | Missing opponent and category components remain unavailable/provisional instead of becoming a 0-0 points matchup. | P3 | matchup | Preserve missing-is-not-zero behavior. |

## WIRE

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | Today reads only local database, sealed stats, cached schedule, roster snapshots, and saved platform observations; recovery commands never execute. | P3 | source boundary | Keep live refresh as an explicit write command. |
| 2 | Saved matchup selection rejects future, wrong-week, wrong-team, partial, and temporally incoherent snapshots. | P3 | selection | Add category-component identity axes when that provider schema exists. |
| 3 | Selected evidence now discloses rejection count and reason codes without leaking the private source URL. | P3 | provenance | Keep private URLs outside generic exported decision evidence. |

## broadcast

| # | Finding | Severity | Section | Recommendation |
|---|---|---|---|---|
| 1 | HTML remains semantic, responsive, and no-script with a visible 503 recovery state. | P3 | Web HTML | Preserve `<main>`, headings, viewport metadata, and text state labels. |
| 2 | The ready page now exposes league, team, stats season/type, date, firmness, legality, evidence age, matchup impact, and deadline above supporting detail. | P3 | active context | Keep this context above the fold on narrow screens. |
| 3 | The date/league/team query remains bookmarkable; evaluation clock is server-owned. | P3 | URL state | Add an explicit replay-time parameter only with authorization and validation semantics. |

## Synthesis

Roles reviewed: 8
P1 blockers: 0 | P2 issues: 0 | P3 notes: 24

Verdict: **APPROVED-WITH-CONDITIONS**

Top finding: decision trust required protecting zero-sample and locked players
and reconciling displaced-starter value; all three corrections are implemented.

Cross-role consensus: HART, PACE, BENCH, and EDGE agree that missing or
incomplete evidence must reduce firmness rather than turn into a zero-valued
player or matchup. KEEL, FORGE, and broadcast agree that one owned contract must
cross the surface boundary without subprocesses or renderer-side decisions.

## Amendments applied

1. Added evaluation-time lock filtering, GP=0 drop protection, current-roster
   provisional evidence, and displaced-starter value reconciliation.
2. Corrected TUI entry refresh and made CLI/TUI/Web render the v2 primary with
   firmness, legality, context, matchup impact, and deadline.
3. Added v2 golden, typed missing-state and snapshot-axis tests, PUCK handoff
   documentation, database/sidecar immutability evidence, and release timing.
4. The full all-targets gate found and closed two integration bookkeeping gaps:
   the unloaded TUI now preserves its designed recovery phrase, and the frozen
   public fetch-module inventory includes `fantasy_today_service`.

Conditions are follow-up depth, not blockers: add a compact successful sealed
L1 parity fixture when the repository has suitable schedule/stats fixtures, and
revisit asynchronous TUI loading only if cold-cache field measurements justify
the extra worker complexity.

## Validation closeout

- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- workspace all-target testing: pass after the two gate-driven corrections;
  CLI/core/fetch and all remaining query/sources/Web/site targets were rerun
  through their complete all-target suites
- `py C:/src/tracker/repos/standards-protocols/roles/tools/check_roles.py .`:
  pass with the repository's pre-existing role-frontmatter warnings
- `git diff --check`: pass (Git emitted Windows line-ending notices only)

## Completion-audit addendum

A second requirement-by-requirement pass was performed before merge. HART
found that the request's season type was documented but the assembler still
loaded regular-season rows; it now carries the axis through stats loading and
context. EDGE and WIRE required stale saved snapshots and missing rules/cache
to have explicit typed outcomes. KEEL, BENCH, and broadcast found a surface
parity gap: CLI/TUI retained v1 alternatives while Web omitted alternatives and
none of the human surfaces showed the decision fingerprint.

Those findings are closed by the shared `FantasyTodaySurfaceDecision`, a
public sealed fixture consumed in core, CLI, TUI, and Web tests, typed recovery
tests, and stale/partial snapshot tests. Web now renders alternatives and a
mobile-wrapping fingerprint; CLI keeps fingerprint output within 80 columns;
TUI renders the same shared decision subset. No new P1 or P2 issue remains.
