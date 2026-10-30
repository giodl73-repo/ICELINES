# Pulse 18 — Responsive containment and keyboard accessibility

Status: selected layout and keyboard acceptance demonstrated; broader mobile
and accessibility release acceptance remains open.

## Change

Leaders and schedule tables now have named, focusable scroll regions, named
tables and visible horizontal-scroll/arrow-key help. The skip-link target is
focusable and has space above it for the sticky context header. Player dialogs
have an accessible name and initial focus on Close. Player buttons have 44px
minimum height. Checkbox labels use an aligned row with a 44px target area;
checkboxes no longer stretch across the entire label. Long provenance and saved
library text wrap; controls and table overflow stay within their containers.
Dialog height is bounded with internal scrolling. Muted text is darker.

## Evidence

Final TypeScript build and `npm run check` passed. Distribution verification
passed **20 shell assets**, four count goldens and all **75 catalog packages**
using actual WASM. Shell plus largest package gzip sum: **397,042 bytes**.
No domain/Rust behavior changed; the prior 99-test result remains scoped to the
query/persistence/acquisition code it exercised.

The host browser's documented viewport override accepted a 360x800 request but
continued rendering at measured width 1280, including a newly opened tab. The
override was reset. `scripts/prepare-browser-responsive-preview.mjs` therefore
embeds the unmodified production distribution, served at localhost:8057, in a
360px frame served at localhost:8056. This is a real narrow browsing context,
not a physical phone, touch simulation or synthetic media-query override.

The production UI loaded the public 2024-25 regular package and the bookmarked
`p>=100` Points query via its normal route/worker/WASM path. Six rows rendered.
Before the vertical scrollbar appeared, document client/scroll widths were
360/360; populated state measured 345/345. The 15px scrollbar accounts for that
change. Leaders scroll region measured client width 313 and scroll width 465,
with tabindex 0: wide table content remains in that region and does not widen
the page. Header context remains visible during vertical scrolling. Cold and
loaded layout were inspected. The first inspected localhost:8055 server served
an older preview; it was excluded from current evidence, and a new current-dist
server was started at localhost:8057.

Iframe locator click and key input failed in the automation backend; vertical
scroll worked. No mobile arrow-key or dialog-interaction pass is inferred from
those failures. Top-level current production distribution independently proved:

- Enter on Skip to analysis focuses the `analysis` section.
- Tab from the named leaders table region focuses the first player button.
- Enter on Nikita Kucherov opens a dialog named Nikita Kucherov.
- Initial dialog focus is the Close button.
- Escape closes it and returns focus to the initiating player button.

Color luminance calculation gives muted text contrast 5.80:1 against the page
background and 6.07:1 against white; links on white give 5.75:1. These calculations
are selected-color evidence, not a complete rendered contrast/a11y audit.

[360px loaded layout](browser-pulse-18-mobile.png).
[Keyboard-opened player dialog](browser-pulse-18-dialog.png).
Screenshots show the responsive/accessibility changes before the final additional
skip-target scroll-margin rule; the final build/verifier includes that rule.

## Remaining acceptance

Physical mobile/touch behavior, keyboard horizontal scrolling, screen-reader
coverage, all mobile failure/loading/recovery states and the compact analysis-first
composition review remain open. Deployed live access or allocated relay, offline
reopening, browser file interchange, legacy migration, resource measurements and
Pages deployment/rollback also remain required. Selected layout evidence does
not close WP-BW-05 wholesale.
