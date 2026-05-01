# Phase T — Transactions Hub

**Status**: Draft v0.2 — incorporates WIRE / TAPE / EDGE / GLASS spec reviews
and BENCH / FORGE plan reviews
**Date**: 2026-04-30
**Spec**: design/specs/transactions.md (v0.2)
**Target**: v0.11.0 (T.0 through T.4); v0.11.1 follow-up (T.5 + T.6)

---

## Goal

Ship a league-wide NHL transactions feed (trades, waivers, signings, IR,
recalls, reassignments) sourced from ESPN's site.api, surfaced as both a
CLI report (`icelines transactions`, `icelines x transactions`) and a new
TUI tab. Bundled per-season snapshots so the feature works offline.

Seven sub-phases, each independently shippable. T.0–T.4 are the v0.11.0
release cut. T.5 adds the TUI surface. T.6 backfills historical seasons.

**Key revisions from review consensus**:
- **No `TransactionSource` trait in v1** (FORGE: premature abstraction;
  promote when source #2 actually lands).
- **Atomic-rename helper lands first** as its own sub-phase T.0 (FORGE:
  used by goalies path too; safer as standalone change).
- **`name.rs` → `name/mod.rs` + `name/aliases.rs`** done in T.1 (FORGE:
  the directory does not exist today).
- **Property tests + golden files for the classifier** (BENCH: fixtures
  alone don't catch reorders).
- **Live-network probe lives in `examples/`, not `tests/`** (BENCH: the
  cross-cutting "no live network in tests" rule applies).
- **Typed error variants** enumerated in T.2 (FORGE: `CircuitBreakerTripped`,
  `EmptyResponseRefused`, `HtmlBodyResponse`).
- **Typed `SnapshotMetaFlags`** in T.3 (FORGE: not unstructured `Value`).
- **Glyph-on-every-kind text snapshot test** in T.5 (BENCH: makes the
  colorblind contract assertable without color).

---

## Sub-phase T.0 — Atomic snapshot writer (split out from T.3)

**Pulled forward from T.3 per FORGE.** Used by T.3 AND by an opportunistic
goalies-path adoption — making it a standalone change keeps the diff small
and lets us prove correctness once.

**Crate boundaries**:
- `icelines-fetch/src/snapshot.rs` — add `atomic_write_json<T: Serialize>(path, value)`:
  writes to `path.tmp`, fsyncs, then `rename(path.tmp, path)`. Also writes
  the previous file (if any) to `path.bak` before the rename, so corruption
  can recover.

**No new public surface beyond the helper.**

**Files modified**:
- `icelines-fetch/src/snapshot.rs` (helper added, exported `pub(crate)`)
- `icelines-fetch/src/repository.rs` OR wherever goalies snapshot writes —
  swap a single existing `std::fs::write(...)` to `atomic_write_json(...)`
  to validate the helper end-to-end before T.3 uses it.

**Tests (L0)**:
- `l0_atomic_write_json_helper` — write a fixture; assert no `.tmp` left,
  content matches.
- `l0_atomic_write_failure_keeps_prior` — pre-create the target with known
  content, force a write failure (e.g. read-only directory) AFTER the
  `.tmp` write but BEFORE the rename; assert original is intact.
- `l0_atomic_write_creates_bak` — overwrite an existing file; assert
  `path.bak` exists and contains the prior content.

**Acceptance**: helper green; goalies-path test (`l1_goalie_repo_*`)
passes via the new write path.

**Estimated effort**: 0.5 day.

---

## Sub-phase T.1 — Schema, classifier, alias table, abbrev map

**Crate boundaries**:
- `icelines-core/src/name/` — **converted from single-file `name.rs` into
  a directory** (FORGE):
  - `icelines-core/src/name/mod.rs` (exports the existing `normalize_name()`)
  - `icelines-core/src/name/aliases.rs` (new — bidirectional alias table)
- `icelines-core/src/transactions/` — new module:
  - `mod.rs` — public types, `CURRENT_CLASSIFIER_VERSION`,
    `TRANSACTIONS_EARLIEST_SEASON`.
  - `classifier.rs` — anchored regexes, `classify()`.
  - `grouping.rs` — `trade_group_id()`, `link_players()`.
  - `sanitize.rs` — control-char strip + whitespace normalize.
- `icelines-fetch/src/schema.rs` — add `RawTransaction` + `RawTransactionTeam`
  with `deny_unknown_fields` ON.
- `icelines-fetch/src/teams.rs` — extend with `espn_to_nhl_abbrev(abbrev,
  season) -> Option<TeamAbbr>` including ARI/PHX/ATL/MOJ historical entries.

**Alias table shape (FORGE)**:
```rust
// icelines-core/src/name/aliases.rs
const ALIAS_TABLE: &[(&str, &[&str])] = &[
    ("Michael",  &["Mike", "Mick"]),
    ("Thomas",   &["Tom", "Tommy"]),
    ("Alexander",&["Alex", "Aleksander"]),
    ("Matthew",  &["Mat", "Matt"]),
    // ...
];

static CANONICAL_BY_VARIANT: OnceLock<HashMap<&str, &str>> = OnceLock::new();
pub fn canonical_for(name: &str) -> &str { /* lookup or passthrough */ }
```

**No network**. Pure logic only.

**Files added/modified**:
- `icelines-core/src/lib.rs` (re-exports)
- `icelines-core/src/name/mod.rs` (was `name.rs`)
- `icelines-core/src/name/aliases.rs` (new)
- `icelines-core/src/transactions/mod.rs` (new)
- `icelines-core/src/transactions/classifier.rs` (new)
- `icelines-core/src/transactions/grouping.rs` (new)
- `icelines-core/src/transactions/sanitize.rs` (new)
- `icelines-core/tests/fixtures/espn_descriptions.json` — captured real
  ESPN strings (≥30 across all kinds incl. negative cases).
- `icelines-fetch/src/teams.rs` (extended)
- `icelines-fetch/src/schema.rs` (extended)

**Tests** (per BENCH — example + property + golden):

L0 — example tests:
- `l0_classify_pto_is_other` — "Signed F X to a PTO" → Other.
- `l0_classify_rights_acquisition_is_other` — "Acquired the rights to RFA F X" → Other.
- `l0_classify_intl_loan_is_other` — "Loaned G X to Sweden for IIHF" → Other.
- `l0_classify_emergency_recall_is_recall`.
- `l0_classify_returned_to_ahl_is_reassignment`.
- `l0_classify_re_signed_is_signing`.
- `l0_classify_unknown_pattern_is_other`.

L0 — property tests (using `proptest`):
- `l0_classifier_property_pto_always_other` — proptest: any string
  containing case-insensitive "PTO" or "professional tryout" classifies
  Other, never Signing.
- `l0_classifier_property_rights_never_trade` — any string with
  "acquired the rights" or "negotiating rights" classifies Other.
- `l0_classifier_property_intl_loan_never_reassign` — "(IIHF|World)" +
  "loaned" classifies Other.
- `l0_sanitize_idempotent` — `sanitize(sanitize(s)) == sanitize(s)` for
  arbitrary unicode.
- `l0_sanitize_preserves_alphanumeric` — `[a-zA-Z0-9]` chars survive.
- `l0_trade_group_id_permutation_invariant` — shuffle teams and players;
  hash is unchanged.

L0 — golden file:
- `l0_classifier_golden_v1` — `insta`-snapshot of `(description, kind)`
  pairs over the full fixture file. Catches accidental table reorder.
  When `CURRENT_CLASSIFIER_VERSION` bumps to 2, this stays as a regression
  anchor for v1; v2 needs its own golden.
- `l0_classifier_other_rate_under_5pct` — load the captured ESPN payload
  fixture; assert `other_rate < 5%`. Locks the spec's observability rule
  into CI, not just live runs.

L0 — abbrev mapping:
- `l0_espn_to_nhl_tb_to_tbl` — `("TB", _) → Some(TBL)`.
- `l0_espn_to_nhl_sj_to_sjs` — `("SJ", _) → Some(SJS)`.
- `l0_espn_to_nhl_ari_pre_2024_25` — `("ARI", "20232024") → Some(ARI)`.
- `l0_espn_to_nhl_ari_post_2024_25` — `("ARI", "20242025") → Some(UTA)`.
- `l0_espn_to_nhl_atl_thrasher_era` — `("ATL", "20102011") → Some(ATL)`.
- `l0_espn_to_nhl_unknown_returns_none` — `("BOGUS", _) → None`.
- `l0_espn_to_nhl_canonical_passthrough` — `("EDM", _) → Some(EDM)`.

L0 — alias table:
- `l0_alias_mike_matches_michael` — `canonical_for("Mike") == "Michael"`.
- `l0_alias_alex_matches_aleksander` — `canonical_for("Alex") == "Alexander"`
  AND `canonical_for("Aleksander") == "Alexander"` (Cyrillic transliteration).
- `l0_alias_unknown_passthrough` — `canonical_for("Tyler") == "Tyler"`.
- `l0_nfd_strip_combining_marks` — `strip_combining("Hörnqvist") == "Hornqvist"`.

**Acceptance**: `cargo test -p icelines-core transactions` green; no
warnings; `proptest` dependency added to `icelines-core` dev-deps;
`insta` dependency added.

**Estimated effort**: 2 days (was 1.5; added properties + golden + alias
module split).

---

## Sub-phase T.2 — `EspnSource` fetcher + L1 mock + historical probe

**Crate boundaries**:
- `icelines-fetch/src/transactions/mod.rs` — new module. Public types
  `FetchOutcome`, `EspnSource`. **No trait in v1** (FORGE: ship as
  concrete struct; promote when source #2 lands).
- `icelines-fetch/src/transactions/espn.rs` — implementation (HTTP client,
  pagination, retry/backoff, schema-fallback to `serde_json::Value`).
- `icelines-fetch/src/error.rs` — extend with new variants.

**Concrete `FetchOutcome` (FORGE: `Vec<String>` not `usize`)**:
```rust
pub struct FetchOutcome {
    pub rows: Vec<RawTransaction>,
    /// Field paths that were dropped via the schema-drift fallback, e.g.
    /// "team.logos[]". Empty = clean parse. Length feeds the observability
    /// counter; specific paths feed the WARN log.
    pub dropped_unknown_schema: Vec<String>,
    /// True when the source signaled partial data (circuit-break tripped
    /// before completion). Caller MUST NOT overwrite a richer snapshot
    /// with a partial one.
    pub partial: bool,
    /// ETag / Last-Modified for conditional re-fetch when supported.
    pub source_etag: Option<String>,
    /// Wall-clock at fetch start. Persisted in the snapshot as `fetched_at`.
    pub fetched_at: String,
}
```

**New error variants** (FORGE: enumerate explicitly):
```rust
// icelines-fetch/src/error.rs additions
pub enum FetchError {
    // ... existing variants ...
    CircuitBreakerTripped { url: String, after_failures: usize },
    EmptyResponseRefused  { season: String },
    HtmlBodyResponse      { url: String, content_type: String },
}
```

**Failure-mode handling per WIRE**:
- 429 → exponential backoff with jitter; max 3 retries; honor `Retry-After`.
- 5xx → same retry policy.
- 200 + empty array → return `FetchOutcome { rows: vec![], partial: false }`;
  **caller's responsibility** to refuse overwrite if a non-empty snapshot
  exists (T.3 handles this).
- 200 + HTML body → detect via `Content-Type`; return `HtmlBodyResponse`.
- Schema drift → `serde_json::Value` extraction; populate
  `dropped_unknown_schema`; continue.
- 3 consecutive non-200s in one paginated run → return `CircuitBreakerTripped`,
  `partial: true`.

**Historical probe — quarantined out of `cargo test`** (BENCH):
- Lives at `icelines-fetch/examples/probe_espn_seasons.rs`, NOT under
  `tests/`. Run manually: `cargo run --example probe_espn_seasons --
  20212022 20252026`.
- Output is hand-pasted into `icelines_core::transactions::TRANSACTIONS_EARLIEST_SEASON`.
- L0 test `l0_transactions_earliest_season_constant_is_set` asserts the
  constant is non-empty and parses as an 8-digit season ID.

**Files added/modified**:
- `icelines-fetch/src/transactions/mod.rs` (new)
- `icelines-fetch/src/transactions/espn.rs` (new)
- `icelines-fetch/src/error.rs` (new variants)
- `icelines-fetch/src/lib.rs` (export EspnSource + FetchOutcome)
- `icelines-fetch/tests/transactions_mock.rs` (new — L1 against httpmock)
- `icelines-fetch/tests/fixtures/espn_transactions_response.json` —
  captured real ESPN payload (one-time capture; never re-fetched in tests).
- `icelines-fetch/examples/probe_espn_seasons.rs` (new — quarantined)

**Tests** (per BENCH):

L1 against `httpmock`:
- `l1_mock_200_fixture_payload` — happy path, rows extracted, no dropped fields.
- `l1_mock_429_with_retry_after_succeeds` — succeeds on retry.
- `l1_mock_429_no_retry_after_uses_backoff` — backs off without the header.
- `l1_mock_429_jitter_bounded` — assert sleep duration ∈ `[base, base*2]`;
  catches a "fixed delay regression."
- `l1_mock_5xx_then_200_no_circuit_break` — 2 fails then success
  succeeds (only ≥3 consecutive trips the breaker).
- `l1_mock_500_x3_circuit_breaks` — `CircuitBreakerTripped`, `partial: true`.
- `l1_mock_partial_paginated_does_not_overwrite` — page 1 ok, page 2 503×3
  → `partial: true`; T.3's snapshot writer (in T.3 tests) refuses overwrite.
- `l1_mock_html_body_returns_html_error` — `Content-Type: text/html`
  → `HtmlBodyResponse`; no panic, no serde feeding.
- `l1_mock_200_truncated_json` — body cuts off mid-array → `SchemaChanged`
  (not panic).
- `l1_mock_unknown_field_drops_to_value_fallback` — extra unknown field;
  row still extracted; `dropped_unknown_schema` contains the field path.
- `l1_mock_response_shape_mutation_proptest` — proptest dropping random
  fields from the captured payload; runner never panics.
- `l1_mock_team_none_routes_to_LEAGUE_bucket` — row with no team payload
  is preserved with `team: None`.
- `l1_mock_pagination_three_pages` — `pageCount: 3`; all rows returned.
- `l0_transactions_earliest_season_constant_is_set` — constant validation.

**Acceptance**: `cargo test -p icelines-fetch transactions` green;
example compiles; `TRANSACTIONS_EARLIEST_SEASON` is set to a real value
verified by running the example.

**Estimated effort**: 1.5 days.

---

## Sub-phase T.3 — `icelines fetch transactions` + snapshot writer + meta flags

**Depends on**: T.0 (atomic-rename), T.1 (Transaction model), T.2 (EspnSource).

**Crate boundaries**:
- `icelines-cli/src/cli.rs` — add `FetchSubcommand::Transactions`.
- `icelines-cli/src/commands/fetch.rs` — add `do_transactions()` handler.
- `icelines-fetch/src/bundled.rs` — add `get_transactions()`,
  `get_transactions_installed()`, `load_transactions_with_fallback()`
  (mirrors goalies pattern).
- `icelines-fetch/src/snapshot.rs` — add typed `SnapshotMetaFlags`
  (FORGE: not unstructured `Value`).

**Provenance envelope**:
```json
{
  "season":              "20252026",
  "source":              "espn",
  "fetched_at":          "2026-04-30T14:32:11-04:00",
  "classifier_version":  1,
  "rows":                [ /* Vec<Transaction> */ ]
}
```
Per-row `classifier_version` is also stored (FORGE: keep both — envelope
is a fast-path skip; per-row handles partial-fetch merge case).

**Typed `SnapshotMetaFlags`** (FORGE):
```rust
// icelines-fetch/src/snapshot.rs
#[derive(Serialize, Deserialize, Default)]
pub struct SnapshotMetaFlags {
    pub transactions_stale:      bool,
    pub transactions_last_error: Option<String>,
}
```
Persisted at `~/.icelines/snapshots/{season}/_meta.json`. Future tiers
add fields with `#[serde(default)]` for backwards compat.

**Re-classification policy on load**:
- If envelope `classifier_version < CURRENT_CLASSIFIER_VERSION`:
  iterate rows, `classify(row.description)`, set `row.kind` and
  `row.classifier_version = CURRENT`. Do NOT downgrade
  (`row.classifier_version > CURRENT_CLASSIFIER_VERSION` → leave alone).

**`fetch all` integration**:
- Call `do_transactions()` as best-effort.
- Failure → log WARN AND set `SnapshotMetaFlags::transactions_stale = true`,
  `transactions_last_error = Some(reason)`.
- Success → clear both.
- `fetch all`'s end-of-run summary lists transactions status alongside
  other tiers.

**Bundle 25-26 transactions**: at end of T.3, run `icelines fetch
transactions` once and copy the resulting snapshot into
`data/seasons/20252026/transactions.json`.

**Files added/modified**:
- `icelines-cli/src/cli.rs` (extend FetchSubcommand)
- `icelines-cli/src/commands/fetch.rs` (do_transactions)
- `icelines-cli/src/main.rs` (dispatch)
- `icelines-fetch/src/bundled.rs` (load_transactions_with_fallback)
- `icelines-fetch/src/snapshot.rs` (SnapshotMetaFlags struct)
- `data/seasons/20252026/transactions.json` (new bundled file)

**Tests**:

L0:
- `l0_snapshot_meta_flags_default_all_false` — `SnapshotMetaFlags::default()`
  has `transactions_stale = false`.
- `l0_snapshot_meta_flags_serde_roundtrip` — JSON round-trip preserves
  fields; missing optional fields parse as defaults.

L1:
- `l1_load_transactions_with_fallback_finds_bundled` — embedded bundle
  loads when no snapshot exists.
- `l1_corrupted_snapshot_truncates_to_bak` — primary fails; `.bak`
  recovery succeeds.
- `l1_corrupted_snapshot_AND_bak_both_bad_returns_err` — both files
  garbage; loader returns Err with hint, no panic.
- `l1_classifier_version_stale_triggers_reclassify` — envelope
  `classifier_version = 0`; loaded rows have `classifier_version =
  CURRENT` and re-evaluated `kind`.
- `l1_classifier_version_downgrade_ignored` — envelope `classifier_version
  = CURRENT + 1` (forward-compat case); loader does NOT downgrade.
- `l1_meta_flag_set_on_fetch_failure` — mock fetcher returns Err; meta
  file has `transactions_stale = true` after `do_transactions()`.
- `l1_meta_flag_cleared_on_fetch_success` — flag was true; successful
  fetch clears it.
- `l1_partial_outcome_does_not_overwrite_richer_snapshot` — existing
  snapshot has 500 rows; new fetch returns `FetchOutcome { partial:
  true, rows: 50 }` → snapshot is preserved.
- `l1_e2e_pipeline_fixture` (cross-phase lock — BENCH) — captured ESPN
  payload → fetch → classify → snapshot → load → assert known row by id.

L2:
- `l2_cmd_fetch_transactions_dry_run_exits_zero`.

**Acceptance**: bundled `data/seasons/20252026/transactions.json` exists
with non-zero row count; full workspace `cargo test` green.

**Estimated effort**: 1 day.

---

## Sub-phase T.4 — `icelines transactions` CLI + `x transactions`

**Pre-flight check** (FORGE): grep `icelines-cli/tests/system_tests.rs`
for any test that asserts on the EXACT count of `ExportShape` variants
or the exact `x --help` output before adding `Transactions`. If such a
test exists, update it in this sub-phase.

**Crate boundaries**:
- `icelines-cli/src/cli.rs` — add `Commands::Transactions { ... }` and
  `ExportShape::Transactions`.
- `icelines-cli/src/commands/transactions.rs` — new file. `run()` loads
  snapshot via `load_transactions_with_fallback`, applies filters,
  re-classifies if `classifier_version` is stale, emits via
  `commands::output::Format`.
- `icelines-cli/src/main.rs` — wire dispatch + `x transactions`.

**Filter validation per EDGE**:
- `--kind unknown` → exit non-zero with valid-list hint.
- `--since 2026-13-40` → exit non-zero with format hint.
- `--since > --until` → exit non-zero with explicit message.
- `--team LEAGUE` → returns rows where `team == None`.
- `--team edm` (lowercase) → normalized to `EDM`.

**Stale-flag display**:
```
WARN: transactions snapshot is N days stale (last fetch failed: <reason>)
```
Read from `SnapshotMetaFlags`.

**Files added/modified**:
- `icelines-cli/src/cli.rs` (Commands::Transactions + ExportShape::Transactions)
- `icelines-cli/src/commands/transactions.rs` (new)
- `icelines-cli/src/commands/mod.rs` (register)
- `icelines-cli/src/main.rs` (dispatch + x dispatch)
- `icelines-cli/tests/system_tests.rs` (new L2 tests)

**Tests** (per BENCH — output-format gaps):

L2:
- `l2_cmd_transactions_exits_zero`.
- `l2_cmd_transactions_csv_emits_header_and_rows`.
- `l2_cmd_transactions_team_edm_filters`.
- `l2_cmd_transactions_team_LEAGUE_returns_teamless`.
- `l2_cmd_transactions_kind_trade_filters`.
- `l2_cmd_transactions_out_writes_file`.
- `l2_cmd_transactions_invalid_kind_exits_nonzero`.
- `l2_cmd_transactions_invalid_since_exits_nonzero`.
- `l2_cmd_transactions_since_after_until_exits_nonzero`.
- `l2_cmd_transactions_csv_description_with_comma_quoted` — description
  containing `,` survives RFC-4180 round-trip.
- `l2_cmd_transactions_csv_description_with_quote_escaped` — embedded
  `"` doubles correctly.
- `l2_cmd_transactions_json_roundtrips_to_struct` — `--json` output
  deserializes back to `Vec<Transaction>` cleanly.
- `l2_cmd_transactions_csv_and_json_mutually_exclusive` — both flags →
  exit non-zero.
- `l2_cmd_transactions_top_combined_with_kind` — `--kind trade --top 3`
  returns exactly 3 trades.
- `l2_cmd_transactions_lowercase_team_normalized`.
- `l2_x_transactions_defaults_to_csv`.
- `l2_cmd_transactions_pre_coverage_season_helpful_message` —
  `--season 19951996` shows the `TRANSACTIONS_EARLIEST_SEASON` message
  (NOT a `run icelines fetch` hint that would 404).

**Acceptance**: all new L2 tests pass; `cargo test --workspace` green;
`icelines x --help` lists `transactions`.

**Estimated effort**: 1 day.

**v0.11.0 ships here.** Tag and push at this point.

---

## Sub-phase T.5 — TUI Transactions tab

**Crate boundaries**:
- `icelines-cli/src/tui/screens/transactions.rs` — new file (flat,
  matching other screens per FORGE). Contains `render()`,
  `render_detail_pane()`, `render_empty_legend_card()`.
- `icelines-cli/src/tui/screens/mod.rs` — register tab in slot 7
  (push Playoffs to slot 8); add `Screen::Transactions` and
  `Screen::TransactionDetail(idx)`.
- `icelines-cli/src/tui/app.rs` — app state (filter team / kind / date,
  selected row, detail-pane open flag); key handlers for `T`/`k`/`d`/`/`.
- `icelines-cli/src/tui/loader.rs` — load transactions in parallel with
  players + goalies; `LoadState.transactions` field.

**GLASS contract**:
- Column order: TEAM → KIND (glyph + bold-color) → DATE → DESCRIPTION
  (ellipsis-truncated, never wrapped).
- Glyph map: `⇄ Trade  $ Signing  ↑ Recall  ↓ Reassign  ⊘ WaiverPlace
  ↻ WaiverClear  + WaiverClaim  ✚ IR  ◇ Other`.
- Color the kind token only (bold). Trade=Cyan, Signing=Yellow,
  Recall=Green, Reassign=DarkGray, WaiverPlace/Clear/Claim=Blue,
  IR=Red, Other=White. (Magenta dropped per GLASS.)
- Title bar: `Transactions · ESPN · as of {fetched_at}` (red text if
  `> 7 days`).
- `T` for team filter (uppercase — avoids Schedule's `t` for "today").
- `k` cycles kind; `d` opens date jump; `/` opens filter input.
- `[STALE]` red prefix when `transactions_stale` OR fetched_at > 7 days.

**Files added/modified**:
- `icelines-cli/src/tui/screens/transactions.rs` (new)
- `icelines-cli/src/tui/screens/mod.rs` (register tab; tab_for_screen + tab_labels)
- `icelines-cli/src/tui/app.rs` (state + key handlers)
- `icelines-cli/src/tui/loader.rs` (load transactions)

**Tests** (per BENCH — GLASS contract is testable via text snapshots):

L1 (using ratatui's `TestBackend` — same pattern as existing TUI tests):
- `l1_tui_glyph_present_for_every_kind` — render fixture with one row
  per `TransactionKind`; **strip ANSI**; assert each glyph
  (`⇄ $ ↑ ↓ ⊘ ↻ + ✚ ◇`) appears once. **This is the colorblind safety test.**
- `l1_tui_only_kind_token_colored` — assert team/date/description spans
  use `Style::default()`; only the kind span carries `Modifier::BOLD` +
  non-default fg.
- `l1_tui_stale_marker_red_prefix` — `[STALE]` present AND styled red
  in title bar.
- `l1_tui_empty_legend_card_includes_all_glyphs` — pre-2018-19 season
  shows the rendered card with all 9 glyphs.
- `l1_tui_description_truncated_not_wrapped` — long description, narrow
  viewport: row count unchanged; ellipsis present.
- `l1_tui_filter_by_team_reduces_rows`.
- `l1_tui_filter_by_kind_reduces_rows`.
- `l1_tui_T_uppercase_cycles_team_filter`.
- `l1_tui_detail_pane_opens_on_enter_closes_on_esc`.
- `l1_tui_tab_in_slot_7` — assert `tab_for_screen(Transactions) == 7`
  AND `tab_labels[7] == "Transactions"`.

**Acceptance**: TUI manual smoke per CLAUDE.md UI rules — start the TUI,
navigate to slot 7, filter by team, open a detail pane, confirm stale
marker behavior. All L1 tests green.

**Estimated effort**: 2 days.

---

## Sub-phase T.6 — Backfill bundled transactions for 21-22 → 24-25

**Prerequisite**: T.2's historical probe must have confirmed which
seasons ESPN's archive covers.

**Process per season** (manual, off-CI):
1. `icelines fetch transactions --season YYYYZZZZ` against ESPN.
2. Inspect snapshot row count + per-kind distribution.
3. Copy snapshot into `data/seasons/YYYYZZZZ/transactions.json`.

**Files modified**:
- `data/seasons/{21-22,22-23,23-24,24-25}/transactions.json` (new files)
- `icelines-fetch/src/bundled.rs` — embed via `include_bytes!`
  (`TRANSACTIONS_20242025`, etc., parallel to `GOALIES_20242025`).

**Tests** (per BENCH — re-classify drift trap):

L1:
- `l1_bundled_transactions_present_for_each_covered_season` —
  `get_transactions(s)` returns Some for every covered season.
- `l1_bundled_other_rate_under_5pct_per_season` — for each bundled
  season: `other_rate < 5%`. Catches "ESPN changed prose, our regex
  regressed, but we re-bundled anyway."
- `l1_bundled_smoke_per_kind` — every bundled season has `> 0` rows
  for at least Trade, Signing, Recall.
- `l1_bundled_reclassify_stable_golden` — for each bundled season,
  force `classifier_version = 0` on every row, run loader, assert
  resulting `kind` distribution matches a golden per-season fixture.
  When `CURRENT_CLASSIFIER_VERSION` bumps, the golden is regenerated
  in the same commit — so a bump that breaks bundled re-classification
  fails the test in CI.

**Acceptance**: TUI Transactions tab shows non-empty rows for any
covered historical season via the season picker.

**Estimated effort**: 0.5 days.

---

## Total estimated effort

| Sub-phase | Effort |
|-----------|--------|
| T.0 atomic snapshot writer | 0.5 d |
| T.1 schema + classifier (incl. property + golden tests) | 2.0 d |
| T.2 ESPN fetcher + probe | 1.5 d |
| T.3 fetch CLI + snapshot + meta flags | 1.0 d |
| T.4 transactions CLI | 1.0 d |
| **v0.11.0 ships** | **6.0 d** |
| T.5 TUI tab | 2.0 d |
| T.6 backfill | 0.5 d |
| **Phase T complete** | **8.5 d** |

---

## Cross-cutting acceptance criteria

Apply at end of every sub-phase:
- `cargo build --release -p icelines-cli` succeeds.
- `cargo test --workspace` green.
- `cargo clippy --workspace --tests -- -D warnings` clean for new code.
- New code observes the no-live-network-in-tests rule.
  - Live probe must be in `examples/` (see T.2).
  - Future test additions must use `httpmock` for any external HTTP.

Apply at end of T.4 (v0.11.0 release gate):
- `icelines transactions --csv` opens cleanly in Excel.
- `icelines x transactions --team EDM --out edm-moves.csv` works end-to-end.
- `icelines fetch all` succeeds even when ESPN is unreachable (degrades
  with the WARN + stale flag, doesn't fail the run).
- `icelines transactions --season 19951996` produces the
  TRANSACTIONS_EARLIEST_SEASON message.

---

## Open risks (revised)

1. **ESPN's endpoint disappears or schema changes**. Mitigation:
   WIRE-mandated schema-drift fallback + cache layer + `_meta.json`
   stale flag mean we degrade visibly, not silently. Worst case: ship
   a PHR-RSS-backed source as a follow-up; the trait shape is intentionally
   absent in v1 but cheap to introduce when needed.
2. **Classifier coverage gaps on first ship**. Mitigation: property tests
   pin invariants (PTO never Signing, etc.); golden file pins example
   set; `other_rate < 5%` CI fixture catches prose drift; `classifier_version`
   lets us re-classify bundled snapshots on a regex update without
   re-fetching.
3. **Player-link false positives**. Mitigation: team-disambiguated +
   ≥0.85 score threshold + alias table + NFD normalization. Below
   threshold returns no link rather than wrong link.
4. **TUI complexity creep in T.5**. Mitigation: filter UX is iterative;
   ship T.5 with column layout + glyph legend, defer detail pane to
   T.5.1 if the right-side split takes too long.
5. **Hidden golden test regressions when bumping classifier_version**
   (BENCH). Mitigation: `l1_bundled_reclassify_stable_golden` per
   season fails CI if a bump silently changes bundled outcomes.
6. **`x --help` golden test breakage** (FORGE). Mitigation: pre-flight
   grep in T.4; update if found.

---

## Memory hooks

After Phase T closes, update `C:/Users/giodl/.claude/projects/C--src-ICELINES/memory/`:
- Move `transactions_hub_plan.md` from "future phase" to "shipped in v0.11.0".
- Add a project memory: `transactions_classifier_drift.md` capturing the
  observability rule (`other_rate < 5%`) and what to do when ESPN changes
  prose patterns.
