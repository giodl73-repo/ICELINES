# Phase Kings Inventory

## Purpose

Inventory the Home/static route rows before converting terse `done` wording into
scoped route evidence.

## Current Surface

| Area | Evidence | Kings posture |
|---|---|---|
| Home HTML | `GET /` | Keep full-document `HomeView` preview rendering with top skaters/goalies, UTF-8 HTML, and dashboard handoff. |
| Static CSS | `GET /static/style.css` | Keep bundled text/css asset serving with cache headers and release ETag. |
| Static JavaScript | `GET /static/htmx.min.js`, `GET /static/dashboard.js` | Keep bundled JavaScript asset serving with cache headers and release ETag. |
| Static image/manifest | `GET /static/icelines.svg`, `GET /static/site.webmanifest` | Keep bundled SVG and webmanifest serving with expected content types. |
| Missing asset | `GET /static/unknown.js` | Keep unknown bundled asset requests at 404. |

## Risks to Avoid

- Claiming new Home or static asset runtime behavior.
- Expanding the Home row into dashboard workspace or leaderboard route claims.
- Claiming browser visual QA, screen-reader, pointer, or keyboard coverage.
- Claiming exhaustive static asset coverage beyond mounted bundled assets.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Result: passed; focused root-route and static asset tests support
   scoped route wording.
3. Matrix wording. Result: passed; the two route rows now carry scoped
   `HomeView` and bundled static asset wording.
4. Closeout. Result: passed; Phase Kings is closed with final route-row claims
   and non-claims recorded.
