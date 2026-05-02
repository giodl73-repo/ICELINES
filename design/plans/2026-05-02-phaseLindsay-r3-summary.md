# Phase Lindsay v0.3 — R3 Verification Summary

**Date**: 2026-05-02
**Reviewers**: HART (R3), KEEL (R3), FORGE (R3), BENCH (R3), WIRE (R3), GLASS (R3), TAPE (R2), EDGE (R2), SCOUT (R2 deltas), PACE (R2 deltas)
**Verdict**: v0.3 is **not ready** to implement. R3 surfaced ~14 still-BLOCKER items across 6 reviewers — the SAME drift pattern R2 caught in v0.2: the v0.3 changelog claims fixes the v0.3 spec body never wrote. v0.4 is a literal spec-body sweep, no new design decisions, ~2-3 hours of doc work.

---

## Headline

R2 caught v0.2 for a "changelog says X, spec body still says Y" drift. v0.3's changelog claimed every R2 fix landed. R3 verified each claim against the actual spec body and found the SAME defect class repeated:

- **HART-R3-B1**: changelog adopts `extra_reports` LRU cascade rule — spec body never declares it.
- **HART-R3-B2**: changelog adopts `repository_version` boundary check at `load_window` — spec body says "stays unchanged."
- **FORGE-R3-B1**: changelog says `ExtraReports` is `BTreeMap` — spec body still shows `HashMap`.
- **FORGE-R3-B4**: `FilterParseError` 7-variant sketch claimed — spec has no sketch.
- **WIRE-R3-B5**: Tier-1 per-report file format claimed — no spec body table for it.
- **WIRE-R3-B1**: `extra_reports` runtime-only stated in plan — never in spec body.
- **WIRE-R3-B4**: `data/api-probe-2026-05-02.txt` artifact promised since R1 — file does not exist.
- **TAPE-R3**: per-endpoint seasonId fence claimed for v0.3 — no spec body section.
- **TAPE-R3**: rate-limit policy entirely missing.
- **EDGE-R2 follow-up**: OnIceGoals trade-window guard never made it into the `read()` code sketch; NaN/inf rejection, multi-filter normalization, empty-key error path all referenced obliquely but never spelled out.

Reviewers who passed are reviewers whose v0.2/v0.3 fixes happened to land in spec body cleanly — KEEL, GLASS, SCOUT (R2 deltas), PACE (R2 deltas). The failure mode is uniformly drift, not design error.

---

## Cleared in v0.3 (4 roles)

- **KEEL R3**: 3/3 BLOCKERs cleared. Alias-map ownership separate methods land in spec; L.5b sweep enumeration in spec body §"Site integration"; HTTP server Tier-2 visibility documented as "Tier-1 typed-only" with explicit cross-ref to AI-07.
- **GLASS R3**: 2/2 BLOCKERs cleared. `[`/`]` keybind consistent across spec sketches; `<space>` collision resolved with `Tab` for section toggle.
- **SCOUT R2 deltas**: 4/4 BLOCKERs cleared. xG family (9 stats) added to Possession + Goalie categories. (Note: SCOUT-R2 FIXITs F2 / F3 / F5 — recategorize FaceoffWinPct + EvenStrengthTimeOnIcePerGame, add PpAssists raw — were claimed in v0.3 prose but body never applied; rolled into v0.4.)
- **PACE R2 deltas**: 4/6 cleared. `f64::EPSILON` replaced with unit-keyed tolerance for `Equals`; MIN_GP guard on derived per-game stats. (F2 deferred to L.6 acceptably; F3 strict-propagation aggregate `read()` rolled into v0.4.)

---

## R3 BLOCKERs (must-fix in v0.4)

### Spec-body drift — v0.3 changelog claimed fixes, body still shows v0.2 (or older) text (10)

