# Scenario Inventory - Pulse 05

Pulse 05 classifies the executable persona/scenario corpus so future product
pulses can add focused tests instead of duplicating broad harness coverage.

## Inventory command record

Count command:

```powershell
$scenarioFiles = @(Get-ChildItem -Path icelines-cli\tests,icelines-web\tests,icelines-query\tests -Filter 'persona*.rs' -File) + @(Get-Item icelines-cli\src\tui\persona_jack_adams.rs)
$scenarioFiles | Sort-Object FullName | ForEach-Object {
  $count = (Select-String -Path $_.FullName -Pattern '^\s*#\[(tokio::)?test\]' | Measure-Object).Count
  [PSCustomObject]@{ Path = (Resolve-Path -Relative $_.FullName); Tests = $count }
}
```

Result: **25 source paths, 2,057 executable persona/scenario tests**. The
legacy `persona_scenarios.rs` file also carries **15 archived TUI checklist
items** (`p081`-`p095`) as comments; they are not executable tests in that
file, and the file documents their superseding TUI harness coverage.

## Counts by product surface

| Surface | Files | Executable tests | Source paths |
|---|---:|---:|---|
| CLI command/query/export/config/fantasy scenarios | 16 | 1,294 | `icelines-cli\tests\persona_*.rs`, excluding `persona_wave23.rs` |
| TUI command/user-flow scenarios | 2 | 114 | `icelines-cli\src\tui\persona_jack_adams.rs`; `icelines-cli\tests\persona_wave23.rs` |
| Query engine library scenarios | 2 | 400 | `icelines-query\tests\persona_wave12.rs`; `icelines-query\tests\persona_wave13.rs` |
| Web HTML/API scenarios | 5 | 249 | `icelines-web\tests\persona_wave8.rs`; `persona_wave17.rs`; `persona_wave19.rs`; `persona_wave21_parity.rs`; `persona_wave22b_envelope.rs` |
| **Total** | **25** | **2,057** |  |

## Source inventory

| Source path | Tests | Surface | Classification |
|---|---:|---|---|
| `icelines-cli\src\tui\persona_jack_adams.rs` | 100 | TUI MDI command/user flows | `test-backed` |
| `icelines-cli\tests\persona_scenarios.rs` | 85 | CLI plus data/catalog regression | `test-backed` with archived checklist notes |
| `icelines-cli\tests\persona_wave2.rs` | 100 | CLI gaps/regressions | `test-backed` |
| `icelines-cli\tests\persona_wave3.rs` | 100 | CLI secondary surfaces/export/fantasy | `test-backed` |
| `icelines-cli\tests\persona_wave4.rs` | 100 | Multi-filter CLI combinations | `test-backed` |
| `icelines-cli\tests\persona_wave5.rs` | 100 | Foster favorites/setup/config/groups | `test-backed` |
| `icelines-cli\tests\persona_wave6.rs` | 100 | Time travel/fetch/date-axis CLI | `test-backed` |
| `icelines-cli\tests\persona_wave7.rs` | 100 | Playoff/Cup-run/game detail CLI | `test-backed` |
| `icelines-cli\tests\persona_wave9.rs` | 100 | Edge cases/cross-feature robustness | `test-backed` |
| `icelines-cli\tests\persona_wave10.rs` | 100 | UX truthfulness/output discipline | `test-backed` |
| `icelines-cli\tests\persona_wave11.rs` | 201 | Legacy filter grammar adversarial | `test-backed` |
| `icelines-cli\tests\persona_wave16.rs` | 100 | New grammar through CLI binary | `test-backed` |
| `icelines-cli\tests\persona_wave18.rs` | 40 | Query subcommands new grammar | `test-backed` |
| `icelines-cli\tests\persona_wave20.rs` | 20 | Player/compare cohort filters | `test-backed` |
| `icelines-cli\tests\persona_wave23.rs` | 14 | TUI filter overlay L2 smoke | `test-backed` |
| `icelines-cli\tests\persona_wave25.rs` | 10 | Career filter L2 smoke | `test-backed` |
| `icelines-cli\tests\persona_foster.rs` | 30 | Foster cross-surface closeout | `test-backed` |
| `icelines-cli\tests\persona_masterton_standalone.rs` | 8 | TUI standalone parser/help smoke | `test-backed` |
| `icelines-query\tests\persona_wave12.rs` | 200 | Art Ross grammar adversarial | `test-backed` |
| `icelines-query\tests\persona_wave13.rs` | 200 | Reporter-style query storylines | `test-backed` |
| `icelines-web\tests\persona_wave8.rs` | 100 | Web router/forms/security smokes | `test-backed` |
| `icelines-web\tests\persona_wave17.rs` | 80 | Web leaders new grammar parity | `test-backed` |
| `icelines-web\tests\persona_wave19.rs` | 27 | Web API leaders new grammar parity | `test-backed` |
| `icelines-web\tests\persona_wave21_parity.rs` | 25 | Web API vs query result parity | `test-backed` |
| `icelines-web\tests\persona_wave22b_envelope.rs` | 17 | Web API JSON envelope correctness | `test-backed` |

The scenario files use stable test IDs (`p###`, `p_wNN_###`, or named persona
tests), so the stop condition for generated artifacts without stable IDs does
not apply.

## Classification buckets

