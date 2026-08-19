# Phase Devils - Dashboard visual QA gate

> Phase Devils owns the post-Islanders dashboard visual QA gate. It turns
> selected screenshot evidence into a repeatable browser-proof boundary without
> overstating browser breadth, touch behavior, focus order, or full accessibility.

**Created:** 2026-06-20
**Status:** Closed - Phase Devils wrapped on 2026-06-20

---

## Frame

Phase Islanders closed the surface-truth cleanup and recorded selected
desktop/mobile dashboard captures for leaders, poach, fantasy, and team-season
workspaces. Those captures prove selected nonblank browser rendering, but they
do not prove full live-browser interaction, touch/focus behavior, every
workspace, or every responsive overflow edge.

Phase Devils should close that specific proof gap. It should improve the
repeatable dashboard visual QA harness before changing surface claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Devils Goal 1 - Visual QA inventory** | The existing capture script is useful but intentionally narrow. | The wave records current capture coverage, script limits, output artifacts, and non-claims. |
| 2 | **Devils Goal 2 - Capture matrix expansion** | Dashboard workspaces are broader than the four Islanders captures. | A repeatable command captures the representative workspace set across desktop, tablet, and mobile sizes, or fences any unsupported workspace family. |
| 3 | **Devils Goal 3 - Automated artifact checks** | Human screenshot inspection is easy to forget or overstate. | The capture gate validates image existence, dimensions, nonblank pixels, and a minimal page-readiness signal. |
| 4 | **Devils Goal 4 - Responsive/focus fence** | Responsive layout and focus behavior need explicit evidence before promotion. | The phase either adds repeatable keyboard/focus/mobile checks or records them as durable deferrals with no claim promotion. |
| 5 | **Devils Goal 5 - Closeout promotion decision** | The surface matrix needs a clear final claim. | `design/specs/surface-parity.md` says exactly what dashboard browser proof exists and what remains outside the claim. |

---

## Non-goals

- Do not claim every browser engine unless the phase runs more than installed
  Edge/Chrome.
- Do not claim full screen-reader accessibility from screenshots.
- Do not claim every workflow mutation path; dashboard GET navigation remains
  read-only and mutations stay POST-backed.
- Do not promote admin, WP-009 workflow, or Signals cache gates in this phase.

---

## Recommended pulse order

1. **Pulse 01 - Plan and inventory.** Record current capture harness coverage,
   dashboard workspace families, and visual QA gaps.
2. **Pulse 02 - Capture matrix harness.** Expand or wrap the existing capture
   script with an explicit workspace and viewport matrix. Result: passed
   2026-06-20 with desktop captures for home, leaders, goalies, and poach;
   tablet captures for favorites, watchlist, and schedule; and mobile captures
   for fantasy, team-season, and player workspaces.
3. **Pulse 03 - Artifact validation.** Add automated checks for expected files,
   dimensions, nonblank pixels, and route readiness. Result: passed 2026-06-20
   with dashboard shell readiness, exact PNG dimension checks, and sampled
   nonblank pixel checks in `scripts/web-dashboard-capture.ps1`.
4. **Pulse 04 - Responsive/focus decision.** Add focused keyboard/mobile checks
   if feasible, or keep them explicitly deferred. Result: passed 2026-06-20;
   Devils retains representative desktop/tablet/mobile capture evidence and
   keeps keyboard focus order, pointer/touch interaction, and screen-reader
   behavior deferred until a browser automation gate exists.
5. **Pulse 05 - Closeout.** Update the wave, plan, surface matrix, and
   validation notes with the final browser-proof claim. Result: passed
   2026-06-20; no active Devils pulse remains.

---

## Closeout

Phase Devils is complete. The dashboard proof claim is now representative
browser-render evidence for the configured desktop/tablet/mobile matrix in
`scripts/web-dashboard-capture.ps1`, with automated route readiness, screenshot
dimension, and sampled nonblank artifact validation.

The phase intentionally does not claim keyboard focus order, pointer/touch
behavior, screen-reader behavior, every browser engine, or exhaustive responsive
overflow coverage for every dashboard workspace. Those require a future browser
automation or manual visual QA wave.

---

## Validation expectations

- Script changes must run locally with installed Edge/Chrome headless.
- Browser proof must be offline and use `icelines --no-live serve`.
- Planning/doc-only edits use `git diff --check`.
- Child repo commit and push first; TRACKER records only the submodule pointer.
