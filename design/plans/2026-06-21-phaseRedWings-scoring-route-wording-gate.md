# Phase Red Wings - Scoring route wording gate

> Phase Red Wings records scoring report, outlook, and tonight-intel route rows
> with precise scoped wording.

**Created:** 2026-06-21
**Status:** Closed - Phase Red Wings complete

---

## Frame

The scoring report rows are already accurate about cached official NHL
play-by-play sources and no local cache creation. Phase Red Wings tightens those
rows into a single route gate covering game/player/team scoring, player/team
outlook, and tonight intel without changing runtime behavior.

---

## Goals

| # | Goal | Why it matters | Acceptance signal |
|---|---|---|---|
| 1 | **Red Wings Goal 1 - Route inventory** | Scoring rows should name ViewModels, source-state, cache recovery, and no-cache-creation boundaries. | A wave inventory names route rows, evidence, and non-claims. |
| 2 | **Red Wings Goal 2 - Evidence gate** | Wording changes need current route proof. | Focused Rocket/scoring route tests pass. |
| 3 | **Red Wings Goal 3 - Scoped route wording** | Existing wording is split across scoring, outlook, and tonight rows. | Route rows name scoring ViewModels, cached play-by-play, outlook pace SVGs, favorites-first intel, and no local cache creation. |
| 4 | **Red Wings Goal 4 - Closeout** | The route inventory should carry final scoped claims. | Phase closeout records final wording and non-claims. |

---

## Non-goals

- Do not change scoring runtime behavior.
- Do not include game detail, streak, analytics-cache, or fantasy rows.
- Do not claim live play-by-play fetches from GET navigation.
- Do not create local cache state from read navigation.

---

## Recommended Pulse Order

1. **Pulse 01 - Plan and inventory.** Result: passed.
2. **Pulse 02 - Evidence gate.** Result: focused scoring route tests passed.
3. **Pulse 03 - Matrix wording.** Result: route rows now carry scoped scoring
   route wording.
4. **Pulse 04 - Closeout.** Result: Phase Red Wings is closed with final
   route-row claims and non-claims recorded.

---

## Closeout

Phase Red Wings closed the Scoring route wording gate. The route rows now record
`GameScoringReportView`, `PlayerScoringProfileView`, `TeamScoringProfileView`,
`PlayerScoringPaceView`, `TeamScoringOutlookView`, and
`TonightScoringIntelView` boundaries, including cached play-by-play
source-state, POST-backed cache-load recovery, outlook pace SVGs, favorites-first
intel, and no local cache creation from GET navigation.

---

## Validation Expectations

- Planning/doc-only edits use `git diff --check`.
- Evidence gates run focused Rocket/scoring route tests.
- No live network dependency in tests.
- Child repo commit and push first; TRACKER records only the submodule pointer.
