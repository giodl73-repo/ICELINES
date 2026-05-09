# Scenario Harness Inventory - 2026-05-09

Purpose: separate executable scenario harnesses from planning/spec scenario
budgets. "Scenario" in this repo currently means three different things:

- Executable persona/harness tests in Rust.
- Broader integration/system tests that behave like scenario coverage.
- Planning-only scenario budgets in `design/specs` and `design/plans`.

## Summary

Counted Rust files under `*/tests/*.rs` plus the in-bin TUI harness
`icelines-cli/src/tui/persona_jack_adams.rs`.

Run active scenario harnesses locally with:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/test-slice.ps1 scenarios
```

| Group | Files | Executable tests | Notes |
|---|---:|---:|---|
| Persona/harness | 25 | 2,055 | Product/user-flow/storyline/adversarial harnesses. |
| Other integration/system | 35 | 609 | System, mock API, golden, matrix, and crate integration tests. |
| Inventory total | 60 | 2,664 | Does not include every inline unit test in `src/**/*.rs`. |

## Active Persona Harnesses

| File | Tests | Surface | CI gate | Status |
|---|---:|---|---|---|
| `icelines-cli/src/tui/persona_jack_adams.rs` | 100 | TUI MDI command/user flows | `ci-cli-tui` via bin tests | Active product harness |
| `icelines-cli/tests/persona_scenarios.rs` | 85 | CLI + data/catalog regression | `ci-cli-persona` | Frozen legacy harness; 15 TUI checklist items are archived/covered elsewhere |
| `icelines-cli/tests/persona_wave2.rs` | 100 | CLI gaps/regressions | `ci-cli-persona` | Frozen regression harness |
| `icelines-cli/tests/persona_wave3.rs` | 100 | CLI secondary surfaces/export/fantasy | `ci-cli-persona` | Frozen regression harness |
| `icelines-cli/tests/persona_wave4.rs` | 100 | Multi-filter CLI combinations | `ci-cli-persona` | Frozen regression harness |
| `icelines-cli/tests/persona_wave5.rs` | 100 | Foster favorites/setup/config/groups | `ci-cli-persona` | Active/frozen hybrid |
| `icelines-cli/tests/persona_wave6.rs` | 100 | Time travel/fetch/date-axis CLI | `ci-cli-persona` | Active/frozen hybrid |
| `icelines-cli/tests/persona_wave7.rs` | 100 | Playoff/Cup-run/game detail CLI | `ci-cli-persona` | Frozen regression harness |
| `icelines-cli/tests/persona_wave9.rs` | 100 | Edge cases/cross-feature robustness | `ci-cli-persona` | Frozen regression harness |
| `icelines-cli/tests/persona_wave10.rs` | 100 | UX truthfulness/output discipline | `ci-cli-persona` | Active quality harness |
| `icelines-cli/tests/persona_wave11.rs` | 201 | Legacy filter grammar adversarial | `ci-cli-persona` | Frozen compatibility harness |
| `icelines-cli/tests/persona_wave16.rs` | 100 | New grammar through CLI binary | `ci-cli-persona` | Active parser parity harness |
| `icelines-cli/tests/persona_wave18.rs` | 40 | Query subcommands new grammar | `ci-cli-persona` | Active parser parity harness |
| `icelines-cli/tests/persona_wave20.rs` | 20 | Player/compare cohort filters | `ci-cli-persona` | Active parser parity harness |
| `icelines-cli/tests/persona_wave23.rs` | 14 | TUI filter overlay L2 smoke | `ci-cli-persona` | Narrow subprocess smoke |
| `icelines-cli/tests/persona_wave25.rs` | 10 | Career filter L2 smoke | `ci-cli-persona` | Narrow subprocess smoke |
| `icelines-cli/tests/persona_foster.rs` | 30 | Foster cross-surface closeout | `ci-cli-persona` | Active Foster harness |
| `icelines-cli/tests/persona_masterton_standalone.rs` | 8 | TUI standalone parser/help smoke | `ci-cli-persona` | Narrow subprocess smoke |
| `icelines-query/tests/persona_wave12.rs` | 200 | Art Ross grammar adversarial | `ci-query` | Active query grammar harness |
| `icelines-query/tests/persona_wave13.rs` | 200 | Reporter-style query storylines | `ci-query` | Active query product harness |
| `icelines-web/tests/persona_wave8.rs` | 100 | Web router/forms/security smokes | `ci-web` | Active web harness |
| `icelines-web/tests/persona_wave17.rs` | 80 | Web leaders new grammar parity | `ci-web` | Active web parity harness |
| `icelines-web/tests/persona_wave19.rs` | 25 | Web API leaders new grammar parity | `ci-web` | Active web parity harness |
| `icelines-web/tests/persona_wave21_parity.rs` | 25 | Web API vs query result parity | `ci-web` | Active parity harness |
| `icelines-web/tests/persona_wave22b_envelope.rs` | 17 | Web API JSON envelope correctness | `ci-web` | Active API shape harness |

## Other Scenario-Like Integration Gates

These are not named persona waves, but they should be treated as product
coverage when deciding whether a scenario is already tested.

| File/group | Tests | CI gate | Notes |
|---|---:|---|---|
| `icelines-cli/tests/system_tests.rs` | 195 | `ci-system` | Broad CLI/system regression harness. |
| `icelines-cli/tests/system_tui_experiences.rs` | 6 | `ci-system` | TUI experience subprocess smokes. |
| `icelines-cli/tests/art_ross_*.rs` | 166 | `ci-cli-art-ross` | Art Ross focused integration gates. |
| `icelines-cli/tests/lindsay_*.rs` | 6 | `ci-cli-lindsay` | Lindsay golden/subprocess gates. |
| `icelines-cli/tests/foster_capability_matrix.rs` | 24 | `ci-cli-smoke` | Foster capability matrix. |
| `icelines-web/tests/l1_*.rs` | 30 | `ci-web` | Web router/static integration smokes. |
| `icelines-fetch/tests/*.rs` | 141 | `ci-fetch` | Fetch/mock/API/storage integration gates. |
| `icelines-core/tests/*.rs` | 11 | `ci-core-integration` | Core integration gates. |
| `icelines-site/tests/*.rs` | 2 | `ci-site` | Site integration gate. |

## Planning-Only Scenario Budgets

Scenario counts in `design/specs/*.md` and `design/plans/*.md` are not
executable unless backed by a Rust `#[test]`. Keep them as planning records,
but avoid treating them as current harness coverage.

Examples:

- `design/plans/2026-05-08-phaseMessier-roster-filters.md`
- `design/plans/2026-05-06-phaseArtRoss-overview.md`
- `design/specs/foster-overview.md`
- `design/specs/web-dashboard.md`

## Policy Going Forward

- Add new product behavior to an active harness, not an old frozen wave, unless
  the new case is explicitly a regression for that old wave's theme.
- Keep old waves as compatibility nets until the equivalent behavior is covered
  by smaller L0/L1 tests and a specific removal note exists.
- When a plan says "N personas", close it by linking the actual Rust test file
  and count.
- Re-run this inventory after large test moves or CI gate changes.
