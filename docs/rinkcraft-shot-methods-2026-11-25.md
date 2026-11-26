# Populate recorded Signature Methods — 2026-11-25

The card now shows measured shot repertoire alongside its preserved scouted
methods. Contract: [Recorded shot repertoire v1](../design/specs/rinkcraft-shot-methods-v1.md).
All calculations and event parsing are Rust; browser code renders the result.

## Frozen result

The existing 377-game regular-season cache supplies samples for 787 current
skaters: 776 have supported types and 11 have no unblocked attempts. There are
31,549 skater unblocked attempts, of which 31,543 have supported labels and six
are unknown. Six goalie attempts are excluded. All eleven supported labels occur.
This is cached coverage, not a complete-season claim. No source fetch was needed.

For example, Fox has 52 typed attempts over 23 cached roster games: 34 wrist,
11 snap, four slap and three backhand. Wrist attempts are 65.4% of his typed
attempts. This records usage, not technical proficiency or a new class assignment.

## Reproduction

Run from the owned worktree with the locally built CLI:

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-shot-methods --season 20262027 --catalog target/unit-links-study/enriched/rinkcraft-stat-attributes.json --game-cache-dir target/goalie-puck-study/cache --output-dir target/shot-methods-study/replay
```

Input SHA-256:
`617db53825c8b7656cc9ea03996a0aa4d0eb1e533cef684323f386e6160b700e`.
Enriched catalog SHA-256:
`93975bc201a7692c375b43c748a0ab5d76af569c2e57bda359552504c50a2f95`.
Source receipts, projections, scripts, screenshots and PDFs remain ignored in
`target/shot-methods-study`. Routine offline refresh with the same game cache
reproduces all 787 raw samples, including capture metadata and fingerprints.

## Validation

- 252 browser tests and 43 focused Rust tests passed, including existing grading,
  skating, pressure, unit-link and refresh contracts.
- Browser, pinned unoptimized WASM and native CLI/Askama builds passed.
- An independent event recount matches every histogram and roster-game count.
  All 3,948 existing profiles retain prior measurements and grade arrays (existing
  floating-point JSON roundtrip differences checked with numerical tolerance).
- All 865 preview profiles project and reproject through the shared WASM engine;
  admitted repertoire records survive serialization without changes.
- Eight live cards passed worker parity, exact Rust values, preservation of
  scouted methods, goalie separation, old-catalog fallback and mobile checks.
  The expanded Fox methods box was visually inspected.
- Eight independently inspected PDFs are single-page 612×792-point Letter sheets,
  retaining Signature Methods while hiding expanded scope.

Preview: [Fox](http://127.0.0.1:8780/?player=8479323), **Signature Methods**.
Acceptance script: `node icelines-browser/e2e/rinkcraft-shot-methods.mjs`.
No hosted publication, push or main merge is included in this step.
