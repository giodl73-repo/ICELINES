# Data-backed Current profile scope

The local profile studio can switch five retained profiles between the complete
2024–25 historical frame and a captured2026–27 season-to-date frame. The selector
replaces the whole primary statistical frame: request/year, GP, minutes, record,
peer ranks, workload, feats, candidate opportunities and snapshot metadata.
Returning to Season restores the historical card exactly. Career stays unavailable.

## Capture and source integrity

33 sorted official NHL pages cover779 skaters and77 goalies. Capture interval is
2026-11-19 02:28:07–02:28:15 UTC (November18 Pacific). This is a dated public
snapshot, not an atomic live-game feed. Provider updates may lag games. Source
pages, hashes, exact queries and timestamps are retained; current as-of must match
the frame's capture endpoint.

The capture script now explicitly decodes UTF-8. During assembly validation,
54 text fields were found corrupted by PowerShell's default decoding; rebuilding
from unchanged raw pages corrected them without changing numerical fields. The
current frame tool verifies raw hashes, routes, dates, totals, unique actors and
every assembled field against those pages (small float serialization tolerance;
counts and names retain their actual values). Older sealed artifacts were not
overwritten; their raw captures remain the source for any later text correction.

## Rust and browser boundaries

`rinkcraft_current_frame` reuses the shared Rust NHL parser, feat evaluator and
deployment parser. Current measured records use all-strength NHL production and
activity; they do not inherit the historical MoneyPuck5v5/xG vectors. Actor/game-
type/year/GP joins, source labels and peer rules are explicit.20skaterGP,
15goalieGP and20measured peers are the existing authored experimental gates.
Insufficient exposure preserves actual numbers but withholds peer ranks/feats;
observed zero is not missing data. No1–20 capability grade is produced.

`prepare_rinkcraft_scope_switch.mjs` assembles explicit current frame patches;
`project_rinkcraft_scope.mjs` merges Rust-authored scope/candidate patches;
`bundle_rinkcraft_scopes.mjs` packages the alternatives and verifies actor/year/
capture consistency. Historical numerical fields and dossier assertions remain
unchanged. The dossier is explicitly historical in Current; no current continuity
or coaching assignment is inferred. Old numerical histories, assessments and
model outputs are not carried into the current numeric frame.

The studio replaces the entire rendered card, remembers scope in the URL, and
accepts a Current deep link. Unsupported Career URLs fail explicitly rather than
showing another scope. Generic browser rendering without a scope handler disables
unavailable transitions. Native/WASM loaders and hosted deployment are not changed.

```text
cargo test -p icelines-sources --example rinkcraft_current_frame --example rinkcraft_scoped_preview --locked
cargo run -p icelines-sources --example rinkcraft_current_frame --locked -- HISTORICAL_FIXTURES REPORTS_JSON TOI_JSON CAPTURE_MANIFEST OUTPUT_FRAME_PATCHES
node tools/prepare_rinkcraft_scope_switch.mjs HISTORICAL_FIXTURES OUTPUT_FRAME_PATCHES CURRENT_BASE
cargo run -p icelines-sources --example rinkcraft_scoped_preview --locked -- CURRENT_BASE SCOPE_INPUT_MANIFEST OUTPUT_SCOPE_PATCHES
node tools/project_rinkcraft_scope.mjs CURRENT_BASE OUTPUT_SCOPE_PATCHES CURRENT_FIXTURES
node tools/bundle_rinkcraft_scopes.mjs HISTORICAL_FIXTURES CURRENT_FIXTURES OUTPUT_BUNDLE
node tools/prepare_rinkcraft_theme_studio.mjs OUTPUT_BUNDLE SITE_DIRECTORY
node icelines-browser/e2e/rinkcraft-current-scope.mjs
```

## Validation

Four focused Rust tests, TypeScript compilation and22 Node tests pass. Five
whole-profile round trips, Current deep link, unsupported Career rejection,
zero/low-GP handling and30current desktop/30mobile theme cases pass with no page
errors. Five current one-page Letter exports retain scope/capture/footer. The
historical five-card checks and old48desktop/24mobile/six-fictional-print theme
checks also pass. Fox current screenshot inspected.

Independent arithmetic checks use the full856-player reference population and
verify202 displayed-feat conditions and28 measured values across the five cards.
33raw hashes verify; six adversarial count/text/year/date/hash/route cases reject;
frame replay is byte-identical. Snapshot assembly comparison also binds all
source fields to the retained raw pages. Empty numerical history is valid and
no longer causes the browser evidence brief to dereference null. Provider/peer
copy is supplied by the frame, avoiding historical-provider captions on NHL data.

Raw capture, all generated frames, source commit/commands, input hashes, receipts,
UTF-8 correction record and frozen site are retained in the dated PUCK artifact
packet. Research definitions/review remain in Rinkcraft. This does not validate
capabilities, release the three candidate rules, change the34-rule live registry,
simulate games, establish Career coverage or add automatic refresh.
