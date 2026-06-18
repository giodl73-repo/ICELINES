# Phase Hurricane — Ship Signals to a surface + product-gap roadmap

> **Named for the Carolina Hurricanes' 2026 Stanley Cup win.** Trophy/team phase
> naming follows the IceLines convention (Norris, Masterton, Art Ross, Jack Adams).
> Phase Hurricane is the product-analytics push: turn the differentiated **Signals**
> bet into a shipped surface, then close the modern-analytics gaps.

**Created:** 2026-06-17
**Status:** Deliverable 1 SHIPPED (WP-010 pulse-03, 2026-06-18) — `icelines signals`
CLI + `signals.v1` JSON live, L0+L2 green, docs/parity/wave updated. Deliverable 1b
(TUI/Web parity) and 2–6 pending.
**Frame:** product evaluation found IceLines is a great *offline/scriptable/fantasy*
tool but is missing the modern public-analytics layer. The highest-leverage,
most-tractable next step is to **ship the already-built Signals metric family to a
real surface** (WP-010 is core-only today). This doc specifies that deliverable
in executable detail and sketches the rest of the roadmap as follow-on pulses.

---

## 0. Product-gap roadmap (priority order)

This is the strategic frame. Each item below is a future work package/pulse; only
**Deliverable 1** is fully specified in this doc.

| # | Gap | Why it matters | Tractability |
|---|---|---|---|
| **1** | **Signals have no user surface** | The one *differentiated* metric bet; methodology + ViewModel already built (WP-010). Shipping it is the cheapest "deep analytics" win. | **High — half-built. Do first.** |
| 2 | MoneyPuck data under-surfaced | Its CSVs already carry on-ice / deployment / shot data we fetch but don't expose. Closest path to real on-ice depth. | Medium |
| 3 | No rest-of-season projections w/ confidence | #1 fantasy ask; today pace is descriptive-only. | Medium-Large |
| 4 | Goalie eval shallow (SV%/GAA); GSAx only emerging | Table stakes for modern goalie analysis. | Medium |
| 5 | "38 seasons" reads deeper than it is (~5 modern Tier-1) | Honesty gap; perspective claims over skeleton seasons. | Small (UX/disclosure) |
| 6 | No visualization (text/tables only) | Loses the "publication-grade" comparison vs HockeyViz/MoneyPuck. | Large |

Conceded out of scope (keep conceded): NHL Edge skating speed, shot-location
heatmaps, predictive "value over replacement", salary-cap value. Revisit only as
deliberate scope expansions.

---

## Deliverable 1 — `icelines signals` CLI surface (WP-010 pulse-03/04)

### Goal
A read-only `icelines signals "<player>"` command (text + `--json`) that renders
the existing `PlayerSignalsView` with full evidence/disclosure honesty. This is
the **minimum promotion step**: it surfaces Signals on one surface (+ JSON twin)
**without** promoting them into `StatId`, leaderboards, or the `--filter` catalog
— exactly what `design/specs/icelines-signals.md` §Promotion rule allows when the
gates below are met.

### What already exists (do not rebuild)
- `icelines-core::signal_metrics` — `SignalMetricId::all()` (3 signals), `descriptor()`,
  `evidence(view)`, `read(view) -> Option<f64>`. Units, polarity, methodology,
  limitations, evidence tiers (`Full`/`Partial`/`Missing`) all defined.
- `icelines-core::view_model::signals::PlayerSignalsView::from_player(ctx, &PlayerView)`
  and `PlayerSignalRow` — carries value `Option<f64>`, `evidence_tier`,
  `missing_inputs`, `methodology`, `limitations`, plus view-level `disclosures` and
  `non_claims`. **It is `Serialize`/`Deserialize` → JSON is nearly free.**

### Implementation steps

**Step 0 — wave bookkeeping.** Decide: extend WP-010 with pulse-03 (recommended)
vs new WP. Create `context/waves/2026-06-02-vtrace-wp010-signals/pulses/pulse-03.md`
and add the row to that wave's `WAVE.md` pulse log. Add an entry to
`design/plans/INDEX.md`.

**Step 1 — core read path (icelines-core/icelines-fetch, if any).** None expected:
`PlayerSignalsView::from_player` already takes a `PlayerView`. Confirm a player can
be resolved + loaded the same way `query player` does:
`icelines_fetch::stats_loader::resolve_player_id_by_name` + `load_player_career_into_repo`
(see `icelines-cli/src/commands/query.rs::run_player` ~line 1685 for the exact
pattern to mirror — historical-name fallback included). Keep all compute in core;
the CLI only resolves + renders.

**Step 2 — new command module `icelines-cli/src/commands/signals.rs`.**
- `pub async fn run_signals(args: SignalsArgs) -> anyhow::Result<()>`.
- Resolve player → load → build `PlayerView` for the active `(season, season_type)`
  → `let view = PlayerSignalsView::from_player(ctx, &player);`
- Text render: one row per signal: `short_label`, value formatted to unit
  (`per 60` → 2 dp) **or `—` / "unavailable"** when `value` is `None`, a polarity
  arrow (higher-better ↑ / lower-better ↓ / neutral ·), and the evidence tier.
  Below the table: methodology + limitations footnotes, then the view's
  `disclosures` and `non_claims` lines verbatim. **Never print 0.0 for a missing
  value** (spec §Evidence contract).
- JSON render (`--json`): serialize a frozen `signals.v1` envelope wrapping
  `PlayerSignalsView` (mirror the envelope shape used by `leaders_json_envelope`
  in `commands/query.rs`). Additive-only.
- Honor `--min-gp` (default to the signal's threshold; below-threshold → `None`
  already handled in core, but surface the reason).