| Bucket | Meaning | Examples | Action |
|---|---|---|---|
| `test-backed` | Executable Rust tests already assert the scenario. | Web POST safety in `icelines-web\tests\persona_wave8.rs::p_w8_007_favorites_form_uses_post`; query grammar parity in `icelines-query\tests\persona_wave12.rs`; CLI markdown export coverage in `icelines-cli\tests\persona_wave3.rs`. | Keep as compatibility net; add narrow L0/L1 tests for new behavior rather than expanding old waves by default. |
| `needs-test` | Product behavior is planned or recently backfilled, but no precise test target owns the scenario yet. | Pulse 06 watch-rule TUI editor UX; Pulse 07 safe web admin operation intents; Pulse 08 missing local career-history store messaging; Pulse 04 CREST/visual capture reproducibility. | Convert in the owning pulse using the target files listed below. |
| `docs-only` | Scenario text is a planning/checklist record, not an executable gate. | `persona_scenarios.rs` comments `p081`-`p095`; planning-only scenario budgets summarized in `design\notes\2026-05-09-scenario-harness-inventory.md`. | Keep if it explains history; do not count as coverage unless linked to a Rust test. |
| `obsolete` | Original scenario has been superseded by a different surface or ViewModel. | The old mkdocs `build` help persona was replaced by `icelines-cli\tests\persona_wave3.rs::p289_serve_help` after the mkdocs cut; team season report/export parity is now represented by `TeamSeasonView` plus `export md team-season`. | Retire only with a replacement note naming the new command/ViewModel/surface. |
| `future-product` | The scenario names behavior intentionally not shipped yet. | Sync `shifts` capability remains locked in `icelines-cli\tests\foster_capability_matrix.rs`; NHL Edge skating speed remains blocked by no public JSON endpoint; live destructive web data install/remove remains deferred unless a safe dry-run/local-only POST contract exists. | Keep as guardrail/deferred scope; do not force into executable product tests before the feature exists. |

## Links to focused test slices

These focused slices should be preferred for new tests when a persona scenario
maps cleanly to a lower-level contract:

| Scenario group | Existing focused slice | Current count | Use for |
|---|---|---:|---|
| CLI subprocess behavior | `icelines-cli\tests\system_tests.rs` | 201 | Command parsing, stdout/stderr contracts, JSON/CSV envelopes. |
| TUI subprocess smoke | `icelines-cli\tests\system_tui_experiences.rs` | 6 | Standalone launch/help/screen startup checks. |
| Human-readable CLI layout | `icelines-cli\tests\prince_cli_visual.rs` | 3 | 80-column/no-color readability and label preservation. |
| Art Ross query grammar | `icelines-cli\tests\art_ross_*.rs` | 166 | CLI/query integration for new filter grammar. |
| Lindsay markdown export | `icelines-cli\tests\lindsay_*.rs` | 6 | Markdown export columns/golden subprocess behavior. |
| Foster sync/config capabilities | `icelines-cli\tests\foster_capability_matrix.rs` | 24 | Capability matrix, locked future capabilities, config errors. |
| Web router/template/API contracts | `icelines-web\tests\l1_*.rs` | 111 | HTML/API route contracts not needing persona-wave breadth. |
| Fetch/data pipeline | `icelines-fetch\tests\*.rs` | 141 | Fixture-backed ingestion, snapshot, API parsing, no live network tests. |
| Core pure logic | `icelines-core\tests\*.rs` | 11 | Cross-module core invariants; prefer inline L0 tests for small pure helpers. |
| Site/static rendering | `icelines-site\tests\*.rs` | 2 | Site rendering contracts that survived the mkdocs cut. |

## Recommended next conversions

| Priority | Scenario | Target crate/surface | Suggested level | Exact file or target |
|---:|---|---|---|---|
| 1 | Watch-rule TUI editor adds/toggles watch candidates without changing poach scoring. | `icelines-cli` TUI | L0 state/action plus narrow L2 smoke | Add focused tests near the watch UI state module discovered in Pulse 06; if it stays subprocess-only, add `icelines-cli\tests\system_tui_experiences.rs` cases named `tui_watch_*`. |
| 2 | Web admin operations expose safe typed POST intents and never mutate from GET. | `icelines-web` admin/router | L1 router test | Add `icelines-web\tests\l1_router.rs` cases or a new `icelines-web\tests\admin_operations.rs` target with fixture-backed POST requests. |
| 3 | Missing local `career_history.json` renders an explicit fetch instruction, not empty success. | `icelines-cli` and `icelines-web` career surfaces | CLI L2 plus web L1 | Add `icelines-cli\tests\system_tests.rs` or `persona_wave25.rs` for CLI messaging; add `icelines-web\tests\l1_router.rs` for `/career` and `/api/v1/career` missing-store behavior. |
| 4 | Visual/CREST captures are reproducible after pulses 03, 06, 07, and 08 settle. | CLI/web visual surfaces | Existing visual smoke plus artifact check | Extend `icelines-cli\tests\prince_cli_visual.rs` for text surfaces; Pulse 04 should write capture artifacts under the active wave and gate them with proof/diff checks. |
| 5 | Team-season markdown export remains source-state truthful. | `icelines-cli` export | Already converted in Pulse 03 | Keep `icelines-cli\src\commands\export.rs::l0_export_team_season_preserves_missing_source_warning` and `icelines-cli\tests\system_tests.rs::l2_cmd_export_md_team_season_to_stdout` as the owning tests. |

## Role-lens notes

- SCOUT: scenario tests that make hockey claims should stay tied to real
  surfaces or shared ViewModels; do not create surface-local interpretations.
- TAPE/WIRE: any conversion that touches fetched data must use bundled data or
  fixtures. No live network test belongs in a scenario conversion.
- FORGE: prefer precise L0/L1 contracts over adding another broad persona wave.
  Existing broad waves remain valuable compatibility nets, but new debt should
  close with targeted test names.
