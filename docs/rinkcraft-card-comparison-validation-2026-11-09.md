# Explicit card comparisons and class context validation — 2026-11-09

CHG-139 / WP-RINKCRAFT-15. UI source baseline `83b1449` plus working tree.
Contract: `design/specs/rinkcraft-card-comparison.md`. Rinkcraft decision 0019 /
producer-applied ten-lens self-review govern interpretation. Prior source,
attribute and contrast protocols remain unchanged; no new quantitative method.

## Change and input boundaries

Primary numerical cards offer an explicit same-source-season/group counterpart
choice. Native uses a required no-JS GET form; browser carries selected IDs and
year through the existing comparison event. Choices describe availability in the
showcase, not similarity, role fit, ranking or coaching advice. Older direct
renderers without choice metadata remain compatible; primary app cards use the
new explicit flow. A same-season native browse link remains separately available.

Comparison views retain both documentary identities beside the numbers: primary
and secondary classes, fantasy analogy, specialty, class purpose and expandable
source/evidence/assessment dates and catalog hash. Existing shared class selectors
use the numerical comparison year. Unsupported periods stay Unscouted; absent or
invalid class catalogs are unavailable. Dates can be retrospective, explicitly
shown. Numerical gaps do not establish class fit or individual mechanisms.

Optional class failure no longer blocks primary numerical profiles either. Native
preserves the profile with nullable class data; HTML and browser availability
metadata distinguish source failure from an unsupported player/period. Statistics
and comparison controls remain available. No core/API schema, numerical comparison
payload, source/class/measurement/rank, history or summary-policy change. No new
source acquisition; the three-season archive (74 player-seasons, 1,020 measures),
legacy 2024–25 catalog and authored class inputs are unchanged.

## Build and provenance

Native CLI/web build, web-library Clippy with warnings denied, TypeScript,
format/diff checks and five focused frontend tests passed. Browser JS/CSS build
and actual package/WASM verification passed: 40 authenticated shell assets,
75 packages, shell hash prefix `f560a04e39b2`, summed gzip shell 514,105 bytes;
shell plus largest package 649,626 bytes, below the unchanged 10 MiB budget.

Shared core/WASM source was not changed in this pass. The existing CHG-138 WASM
artifact is reused: SHA256
`5cdec9377d3d8c13068242c3193222ab14b0670d9525544a86578667c2321f69`.
Its retained build-info records parent `22e9fed` plus dirty working tree before
the CHG-138 `83b1449` source commit (Rust 1.95.0 / wasm-bindgen 0.2.118). Current
native/HTML/TS/CSS is based on `83b1449` plus this working tree. No fresh WASM,
clean final-commit rebuild, publication or hosted CI claim is made.

## Executed acceptance

- `rinkcraft-card-comparison.mjs`: ten native picker lists exactly match shared
  source-year/group candidates, excluding self; browser covers forwards, defenders
  and goalies. Required choice prevents an empty handoff. Explicit peer replaces
  any previous/default partner, and both selected IDs/year reach the comparison.
- Barkov's older primary fallback carries 2024–25, with classes selected for that
  source year. Marner's unsupported 2024–25 class is Unscouted while numerical
  data remains usable. Supported class names/specialties/dates and catalog hash
  match the existing shared selector. Retrospective dates remain inspectable.
- Browser 404 and malformed class assets retain numerical comparisons and primary
  cards (six facets and explicit counterpart controls). Unavailable catalogs are
  not labeled as individual Unscouted records. A delayed class-source test changes
  the requested counterpart before release; no old counterpart overwrites the new
  comparison. Version/generation guards also prevent abandoned card handoffs.
- Separate managed loopback native server with a configured nonexistent class
  source retained both comparison data and the primary six-facet card/picker.
  Its generic unavailable state exposes no local path. Verification JSON retained;
  auxiliary port 8767 was stopped after the check.
- Native no-JS, browser offline, legacy consumers, 390 px layouts and class-panel
  text contrast >= 4.5:1 across all three themes passed. Native and browser
  screenshots inspected: class identity stays compact, adjacent numerical gaps
  retain raw/rank values and limits, and full tables remain present.
- Existing latest-source regression passed all 25 exact native/WASM snapshots,
  bounds/fallback/exact/missing/legacy semantics and period handoff. Integrated
  numerical comparison regression passed three exact frozen pairs, temporal/
  playoff/unscouted and offline/mobile behavior. Contrast regression passed 133
  compatible latest-source pairs, nine exact native/WASM comparisons, coverage/
  definition guards, legacy/no-JS/offline/mobile and three themes. Attributes-first
  regression passed all 25 preserved vectors, nine exact native/WASM cards,
  first-screen placement, feature links and missing transport evidence.

The new test waits for asynchronous picker options before inspecting them, and
uses the existing Show all players control when opening rows from the virtualized
list. Earlier timing/offscreen-row failures are not counted as passes. The source-
availability inconsistency found during review was fixed in the application and
rechecked in both consumers.

Ignored results/screenshots: `target/rinkcraft-card-comparison-check/`; logs and
native optional-source result: `target/rinkcraft-card-comparison-*`. Research run
retains unchanged input hashes, commands, results and actual source/build records.
Deterministic UI/selection work; seed/trials/predictive split N/A. Software checks
are not empirical role, style, skill, chemistry or class-fit validation. Latest
supplied numerical season remains 2025–26, not live 2026–27.

Final player-history regression passed: four current/resident regular/playoff
windows, exact values, unchanged active window, stale-dialog closure, missing-not-
zero, goalie values, offline reads and 390 px. This does not establish full-career,
phone or memory-budget acceptance. Final managed previews remain on loopback
8765/8766; auxiliary 8767 is closed.
