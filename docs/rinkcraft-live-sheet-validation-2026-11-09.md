# Full-app Rinkcraft sheet and hockey CSS — 2026-11-09

CHG-143 / WP-RINKCRAFT-19. Baseline `9d01c1d`. Owner approved the form and
requested hockey design details through pure CSS. Production changes are limited
to the shared sheet stylesheet and browser host stylesheet; no form markup,
TypeScript/Rust processing, API, classification, source catalog or scoring changes.

## Design

Retain the approved Letter-format, multi-box category layout. Add a cool ice
palette, blue/red header rules, small decorative CSS rink, blue section headings,
capability-card top edges, lightly tinted response boxes and tabular record
counters. Decoration encodes no measured value or assessment. On narrow screens
the rink mark disappears; all labels, capabilities and missingness remain.
Suppress the duplicate browser host title visually while retaining its accessible
label, and remove the host's duplicate top frame. No images/assets or D&D section
labels are introduced on the front.

## Application build and provenance

Build the native CLI from the isolated F: branch using the existing F: target
cache. Run the resulting binary on 127.0.0.1:8772 with explicit research catalog
paths; existing servers/checkouts on other ports remain unchanged.

Copy existing ignored public browser packages/WASM/catalogs from C: into the
isolated worktree, preserving their exact bytes, and run `npm run build --prefix
icelines-browser`. Run the actual distribution on 127.0.0.1:8773. These are full
local applications, distinct from the 8771 generated review snapshots.

Retained WASM SHA256:
`a3f9cac216f8cc8bace2a33b84c1c488a19c2986f1d935bf6b993e18f0b816e4`.
Its original `build-info.json` remains at `b9aaa18` with dirty-tree provenance;
no new full-WASM build is claimed. Current shell hash prefix `c24efc08cf34`:
41 assets, 75 authenticated packages, shell plus largest-package gzip 659,803
bytes. The artifact verifier actually loads the retained WASM and queries it.

## Validation

Native CLI build and browser shell build/verifier pass. Eight focused
profile/sheet unit tests pass. Six fixture cases pass on each native/browser
host stylesheet, including missing catalog and source-only goalie; twelve
exports independently pass pypdf one-page 612×792pt Letter checks with separate
Attributes and Measured Record. Reference comparison still passes all seven
controls and desktop/mobile geometry. Whole styled defense and actual native goalie/browser defense sheets were viewed. Mobile identity fields use two columns so status words remain whole.

The existing actual-WASM app test passes engine/dialog parity, resident regular
season/competition isolation, offline history selectors, missing data and mobile.
The native no-JS app test passes Fox, Kartye, Shesterkin, a bounded history window,
playoff scope, source/missing-player errors and 390px overflow.

New actual-app acceptance passes for all 25 native and all 25 browser profiles. It checks exact
native API/shared-WASM cards, rendered record values, six unrated capabilities,
goalie-specific labels, closed/reachable evidence, keyboard/no-JS comparison,
mobile/offline browser comparison and three themes. Results and screenshots go
to ignored `target/rinkcraft-live-sheet-check/`.

Commands include `cargo build -p icelines-cli --bin icelines --locked`,
`npm run build --prefix icelines-browser`, `node scripts/verify-browser-dist.mjs`,
`node icelines-browser/e2e/rinkcraft-app-sheet.mjs`, and existing fixture/native/
WASM/reference E2Es. Tests use the F: app ports, not the alternative C: build.

No merge, push, hosted build or publication. Software/rendering acceptance does
not validate the proposed capability scales; all new capabilities remain Unrated.


The 25-player app-backed check passed before the final mobile-only identity-grid
polish. The final CSS then passed both six-case host-style fixture runs and the
full browser history/offline smoke again; final native/browser mobile inspection
is recorded with the delivered build. No values or data logic changed between
these runs. The test opens the library panel before season changes, reflecting
its normal close-on-load interaction rather than changing product behavior.

Final delivered-CSS smoke passes actual native no-JS and actual offline WASM at
390px, including one-line status fields and no horizontal overflow. Artifacts:
`target/rinkcraft-live-sheet-check/final-style-smoke.json` and final mobile PNGs.
The final mobile image was visually inspected.
