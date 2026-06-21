# Phase Kraken Admin Inventory

## Purpose

Inventory admin read route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Kraken Admin posture |
|---|---|---|
| HTML admin shell | `GET /admin` | Keep read-oriented operational shell wording with `DataStatusView`, `SnapshotView`, runtime `ConfigView`, scoped POST forms, no-cache-creation, and deferrals. |
| Data status JSON | `GET /api/v1/admin/data-status` | Keep read-only `DataStatusView` wording with root, filters, totals, missing-source empty state, and no cache creation. |
| Snapshot JSON | `GET /api/v1/admin/snapshots` | Keep read-only `SnapshotView` wording with active/selected state, sealed/inactive metadata, totals, and empty state. |
| Config JSON | `GET /api/v1/admin/config` | Keep runtime `ConfigView` wording with web config rows, selected state, and persistent report-toggle warning. |

## Risks to Avoid

- Mounting or implying web data install/remove.
- Adding persistent report-toggle writes to web admin.
- Expanding browser snapshot maintenance beyond activate/delete POST routes.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused admin read tests cover the HTML
   operational shell and JSON ViewModel contracts.
3. Matrix wording. Result: passed; admin read rows now carry scoped
   read/deferral wording.
4. Closeout. Result: passed; Phase Kraken Admin is closed with final route-row
   claims and non-claims recorded.
