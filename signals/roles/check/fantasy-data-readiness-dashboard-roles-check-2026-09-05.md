---
skill: roles-check
topic: fantasy-data-readiness-dashboard
date: 2026-09-05
roles_used: [hart, keel, tape, forge, pace, bench, edge, wire, scout, glass, crest, broadcast]
p1_count: 0
verdict: APPROVED-WITH-CONDITIONS
---

# Roles Check — Fantasy Data-Readiness Dashboard

## Artifact identification

- Artifact: `design/plans/2026-09-05-fantasy-data-readiness-dashboard.md`
- Type: cross-crate contract, read-only assembly, and CLI/TUI/Web design
- Signals: data provenance, privacy, filesystem safety, schema evolution,
  accessibility, browser state, and testability

## Role selection

All twelve installed roles apply. HART, KEEL, FORGE, and WIRE govern the new
contract and crate boundaries; TAPE and PACE govern evidence semantics; BENCH
and EDGE govern verification and failure behavior; SCOUT constrains the hockey
claim; GLASS, CREST, and broadcast govern the three interactive surfaces.

## Findings

| # | Role | Finding | Severity | Section | Recommendation |
|---:|---|---|---|---|---|
| 1 | HART | Readiness must not become a second source model. | P2 | Evidence assembly | Project existing typed Today evidence. |
| 2 | HART | Season-scoped evidence needs an explicit stats-season axis. | P2 | Contract | Carry `stats_season` in request and view. |
| 3 | HART | A snapshot fingerprint must derive from the sealed view. | P3 | Contract | Fingerprint stable serialized material state. |
| 4 | KEEL | Surface-local readiness policy would drift. | P2 | Surfaces | Use one core builder and one fetch assembler. |
| 5 | KEEL | TUI is long-lived while CLI/Web are one-shot. | P3 | Surfaces | Store only the projected TUI view; assemble per Web request. |
| 6 | KEEL | Existing Today schemas must remain compatible. | P2 | Compatibility | Add `fantasy_readiness.v1`; do not alter Today v1/v2. |
| 7 | TAPE | Missing timestamps cannot mean freshly observed. | P2 | Contract | Preserve optional observed/fetched timestamps. |
| 8 | TAPE | Missing optional evidence cannot become numeric zero. | P2 | Evidence assembly | Grade it provisional and state the absence. |
| 9 | TAPE | Source families must remain visible downstream. | P3 | Contract | Carry source-family identifiers per check. |
| 10 | FORGE | A read must not create or migrate SQLite state. | P2 | Evidence assembly | Use immutable existing-database open paths only. |
| 11 | FORGE | Policy belongs in core rather than renderers. | P3 | Contract | Keep aggregation pure and renderers presentational. |
| 12 | FORGE | Root assembly errors need typed recovery. | P3 | Evidence assembly | Return a blocked view instead of an opaque failure. |
| 13 | PACE | A scalar confidence score would invent precision. | P2 | Contract | Use deterministic ready/provisional/blocked states. |
| 14 | PACE | Required and optional inputs affect state differently. | P3 | Contract | Block only on blocked required checks. |
| 15 | PACE | Readiness does not establish recommendation quality. | P3 | Outcome | Describe evidence support only. |
| 16 | BENCH | Aggregation boundaries require pure tests. | P2 | Verification | Test required blockers and optional degradation. |
| 17 | BENCH | Filesystem non-mutation needs direct evidence. | P2 | Verification | Test a missing DB path remains absent after assembly. |
| 18 | BENCH | Each surface needs a contract-facing test. | P3 | Verification | Cover CLI parsing, TUI recovery, and Web filter parsing. |
| 19 | EDGE | Duplicate workflow/check IDs are ambiguous. | P2 | Contract | Reject duplicates at the builder boundary. |
| 20 | EDGE | Non-ready rows without recovery strand the user. | P2 | Contract | Reject them as invalid input. |
| 21 | EDGE | Unknown workflow query values must not silently broaden scope. | P3 | Web | Return an invalid-request response with allowed names. |
| 22 | WIRE | Workflow and requirement names are API vocabulary. | P2 | Contract | Use typed snake-case serialization and version the schema. |
| 23 | WIRE | Network access during readiness would violate predictability. | P2 | Evidence assembly | Inspect caches and local state only. |
| 24 | WIRE | Root failure still needs machine-readable state. | P3 | Evidence assembly | Emit `fantasy_readiness.v1` blocked output where possible. |
| 25 | SCOUT | Data readiness is not player-value validation. | P2 | Outcome | Keep hockey-quality claims out of this contract. |
| 26 | SCOUT | Status and goalie evidence have different meanings. | P3 | Evidence assembly | Retain distinct checks and recovery commands. |
| 27 | SCOUT | Trade readiness needs league-wide roster evidence. | P3 | Evidence assembly | Inspect all saved team rosters, not only the user team. |
| 28 | GLASS | Color cannot be the sole readiness encoding. | P2 | Surfaces | Print explicit state labels on every surface. |
| 29 | GLASS | The first screenful must expose recovery. | P3 | Surfaces | Show counts and first non-ready command before detail. |
| 30 | GLASS | Terminal layouts must remain useful at narrow widths. | P3 | TUI | Keep the embedded summary compact and line-oriented. |
| 31 | CREST | A dashboard ordered by implementation detail will feel accidental. | P3 | Surfaces | Lead with overall state, counts, then workflow sections. |
| 32 | CREST | Repeated decorative status treatments add noise. | P3 | Surfaces | Use restrained borders and semantic typography. |
| 33 | CREST | CLI, TUI, and Web should feel related but medium-appropriate. | P3 | Surfaces | Share vocabulary and hierarchy, not identical layout. |
| 34 | broadcast | Browser filters must be bookmarkable. | P2 | Web | Keep workflow, league, team, season, and date in query parameters. |
| 35 | broadcast | Private local state should not be cached. | P2 | Web | Set `Cache-Control: no-store` for success and error responses. |
| 36 | broadcast | HTML needs semantic context and non-color state. | P3 | Web | Use main/section headings, time elements, labels, and recovery text. |

## Synthesis

Roles reviewed: 12
P1 blockers: 0 | P2 issues: 18 | P3 notes: 18

Verdict: **APPROVED-WITH-CONDITIONS**

Top finding: readiness must remain a projection over authoritative local
evidence, never a parallel persistence model or an implicit fetch path.

Cross-role consensus: HART, KEEL, FORGE, TAPE, and WIRE require one typed,
read-only, source-visible contract; BENCH and EDGE require fail-closed checks;
GLASS, CREST, and broadcast require labeled state and visible recovery.

## Amendments

1. Implement one pure `fantasy_readiness.v1` core projection and one read-only
   fetch assembler, then make all renderers consume it.
2. Reject duplicate IDs and missing recovery commands; add root-blocked,
   no-write, CLI, TUI, and Web tests.
3. Keep browser filters sticky and all readiness responses uncached; present
   explicit state labels and recovery before evidence detail.

This is a simulated role review checklist, not human approval.
