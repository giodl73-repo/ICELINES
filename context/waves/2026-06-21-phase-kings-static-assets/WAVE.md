# Phase Kings Static Assets

## Scope

Plan and execute the static asset route-row wording gate. The wave does not add
runtime behavior; it records existing `/static/:asset` HTTP evidence.

## Entry Posture

- `/static/:asset` serves a fixed set of bundled assets by asset name.
- Assets include `style.css`, `htmx.min.js`, `dashboard.js`, `icelines.svg`, and
  `site.webmanifest`.
- Responses carry per-asset content types, immutable cache headers, and
  release-version strong ETags.
- Unknown assets return 404.
- The route is not a filesystem-backed static mount.

## Goals

1. Inventory static asset route evidence.
2. Validate focused static asset route evidence.
3. Tighten route-row wording to bundled asset list, content types, cache
   headers, release ETags, manifest metadata, layout CSS guards, unknown 404s,
   and filesystem static-mount non-claims.
4. Preserve exact non-claims around directory listing, extension fallback, new
   assets, and runtime behavior changes.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Kings Static Assets goals | passed; see `KINGS-STATIC-ASSETS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Static asset route evidence gate | passed; focused route tests support scoped wording, see `pulses/pulse-02.md` |
| 03 | Static asset route wording gate | passed; row now carries scoped bundled-asset wording, see `pulses/pulse-03.md` |
| 04 | Close Phase Kings Static Assets | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused static asset route tests.
- No runtime behavior changes are part of this gate.

## Closeout

Phase Kings Static Assets is closed. The static asset row now records
bundled-name dispatch for CSS/JS/SVG/webmanifest assets, per-asset content
types, immutable cache headers, release-version shared strong ETags, PWA
manifest metadata, dashboard/layout CSS guards, unknown-asset 404 behavior, and
no filesystem static-mount claim.

The claim remains bounded. The row does not promote filesystem static serving,
directory listing, extension fallback, new assets, or runtime behavior changes.
