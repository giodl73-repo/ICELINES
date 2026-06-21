# Phase Jets

## Scope

Plan and execute the Transactions route-row wording gate. The wave does not add
new transaction behavior; it records the existing `/transactions` and
`/api/v1/transactions` routes with scoped `TransactionsView` wording.

## Entry Posture

- The Transactions family row already names `TransactionsView`.
- CLI, TUI, Web HTML, and Web JSON row projection share the same contract.
- The shared contract handles the `LEAGUE` teamless bucket.
- Web JSON tests cover successful data/meta envelopes and typed unavailable
  source errors.
- The route inventory still uses short project wording for Transactions rows.

## Goals

1. Inventory the Transactions route rows and evidence.
2. Validate focused Transactions route evidence.
3. Tighten route-row wording to scoped `TransactionsView` and envelope claims.
4. Preserve exact non-claims around mutation, live-source guarantees, and
   roster/fantasy transaction behavior.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Jets goals | passed; see `JETS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Transactions route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Transactions route wording gate | passed; rows now carry scoped `TransactionsView` wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Jets | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused Transactions Web route tests.
- No live network dependency in tests.

## Closeout

Phase Jets is closed. Transactions route rows now record `TransactionsView`
projection, kind/team filters, `LEAGUE` teamless bucket handling, data/meta
metadata, and typed unavailable-source errors.

The claim remains bounded. The rows do not promise mutation, import/editing
workflows, live-source availability, or roster/fantasy transaction behavior.
