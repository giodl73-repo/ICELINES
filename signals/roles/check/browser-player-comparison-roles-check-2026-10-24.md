# Browser two-player comparison review

Six lenses applied by one agent. Scope: two current-season skater/goalie query
projections, retained in the tab only. No Rust, worker protocol, dependency,
network, persistence, sharing or hosted-workflow change.

- **HART — PASS:** IDs distinguish selections; two distinct players share exact
  revision, season, season type and kind. Existing Rust fields supply all values;
  null stays unavailable and zero stays zero. Context changes clear projections.
- **KEEL — PASS, scoped:** Comparison presents existing shared query rows without
  formulas or duplicate domain logic. It does not claim full native CompareView,
  cross-season or career parity. Only two small row copies are retained, never
  dataset bytes or a complete previous result. Dialog callbacks release on close.
- **PACE — PASS:** Uses the leaders' display precision and Rust-supplied pace,
  save percentage and GAA. Different games played remain visible. No synthetic
  winner, statistical equivalence, performance saving or memory-budget claim.
- **BENCH — PASS:** All 175 browser units pass, including duplicate/limit/removal,
  stale-context and null/zero checks. New real Chromium checks compare skater and
  goalie cells to actual query exports, select while offline, preserve picks
  through filtering, and clear on revision import, season and player-kind change.
  Development provenance is explicitly dirty until the source commit. Existing
  clean-artifact browser suites are separate final regression checks.
- **EDGE — PASS:** A delayed detail response is ignored after query/engine change;
  its selection callback checks again. Two-player limits require explicit removal.
  A failed query, unload or engine recovery releases picks. Source/name content
  uses textContent; no HTML interpretation or IDs added to share links.
- **GLASS — PASS, desktop emulation:** Named table/scroll region, column and row
  headers, explicit remove/clear controls and focus restoration. Narrow Fresh Ice
  initially clipped one column; corrected to fit both columns and added a width
  assertion. Night Game, Fresh Ice and Hockey Club screenshots inspected at 390px.
  Physical Android/TalkBack remains deferred; this is not a phone acceptance claim.

No unresolved P1/P2 found for this bounded slice. The 250 MiB browser target stays
unmet; publishing remains manual and separate from local preview/review.
