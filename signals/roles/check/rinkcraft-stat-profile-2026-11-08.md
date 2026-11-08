# Rinkcraft stat profile review — 2026-11-08

Producer-applied HART / TAPE / PACE / BENCH / FORGE / EDGE review, not independent
scientific or hockey-coaching validation. CHG-125 / IF-RINKCRAFT-PROFILE-001.

- HART: one normalized player/season/type row; catalog applicability, era and
  traded-row guards retained. Position remains per season; regular/playoff isolated.
- TAPE: report-aware hydration retains optional source state. Actual career rehearsal
  exposed active snapshot season shadowing; regular bios/stats, goalies, chunked
  reads and realtime legacy fallback now require the requested season. User caches
  were not rewritten. Regressions cover mismatch and optional-report rehydration.
- PACE: career percentages use summed numerators/denominators; actual TOI totals
  differ from average seconds. Unsupported/model-specific rates stay unavailable
  in multi-season summaries. Raw changes are not role-change estimates.
- BENCH: synthetic core boundaries, source-loader regressions and CLI interface
  checks; focused results below. No live network in tests.
- FORGE / EDGE: pure core builder, I/O in fetch, thin CLI and serialized view;
  zero denominators, missing years, empty windows and ambiguous names explicit.

Job families navigate evidence; no validated class score, causal attribution,
directional technique from box scores, persistent finishing skill or lineup fit.
Whole-season intervals only; exact dates and other surfaces need further adapters.
Input hashes/provider-model provenance beyond source state are not certified.

## Local validation

- Core library: 1,189 passed, including six new focused profile tests.
- Fetch library: 590 passed, including five focused profile/season-source tests.
- CLI integration: two new help/window-validation and historical-goalie playoff tests passed; compiled binary
  runs also exercise full-name resolution, career, bounded seasons and playoffs.
- `cargo fmt --all -- --check` and whitespace checks passed.
- Production lint: `cargo clippy -p icelines-core -p icelines-fetch -p icelines-cli
  --lib --bins --locked -- -D warnings` passed.
- The attempted `--all-targets` lint gate failed on three pre-existing
  `chunks_exact_to_as_chunks` test-code lints in `isolated_impact.rs`,
  `organization_window_adapters.rs` and `team_season_forecast.rs`. Those files are
  unchanged; all-target lint success is not claimed.

Actual profiles were generated for three positions: Kartye (four regular seasons),
Fox (eight), Shesterkin (seven). The first two include loaded 2026–27 rows; the
goalie's latest available row is 2025–26, explicitly marked. Additional rehearsals
check a three-season Fox range and a separate Shesterkin playoff profile.
Generated research artifacts remain in Rinkcraft's ignored output area.
The goalie rehearsal also exposed missing goalie career fan-out; fixed through
the canonical goalie mapper, with explicit-store injection and regression coverage.

These results do not claim full-workspace tests, test-target lint success, release,
other-surface parity or scientific role calibration. Whole-season profiles are
implemented; arbitrary calendar dates and validated class scores remain follow-on.
