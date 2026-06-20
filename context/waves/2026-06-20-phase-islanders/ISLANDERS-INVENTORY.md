# Phase Islanders Inventory

## Current surface-parity posture

| Area | Current state | Islanders disposition |
|---|---|---|
| Surface parity matrix header | `design/specs/surface-parity.md` still says `Draft - Campbell seed, Ted Lindsay owns web verification`, while many rows have later VTRACE evidence. | Refresh status and add an active-partials rollup before moving individual claims. |
| Admin operations | `/admin` and admin JSON routes exist; safe POST-backed config, snapshot, data verify, and game-cache warmer paths are mounted and tested. Web data install/remove and persistent report-toggle writes remain deferred. | Keep dangerous operations deferred unless a safer contract exists; make the matrix and docs explicit. |
| Docs route | `/docs` renders `COMMANDS.md` through `DocsView`; TUI overlay and menu docs paths exist. | Verify docs route/menu wording does not advertise stale mkdocs/static-site or unimplemented operations. |
| Dashboard partials | Workspace partial route tests exist and `scripts/test-slice.ps1 web-captures` can produce desktop/mobile captures. | Decide whether Islanders records selected capture evidence or keeps live capture deferred. |
| Cache-backed surfaces | WP-009 first-route Web/API evidence exists for named cache report, coach dashboard, opponent scout, player evidence card, line explorer, goalie readiness, practice focus, postgame review, postgame adjustments, and agent evidence. | Roll up first-route evidence separately from broader workflow completion. |
| Signals promotion | Rangers explicitly kept `signals-roster` outside analytics cache until a separate gate. | Out of scope for Islanders unless a new Signals cache-promotion gate is opened. |
| Lean CLI | Rangers recorded target-not-met audit for FLETCH/SLICE and missing `cli` feature. | Out of scope for Islanders; keep as future dependency wave. |

## Risks to avoid

- Treating a selected route test as full browser interaction evidence.
- Treating first cache consumer routes as broad coaching/scouting workflow
  completion.
- Making web admin install/remove casual or GET-backed.
- Advertising persistent report toggles from web when the durable CLI/TUI config
  contract is not shared.
- Reviving stale mkdocs/static-site claims from older docs.

## Pulse map

1. Inventory and plan.
2. Surface parity matrix refresh.
3. Admin/docs truth pass.
4. Dashboard proof or explicit deferral.
5. Cache-backed partial rollup.
6. Phase closeout.
