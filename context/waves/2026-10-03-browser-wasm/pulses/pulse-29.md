# Browser WASM pulse 29 — real IndexedDB legacy-library migration

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-03 / WP-BW-06. Previous turn
made verified progress on application updates. Goal remains active.

Added `scripts/prepare-browser-migration-preview.mjs`, a disposable local setup
page beside an unmodified production application. It seeds an unused local
origin with schema 1 (`datasets` and `settings`), the real 2024–25 regular-season
package, and its active pointer. It refuses existing libraries and never deletes
or overwrites them. Setup buttons deliberately hold/release the old connection,
inspect persisted bytes/schema, and attempt an older schema-one open.

The application performs its own schema-2 migration. No migration, library or
engine implementation was substituted. All 101 copied production files matched
the current distribution's SHA-256 hashes. The two setup files are local harness
files and are absent from the production distribution/offline allowlist.

## Actual browser evidence

Preview: `http://127.0.0.1:8065/ICELINES/workbench/`. Server session 72604.
Current shell build:
`c51a89c39120ec3513e9985af2086aed81e2cdc2d274213c946f44a8dec76b05`.

Seeded 908,625 original package bytes with SHA-256
`ba4f8b9ab5e8882652099025360d8409b214a917d03ccc3c326a603a7ab86479`.
ID and active pointer: `legacy:20242025`.

While the legacy connection remained open, the actual production app reported
that another tab must close before storage can update. The setup page received
versionchange and deliberately held its connection. Load season and Restart
engine remained enabled; in-memory acquisition was not exercised in this held
state. This establishes the blocked message and available controls, not a full
blocked-storage usability claim.

After closing the held connection, inspection of actual IndexedDB returned:

- Version 2; stores `datasets`, `schedules`, `settings`.
- Original package length and SHA-256 unchanged.
- Stored revision and active pointer unchanged.

The app's Retry local storage action recovered the library. Load saved activated
the legacy season through the real WASM engine. `p >= 100` returned six skaters,
led by Kucherov's 121 points. A subsequent schema-one database open returned
`VersionError`; another inspection confirmed the same original bytes and active
pointer remained intact. Reloading the actual application restored all 905
skaters from that local selection, and a fresh filtered query again returned six.

![Migrated saved-season query](../screenshots/pulse-29-migrated-query.png)

`node --check scripts/prepare-browser-migration-preview.mjs` passed. Production
source was unchanged this pulse; the 121-test result remains the pulse-28 result.

## Remaining gates

This proves the selected schema-one to schema-two migration, held-connection
recovery, old-schema refusal and byte/pointer preservation in this real browser.
Interrupted migration and quota faults remain deterministic fixture evidence;
broader browser/device migration, saved schedule upgrade/rollback and remote
clean-artifact restore remain open. Pages deployment/source migration, deployed
live access and representative mobile/resource acceptance remain open. Relay
hosting remains deferred until deployed-origin testing. No remote mutation ran.
