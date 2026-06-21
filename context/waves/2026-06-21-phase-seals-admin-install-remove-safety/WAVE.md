# Phase Seals Admin Install/Remove Safety

## Scope

Promote web admin data install/remove from deferred, unmounted routes to a
bounded POST-backed safety contract.

## Entry Posture

- `/admin` renders data status, verify, config, snapshot, and game-cache forms.
- Data install/remove were previously described as deferred and unmounted.
- CLI data install can fetch live source data; web install must not inherit that
  behavior.
- Remove needs exact confirmation and path scoping before it is safe to expose.

## Goals

1. Inventory the admin install/remove route contract and non-claims.
2. Add POST-backed JSON and HTML install routes for embedded bundled seasons.
3. Add POST-backed JSON and HTML remove routes scoped to installed season dirs.
4. Update route tests and surface wording to match the mounted safety contract.
5. Close the phase with validation evidence recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Seals goals | passed; see `SEALS-ADMIN-INSTALL-REMOVE-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Admin data install/remove implementation | passed; mounted JSON/HTML routes with bundled-season install and scoped remove, see `pulses/pulse-02.md` |
| 03 | Evidence and route wording | passed; focused admin tests and docs updated, see `pulses/pulse-03.md` |
| 04 | Close Phase Seals | passed; final bounded route claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- `cargo test -p icelines-web --test l1_router admin_data`
- `cargo test -p icelines-web --test l1_router admin`

## Closeout

Phase Seals is closed. Admin install/remove are now mounted only through the
bounded contract: install writes embedded bundled data plus manifest after exact
confirmation, remove deletes only the validated installed season directory after
exact confirmation, and neither route performs arbitrary filesystem mutation or
live release-data fetches.
