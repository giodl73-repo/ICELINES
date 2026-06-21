# Phase Kings Static Assets - Static asset route wording gate

> Phase Kings Static Assets records the bundled `/static/:asset` HTTP contract
> with precise asset, header, cache, manifest, and non-filesystem boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Kings Static Assets complete

---

## Frame

The static asset route already serves a fixed set of compiled-in web assets.
Phase Kings Static Assets tightens the route matrix so the row names bundled
asset-name dispatch, per-asset content types, immutable one-year cache headers,
release-version strong ETags shared across assets, PWA manifest metadata, guarded
dashboard/layout CSS primitives, unknown-asset 404 behavior, and filesystem
static-mount non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Kings Static Assets Goal 1 - Route inventory** | The static row should name exact bundled assets and HTTP boundaries. | A wave inventory names route evidence and non-claims. |
| 2 | **Kings Static Assets Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused static asset tests pass. |
| 3 | **Kings Static Assets Goal 3 - Scoped route wording** | Existing row is accurate but terse for static serving safety. | Row names asset list, content types, cache headers, ETags, manifest/layout guards, and non-claims. |
| 4 | **Kings Static Assets Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not claim filesystem-backed static serving.
- Do not claim directory listing or extension-based fallback behavior.
- Do not add new assets.
- Do not change runtime behavior.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused static asset tests passed.
3. **Pulse 03 - Matrix wording.** Result: static asset row now carries scoped
   bundled-asset wording.
4. **Pulse 04 - Closeout.** Result: Phase Kings Static Assets is closed with
   final route-row claims and non-claims recorded.

---

## Closeout

Phase Kings Static Assets closed the static asset route wording gate. The row
now records bundled-name dispatch for CSS/JS/SVG/webmanifest assets, per-asset
content types, immutable cache headers, release-version shared strong ETags, PWA
manifest metadata, dashboard/layout CSS guards, unknown-asset 404 behavior, and
no filesystem static-mount claim.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run `cargo test -p icelines-web --test l1_static`.
- Child repo commit and push first; TRACKER records only the submodule pointer.
