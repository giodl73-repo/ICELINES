# Browser WASM acceptance ledger

Date: 2026-10-08. Requirement: REQ-BROWSER-001; work packages WP-BW-01..06.
This reconciles implemented features with the plan's remaining acceptance gates.
It does not declare the full plan complete.

## Current release

- Source: `696c656bead479bde0048059b551c49aa53856eb` (PR #81; includes #79 and #80).
- Publication: run `37854895167`, attempt 2, successful build and deploy.
- Shell: `5875fe2396a1a1f19c105355cc597323613e0c10924318b8279b51789d0b445f`.
- Retained clean artifact verified: 103 files, 75 public packages.
- Pages source: Actions workflow. `master` and `gh-pages` branch policies exist;
  the existing workflows publish from master and compose documentation from gh-pages.
- Attempt 1 failed before running deployment steps: master was excluded by the
  old gh-pages-only environment rule. Switching Pages to workflow mode added
  the master rule; retrying only the failed job succeeded.

## Feature and gate disposition

| Plan requirement | Evidence | Disposition |
|---|---|---|
| Shared Rust normalization and worker-owned WASM | Native/WASM corpus, distributed-WASM verifier, CI browser build | Implemented; scoped parity corpus passes |
| Catalog, regular/playoff selection, historical packages | Clean retained artifact has 75 packages; CI loads every package in actual WASM | Implemented |
| Skater/goalie filters, sort, sample floors and player details | Distributed count-order goldens, native parity corpus, deployed player/goalie walkthroughs | Implemented |
| Session memory versus explicit saving | Library transaction/conflict tests and deployed save/reload plus unsaved-data discard observations | Implemented |
| Imports, archive bounds and JSON/CSV/package export | Resource preflight and archive fixtures; actual file-picker and downloaded-byte evidence in deployed-file-roundtrip-20261006.json | Implemented; phone picker latency unmeasured |
| Live stats/schedule and bounded relay | Cloudflare relay browser refresh evidence; PR #79 stable ordering/coverage guards; 940-skater and eight-leader observations | Implemented; source observation timestamps remain explicitly unknown where upstream omits them |
| Polling, cancellation, previous-good preservation | Polling/acquisition tests and deployed cancellation evidence | Implemented |
| Responsive UI, keyboard, share routes and source disclosures | Selected 360px/desktop tests and deployed export/view checks | Partial acceptance; not a physical-phone or full assistive-technology claim |
| Appearance choices | Night Game, Fresh Ice, Hockey Club; live switching, preference reload and cross-tab observations | Implemented |
| Exact release publication and documentation preservation | Retained artifact, deployment run and deployed-release-20261008.json | Static-byte evidence; not browser/offline proof |
| Application update with saved data | Live two-tab update to 696c656b restored saved season and 49-game schedule; restored p >= 100 returned eight skaters | Selected deployed update passes |
| Deployed offline reopening and saved query | Local server-stopped reopen passes; deployed cached-shell checks pass | OPEN: disable networking in an isolated browser, close/reopen the deployed URL and run a fresh saved-data query |
| Previous-version rollback and forward restore | Retained artifacts and rollback workflow; local rollback rehearsals | OPEN: remote rehearsal must preserve current documentation and saved bytes; no production rollback performed in this follow-up |
| Whole-browser staging/resident peak memory | Encoded-input limits, WASM high-water measurements, ownership/transfer evidence | OPEN: total browser peak must be measured on representative hardware; WASM linear memory is insufficient |
| Physical mobile, touch/picker and latency | Desktop browser at narrow viewport | OPEN: real mobile browser/device measurements required |
| Compare/history/team/cards/fantasy | Plan WP-BW-07 requires separately scoped contracts and parity | Later waves; not implemented or implicitly accepted by the first slice |

## Next acceptance actions

1. In an isolated browser profile, load and save a season on the deployed URL.
   Wait for verified offline-shell readiness. Disable networking for that browser,
   close/reopen the workbench and run a new filter and player query. Record the
   build, browser/device, exact actions, result and network failure evidence.
2. Rehearse a retained previous-good release through `browser-rollback.yml`, then
   restore this exact release using its retained artifact. Compare saved-package
   fingerprints and current documentation before/after. This temporarily changes
   the public site and is not inferred from a local preview rehearsal.
3. Measure browser-process/staging peak separately from WASM memory, with an
   isolated profile and attributed processes. Run the all-package/resident/import
   cases and warm-query measurements; disclose baseline and instrument limitations.
4. Run the responsive/picker/keyboard flows on a physical phone. Record mobile
   engine, device and timing; do not substitute a desktop viewport capture.

The earlier in-app browser checks had viewport and visibility controls but no
offline-network or whole-process-memory control. In this resumed acceptance
session, tool discovery exposes no browser controller, and the Windows
computer-use helper probe returned an unavailable native pipe. These tool limits
do not prove the app fails offline or remove the acceptance requirement.

Prepared rollback selection is recorded in
`evidence/prepared-rollback-20261008.json`: previous artifact 0caaeaeb and current
artifact 696c656b both pass the retained-artifact verifier. Their local composed
outputs preserve identical inventories of all 95 documentation files. This is
preparation, not a remote rehearsal; public rollback approval remains pending.
