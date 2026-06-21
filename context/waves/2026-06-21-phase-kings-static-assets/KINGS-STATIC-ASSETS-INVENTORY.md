# Phase Kings Static Assets Inventory

## Purpose

Inventory the static asset route row before tightening its route wording.

## Current Surface

| Area | Evidence | Kings Static Assets posture |
|---|---|---|
| Asset list | `GET /static/:asset` | Keep fixed bundled asset-name dispatch for CSS, JavaScript, SVG, and webmanifest files. |
| HTTP headers | static L1 tests | Keep content type, immutable cache-control, and release-version ETag claims. |
| Manifest/layout | static L1 tests | Keep PWA manifest metadata and guarded dashboard/layout CSS primitives. |
| Missing asset | static L1 tests | Keep unknown asset 404 behavior and no extension fallback. |

## Risks to Avoid

- Claiming filesystem-backed static serving.
- Claiming directory listing.
- Claiming extension-based fallback.
- Claiming new assets.
- Changing runtime behavior while performing a wording gate.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused static tests cover content types,
   cache headers, ETags, manifest metadata, CSS guard tokens, and unknown 404s.
3. Matrix wording. Result: passed; static asset row now carries scoped
   bundled-asset wording.
4. Closeout. Result: passed; Phase Kings Static Assets is closed with final
   route-row claims and non-claims recorded.
