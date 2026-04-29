# Spec → Code Delta Catch-Up Plan

**Status**: Draft
**Date**: 2026-04-28
**Goal**: Close the gap between what the 28 specs in `design/specs/`
authoritatively describe and what the codebase actually implements.

---

## Source of this plan

After the Phase 7e ship, two audits were run:
1. Spec-vs-code drift across all specs (existing + 10 new).
2. Code-without-spec coverage (the "homeless features" audit).

Drift findings landed in spec edits (the specs are now accurate).
This plan tracks the inverse: features and tests the specs *describe*
but the code does not yet provide.

Each item below cites the spec section it satisfies. Items are
grouped by phase. Phase numbers continue from Phase 7 (already shipped)
into a new "Phase 8 — Spec Catch-Up" sequence.

---

## Phase 8a — Test coverage gaps (P0, ~4 hours)

The fastest, lowest-risk pass. Three specs ship features that work
but are untested. Adding the tests turns the specs from "claims" into
"verified contracts".

### 8a.1 — Scouting reports (`scouting-reports.md`)

Implement these tests in `icelines-cli/src/commands/scouting.rs`
(and `tests/system_tests.rs` for L2):

| Tier | Test |
|------|------|
| L0 | `format_terminal_includes_all_eight_sections` |
| L0 | `format_markdown_uses_h2_headings` |
| L0 | `format_json_has_section_keys` |
| L0 | `unknown_format_errors_with_valid_options_listed` |
| L0 | `low_gp_skips_current_season_numerics` |
| L1 | Round-trip against `mock_nhl_api.rs` fixture players |
| L2 | `l2_cmd_scouting_terminal_exits_zero` |
| L2 | `l2_cmd_scouting_json_parses` (jq -e .) |
| L2 | `l2_cmd_scouting_unknown_format_exits_nonzero` |

Acceptance: 9 tests added, all green.

### 8a.2 — Admin overlay (`tui-admin-overlay.md`)

In `icelines-cli/src/tui/app.rs::tests` and
`screens/misc.rs::tests`:

| Tier | Test |
|------|------|
| L0 | `l0_admin_overlay_opens_on_capital_F` |
| L0 | `l0_admin_overlay_closes_on_esc` |
| L0 | `l0_admin_overlay_blocks_other_keys` |
| L0 | `l0_admin_overlay_does_not_open_on_lowercase_f` |
| L0 render | `l0_render_admin_idle_phase_shows_no_install` |
| L0 render | `l0_render_admin_downloading_phase_shows_spinner` |
| L0 render | `l0_render_admin_error_phase_shows_red` |

Acceptance: 7 tests added, all green.

### 8a.3 — Headshot rendering (`headshot-rendering.md`)

`tui/headshot.rs` has zero tests today. Add `#[cfg(test)] mod tests`:

| Tier | Test |
|------|------|
| L0 | `braille_dot_bit_layout_matches_unicode` |
| L0 | `threshold_dither_solid_black_sets_all_dots` |
| L0 | `threshold_dither_solid_white_clears_all_dots` |
| L0 | `cache_get_set_roundtrip` |
| L0 | `is_loading_detects_placeholder` |
| L0 | `is_error_detects_placeholder` |

Acceptance: 6 tests added, all green.

**Phase 8a total**: ~22 new tests, no production code change.

**Status: Implemented (2026-04-28)** — 27 tests added (5 bonus L0 tests
ride along with the planned 22). Three small refactors enabled testing:
extracting `validate_format` + `render_report` from scouting's `run()`;
adding `InstallState::force_phase` (cfg(test)); extracting
`pixels_to_braille()` from headshot's dither pipeline. All workspace
tests green; clippy clean for changed files.

---

## Phase 8b — Scores auto-refresh (P1, ~3 hours)

