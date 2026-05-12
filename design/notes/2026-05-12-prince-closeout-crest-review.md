# Prince closeout CREST review

**Date**: 2026-05-12
**Scope**: Prince of Wales visual-system closeout across TUI, web, CLI, and
markdown/report surfaces.
**Verdict**: PASS WITH NOTES.

## Reviewed evidence

- TUI render contracts: Team/Depth, Goalies, Schedule/Scores, and Poach at
  80x24 and 120x32 via `cargo test -p icelines-cli prince_tui`.
- Web shared style contracts: representative route layout classes for home,
  leaders, player, team/depth, goalies, scores/schedule/playoffs, fantasy, and
  poach via `cargo test -p icelines-web l1_static_css_contains_prince_route_layout_classes`.
- CLI readability contracts: `query leaders`, `query goalies`, and `poach`
  under `NO_COLOR=1` and `COLUMNS=80` via
  `cargo test -p icelines-cli --test prince_cli_visual`.
- Markdown/report contracts: poach report markdown behavior remains covered by
  `cargo test -p icelines-cli l0_poach`.

## Roles review

| Role | Verdict | Notes |
|---|---|---|
| CREST | PASS WITH NOTES | IceLines now has a deliberate visual grammar: shared semantic tokens, tighter TUI scan rhythm, route-level CSS primitives, and 80-column CLI fences. Remaining risk is visual polish that needs real browser screenshots, especially compare/docs/game/favorites/watchlist secondary routes. |
| GLASS | PASS | Representative surfaces preserve readable labels, no-color CLI output, source-state text, and compact table rhythm. |
| BROADCAST | PASS WITH NOTES | Web representative routes are server-rendered and more coherent. Remaining inline styles on non-representative routes should be cleaned in a later maintenance pass. |
| KEEL | PASS | TUI, CLI, web, and report outputs now share named visual meanings instead of one-off renderer styling for the major product paths. |
| HART/TAPE | PASS | Season/type/source truth stays visible where the ViewModels provide it; source completeness is not inferred by renderers. |
| PACE | PASS WITH NOTES | Dense tables are still intentionally dense. The new fences keep them bounded, but future analytics-heavy views should add drilldown instead of widening default output. |

## Known tradeoffs

- Prince closes on practical render/test evidence, not full screenshot
  automation. Browser screenshots remain valuable for Jim Gregory release
  hardening.
- Secondary web routes still contain incidental inline styles. They are outside
  the representative Prince exit set but should be normalized before a major
  public UI push.
- CLI readability now has 80-column coverage for leaders, goalies, and poach;
  team and fantasy subcommands should be added to the same fence when their
  output stabilizes further.

## Closeout decision

Prince of Wales can close. The product no longer relies on taste arguments or
surface-local styling for its major visual paths. Remaining work is release
hardening and broadening the visual fences, which belongs in Jim Gregory and
routine polish slices.
