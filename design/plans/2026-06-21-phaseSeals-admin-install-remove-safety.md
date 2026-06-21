# Phase Seals - Admin install/remove safety

> Phase Seals promotes web data install/remove from an unmounted deferral to a
> bounded confirmation-backed admin mutation contract.

**Created:** 2026-06-21
**Status:** Closed - Phase Seals complete

---

## Frame

Admin already exposes safe POST-backed config, data verify, snapshot, and
game-cache operations. Phase Seals adds a narrow web contract for data
install/remove without importing live-fetch behavior into the browser surface:
install writes embedded bundled season files only, and remove deletes only the
validated installed season directory.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Seals Goal 1 - Contract inventory** | The old matrix treated install/remove as deferred/unmounted. | A wave inventory records the new bounded route contract and non-claims. |
| 2 | **Seals Goal 2 - Install route safety** | Browser install must not trigger live source fetches. | JSON/HTML routes validate bundled seasons, exact confirmation, and write only embedded bundle files plus manifest. |
| 3 | **Seals Goal 3 - Remove route safety** | Browser remove is destructive and must stay path-scoped. | JSON/HTML routes validate YYYYZZZZ season ids, exact confirmation, and remove only `~/.icelines/seasons/<season>`. |
| 4 | **Seals Goal 4 - Evidence and wording** | Route claims must match tested behavior. | Focused admin route tests pass and surface docs name the bounded contract. |

---

## Non-goals

- Do not perform live source fetches from web install.
- Do not install non-bundled seasons from the browser.
- Do not remove arbitrary filesystem paths.
- Do not add persistent report-toggle writes.

---

## Closeout

Phase Seals is closed. Web admin install/remove routes are mounted and scoped:
install writes embedded bundled regular/playoff files and a SHA-256 manifest
under `~/.icelines/seasons/<season>/bundle-<season>` after exact
`INSTALL <season>` confirmation; remove deletes only
`~/.icelines/seasons/<season>` after exact `REMOVE <season>` confirmation.

---

## Validation Expectations

- Route changes use focused `icelines-web --test l1_router admin` tests.
- Docs and matrix updates use `git diff --check`.
- Child repo commit and push first; TRACKER records only the submodule pointer.
