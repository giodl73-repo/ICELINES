# Phase Lindsay v0.2 — R2 Review Summary

**Date**: 2026-05-02
**Reviewers**: HART (R2), KEEL (R2), FORGE (R2), BENCH (R2), WIRE (R2), GLASS (R2), SCOUT (fresh), PACE (fresh)
**Verdict**: v0.2 is **not ready** to implement. ~24 new BLOCKERs across 8 reviewers. Most are spec-text drift (v0.2 changelog claims weren't reflected in the spec body); a handful are real new issues. v0.3 required before L.1.

---

## Headline

R1 v0.1→v0.2 was a **changelog-only fix**. The plan changelog says "fixed" on 24 BLOCKERs but the spec body still carries v0.1 wording on most of them (bracket keybind, applies_to gates, member iterator return, pacing formulas, etc.). Multiple reviewers caught the same drift independently:

- **GLASS-R2-B1**: spec line 340 still shows `← / →`, line 350 still `<`/`>` — three different keybind specs across two docs.
- **FORGE-R2-B6**: spec:84 still says "120 cases planned" while spec:46 says "98 cases v0.2."
- **KEEL-R2-F1**: spec has zero mention of axum HTTP — only the plan does.
- **WIRE-B4**: `data/api-probe-2026-05-02.txt` artifact promised, file does not exist.
- **WIRE-F1**: `ReportKind::supports(season_type)` adopted in changelog, missing from both plan and spec.

Fresh-eye SCOUT and PACE also surfaced **real methodology + domain gaps** v0.1 missed entirely:

- **SCOUT-B1, B2**: zero expected-goals stats for skaters or goalies. Modern eval is anchored on GSAx (goalies) and IxG (skaters). Missing now means re-fan every consumer surface later.
- **PACE-B1**: `f64::EPSILON` for `FilterOp::Equals` silently over-filters on every float-valued stat (SavePct, percentages, rates). User typing `--filter "save-pct==0.92"` gets zero rows because `0.92_f64 != (0.92_f32 as f64)` at ~1.2e-8.
- **PACE-B2**: derived per-game stats (PointsPerGame, GoalsPerGame) bypass `MIN_GP` guard. Player with GP=2, 1G+1A sorts above McDavid.

---

## Consolidated R2 BLOCKERs (must fix in v0.3)

### Methodology / domain (4)

- **L2-B1** [PACE-B1] **`f64::EPSILON` for `Equals` is wrong.** Counts use exact integer; rates/percentages use `1e-6` keyed off `StatUnit`. OR forbid `==` on float-valued stats.
- **L2-B2** [PACE-B2] **MIN_GP guard missing on derived per-game/per-82 stats.** `PointsPerGame`, `GoalsPerGame`, `AssistsPerGame` produce noise for low-GP. Either inherit Pace82's MIN_GP=10 gate or document the asymmetry.
- **L2-B3** [SCOUT-B1] **GSAx / xGA/60 missing for goalies.** Goalie evaluation post-2018 is anchored on Goals Saved Above Expected. Add `GoalieXgAgainst`, `GoalieXgAgainstPer60`, `GoalsSavedAboveExpected`, `Gsax60` to Goalie category.
- **L2-B4** [SCOUT-B2] **IxG / xG family missing for skaters.** Catalog has Corsi/Fenwick (`SatPct`/`UsatPct`) but no `IxG`, `IxgPer60`, `OnIceXgFor`, `OnIceXgAgainst`, `XgForPct`. Corsi without xG is 2012-era analytics.

### Documentation drift — v0.2 said it, spec body didn't (8)

- **L2-B5** [GLASS-R2-B1] **Spec sketches at lines 340, 350 contradict plan changelog on `[`/`]` keybind.** Three different specs across two docs.
- **L2-B6** [GLASS-R2-F4] **`<space>` collision still in spec line 329.** Plan resolution (Tab) didn't reach the spec.
- **L2-B7** [FORGE-R2-F12] **`members(c) -> &'static [StatId]`** — spec:77 still shows `impl Iterator`. Fix decided in changelog, body unchanged.
- **L2-B8** [FORGE-R2-B2] **`#[non_exhaustive]` mention still in plan:423 risk section.** Stale v0.1 text.
- **L2-B9** [KEEL-R2-F1] **Spec has zero mention of axum HTTP integration.** v0.2 plan added L.5 deliverable; spec § integration sections (lines 355-389) cover Site + Fantasy but not HTTP.
- **L2-B10** [WIRE-F1] **`ReportKind::supports(season_type)`** — review summary listed as L-F15 but neither plan nor spec carries it forward.
- **L2-B11** [WIRE-F5] **`load_report_with_fallback<T>`** — review summary L-F17 but no scheduling in L.1 or L.2 deliverables.
- **L2-B12** [FORGE-R2-B5] **DI-25 ("byte-identical") imprecise.** Byte-identical to *what*? Frozen golden vs round-trip. Pin: legacy fixture checked in pre-L.5; load+re-serialize must equal same bytes.

### Real new design gaps (6)

- **L2-B13** [HART-R2-B1] **`extra_reports` LRU lifecycle undefined → eviction leak.** When `StatsRepository`'s primary LRU evicts a `(season, season_type)` window, the `extra_reports` entries for the same window stay resident. Specify cascade-eviction; add as DI-12.
- **L2-B14** [HART-R2-B2] **`repository_version` bump strategy not specified for downgrade.** v=2 binary writes a v=2 snapshot; v=1 binary later attempts to read — error must surface at `StatsRepository::load_window`, not deferred.
- **L2-B15** [WIRE-B1] **`extra_reports` persistence to disk is undefined.** Plan L.6 says Tier-2 lives in `extra_reports` but never says whether it's written to `~/.icelines/snapshots/<season>/extra/<kind>.json` or runtime-only.
- **L2-B16** [WIRE-B5] **File format on disk for new typed Tier-1 substructs is undefined.** Plan claims "new files, not new fields" but spec body has the substructs on `SeasonStats`. Pin: separate per-report files; substructs populated at load.
- **L2-B17** [KEEL-R2-B3] **HTTP server Tier-2 visibility policy.** Either HTTP is skater/goalie-typed-only (Tier-2 invisible) OR add `/api/report/:kind/:player_id`. Pick.
- **L2-B18** [PACE-F2] **`extra_reports` LRU cap undefended.** Worst case 1000 × 4 × 14 × ~10KB ≈ 560MB resident. Cap must be specified.

### Schema + ownership precision (3)

- **L2-B19** [KEEL-R2-B1] **Alias-map ownership ambiguous.** Snake_case (TOML) vs hyphen-case (CLI) — neither plan nor spec says where the disambiguation lives. Fix: separate methods `StatId::toml_aliases()` + `StatId::cli_aliases()` in `icelines-core`.
- **L2-B20** [KEEL-R2-B2] **L.5b sweep enumeration missing.** Site rename touches: rendered headers, CSS class names, URL anchors, search-index terms. Enumerate which inherit `StatId::label()` vs `StatId::cli_key()` vs stay free-form.
- **L2-B21** [FORGE-R2-B1] **`ExtraReports` map iteration determinism.** `HashMap` iteration order is non-deterministic → snapshot non-determinism. Use `BTreeMap` or `IndexMap`.

### Test contract precision (3)

- **L2-B22** [BENCH-R2-1] **Fixture-variant catalog not a sub-deliverable.** L.2 row should hoist the 6-variant catalog (skater-modern, skater-pre-2005, center, goalie, traded-multistint, GP=0) as an explicit deliverable.
- **L2-B23** [BENCH-R2-2] **Captured-stdout golden timing wrong.** Capture must occur BEFORE L.3 (output ordering changes ride here), reassert after L.3 AND after L.5. Two fences, not one.
- **L2-B24** [BENCH-R2-3] **5 legacy schemes unnamed.** Specify: yahoo-standard, espn-standard, custom-points-only, head-to-head-9cat, rotisserie-with-goalie.

---

## R2 FIXITs

Sixteen FIXITs: scope, naming, alignment. Notable:

- **L2-F1** [SCOUT-S5] Career table default columns should be `GP G A P +/- PIM PPG SHG GWG Shots S% TOI/G` — Hits/Blocks belong in Two-way preset, not default.
- **L2-F2** [SCOUT-S6] `FaceoffWinPct` recategorize to `TwoWay`. Most faceoffs happen at even strength.
- **L2-F3** [SCOUT-S7] `EvenStrengthTimeOnIcePerGame` → `TimeOnIce` category (sourced from goalsForAgainst is TAPE concern not domain).
- **L2-F4** [SCOUT-S9] Era axis: `available_since(Hits) == 20052006`, not 1997. 1997-2004 data exists but inconsistent.
- **L2-F5** [SCOUT-S11] Add `PpAssists` (raw count) — currently only `PpAssistsPer60` is listed.
- **L2-F6** [HART-R2-F2] DI-11 should enumerate affected `StatId`s, not the whole OnIceGoals category — `EvenStrengthTimeOnIcePerGame` is TOI, not goals; should not silently None on trade.
- **L2-F7** [HART-R2-F4] `is_goalie()` per-row, not `pos == Position::Goalie` — emergency-backup-goalie scenarios.
- **L2-F8** [PACE-F1] Per-60 floor: methodology note. Soft floor `None if TOI < 300s` OR document user-responsibility split.
- **L2-F9** [PACE-F3] Multi-season aggregate `read()` semantics: strict propagation — `None` if any window is missing.
- **L2-F10** [FORGE-R2-B3] `StatId::sort_cmp(self, a, b) -> Ordering` signature explicit. `None < Some(_)` regardless of `higher_is_better`.
- **L2-F11** [FORGE-R2-B4] `FilterParseError` enum sketch: ~7 variants (`EmptyInput`, `MissingOp`, `UnknownStat`, `BadNumber`, `NotFinite`, `LocaleSeparator`, `MultipleOps`).
- **L2-F12** [FORGE-R2-B6] Grep CI pattern for L.5b sweep: `\b(Goals|Assists|Points|Hits|Blocks|Saves|GAA)\b` over `icelines-site/src/**/*.rs` + `templates/**/*.{html,md}` excluding `// ` and `<!-- `. Allowlist file at `icelines-site/.stat-name-allowlist`.
- **L2-F13** [BENCH-R2-6] `extra_reports` L0 tests (eviction policy, key collision, seasonId fence) absent from test impact table.
- **L2-F14** [WIRE-B4] Probe artifact `data/api-probe-2026-05-02.txt` is a deliverable — move L-B13 to L.1 entry-criterion checklist.

---

## R2 NITs

- Stat count drift (98 vs 120) STILL unreconciled across 4 spec/plan locations.
- Test count baseline tracking — projected post-Lindsay totals not published.
- `Pace82` → `PointsPace82` rename for symmetry with `GoalsPer82`/`AssistsPer82`.
- Capitalization inconsistency: `Gaa`/`Pim`/`Toi`/`Gwg` vs `Wins`/`Saves` — initialisms should be uppercase.
- AI-08 (catalog homogeneity guard) overlaps Hart.6.6's existing function; cite the existing helper.

---

## What v0.3 does

1. **Critical (must-fix-before-implementation):** L2-B1, B2, B3, B4 (PACE methodology + SCOUT xG family). These change the catalog surface materially.
2. **Spec body sweep:** L2-B5 through L2-B12 — fix the v0.2 changelog drift in the spec body.
3. **Pin design decisions deferred from R1/R2:** L2-B13 through L2-B21 (extra_reports lifecycle/persistence/cap, file format, alias ownership, HTTP Tier-2, sweep enumeration).
4. **Test contract precision:** L2-B22 through L2-B24 (fixture catalog, golden timing, named schemes).
5. **Apply FIXITs F1-F14** as spec body edits.
6. **NITs**: park in PITFALLS.md + INDEX.md follow-ups.

---

## Verdict

R2 is doing its job — it caught what R1 wouldn't have without a fresh pass. The v0.2 changelog was good intent, weak execution. v0.3 is the actual fix: spec body must match the changelog claims; new domain stats (xG/GSAx) must be added now or the catalog ships incomplete.

**Estimated v0.3 cost**: ~6 hours of doc work. No code yet. After v0.3 ships, L.1 implementation kicks off cleanly.
