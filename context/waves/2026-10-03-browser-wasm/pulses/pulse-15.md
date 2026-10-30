# Browser WASM pulse 15 — independent durable schedules

Date: 2026-10-03. Parent: WP-BW-03 / WP-BW-04 / WP-BW-05 / REQ-BROWSER-001 / CHG-119.

## Implemented lifecycle

IndexedDB schema 2 adds a `schedules` store in the existing deployment-path
namespace. The upgrade transaction preserves season packages and their active
pointer. An interrupted migration rolls back; unsupported future versions are
refused without deletion. Old schema-1 builds cannot open upgraded storage;
deployment rollback must use a compatible build or exported backups.

Schedule interchange schema 1 contains kind, requested date, source, unknown
observation time, fetch timestamp and raw NHL payload. Its digest covers the
entire envelope, binding context and timestamps to the data. Storage validation
refuses corrupt bytes, context substitution, future/unknown schema fields and
invalid dates. Bounds: 2 MiB source payload, 3 MiB interchange envelope. Saved
entries retain headers only; selected loads recheck their listed revision and
project the raw payload through shared Rust before activation.

The UI now supports explicit save, independent saved schedule selection,
remove-saved/unload-memory, export/import and a per-date refresh-save policy.
Successful opted-in refreshes transactionally replace the saved record and
schedule active pointer. Failed saves leave fresh memory usable and preserve
the previous saved snapshot. New dates stay session-only. Portable imports
never enable the refresh-save policy. Startup restores the active saved schedule
without fetching NHL, even when season startup fails; restoration sets the date
control to the saved context. Cross-tab notifications refresh both libraries.

Storage failure clears stale saved-status claims while retaining memory and
exports. Retry is explicit. Late saved loads, imports and acquisition/projection
results cannot replace a newly selected context. Season and schedule pointer
writes/removals do not affect each other. The old storage test harness now opens
schema 2 rather than trying to reopen an upgraded database at schema 1.

## Evidence

TypeScript check/build and **85 Node browser tests passed**. New cases cover:

- Additive upgrade preserving season bytes/pointer; old-build version refusal.
- Interrupted migration rollback and successful retry.
- Independent family save/restore/removal, lightweight schedule entries.
- Revision conflicts, concurrent writers and quota rollback of staged writes.
- Envelope integrity/context binding, unsupported schemas, invalid dates/limits.
- Portable package round trip; original corrupt records preserved.
- Opted-in complete refresh replacement, fresh-memory/old-save failure behavior.
- No-network restore with reprojection, new-date session policy, denied storage.
- Late saved-load cancellation, unload/remove independence, import policy reset.

Distribution verification passed 17 shell assets, all 75 public packages through
actual WASM, count-order goldens and schedule projection. Final shell plus
largest single public package: **392,583 bytes gzip**, below the 10 MiB bound.

Actual browser at `http://127.0.0.1:8056/` used the production UI, real
IndexedDB/worker/WASM and fixture schedule responses. Saved a week, enabled the
refresh-save policy, refreshed and reloaded: schedule restored with checked
policy and its date control, alongside an independently saved season. Unloading
the schedule preserved its saved entry; loading that entry restored the rows.
Removing the saved schedule preserved in-memory scores and the season's saved
entry. Saved the schedule again after that test.

The harness quota switch throws on the schedule pointer write inside the real
IndexedDB transaction. Refresh then displayed fresh in-memory data at
`2026-10-04T03:01:21.587Z`, while the saved library retained the prior timestamp
`2026-10-04T02:57:17.976Z`. Reload restored that prior saved revision and rows.
This is synthetic quota injection plus actual browser transaction/recovery
evidence, not a real exhausted disk.

[Saved schedule screenshot](browser-pulse-15.png).
[Quota recovery screenshot](browser-pulse-15-quota.png).

## Remaining acceptance

No deployed live/CORS or relay proof is claimed. Schedule polling, full browser
file-picker/download round-trip, browser migration with populated legacy data,
cross-tab UI acceptance, offline reopening, full memory/mobile checks and Pages
publication/rollback remain open. The download observation API timed out after
the export click, so this pulse does not claim a verified browser-downloaded
backup; deterministic envelope round-trip is verified separately. Fixture
harness index modifications intentionally fail shell integrity; this harness
does not test offline shell installation. Capability remains in progress.
