# Visual capture inventory

Pulse 04 reviewed the Prince visual-system evidence through CREST, broadcast,
GLASS, and TAPE.

## Existing visual fences

| Surface | Evidence | Coverage |
|---|---|---|
| TUI | `cargo test -p icelines-cli prince_tui` | Renders representative Team, Goalies, Schedule, and Poach surfaces at 80x24 and 120x32; 2 matching tests ran. |
| CLI | `cargo test -p icelines-cli --test prince_cli_visual` | Runs `query leaders`, `query goalies`, and `poach` with `NO_COLOR=1` and `COLUMNS=80`; 3 matching tests ran. |
| Web CSS/layout | `cargo test -p icelines-web l1_static_css_contains_prince_route_layout_classes` | Pins the shared route layout class vocabulary; 1 matching test ran. |
| Web screenshots | `powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 web-captures` | Builds release CLI, starts local `serve`, and captures dashboard screenshots with installed Edge/Chrome. |

## Generated captures

The browser tooling was available. The capture gate wrote generated screenshots
to `dist/web-dashboard-captures/`:

| Capture | Viewport | Surface |
|---|---:|---|
| `dashboard-leaders-desktop.png` | 1440x900 | Dashboard workspace with leaders table. |
| `dashboard-poach-desktop.png` | 1440x900 | Dashboard workspace with poach/fantasy decision board. |
| `dashboard-fantasy-mobile.png` | 390x844 | Mobile dashboard fantasy workspace. |
| `dashboard-team-season-mobile.png` | 390x844 | Mobile dashboard team-season workspace. |

These files are generated under `dist/` and are not committed source artifacts.
The durable wave evidence is this inventory plus the capture command and paths.

## CREST notes

| Surface | Verdict | Notes |
|---|---|---|
| Dashboard | PASS WITH NOTES | Desktop captures exercise the MDI shell with a central workspace, context panes, and sticky command bar; remaining polish is secondary-route screenshot breadth. |
| Fantasy/poach | PASS WITH NOTES | Poach desktop and fantasy mobile captures preserve the decision-first action/evidence/risk hierarchy without requiring color as the only cue. |
| Team season | PASS WITH NOTES | Mobile team-season capture exercises compact route embedding and context preservation after Presidents Trophy parity work. |
| Reports/markdown | PASS WITH NOTES | Report/markdown capture remains represented by Prince.5 `l0_poach` markdown behavior and the visual-system report hierarchy rules; no new browser-only report screenshot was required for this pulse. |

## Backcheck result

No browser-tooling blocker was hit. Pulse 04 leaves the Prince closeout verdict
as `PASS WITH NOTES` and records concrete capture regeneration commands for
future visual regressions.
