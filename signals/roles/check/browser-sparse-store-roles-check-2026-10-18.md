---
skill: roles-check
topic: browser-sparse-store
date: 2026-10-18
roles_used: 6
p1_count: 0
verdict: APPROVED-SCOPED
---

# Prepared browser sparse ownership review

One agent applies HART/FORGE/BENCH/PACE/KEEL/EDGE lenses. Scope: CHG124,
separate SparseSeasonRepository, shared borrowed views/comparators/serialization,
opt-in browser backend and actual native/WASM verification. No independent panel.

| ID / role | Finding | Severity / state | Evidence | Resolution / condition |
|---|---|---|---|---|
| H1 HART | This owner is narrower than full StatsRepository | P2 resolved | prepared-store API / spec | Explicitly no roster overlays/Tier2/LRU; BrowserEngine owns windows |
| H2 HART | Identity versions must remain scoped by prepared window | P3 verified | ActivePackage ownership | No cross-package identity sharing; original merge rules reused |
| H3 HART | Widths, absent/zero values and trade stints must survive | P3 verified | original catalog/typed/JSON matrix | All eight payload types retained; no stat narrowing |
| F1 FORGE | Generic default inference could break mutable/Box reference coercions | P2 resolved | concrete PlayerView alias and coercion test | Separate ReadPlayerView; retain old constructors/field types/Copy behavior |
| F2 FORGE | Slot width and swapped ownership can break mutation correctness | P3 verified | checked indices /1200 mixed operations | Precheck before mutation; repair moved slots/owners; validate invariants |
| F3 FORGE | Borrowed reads must prohibit mutation of their owner | P3 verified | E0502 doctest / marker assertion | No fabricated row reference or clone cache; sparse owner !Send/!Sync |
| B1 BENCH | Shared formulas alone could conceal migration errors | P2 resolved | native original/default/sparse replay | Nine native samples match all 78 states; actual WASM directly matches 1,560 query strings/51,435 details/78 exports |
| B2 BENCH | Player detail serialization cannot lose payload fields | P2 resolved |15-field borrowed serialization / exact original JSON checks | No owned SeasonStats reconstruction in production |
| B3 BENCH | Both feature modes and full callers must compile | P3 verified | core/query/browser tests and native/WASM checks | Opt-in feature, current full repository remains available |
| P1 PACE | Omitting unused indexes contributes to storage savings | P3 documented | spec / native allocation scope | Attribute full prepared-owner result, not solely payload packing |
| P2 PACE | Wider handles may increase query scratch | P3 verified | separate20-query requested allocations | Query peak unchanged; first-eight cumulative bytes +0.76%, final sweep -1.86%; retained bytes zero |
| P3 PACE | Native bytes are not WASM/whole-browser acceptance | P3 documented | platform scopes / fixed budget | Separate actual WASM replay and full browser/device gates |
| K1 KEEL | Source exports and revisions must remain original package facts | P3 verified | retained SeasonPackage / per-window hashes | Never export normalized reconstructed packages |
| K2 KEEL | Refresh must remain staged and window limits unchanged | P3 verified | original activation/LRU code; invalid refresh/import/reselection replay | No new activation or eviction policy |
| K3 KEEL | Default deployment must not select unaccepted candidate | P3 verified | default=[] / sparse-storage feature | No artifact publication or merge authorized by this experiment |
| E1 EDGE | Missing goalie values and ranking ties have special ordering | P3 verified | unchanged comparator expressions / golden stat order | Legacy wrappers delegate to one read comparator |
| E2 EDGE | Replacements must release old payloads rather than append forever | P3 verified | dense owner release/pack/swap repair | Whole-window browser replacement also drops the old owner |
| E3 EDGE | Production API compatibility needs downstream evidence | P2 resolved | workspace and all feature gates | Nine-crate workspace and 1,921 production test/doc executions pass; default WASM check and sparse release build pass; failures/recovery retained |

Three amendments: keep concrete legacy view/coercion and comparator signatures;
name the narrower prepared-data owner and opt-in boundary; require original-engine
parity plus measured owner/query/platform tradeoffs. All five P2 resolved.
Native/WASM checks and actual replay pass; scoped approval covers this opt-in
candidate. See context/waves/2026-10-03-browser-wasm/evidence/miser-browser-sparse-local.json. Browser 250 MiB
and Android acceptance remain separate, with no budget waiver or publication claim.
