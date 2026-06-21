# Phase Jets Inventory

## Purpose

Inventory the Transactions route rows before tightening their route wording.

## Current Surface

| Area | Evidence | Jets posture |
|---|---|---|
| Transactions HTML | `/transactions?kind=...&team=...` | Keep `TransactionsView` feed rendering with kind/team filters and shared row projection. |
| Transactions JSON | `/api/v1/transactions?kind=...&team=...` | Keep stable data/meta envelope with active filters, total, out-of-coverage, and earliest-season metadata. |
| Teamless bucket | `LEAGUE` rows in `TransactionsView` | Keep teamless transaction rows represented through the shared contract. |
| Missing source | `l1_transactions_json_missing_source_returns_typed_error` | Keep typed shared error envelope when bundled/snapshot data cannot be loaded. |

## Risks to Avoid

- Claiming transaction mutations, imports, or editing.
- Claiming live source availability beyond bundled/snapshot fallback.
- Treating missing transaction source state as a successful empty feed.
- Broadening Transactions into roster-state or fantasy transaction claims.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused Transactions route tests cover
   success envelope and typed unavailable-source errors.
3. Matrix wording. Result: passed; the two route rows now carry scoped
   Transactions wording.
4. Closeout. Result: passed; Phase Jets is closed with final route-row claims
   and non-claims recorded.
