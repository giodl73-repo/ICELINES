# Phase Maple Leafs Inventory

## Purpose

Inventory Career/cohort leaders before deciding whether the partial status
should remain a deliberate TUI handoff or be promoted through a dedicated TUI
board.

## Current Surface

| Item | Evidence | Maple Leafs posture |
|---|---|---|
| CLI command | `query career --league ...` | Canonical cohort table and JSON surface through `CareerView`. |
| TUI command bar | `:career league=OHL season=20142015 top=8` | Deliberate handoff that flashes exact CLI and Web targets; not a native board. |
| Web route | `/career` | Canonical HTML cohort table through shared page shell. |
| JSON route | `/api/v1/career` | Canonical data/meta envelope through `CareerView`. |
| Dashboard workspace | `/dashboard?workspace=/career?...` | Summary shell routes users to the canonical `/career` page. |
| Cold-store guidance | `~/.icelines/career_history.json` missing path | Must show explicit `icelines fetch career --bundled-seasons 5` instruction. |

## Promotion Blockers

- Do not imply career-history data is bundled on cold install.
- Do not fetch live career-history data from Web or TUI read surfaces.
- Do not add a TUI board unless it provides fields or workflows beyond the
  canonical CLI/Web cohort table.
- Do not mark Career/cohort leaders fully done while the TUI surface remains a
  handoff.

## Recommended Pulse Map

1. Plan and inventory. Result: passed.
2. Evidence gate. Run focused CLI/TUI/Web career tests and record whether the
   handoff remains sufficient. Result: passed; focused evidence supports
   deliberate TUI handoff.
3. Surface-matrix wording. Tighten partial wording if evidence shows the
   deliberate handoff needs clearer boundaries.
4. Closeout. Record the final Career/cohort leaders decision.
