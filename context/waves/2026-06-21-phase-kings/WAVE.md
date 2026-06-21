# Phase Kings

## Scope

Plan and execute the Home/static route-row wording gate. The wave does not add
new Home or asset behavior; it records the existing `GET /` and
`GET /static/:asset` routes with scoped route evidence.

## Entry Posture

- The League/home preview matrix row already names `HomeView`.
- The root route test locks full HTML, HTML content type, UTF-8 charset,
  template content, and the dashboard handoff.
- The static asset test target locks CSS, JavaScript, SVG, webmanifest, cache
  headers, release ETags, and unknown-asset 404 behavior.
- The route inventory still uses terse `done` wording for the Home and static
  asset rows.

## Goals

1. Inventory the Home/static route rows and evidence.
2. Validate focused root-route and static asset Web route evidence.
3. Tighten route-row wording to scoped route claims.
4. Preserve exact non-claims around runtime behavior changes, browser visual QA,
   and exhaustive asset coverage.
5. Close the phase with final route-row wording recorded.

## Pulse Log

| Pulse | Scope | Result |
|---|---|---|
| 01 | Plan and inventory Phase Kings goals | passed; see `KINGS-INVENTORY.md` and `pulses/pulse-01.md` |
| 02 | Home/static route evidence gate | passed; focused root-route and static asset tests support scoped route wording, see `pulses/pulse-02.md` |
| 03 | Home/static route wording gate | passed; route rows now carry scoped `HomeView` and bundled asset claims, see `pulses/pulse-03.md` |
| 04 | Close Phase Kings | passed; final scoped claims and non-claims recorded, see `pulses/pulse-04.md` |

## Validation Posture

- Planning/doc-only edits use `git diff --check`.
- Evidence gates use focused root-route and static asset Web tests.
- No live network dependency in tests.

## Closeout

Phase Kings is closed. The Home route row now records that `GET /` renders the
full-document `HomeView` preview with top skaters/goalies, UTF-8 HTML, and the
dashboard handoff. The static asset row now records that `GET /static/:asset`
serves bundled CSS, JavaScript, SVG, and webmanifest assets with content types,
cache headers, release ETags, and unknown-asset 404 behavior.

The claim remains bounded. No runtime route behavior changed, and this phase
does not claim browser visual QA, pointer/keyboard coverage, or exhaustive
coverage beyond the mounted bundled assets.
