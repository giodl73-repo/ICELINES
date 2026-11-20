# Rinkcraft public player preview

Goal: an accessible player/season explorer using the compact Rinkcraft card,
with Current as the default and clear source dates. Reusable source remains in
IceLines. No merge or replacement of the existing IceLines Pages site is needed.

## Data and interpretation

The Rust `rinkcraft_preview_catalog` example accepts an explicit checksummed
manifest of complete regular-season NHL reports and TOI reports. It evaluates
the existing 34-rule feat registry against full same-season position populations,
uses the shared workload builder, selects dated documentary classes through the
existing catalog, and writes one profile per player-season plus a small index.
No source acquisition or network requests occur in the builder.

The initial export has 3,941 player-season records:

| Season | Skaters and goalies |
| --- | ---: |
| 2026–27 Current | 858 |
| 2025–26 | 1,038 |
| 2024–25 | 1,023 |
| 2023–24 | 1,022 |

Current capture: 2026-11-19T22:52:41.2471712Z, 781 skaters and 77 goalies.
Current means captured season-to-date statistics, not an automatically refreshed
feed. The player population consists of season participants in these reports;
it does not assert active-roster, injury or transaction status. A player absent
from a season is unavailable, never silently substituted or assigned zero stats.

Measured Record shows all-strength per-game activity/outcome values and their
matching rank population. Goalie measures use their own references. The six
capability fields remain unrated. Source selection and measurement stay in Rust;
TypeScript handles filtering, navigation and presentation.

Lower-tail observations use the existing exploratory P20 policy and its
whitelist. They share a helper with the salary-contrast adapter. These initial
universal cards count registry feats, excluding the six additional experimental
task patterns available in selected research cards. PP-rate/conversion lower
tails and faceoff tasks require their conditional input screens and are not
silently inferred from the baseline registry. The earlier research report and
five-player theme studio remain separate, richer evidence surfaces.

Missing class, methods, development or coaching duties remain unscouted or
undocumented. The framework's known defensive measurement gaps remain explicit.
No capability, simulator, penalty, role-assignment or statistical threshold
changes are made by this presentation/export work.

## Interface

- Current default, with selected-season and capture date/time visible.
- Name/team search including accent normalization; team and position filters.
- Full season participant selector, including goalies.
- Shareable player/season/theme URL and six existing sheet themes.
- Explicit empty search, unavailable player-season, load failure and early-season
  exposure states. Stale source links are cleared when a selection changes.
- Source links, report hashes, measured conditions and expandable assessment gaps.

## Build and validation

```text
cargo test -p icelines-sources --example rinkcraft_preview_catalog --example rinkcraft_feat_salary_contrast --locked
cargo run -p icelines-sources --example rinkcraft_preview_catalog --locked -- INPUT_MANIFEST NEW_DATA_DIRECTORY
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-preview.test.mjs icelines-browser/tests/rinkcraft-observations.test.mjs icelines-browser/tests/rinkcraft-sheet.test.mjs icelines-browser/tests/rinkcraft-profile.test.mjs
node tools/prepare_rinkcraft_public_preview.mjs NEW_DATA_DIRECTORY STATIC_DIRECTORY
node icelines-browser/e2e/rinkcraft-public-preview.mjs
```

Nine focused Rust tests and 27 Node tests passed. The browser check passed 20
player/season examples across four seasons, six mobile themes, combined filters,
deep-link reload, missing-player isolation and three single-page Letter exports.
Desktop Killorn and mobile screenshots were inspected. This does not establish
physical-device or broad assistive-technology acceptance.

The same browser checks passed on the published HTTPS origin in a fresh isolated
browser, including all four seasons, filters, deep links, mobile themes and
printing, with zero page errors. An independent Python rational-arithmetic audit
checked all 3,941 exports (1,371 distinct players), 58,544 metric/rank comparisons,
2,490 lower-tail profile sets, and identity/scope/workload bindings. Date-only
historical captures remain date-only in the interface. These checks do not prove
that the provider's underlying observations are complete scouting measurements.

## Separate preview hosting

Sites project ID: `appgprj_6aff7eac4c1081919202e36a7730dc2a`.
Manifest: ignored `target/sites/rinkcraft-preview/.openai/hosting.json`.
Static artifact source: ignored `target/sites/rinkcraft-preview/dist`.
Published origin: `https://rinkcraft-nhl-preview.ivoryorchid34.chatgpt.site`.
Native deployment `appgdep_6aff8089718c8191940141c5ed69d2e0` succeeded for saved
version 1 (`appgprj_6aff7eac4c1081919202e36a7730dc2a~appgver_1b39b9cc7f0c8191962c292feaa2db0f`),
with static-source commit `b8d6866a19063bb8aa5e9a5c8fda0eb41fca31f0`.
The owner explicitly requested a public preview; its audience was set to public.
No account IDs or credentials belong in tracked source.

The Sites source commit identifies the exact pushed static artifact. Retain that
commit, saved-version/deployment identifiers, input hashes, ICELINES source
commit and validation records with the local publication packet. Current-season
refresh is a new immutable capture and rebuild; no scheduled automation was
created. IceLines GitHub workflows remain manual-only.
