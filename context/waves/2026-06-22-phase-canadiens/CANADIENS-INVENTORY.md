# Phase Canadiens Inventory

## Roadmap

| Surface | Evidence | Result |
|---|---|---|
| Major-stats order | `design/plans/2026-06-22-phaseCanadiens-major-stats-roadmap.md` | Active roadmap orders strength-state splits, advanced source authority, Signals promotion, shift-data policy, browser QA, packaging, editorial workflows, and freshness authority. |
| Plan index | `design/plans/INDEX.md` | June 22 Canadiens slices are listed as closed under the active Canadiens roadmap. |
| Wave index | `design/waves/PHASES.md` | This wave records the repo-local execution history for the June 22 train. |

## Implemented Slices

| Slice | Evidence | Result |
|---|---|---|
| Strength-state foundation | `design/plans/2026-06-22-phaseCanadiensStrength-*.md`; commits `7e5bd68` through `7aba53f` | Cached NHL play-by-play scoring surfaces now carry normalized strength-state labels, owner-side context, structured split fields, summary rows, and stable HTML/event hooks. |
| Source authority | `design/plans/2026-06-22-phaseCanadiensSource-*.md`; commits `6803cdd` through `5b36a60` | Scoring, Tonight Intel, outlook, streak, MoneyPuck, goalie-source, and data-status surfaces expose bounded source authority and blocked adjacent claims. |
| Signals authority | `design/plans/2026-06-22-phaseCanadiensSignals-*.md`; commits `4a75793` through `f286d2c` | Signals CLI/Web/Markdown/roster authority copy is shared without cache, catalog, filter, leaderboard, or `StatId` promotion. |
| Shift policy | `design/plans/2026-06-22-phaseCanadiensShifts-*.md`; commits `ef48e4e` through `70d9875` | `sync.capabilities.shifts=off` is reaffirmed across config, docs, CLI mates fallback, and TUI handoffs. |
| Browser labels | `design/plans/2026-06-22-phaseCanadiensBrowser-*.md`; commits `d449e51` through `fef458d` | Dashboard action, pinned, target, score, and ring links carry selected accessible labels with focused route coverage. |
| VTRACE closeout docs | commit `d0712a0` | Communications strategy and trace/spec/review rows reflect the completed source-authority train. |

## Non-Claims

- No new public leaderboard, ranking, or filter contract is promoted.
- No Signals metric enters analytics cache, `StatId`, or public leaderboards.
- No goalie xGA, GSAx, or high-danger save percentage source is claimed.
- No shift-data fetch, bundle, historical join, or deployment-source capability
  is unlocked.
- Browser labels are selected route evidence, not full cross-browser, touch,
  focus, or screen-reader certification.

## Validation Posture

The individual plan files list their focused validation commands. This backfill
wave is documentation-only and should pass:

```powershell
git diff --check
```
