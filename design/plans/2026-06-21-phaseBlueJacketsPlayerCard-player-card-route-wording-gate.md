# Phase Blue Jackets Player Card - Player card route wording gate

> Phase Blue Jackets Player Card records the player-card HTML and JSON routes
> with precise ViewModel, chart, envelope, and adjacent-route boundaries.

**Created:** 2026-06-21
**Status:** Closed - Phase Blue Jackets Player Card complete

---

## Frame

The player-card routes already project `PlayerCardView`. Phase Blue Jackets
Player Card tightens the route matrix so the rows name headshot fallback,
Signals handoff, aligned profile fields, career metric/chart evidence, JSON row
identity, shared error envelopes, no shared-repo career-window mutation on read,
and adjacent Signals/scoring/streaks/scouting non-claims.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Blue Jackets Player Card Goal 1 - Route inventory** | Player-card rows should name ViewModel evidence and adjacent-route boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Blue Jackets Player Card Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused player HTML/JSON tests pass. |
| 3 | **Blue Jackets Player Card Goal 3 - Scoped route wording** | Existing rows are accurate but terse for row identity, chart, and error-shape claims. | Rows name `PlayerCardView`, headshot/signals/chart evidence, no-mutation behavior, and shared envelopes. |
| 4 | **Blue Jackets Player Card Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change player runtime behavior.
- Do not promote Signals, scoring, streaks, or scouting route behavior into the player-card rows.
- Do not add live fetch or local-store creation from player-card reads.
- Do not mutate shared repository career windows from JSON reads.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused player-card tests passed.
3. **Pulse 03 - Matrix wording.** Result: player-card rows now carry scoped wording.
4. **Pulse 04 - Closeout.** Result: Phase Blue Jackets Player Card is closed with final route-row claims and non-claims recorded.

---

## Closeout

Phase Blue Jackets Player Card closed the player-card route wording gate. The
rows now record `PlayerCardView` HTML/JSON projection, headshot fallback,
Signals handoff, career metric/chart evidence, row identity, shared error
envelopes, no shared-repo career-window mutation on read, and adjacent-route
non-claims.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused player-card Web route tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