- **L3-B1** [HART-R3-B1] **`extra_reports` cascade-eviction (DI-12).** Spec body §"Repository lifecycle" missing entirely; need cascade rule + L0 test name.
- **L3-B2** [HART-R3-B2] **`repository_version` boundary check at `load_window` (DI-28).** Spec body must pin the failure point + L1 test name.
- **L3-B3** [FORGE-R3-B1] **`ExtraReports: BTreeMap`.** Spec §Public types still has `HashMap` from v0.1.
- **L3-B4** [FORGE-R3-B4] **`FilterParseError` 7-variant enum.** No sketch in spec body.
- **L3-B5** [FORGE-R3-B5] **DI-25 frozen-golden precision.** Spec still reads "byte-identical" without saying *to what* — round-trip self-equality vs frozen capture.
- **L3-B9** [WIRE-R3-B4] **`data/api-probe-2026-05-02.txt` artifact.** Promised since R1; still missing.
- **L3-B10** [WIRE-R3-B5] **Tier-1 per-report file format.** Spec body has no substruct→filename→endpoint table.
- **L3-B11** [WIRE-R3-B1] **`extra_reports` runtime-only declaration.** Plan §"v0.2 → v0.3 changelog" L2-B15 says yes; spec body never says it.
- **L3-B12** [WIRE-R3-F5] **`load_report_with_fallback<T>`.** Signature claimed L.1 deliverable; never sketched.
- **L3-B13/B14** [TAPE-R3] **Per-endpoint seasonId fence + rate-limit policy.** Both claimed; both missing from spec body.

### Test contract precision (3)

- **L3-B6** [BENCH-R3-1] **`stat_catalog_variants.rs` named L.2 deliverable.** Plan L.2 row vague; needs explicit fixture filename + 6-variant enumeration.
- **L3-B7** [BENCH-R3-2] **Two-fence stdout golden capture timing.** Plan still implies post-L.5 only; sort ordering changes ride L.3, not L.5 — capture must happen pre-L.3 and reassert at TWO fences (post-L.3, post-L.5).
- **L3-B8** [BENCH-R3-3] **Five named legacy schemes.** Plan + spec mention "legacy schemes" but never list the 5 names.

### EDGE-R2 grammar precision (4)

- **L3-B15** [EDGE-R2] **OnIceGoals trade-window guard explicit in `read()`.** Spec sketch should fire DI-11 at category boundary, not require remembering per-arm.
- **L3-B16** [EDGE-R2] **NaN/inf rejection at construction.** Spec mentions in II-05 but `StatFilter` declaration doesn't show the gate.
- **L3-B17** [EDGE-R2] **Multi-filter same-StatId normalization.** No rule for `--filter "hits-min 50" --filter "hits-min 100"` composition.
- **L3-B18** [EDGE-R2] **Empty/whitespace stat-key error path.** `EmptyStatKey` variant needs to cover both empty and whitespace-only.

### SCOUT/PACE v0.3 follow-through (4)

- **L3-B19** [PACE-R2 F3] **`aggregate_read()` strict-propagation rule.** Multi-season `query player --seasons N` behavior unspecified.
- **L3-B20** [SCOUT-R2 F2] **`FaceoffWinPct` actually moved to TwoWay.** v0.3 prose says yes; v0.3 body lists it under SpecialTeams.
- **L3-B21** [SCOUT-R2 F3] **`EvenStrengthTimeOnIcePerGame` actually moved to TimeOnIce.** Same pattern as B20.
- **L3-B22** [SCOUT-R2 F5] **`PpAssists` raw count.** Listed only `PpAssistsPer60`; raw missing.

---

## What v0.4 does

1. **Spec-body sweep.** Apply L3-B1 through L3-B22 to the spec body. No new design decisions — every fix is transcribing what the changelogs already claimed, plus the four SCOUT/PACE category corrections.
2. **Plan refresh.** Sub-phase deliverables list explicit named files (`stat_catalog_variants.rs`, the 5 named scheme TOMLs); two-fence stdout-golden capture timing pinned in L.3 + L.5; new invariants DI-12, DI-26, DI-27, DI-28, DI-29, AI-09 added to plan invariant table.
3. **Stat count update.** 107 → 108 (`PpAssists` added; category moves are zero-net).
4. **No code yet.** v0.4 is doc-only. After v0.4 ships and (optionally) R4 verifies clean, L.1 implementation begins.

---

## Verdict

R3 confirmed the v0.2 → v0.3 → v0.4 pattern: each version's changelog is good intent, weak execution. The cost of skipping a body-vs-changelog audit is exactly one more round of drift detection. v0.4 is the literal sweep — once it ships, L.1 can kick off cleanly.

**Estimated v0.4 cost**: ~2-3 hours of doc work. No new design decisions. Recommended R4 sanity pass before L.1: 6 spot-checks across the still-BLOCKER list, ~30 minutes of reviewer time per role.
