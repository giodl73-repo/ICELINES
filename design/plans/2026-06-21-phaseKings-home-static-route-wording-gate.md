# Phase Kings - Home/static route wording gate

> Phase Kings records the Home and static asset route rows with precise scoped
> wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Kings complete

---

## Frame

The League/home preview row already names `HomeView`, and the route tests cover
the root HTML shell. Static assets are also covered by their own L1 test target
for CSS, JavaScript, SVG, webmanifest, cache headers, ETags, and missing assets.

The remaining issue is route-row precision. The route inventory still uses
terse `done` wording for `GET /` and `GET /static/:asset`. Phase Kings tightens
those rows without changing route behavior or claiming browser coverage beyond
the existing HTTP asset tests.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Kings Goal 1 - Route inventory** | Core route wording should name the exact ViewModel and asset-contract evidence. | A wave inventory names route rows, evidence, and blockers. |
| 2 | **Kings Goal 2 - Evidence gate** | Wording changes need current focused route proof. | Focused Home and static asset tests pass. |
| 3 | **Kings Goal 3 - Scoped route wording** | Terse `done` wording hides the root HTML and static-asset boundaries. | Route rows name `HomeView`, full-document HTML, asset types, cache headers, ETags, and 404 behavior. |
| 4 | **Kings Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change Home or static asset runtime behavior.
- Do not add browser automation or visual QA claims.
- Do not expand the Home row into all dashboard or leaderboard behavior.
- Do not claim exhaustive asset coverage beyond the mounted bundled assets.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused root-route and static asset Web
   tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped
   Home/static route wording.
4. **Pulse 04 - Closeout.** Result: Phase Kings is closed with final route-row
   claims and non-claims recorded.

---

## Closeout

Phase Kings closed the Home/static route wording gate. `GET /` now records the
full-document `HomeView` preview contract with the dashboard handoff, and
`GET /static/:asset` records the bundled CSS/JS/SVG/webmanifest asset contract
with cache headers, release ETags, and unknown-asset 404 behavior.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused root-route and static asset Web route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