Specced in `scores.md` lines 130–136 ("Auto-Refresh — poll every 30s
when active, pause when inactive, show timestamp in nav bar"). Phase
7c shipped without this — manual `r` only.

Tasks:
- Add a polling loop owned by the TUI event handler that ticks every
  30s **only while `Screen::Tonight` is active and `scores_date` is
  empty (live)**.
- On tick: `force_fetch(tonight_cache, "")` with a debounce so user
  navigation doesn't double-fire.
- Render an `Updated Xs ago` indicator in the Scores title bar.
- Tests:
  - `l0_scores_auto_refresh_paused_off_tab` — switching tabs
    cancels the timer
  - `l0_scores_auto_refresh_paused_on_past_date` — historical dates
    don't auto-refresh
  - L1 render snapshot of the "Updated 14s ago" indicator

Acceptance: Active Scores tab on a live date silently refreshes
every 30s; nav indicator visible; switching tabs or dates suspends.

**Status: Implemented (2026-04-28)** — `should_auto_refresh` pure
decision function tested with synthetic Instant clocks (no flakes).
Indicator renders in the Scores title bar (close enough to nav for
the user; no nav-bar refactor needed). 10 tests added (8 decision
+ 2 render snapshot). Timer state machine: armed on tab entry / `t`
jump / picker apply with empty input; disarmed on `←/→`, picker
apply with explicit date, and tab leave.

---

## Phase 8c — Historical playoff bundling (P1, ~6–8 hours)

Specced in `playoffs.md` lines 145–179 (`playoffs.json` per season
bundle) and lines 88–109 (Historical Season View). Phase 7e shipped
the live API path only; historical brackets show "Historical playoff
data not bundled for this season" today.

Tasks:
- Define `PlayoffsBundle` struct in `icelines-fetch::schema` matching
  the JSON layout in `playoffs.md` §Data Bundling.
- Generate `playoffs.json` for each of 38 historical seasons.
  Source: NHL stats API + boxscore endpoints (one-off CI job, not
  per-user fetch).
- Update GitHub Releases tarballs to include the new file.
- `bundled::get_playoffs(season)` and `get_playoffs_installed(season)`
  loaders following the bios/stats pattern.
- Wire `Screen::Playoffs` to fall back to `playoffs.json` when
  `active_season != CURRENT_SEASON_STR`.
- Replace the "v2 placeholder" line in `render_series_detail` with
  the actual game log + goal scorers.
- L0 + L1 + L2 tests for the historical-bundle path.

Acceptance: Pressing `y` → 1993-94 → tab to Playoffs shows the full
NYR Stanley Cup bracket with game-by-game results and goal scorers.

---

## Phase 8d — Markdown export writers only (P2, ~10–15 hours)

`export-markdown.md` is the only spec marked `Planned`. Phase 8d
ships **the IceLines side only** — the `icelines export md` command
that writes deterministic markdown tables. **No proof / DASHBOARD-SPEC
runtime dependency is added in Phase 8.** The TUI continues to
hand-render its widgets exactly as it does today.

Tasks (per shape, ~1.5 hours each):
- `export md leaders` — top-N leaderboard
- `export md team <ABBR>` — single team's lineup card
- `export md depth` — cross-team line-value rankings
- `export md fantasy` — active league standings
- `export md compare --p1 X --p2 Y` — head-to-head
- `export md series <LETTER>` — playoff series log
- `export md roster` — all teams' rosters

Each writes a `~/.icelines/reports/{shape}.md` file with YAML
front-matter and a markdown table per `export-markdown.md` schemas.
The output is a static artifact users can pipe to any markdown
consumer (mkdocs, pandoc, GitHub, proof if/when it's ready).

The actual TUI ↔ proof integration (TUI screens consuming compiled
proof output instead of hand-rendered widgets) is **deferred to
Phase 9 or later**, gated on proof DASHBOARD-SPEC publication. It's
a separate concern from shipping the writer.

Acceptance: Each shape produces deterministic, byte-stable markdown.
L1 tests verify column order + front-matter validity per shape. No
runtime proof dependency exists in the binary; `cargo tree -p
icelines-cli` does not list `proof_lib` or any proof crate.

---

## Phase 8e — Fantasy v1 polish (P2, ~5–8 hours)

`fantasy-leagues.md` flags four HIGH/MED items in Future Work.

Tasks:
1. **Goalie scoring** (HIGH):
   - Extend tier-2 fetch to include goalie stats (wins, SV%, SO).
   - `Goalie` schema in `icelines-fetch`.
   - `compute_goalie_score(stats, weights, gp_minutes)` in
     `icelines-core::scheme`.
   - Update `team_total` to include goalies on the roster.
   - Custom schemes opt in via `[goalie_weights]` TOML section.

2. **Roster shape enforcement** (MED):
   - Add `roster_shape` field to the scheme TOML
     (e.g. `forwards = 14, defense = 4, goalies = 2, ir = 2, bench = 1`).
   - `team-add` rejects with friendly error if it would exceed the
     position cap.

3. **Head-to-head matchups** (MED):
   - New `fl_matchups` table (week, home_team, away_team, scores).
   - `fantasy schedule generate --weeks 24` builder.
   - `fantasy standings --week N` weekly view.

4. **Daily delta scoring** (LOW per spec; backlog):
   - Already in `design/plans/INDEX.md` backlog.
   - Defer to Phase 9 unless a user asks.

Acceptance: Goalie scoring works end-to-end with a Yahoo-derived
scheme; roster shape rejects over-position adds; weekly matchups
produce a head-to-head standings view.

---

## Phase 8f — Quality-of-life (P3, ~6–10 hours)

Smaller items spread across multiple specs.

| Item | Spec | Effort |
|------|------|--------|
| `snapshot prune --keep N` | `snapshot-operations.md` | 1h |
| `snapshot diff <A> <B>` | `snapshot-operations.md` | 2h |
| Data bundle SHA-256 verification | `data-bundles.md` | 1h |
| `scheme show --source` | `scheme-customization.md` | 30m |
| ESPN / Sleeper / Fantrax CSV detection | `scheme-customization.md` | 1h each |
| `group export <NAME>` / `import` | `group-management.md` | 1h |
| `group rename <old> <new>` | `group-management.md` | 30m |
| Admin overlay in-overlay actions | `tui-admin-overlay.md` v2 | 2h |
| Admin overlay `:` command prompt | `tui-admin-overlay.md` v2 | 3h |
| MoneyPuck historical xG | INDEX backlog | 4h |
| `--season YYYYZZZZ` flag on query commands | INDEX backlog | 2h |

Pick by user demand; no blocking order between items here.

---

## Phase 8g — Big lifts (P4, deferred)

Larger lifts that require new data sources or substantial new
infrastructure. Listed for tracking, not scheduled:

- **NHL Edge skating speed stats** — new fetcher + schema + UI
  surface (~6h).
- **Strength-state 5v5/PP/PK splits** — requires play-by-play +
  shift data (~15h).
- **CI: cargo fmt + cargo audit** — workflow additions (~2h).

**proof DASHBOARD-SPEC runtime integration is explicitly out of
scope for Phase 8.** The TUI keeps its hand-rendered widgets through
the full Phase 8 rollout. Phase 8d ships the *static markdown
artifacts* that proof can consume later, but does not link against
any proof crate at runtime. The full TUI-as-proof-renderer story
moves to a future phase, gated on proof's DASHBOARD-SPEC shipping.

---

## Blocking order

```
Phase 8a (tests)
  └── independent — can ship today

Phase 8b (scores auto-refresh)
  └── independent — can ship today

Phase 8c (playoff history)
  └── needs CI job to generate playoffs.json bundles
  └── then bundle release per season

Phase 8d (Phase 6 export)
  └── partially independent
        └── proof DASHBOARD-SPEC integration blocks completion

Phase 8e (fantasy polish)
  └── 8e.1 goalie scoring blocks 8e.2 roster shape (shape rules need goalies)

Phase 8f (QoL)
  └── all independent

Phase 8g (big lifts)
  └── deferred — no current commit
```

8a, 8b, 8f are parallelizable; 8c and 8e have internal sequencing
but don't block each other; 8d is mostly independent.

---

## Suggested rollout

| Sprint | Phases | Outcome |
|--------|--------|---------|
| Week 1 | 8a + 8b | All shipped specs verified; auto-refresh closes the last 7c gap |
| Week 2 | 8c | Historical playoffs become a real feature, not a placeholder |
| Week 3 | 8e.1–8e.3 | Fantasy gains goalies + roster shape + matchups |
| Week 4 | 8d (export shapes 1–4) | Half of `export md` shipped |
| Week 5 | 8d (shapes 5–7) + 8f cherrypicks | Export complete; small QoL items |

Total: ~5 weeks of one-engineer time at half-capacity, or 2 weeks
at full focus. Phases 8g remain explicitly deferred.

---

## Acceptance for "specs are authoritative again"

When this plan finishes:
- Every spec marked `Implemented` in `design/specs/INDEX.md` has full
  L0/L1/L2 coverage matching the test names cited in the spec.
- `export-markdown.md` flips from `Planned` → `Implemented`.
- `playoffs.md` historical-bracket section becomes load-bearing
  rather than placeholder.
- `scores.md` auto-refresh works as specified.
- `fantasy-leagues.md` Future Work reduces to truly v2+ items.

The spec ↔ code drift contract: **whenever a spec says a behavior
is `Implemented`, the running build does that behavior, and a test
proves it.**
