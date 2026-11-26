# Faceoff manpower context — 2026-11-25

Under Pressure's **Contested draws** response now expands by player-relative
skater balance. Contract: [Faceoff manpower partitions v1](../design/specs/rinkcraft-draw-strength-v1.md).
Existing overall results, late-close score splits, defensive actions, matched
late-lead exposure and goalie responses retain their meanings.

## Frozen sample

The existing 377 finalized-game cache contains 21,664 contests involving 449
players. Each contest has two player observations. Standard unequal-skater
situations account for 4,071 contests: the advantaged side won 2,217 (54.5%) and
the disadvantaged side 1,854 (45.5%). This includes lineup selection and other
context; it does not establish a causal manpower effect or a simulation bonus.
Even-skater and other categories each have aggregate 50% win rates by construction.

Sebastian Aho illustrates the new view:

| Reported context | Wins / contests | Win percentage |
|---|---:|---:|
| Even skaters | 178 / 311 | 57.2% |
| Skater advantage | 46 / 85 | 54.1% |
| Skater disadvantage | 6 / 14 | 42.9% |
| Other / empty net | 3 / 5 | 60.0% |

These sum to the existing 233/415 aggregate. Counts remain visible, particularly
for small subsets; no observed opportunities is not a zero-percent skill. No
unknown strength codes occur in this frozen sample, but the parser and display
retain that category for missing or unsupported inputs.

## Implementation

Rust parses the event code in away-goalie/away-skaters/home-skaters/home-goalie
order, resolves each actor's roster team, and aggregates optional manpower
partitions. Both goalies must be present with 3–5 skaters per team for the
even/advantage/disadvantage categories. Empty nets and unusual six-skater states
remain separate. Existing projections without the optional split remain valid.

`profile-records --game-cache-dir` and routine offline refresh use the same parser.
The existing breakdown renderer supplies the display; only its context heading
changes for the overall-draw entry. No new source fetch, grade recipe, feat or
class assignment is involved. Compact print hides the expanded rows.

## Reproduction and validation

```powershell
target/native-validation/debug/icelines.exe --no-setup --no-live profile-records --season 20262027 --catalog target/shot-methods-study/enriched/rinkcraft-stat-attributes.json --game-cache-dir target/goalie-puck-study/cache --output-dir target/draw-strength-study/enriched
node icelines-browser/e2e/rinkcraft-draw-strength.mjs
py target/draw-strength-study/verify.py
```

The original input catalog SHA-256 is
`93975bc201a7692c375b43c748a0ab5d76af569c2e57bda359552504c50a2f95`.
Choose a fresh output directory for replay. Generated catalogs, source receipts,
projections, independent recount, screenshots and PDFs stay ignored under
`target/draw-strength-study`.

Validation covers actor-relative advantage reversal, even 3v3, empty nets,
missing/malformed codes, reconciled partitions, legacy absence and preserved
prior pressure semantics. The browser test additionally checks the different
manpower and score-state headings. Live-card acceptance checks the existing
production and penalty views alongside the new split.

255 browser tests and 20 focused Rust tests passed (eight core pressure tests,
six source pressure tests, three source shot-method tests and three refresh tests).

Browser, pinned unoptimized WASM and native CLI/Askama builds passed. Independent
recount matches all 787 raw partitions and their derived totals; all 3,948 prior
profiles are unchanged apart from this partition (existing floating-point JSON
roundtrip noise compared with numerical tolerance). Routine offline refresh
reproduces the same 787 partitions. All 865 local cards project and reproject
through the shared WASM engine.

Eight live cards passed exact breakdown values, actual worker parity, retained
production/penalty records, unchanged grades, goalie withholding, old-card
fallback and 390-pixel mobile layout. Aho's expanded draw panel was visually
inspected. Eight independent PDF checks confirm single-page 612×792-point Letter
exports, retaining Under Pressure while hiding expanded manpower details.

Enriched catalog SHA-256:
`22c98248e403016b7e9a4e34e140bbb03ff37e2b9d3beb4a480a9744c7500315`.

Preview: [Aho](http://127.0.0.1:8780/?player=8478427), **Under Pressure →
Contested draws**. No hosted publication, push or merge is part of this change.
