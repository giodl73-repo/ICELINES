# Phase Flyers

## Scope

Plan and execute the admin operation safety gate left after Phase Islanders and
Phase Devils. The wave focuses on web admin install/remove deferrals,
persistent report-toggle deferrals, and the route/test fences that keep admin
mutations safe.

## Entry posture

- Phase Devils is wrapped as of 2026-06-20.
- The active surface matrix marks Admin operations partial.
- `/admin` and admin JSON routes expose safe runtime config, data verify,
  snapshot activate/delete, and game-cache warmer operations.
- Web data install/remove remain unmounted and explicitly deferred.
- Persistent report-toggle writes remain deferred to CLI/TUI because web runtime
  config does not write `~/.icelines/config.toml`.

## Goals

1. Inventory current admin routes, tests, and deferrals.
2. Decide whether web data install/remove stay deferred or get a small safe
   contract.
3. Decide whether persistent report toggles stay deferred or get a shared
   durable config contract.
4. Preserve or strengthen focused admin route safety gates.
5. Close the phase with exact surface-matrix wording.

## Pulse log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Flyers goals | passed; see `FLYERS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Decide web data install/remove boundary | passed; install/remove stay deferred and unmounted, see `pulses/pulse-02.md` |
| 03 | Decide persistent report-toggle boundary | passed; report-toggle writes stay deferred to CLI/TUI durable config, see `pulses/pulse-03.md` |
| 04 | Run focused admin safety regression gate | passed; `l1_admin_` route family covers deferrals and safe mutations, see `pulses/pulse-04.md` |
| 05 | Close Phase Flyers and update surface matrix | passed; admin operations stay partial with explicit durable deferrals, see `pulses/pulse-05.md` |

## Closeout

Phase Flyers is closed. Web admin keeps its implemented safe mutation surface:
runtime active-season config, data verify, game-cache warmers, and snapshot
activate/delete. Web data install/remove remain deferred and unmounted, and
persistent report-toggle writes remain a CLI/TUI durable config handoff.

The active surface matrix now names these as intentional durable deferrals rather
than ambiguous missing admin work.

## Validation posture

- Planning/doc-only edits use `git diff --check`.
- Admin route changes use focused `icelines-web --test l1_router` tests.
- No live network dependency in tests.