**Step 3 — register the command.**
- Add `Signals { player: String, season: Option<u32>, json: bool, min_gp: Option<u32> }`
  to the subcommand enum in `icelines-cli/src/cli.rs` with a `///` `long_about`
  that includes: one example, the unit/polarity legend, and **the non-claim
  sentence** (not a prediction/betting/injury/deployment/coaching tool).
- Dispatch it in `icelines-cli/src/main.rs` to `commands::signals::run_signals`.
- Add `mod signals;` to the commands module.

**Step 4 — tests (required by CLAUDE.md: new command ⇒ L2).**
- **L0** in `commands/signals.rs` (or core if render helpers land there): value
  formatting, `None` → "unavailable" (never 0.0), polarity arrow mapping, evidence
  tier label.
- **L2** in `icelines-cli/tests/` (new `signals_system.rs` or extend an existing
  system test): invoke the compiled binary `signals "Connor McDavid" --json`,
  assert envelope version, 3 rows, and that a known-missing-input case renders
  `null`/unavailable rather than `0`. Use bundled data only — **no live calls.**
- Consider a persona-wave scenario (`persona_wave11.rs` next index) for the
  "blogger checks a player's physical-engagement signal" flow.

**Step 5 — docs + matrix (same change, per CLAUDE.md docs rule).**
- `COMMANDS.md`: add `signals` with examples + the unit/polarity legend.
- `design/specs/surface-parity.md`: add a Signals row (CLI `done`, JSON `done`,
  TUI/Web `planned`).
- `design/specs/icelines-signals.md` §Promotion rule: record that the CLI/JSON
  surface is now live with product copy + disclosure; keep StatId/leaderboard
  promotion explicitly NOT done.
- README: optional one-line mention under the query/analysis section.

### Promotion-rule gate (must all be true before marking pulse done)
From `design/specs/icelines-signals.md`:
- [ ] product-copy review for the CLI surface (labels, methodology, limitations
      reviewed — run the `.roles` panel: **scout** correctness, **wire** schema,
      **bench** tests; `/review-specs`).
- [ ] source/completeness disclosure for unavailable + partial evidence (the
      `—`/"unavailable" + evidence-tier rendering).
- [ ] parity evidence IF >1 surface renders it. Pulse-03 ships **CLI + JSON twin
      from the same ViewModel** → document them as one ViewModel, two encodings.
      Defer TUI/Web to pulse-04 *with* a parity fence.
- [ ] cache-envelope methodology — N/A (not cached this pulse).
- [ ] explicit refusal of predictive/betting/injury/deployment/coaching claims
      (the `non_claims` line is printed and in `long_about`).

### Validation / green-bar checklist
- [ ] `cargo fmt --check` clean
- [ ] `cargo clippy -- -D warnings` clean (note: pre-existing unrelated lint debt
      in `icelines-fetch` / `icelines-web/tests/l1_router.rs` — don't expand it)
- [ ] `cargo test` green incl. new L0/L2
- [ ] `target/release/icelines signals "Connor McDavid"` and `--json` both sane,
      offline
- [ ] commit identity is `giodl73@gmail.com` (verify `git config user.email`)

### Estimated size
~1 new command module (~250 LOC render), enum+dispatch wiring (~20 LOC), ~8 L0 +
~3 L2 tests, doc edits. **One focused session.** No core/algorithm work — the math
and ViewModel are done.

---

## Deliverable 1b (follow-on pulse-04) — TUI + Web parity
- TUI: a key on the player card (e.g. `i` for "signals/insights") opening a
  Signals panel that renders the same `PlayerSignalsView`.
- Web: `/player/:id/signals` HTML + `/api/v1/player/:id/signals` JSON twin.
- Adds the **cross-surface parity fence** (CLI row == Web JSON row == TUI values),
  satisfying the promotion rule's parity clause for the full surface set. Mirror
  an existing parity test like `l2_query_goalies_cli_and_web_row_identity_match`.

---

## Deliverables 2–6 (sketches — separate work packages)

**2. Surface MoneyPuck on-ice/deployment (WP-009 cache or new).** Audit
`icelines-fetch/src/moneypuck.rs`: list every column the CSV provides vs every
`Player` field populated. Surface the unused on-ice/deployment columns as catalog
stats (with `requires fetch money-puck` gating, like existing xGF%). Cheapest path
to real on-ice depth; no new data source.

**3. Rest-of-season projections w/ confidence.** Extend the existing
`project` command (pace/regressed modes) with a projection that carries a
confidence band. Keep it descriptive + clearly bounded (Non-Goals still forbid
betting). New `icelines-core` module; L0-heavy.

**4. Goalie GSAx + workload.** Build on the emerging `gsax` work
(`grep -ri gsax icelines-core`) into a goalie analytics view: GSAx, high-danger
SV% if derivable, workload. New goalie ViewModel fields + parity.

**5. Season-depth honesty.** Make the `MODERN_BUNDLED_SEASONS` (~5) vs
`BUNDLED_SEASONS` (38) split explicit at every perspective/`--seasons` answer:
a completeness line on streak/career/seasons-aggregate output. Small, high-trust.

**6. Minimal visualization.** TUI sparklines for career trends + simple SVG charts
in `export`/web reports. Largest; do last or defer.

---

## Open decisions for the executor
1. **WP-010 pulse-03 vs new phase name?** Recommend pulse-03 (continuation) — the
   work package already exists and is the right home. Rename to a trophy-phase only
   if it grows past Signals.
2. **`--json` envelope name:** propose `signals.v1`. Confirm against the envelope
   naming in `docs/schemas` / existing `*.v1` envelopes before freezing.
3. **Does pulse-03 include TUI/Web, or just CLI+JSON?** Recommend CLI+JSON only,
   TUI/Web as pulse-04, to keep the parity-evidence clause cheap to satisfy.
