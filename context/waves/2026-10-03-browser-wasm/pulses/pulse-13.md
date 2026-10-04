# Browser WASM pulse 13 — bounded saved-library retention

Date: 2026-10-03. Parent: WP-BW-03 / WP-BW-05 / REQ-BROWSER-001 / CHG-119.

The UI saved-library cache now contains `SavedDatasetEntry` headers, revisions,
refresh policy and byte lengths. It retains neither package bytes nor decoded
player arrays for inactive saved seasons. Listing reads keys, then verifies each
record sequentially before projecting its header. This preserves existing
digest/schema refusal without bulk `getAll` reads or a database migration.
Records deleted between key enumeration and reading are skipped. Listing is not
an atomic snapshot across tabs; selection rechecks the listed revision.

`loadSaved(id, expectedRevision)` reads and validates one selected package and
refuses removed or replaced revisions. Startup, recovery, local links, public
catalog reuse and saved-library selection use that API. Acquisition generation
checks prevent delayed saved reads from activating after a context change.
Refresh-policy changes save the matching active package with the existing
transactional revision guard, and avoid mutating a newly selected active package
when the earlier save completes.

Evidence: TypeScript check and all 69 Node browser tests passed. New tests cover
24 saved fixture packages, header-only retained entries, refusal of bulk payload
reads, lazy selected bytes, replaced/deleted revisions, independent ownership,
and corrupt selected records. Existing integrity, quota rollback, concurrent
writer, privacy, worker and offline tests remain green.
Distribution verification passed 16 shell assets, four count-order goldens and
all 75 public packages through actual WASM loading and queries. Largest shell
plus single-package gzip sum: 377,872 bytes. Whitespace checks passed.

Actual browser: fresh localhost origin `http://127.0.0.1:8055/`, production dist,
Codex in-app browser. Loaded and saved 2024–25 regular and playoff packages;
library showed two entries. Selected the regular saved copy, enabled keeping
refreshes saved, reloaded its public view, and observed Saved locally, 905
players, two library entries and the retained checked policy.
[Screenshot](browser-pulse-13.png).

This bounds retained library payloads, not total browser memory. Listing still
hashes and parses each package and can transiently hold one full package plus its
decoded rows. No actual large-library browser heap measurement, maximum-import
peak, mobile budget or full 250 MiB memory acceptance is claimed. The broader
live, offline reopening, score/schedule, deployment and rollback gates remain open.
