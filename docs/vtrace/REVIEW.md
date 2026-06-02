# VTRACE Review Record

## 2026-06-01 WP-009 Pulse 06 Coach Dashboard Cache Surface Review

**Scope:** First coach-specific product route for the major analytics cache.

**Decision:** `partial_pass`

Pulse 06 adds `/coach/dashboard` and `/api/v1/coach/dashboard` as
active-context coach surfaces. The routes default to
`coach_dashboard:<season>:<season_type>` from Web config and render through the
same `AnalyticsCacheStore`, consumer envelope, and `AnalyticsCacheConsumerView`
path as the named-cache report.

**Claims accepted**

- A coach-specific HTML route and JSON twin can read the active coach cache key
  without requiring the generic report query contract.
- Missing coach cache records produce explicit unavailable state and do not
  create cache directories during read.
- Existing coach cache records preserve metrics, source state, quality,
  methodology, disclosure, and non-claim copy through the consumer ViewModel.

**Claims not accepted**

- This is not a finished multi-panel coach dashboard suite.
- This does not add scout, player card, line, goalie, practice, postgame, agent,
  or predictive surfaces.
- This does not imply betting, injury, deployment, line-chemistry, or autonomous
  coaching authority.

**Status**

- WP-009 status: `partial`.
- VAL-011 status: `partial`.
- Continue broader hockey surfaces only behind the same cache envelope, active
  context, no-live-read behavior, and copy-review constraints.

## 2026-06-01 WP-009 Pulse 05 Named-Cache Web Report Review

**Scope:** First product-facing report surface for the major analytics cache.

**Decision:** `partial_pass`

Pulse 05 adds `/reports/analytics-cache` and
`/api/v1/reports/analytics-cache` as narrow cache-key-driven surfaces. The
handlers read an existing `AnalyticsCacheStore` record, project it through the
consumer envelope and `AnalyticsCacheConsumerView`, and render the preserved
metrics, source state, quality, methodology, disclosures, and non-claims without
recomputing analytics or fetching live data.

**Claims accepted**

- A named cache record can render as HTML and JSON through Web routes.
- Missing cache records produce explicit unavailable state instead of fabricated
  data.
- Route inventory and surface-parity records include the new report routes.

**Claims not accepted**

- This is not a full coach dashboard, opponent scout, player evidence card, line
  explorer, goalie readiness screen, practice report, postgame report, or agent
  action surface.
- This does not add a cache builder, metric catalog UI, automatic metric
  discovery, or broad invalidation/freshness family evidence.
- The report does not imply prediction accuracy, betting value, injury/deployment
  insight, line-chemistry causality, or autonomous coaching authority.

**Status**

- WP-009 status: `partial`.
- VAL-011 status: `partial`.
- Continue with broader product-facing surfaces only behind the same cache
  envelope and copy-review constraints.

## 2026-06-01 WP-009 Pulse 04 Consumer ViewModel Review

**Scope:** First downstream consumer fixture for the major analytics cache.

**Decision:** `partial_pass`

Pulse 04 adds an internal dashboard-style ViewModel that consumes an
`AnalyticsCacheConsumerEnvelope` and preserves the cache envelope, source state,
freshness, quality, methodology, disclosures, non-claims, supported metrics, and
prepared metric rows without recomputing cache meaning. A fetch-store fixture
proves a strict JSON cache record can feed the same consumer view.

This review does not claim shipped coach dashboard, scout report, player-card,
line, goalie, practice, postgame, or agent surfaces.

### Evidence inspected

- `icelines-core/src/view_model/analytics_cache_consumer.rs`
- `icelines-core/src/view_model/mod.rs`
- `icelines-core/src/lib.rs`
- `icelines-fetch/src/analytics_cache_store.rs`
- `context/waves/2026-06-01-vtrace-wp009-analytics-cache/pulses/pulse-04.md`
- `cargo test -p icelines-core analytics_cache --quiet`
- `cargo test -p icelines-fetch analytics_cache_store --quiet`
- `cargo fmt --check`
- `cargo clippy -p icelines-core --lib --tests -- -D warnings`
- `cargo clippy -p icelines-fetch --lib --tests -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
- `git diff --check`

### Accepted risks

- The consumer ViewModel is an internal fixture, not a shipped user-facing
  screen.
- Public product copy for coach/scout/report/card/line/goalie/practice/postgame
  surfaces still needs its own DCR, evidence, and review.

## 2026-06-01 WP-009 Pulse 03 Cache Store/Read Review

**Scope:** Strict `icelines-fetch::analytics_cache_store` storage/read and
invalidation slice.

**Decision:** `partial_pass`

Pulse 03 adds the first production-oriented analytics cache store/read path. The
store writes only validated `AnalyticsCacheRecord` JSON, reads through the core
schema/metric compatibility validator, refuses missing caches without creating
storage, preserves stale/partial/missing source state, supports explicit
invalidation by key, and leaves the previous record intact when an invalid
rebuild candidate is rejected.

This review does not claim shipped dashboard, report, player-card, line, goalie,
practice, postgame, or agent surfaces.

### Evidence inspected

- `icelines-core/src/analytics_cache.rs`
- `icelines-fetch/src/analytics_cache_store.rs`
- `icelines-fetch/src/lib.rs`
- `context/waves/2026-06-01-vtrace-wp009-analytics-cache/pulses/pulse-03.md`
- `cargo test -p icelines-core analytics_cache --quiet`
- `cargo test -p icelines-fetch analytics_cache_store --quiet`
- `cargo fmt --check`
- `cargo clippy -p icelines-core --lib --tests -- -D warnings`
- `cargo clippy -p icelines-fetch --lib --tests -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
- `git diff --check`

### Accepted risks

- The store is available as a typed Rust contract but is not wired to a CLI/Web
  command or product surface yet.
- Downstream hockey surfaces must continue to treat cache-backed analytics as
  pending until their own consumer fixtures and product-copy reviews pass.

### Closeout

- WP-009 status: `partial`.
- VAL-011 status: `partial`.
- Next action: attach the store to a selected dashboard/report/card-style
  consumer fixture without allowing renderer-local source-state or methodology
  recomputation.

## 2026-06-01 WP-009 Pulse 02 Initial Core Cache Contract Review

**Scope:** Initial `icelines-core::analytics_cache` schema/source-state/consumer
contract slice.

**Decision:** `partial_pass`

Pulse 02 adds a versioned analytics cache record and consumer-envelope contract
in `icelines-core`, reusing existing `ViewWindow`, `SourceState`, `MetricCell`,
`ViewWarning`, and disclosure/non-claim vocabulary. The focused tests prove
selected serde compatibility, local snapshot source-state preservation, top-level
and metric-level live-fetch-source refusal, newer-schema refusal, unsupported metric refusal,
coach-dashboard consumer-envelope preservation, consumer-contract mismatch
refusal, and unsupported-consumer refusal.

This review does not claim production cache storage, cache rebuild scheduling,
downstream dashboards/reports/cards, broad stale/partial/missing source-state
fixtures, or complete invalidation behavior.

### Evidence inspected

- `icelines-core/src/analytics_cache.rs`
- `icelines-core/src/lib.rs`
- `cargo test -p icelines-core analytics_cache --quiet`
- `cargo fmt --check`
- `cargo clippy -p icelines-core --lib --tests -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
- `git diff --check`

### Accepted risks

- The initial proof is in-core and fixture-oriented; a future storage/read slice
  must prove tempdir-backed missing, stale, partial, invalidated, and absent cache
  behavior without live API calls.
- Downstream hockey surfaces must continue to treat cache-backed analytics as
  pending until their own consumer fixtures and product-copy reviews pass.

### Closeout

- WP-009 status: `partial`.
- VAL-011 status: `partial`.
- Next action: implement storage/read and broader source-state/invalidation
  fixtures before attaching dashboard, report, card, line, goalie, practice, or
  postgame surfaces.

## 2026-06-01 WP-009 Major Analytics Cache Specification Baseline Review

**Scope:** Coach/analyst product-direction DCR for a shared major analytics cache
foundation before downstream hockey screens, reports, cards, line views, goalie
views, practice/postgame reports, or agent surfaces claim cache-backed analytics.

**Decision:** `target_spec_pending`

CHG-072 accepts the major analytics cache as the next ICELINES product
foundation. The accepted baseline defines required cache record fields,
provenance/freshness/source-window/quality/invalidation semantics, no-live read
behavior, consumer-envelope boundaries, and public non-claims. This review does
not close implementation: no production cache, schema fixture, builder/read path,
invalidation/rebuild fixture, or consumer demo is claimed yet.

### Evidence inspected

- `C:\src\proof\target\debug\proof.exe check C:\src\TRACKER\repos\applied-systems\icelines\docs\vtrace --errors-only`
- `git -C C:\src\TRACKER\repos\applied-systems\icelines diff --check`
- `docs/vtrace/CONOPS.md`
- `docs/vtrace/MISSION.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/CHANGE_CONTROL.md`
- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/TRACE.md`
- `docs/vtrace/WORK_PACKAGES.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/INTEGRATION_PLAN.md`
- `docs/vtrace/STAGE_EXECUTION.md`

### Required implementation follow-up

- Select cache storage path, schema compatibility policy, first metric families,
  and first consumer fixture before Rust implementation begins.
- Add schema/source-state/invalidation fixtures for complete, stale, partial,
  missing, unsupported, and schema-incompatible inputs.
- Prove read paths do not fetch live data or zero-fill missing source state.
- Prove a first consumer envelope preserves cache evidence instead of recomputing
  source-state, confidence, methodology, warnings, or disclosure locally.

### Accepted risks

- Future screens or reports could overstate cache-backed decision support if
  product copy outruns evidence.
- Stale, partial, unsupported, or mismatched source state could be amplified if
  invalidation and quality/completeness semantics are not enforced by fixtures.

### Closeout

- WP-009 status: `target_spec_pending`.
- VAL-011 status: `target_spec_pending`.
- Next action: open implementation pulses for schema/source-state/invalidation/
  consumer evidence.

## 2026-06-01 WP-008 Integration Rehearsal Closeout Review

**Scope:** Final validation rehearsal, broad workspace gates, Lindsay L3 golden
refresh, MDI clippy fix, Web fantasy read-only stabilization, and VTRACE trace
alignment.

**Decision:** `closed_with_risk`

Pulse 01 confirms that the final default workspace gates pass after stale
Lindsay L3 expected outputs were refreshed, the MDI layout test initializer was
changed to direct struct initialization, and the no-live/date plus plain
no-live/dry-run persona regressions were fixed while preserving the
`--for-favorites` no-live cache-write refusal. The Web fantasy route gate also
confirms immutable read-only FantasyDb GETs can observe freshly written rows
without creating SQLite sidecars. VAL-001 through VAL-010 now have evidence or
explicit disposition, with selected-evidence breadth limits retained as residual
risk.

### Evidence inspected

- `context/waves/2026-06-01-vtrace-wp008-integration/WAVE.md`
- `context/waves/2026-06-01-vtrace-wp008-integration/pulses/pulse-01.md`
- `cargo fmt --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `cargo test -p icelines-cli --test persona_foster`
- `cargo test -p icelines-cli --test persona_wave6`
- `cargo test -p icelines-web --test l1_router`
- `LINDSAY_L3_REGEN=1 cargo test -p icelines-cli --test lindsay_l3_golden -- --nocapture`
- `icelines-cli/src/commands/fetch.rs`
- `icelines-cli/src/commands/tonight.rs`
- `icelines-cli/src/tui/mdi.rs`
- `icelines-fetch/src/fantasy_db.rs`
- `icelines-cli/tests/fixtures/lindsay_l3_pre/**`
- `docs/vtrace/TRACE.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/WORK_PACKAGES.md`
- `docs/vtrace/STAGE_EXECUTION.md`
- `docs/vtrace/INTEGRATION_PLAN.md`
- `docs/vtrace/CODE_RIGOR.md`

### Accepted risks

- The closeout is not an exhaustive live-browser, touch/focus, full interactive
  TUI, full report/export matrix, broader transcript, or broader local-state
  proof. Those remain accepted breadth risks unless reopened by a targeted wave.
- WP-007 remains `target-not-met_dispositioned`; no standalone/lean dependency
  support claim may be promoted until dependency surgery and Cargo feature-gating
  pass.

### Closeout

- WP-008 status: `closed_with_risk`.
- S6 readiness/transition: `pass_with_risk`.
- Next action: mirror validated files to TRACKER and keep portfolio
  snapshot/submodule-pointer work separate.

## 2026-05-31 WP-007 Pulse 01 Review

**Scope:** Dependency graph, FLETCH/SLICE command-surface inventory, and lean
CLI target disposition.

**Decision:** `target-not-met_dispositioned`

Pulse 01 confirms that standalone/lean dependency claims must not be promoted.
`fletch-core` remains a path dependency through `icelines-fetch`, `slice-core`
remains a git dependency through `icelines-query`, a second `slice-core` rev is
present transitively through `fletch-core`, and the documented lean command fails
before compilation because no selected package exposes feature `cli`.

### Evidence inspected

- `Cargo.toml`
- `icelines-fetch/Cargo.toml`
- `icelines-query/Cargo.toml`
- `icelines-fetch/src/fletch.rs`
- `icelines-query/src/slice_selectors.rs`
- `icelines-cli/src/cli.rs`
- `icelines-cli/src/commands/fetch.rs`
- `cargo tree -i fletch-core`
- `cargo tree -i 'git+https://github.com/giodl73-repo/SLICE?rev=353564781f6cad53fc5a934178a7927824824e3e#slice-core@0.1.0'`
- `cargo tree -i 'git+https://github.com/giodl73-repo/SLICE?rev=50b63a2eefc66916e9a015a915c845c28d80773c#slice-core@0.1.0'`
- `cargo build --no-default-features --features cli`

### Accepted disposition

- Owner: maintainer/release lens.
- Revisit trigger: dependency removal and Cargo feature-gating PR is ready.
- WP-008 may rehearse validation with `VAL-009`, `REQ-DEP-001`, and
  `REQ-LEAN-001` explicitly target-not-met; no release note, docs, or readiness
  row may claim standalone/lean support until the feature build passes.

## 2026-05-31 Project Baseline Catch-Up Review

**Scope:** Project-facing documentation alignment to the VTRACE specification
baseline.

**Decision:** `accepted_docs_alignment`

The public and developer-facing docs now state that `docs/vtrace/` is the
controlling specification baseline for mission, requirements, design,
interfaces, work packages, verification, validation, review, and change control.
Stale quick-start, static-site, repo-path, crate-ownership, and historical TUI
draft language now defers to the VTRACE baseline where conflicts exist. This is
docs-only evidence and does not close any implementation package row.

### Evidence inspected

- `README.md`
- `COMMANDS.md`
- `CODEBASE.md`
- `CLAUDE.md`
- `SPEC.md`
- `design/specs/platform-contracts.md`
- `design/specs/viewmodels.md`
- `docs/vtrace/*`
- `git -C C:\src\ICELINES diff --check -- README.md COMMANDS.md CODEBASE.md CLAUDE.md SPEC.md design/specs/platform-contracts.md design/specs/viewmodels.md docs/vtrace`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This aligns documentation source-of-truth language only; implementation
  closure remains tied to the work-package evidence rows.
- Historical docs such as `SPEC.md` remain available as supporting references,
  but they are not the controlling baseline when they conflict with VTRACE.

## 2026-05-31 WP-005 Pulse 06 Close Review

**Scope:** Selected snapshot integrity and missing-file fixture boundary.

**Decision:** `passed_with_risk`

Pulse 06 adds selected L0 source-state evidence for sealed snapshot integrity
checks. Sealed snapshot reads reject changed tracked bytes with
`SnapshotError::IntegrityViolation`, and snapshot verification reports deleted
tracked files as `MISSING` instead of accepting incomplete source state.

### Evidence inspected

- `icelines-fetch/src/snapshot.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-06.md`
- `cargo test -p icelines-fetch snapshot::tests::l0_snapshot --lib -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`

### Accepted risks

- This closes only selected snapshot integrity and missing-file boundaries.
  Schema drift, newer schema, broader missing-source, abbreviation drift, and
  partial-fetch resume evidence remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 07 Close Review

**Scope:** Selected chunked snapshot manifest schema drift and newer-schema
refusal boundary.

**Decision:** `passed_with_risk`

Pulse 07 records existing L0 evidence for chunked snapshot manifest schema
compatibility. v1 manifests promote into the v2 in-memory `reports` shape, v2
manifests round-trip through the nested shape, new report keys remain isolated
from legacy accessors, deterministic iteration is preserved, and v3/newer
manifests fail with a `RepoVersionUnknown`-shaped error.

### Evidence inspected

- `icelines-fetch/src/snapshot.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-07.md`
- `cargo test -p icelines-fetch snapshot::tests::l0_lindsay_chunked_manifest --lib -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`

### Accepted risks

- This closes only selected chunked snapshot manifest schema compatibility and
  newer-schema refusal boundaries. Upstream payload schema drift, broader
  missing-source, abbreviation drift, and partial-fetch resume evidence remain
  pending WP-005 work.

## 2026-05-31 WP-005 Pulse 08 Close Review

**Scope:** Selected MoneyPuck CSV required-column and malformed-row drift
boundary.

**Decision:** `passed_with_risk`

Pulse 08 adds checked MoneyPuck CSV parsing for the user-facing fetch path.
Missing required columns and malformed numeric rows now fail explicitly instead
of being silently dropped or snapshotted as trusted source state.

### Evidence inspected

- `icelines-fetch/src/moneypuck.rs`
- `icelines-cli/src/commands/fetch.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-08.md`
- `cargo test -p icelines-fetch moneypuck::tests::l0_parse_csv_checked --lib -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`
- `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings`

### Accepted risks

- This closes only selected MoneyPuck CSV required-column and malformed-row drift
  boundaries. Broader missing-source, abbreviation drift, and partial-fetch
  resume evidence remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 09 Close Review

**Scope:** Selected FLETCH generic HTTP cache/refresh fallback boundary.

**Decision:** `passed_with_risk`

Pulse 09 adds selected L0 httpmock/tempdir evidence for generic FLETCH HTTP
fetch cache behavior. After a successful source fetch populates verified cache,
a non-forced unavailable-source fetch returns the cached bytes, while a forced
refresh continues to fail loudly instead of hiding source unavailability.

### Evidence inspected

- `icelines-fetch/src/fletch.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-09.md`
- `cargo test -p icelines-fetch fletch::tests::fetch_generic_http_bytes_uses_cached_object_when_source_unavailable --lib -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`

### Accepted risks

- This closes only selected generic FLETCH HTTP cache fallback and
  forced-refresh refusal boundaries. Broader missing-source, abbreviation drift,
  and partial-fetch resume evidence remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 10 Close Review

**Scope:** Selected player landing upstream payload schema-drift boundary.

**Decision:** `passed_with_risk`

Pulse 10 records existing L1 httpmock evidence for player landing schema drift.
A malformed `200` JSON response without the required `seasonTotals` structure
surfaces a schema-related error instead of becoming trusted career-history source
data.

### Evidence inspected

- `icelines-fetch/tests/career_landing_mock.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-10.md`
- `cargo test -p icelines-fetch --test career_landing_mock l1_fetch_player_career_history_surfaces_schema_error -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --test career_landing_mock --no-deps -- -D warnings`

### Accepted risks

- This closes only selected player landing schema-drift evidence. Broader
  missing-source, abbreviation drift, and partial-fetch resume evidence remain
  pending WP-005 work.

## 2026-05-31 WP-005 Pulse 11 Close Review

**Scope:** Selected ESPN/NHL abbreviation-drift boundary.

**Decision:** `passed_with_risk`

Pulse 11 records existing L0/L1 evidence for team abbreviation drift. ESPN
shorthand, relocation, and unknown abbreviations normalize or surface explicitly,
and bundled transaction rows stay canonical across covered seasons with named
historical exceptions.

### Evidence inspected

- `icelines-fetch/src/teams.rs`
- `icelines-fetch/src/transactions/convert.rs`
- `icelines-fetch/tests/transactions_storage.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-11.md`
- `cargo test -p icelines-fetch teams::tests::l0_espn_to_nhl --lib -- --nocapture`
- `cargo test -p icelines-fetch transactions::convert::tests::l0_convert --lib -- --nocapture`
- `cargo test -p icelines-fetch --test transactions_storage l1_bundled_team_abbrevs_all_canonical -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --lib --test transactions_storage --no-deps -- -D warnings`

### Accepted risks

- This closes only selected abbreviation-drift evidence. Broader missing-source
  and partial-fetch resume evidence remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 12 Close Review

**Scope:** Selected player landing missing-source boundary.

**Decision:** `passed_with_risk`

Pulse 12 records existing L1 evidence for player landing missing-source
behavior. A missing upstream player landing is skipped and reported while
adjacent valid career histories are still collected.

### Evidence inspected

- `icelines-fetch/tests/career_landing_mock.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-12.md`
- `cargo test -p icelines-fetch --test career_landing_mock l1_fetch_all_career_histories_collects_and_skips -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --test career_landing_mock --no-deps -- -D warnings`

### Accepted risks

- This closes only selected player landing missing-source evidence. Partial-fetch
  resume evidence remains pending WP-005 work.

## 2026-05-31 WP-005 Pulse 13 Close Review

**Scope:** Selected career-history partial refresh resume/flag boundary and
WP-005 closeout.

**Decision:** `passed_with_risk`; `WP-005` is `closed_with_risk`

Pulse 13 adds L0 evidence that a partial career-history refresh preserves
existing cached player histories while merging successful new histories and
stamping the refreshed blob. Paired with the pulse 12 batch skip evidence, this
closes the selected resume/flag boundary for WP-005.

### Evidence inspected

- `icelines-fetch/src/career_landing.rs`
- `icelines-fetch/tests/career_landing_mock.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-13.md`
- `cargo test -p icelines-fetch career_landing::tests::l0_store_partial_refresh_preserves_existing_histories --lib -- --nocapture`
- `cargo test -p icelines-fetch --test career_landing_mock l1_fetch_all_career_histories_collects_and_skips -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-fetch --lib --test career_landing_mock --no-deps -- -D warnings`

### Accepted risks

- Broader `data install`, `fetch boxscore --for-favorites`, `data-status`, and
  `snapshot verify` transcript breadth beyond the selected pulse 05 cases
  remains accepted WP-008 integration rehearsal risk.

## 2026-05-31 WP-005 Pulse 05 Close Review


**Scope:** Selected data/fetch/snapshot command transcript boundary.

**Decision:** `passed_with_risk`

Pulse 05 adds selected L2 subprocess evidence for lockout data install,
data-status, fetch sync, snapshot verify, and no-live fetch command surfaces. The
lockout install no-op remains a true no-home-write no-op, and live-only fetch
commands refuse under `--no-live` before constructing live client state.

### Evidence inspected

- `icelines-cli/src/commands/data.rs`
- `icelines-cli/src/commands/fetch.rs`
- `icelines-cli/tests/system_tests.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-05.md`
- `cargo test -p icelines-cli --test system_tests l2_wp005 -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test system_tests --bin icelines --no-deps -- -D warnings`

### Accepted risks

- This closes only selected command transcript and no-live fetch refusal
  boundaries. Schema drift, integrity mismatch, newer schema, missing-source,
  abbreviation drift, broader transcript breadth, and partial-fetch resume
  evidence remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 03 Close Review

**Scope:** Selected shift capability lock/refusal boundary.

**Decision:** `passed_with_risk`

Pulse 03 records the existing shift-level refusal evidence. Unsupported
per-shift parsing remains locked off in the capability matrix, and cross-process
CLI config attempts to enable it fail with explicit refusal copy.

### Evidence inspected

- `icelines-cli/src/config.rs`
- `icelines-cli/tests/foster_capability_matrix.rs`
- `icelines-cli/tests/system_tests.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-03.md`
- `cargo test -p icelines-cli --test foster_capability_matrix shifts -- --nocapture`
- `cargo test -p icelines-cli --test system_tests l2_foster08_config_set_shifts_favorites_rejected -- --nocapture`

### Accepted risks

- This closes only the selected shift capability refusal boundary. Data
  install/fetch/status transcript, partial-fetch resume, and upstream failure
  fixture breadth remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 04 Close Review

**Scope:** Selected upstream retry/failure fixture boundary.

**Decision:** `passed_with_risk`

Pulse 04 records the existing httpmock evidence for rate-limited and transient
upstream fetch failures. The NHL API client surfaces typed failures for 429 and
503, preserves generic 500 status, avoids retrying non-retryable 4xx responses,
and bounds retry/backoff behavior.

### Evidence inspected

- `icelines-fetch/src/nhl_api.rs`
- `icelines-fetch/tests/fetch_retry_l15.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-04.md`
- `cargo test -p icelines-fetch --test fetch_retry_l15 -- --nocapture`

### Accepted risks

- This closes only the selected retry/failure slice. Schema drift, integrity
  mismatch, newer schema, missing-source, CSV/column drift, abbreviation drift,
  command transcripts, and resume evidence remain pending WP-005 work.

## 2026-05-31 WP-005 Pulse 02 Close Review

**Scope:** Selected offline/query no-live smoke boundary.

**Decision:** `passed_with_risk`

Pulse 02 closes the selected offline smoke slice: `--no-live schedule` now
returns an explicit disabled-live source-state message without creating data or
live API cache state, and `--no-live query leaders` still returns a bundled-data
JSON envelope without creating local cache state.

### Evidence inspected

- `icelines-cli/src/commands/tonight.rs`
- `icelines-cli/tests/system_tests.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-02.md`
- `cargo test -p icelines-cli --test system_tests l2_cmd_no_live -- --nocapture`
- `cargo test -p icelines-cli commands::tonight --bin icelines -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test system_tests --bin icelines --no-deps -- -D warnings`

### Accepted risks

- This is selected CLI offline smoke only; fetch failure mocks, data command
  transcripts, partial-fetch resume, and locked shift-level refusal remain
  pending WP-005 work.
- JSON/CSV schedule output under `--no-live` returns an empty machine-readable
  shape rather than full source-state metadata; broader CLI/Web source-state
  propagation remains WP-008 residual work unless closed by later WP-005 pulses.

## 2026-05-31 WP-005 Pulse 01 Close Review

**Scope:** Selected snapshot seal/refusal boundary for offline/fetch source-state
safety.

**Decision:** `passed_with_risk`

Pulse 01 opens WP-005 with focused evidence that a named snapshot read refuses an
unsealed snapshot before trusting existing file bytes. This protects the
partial-write/unfinished snapshot boundary by returning `NotSealed` instead of
deserializing draft data as trusted source state.

### Evidence inspected

- `icelines-fetch/src/snapshot.rs`
- `context/waves/2026-05-31-vtrace-wp005-offline-fetch/pulses/pulse-01.md`
- `cargo test -p icelines-fetch l0_snapshot_read_named_refuses_unsealed_snapshot --quiet`

### Accepted risks

- This is selected snapshot-store evidence only; offline launch/query smoke,
  fetch failure mocks, data install/fetch/status transcript, partial-fetch
  resume, and locked shift-level refusal remain pending WP-005 work.
- Broad workspace clippy remains a known unrelated blocker; later WP-005 pulses
  must record affected-slice rationale until that debt is retired.

## 2026-05-31 WP-004 Pulse 01 Close Review

**Scope:** Selected Markdown export public-copy disclosure guardrail.

**Decision:** `passed_with_risk`

Pulse 01 adds a near-top `## Disclosure` section to Markdown exports. The
selected leaders export evidence proves the disclosure appears immediately after
front matter, before report context and table content, names the data/source
scope, and explicitly refuses unsupported era-adjusted, predictive, betting,
injury, special-teams, deployment, or linemate-analysis meaning.

### Evidence inspected

- `icelines-cli/src/commands/export.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-01.md`
- `cargo test -p icelines-cli l0_export_leaders -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --no-deps -- -D warnings`

### Accepted risks

- This covers selected Markdown export disclosure only; lockout, October
  rollover, ambiguous/Unicode/duplicate names, trade continuity, GP thresholds,
  active streaks, and skeleton/completeness fixtures remain open.
- Full report/export matrix and `VAL-002`/`VAL-004` closure remain pending for
  later WP-004 pulses.

## 2026-05-31 WP-004 Pulse 02 Close Review

**Scope:** Selected active-streak status label.

**Decision:** `passed_with_risk`

Pulse 02 adds `current_status` to player and team-player streak ViewModel rows.
Selected evidence proves current nonzero streaks are labeled `ongoing`, loaded
broken streaks are labeled `inactive`, and CLI/TUI streak outputs render that
shared status label as an additive column instead of requiring readers to infer
active state from the numeric `current` value.

### Evidence inspected

- `icelines-core/src/view_model/streaks.rs`
- `icelines-cli/src/commands/streaks.rs`
- `icelines-cli/src/tui/screens/player_streaks.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-02.md`
- `cargo test -p icelines-core view_model::streaks::tests:: --quiet`
- `cargo test -p icelines-cli streaks --quiet`
- `cargo fmt --check`
- `cargo clippy -p icelines-core --lib --tests -- -D warnings`
- `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings`

### Accepted risks

- This covers selected player/team-player streak status labeling only; lockout,
  October rollover, ambiguous/Unicode/duplicate names, trade continuity, GP
  thresholds, and skeleton/completeness fixtures remain open.
- Full report/export matrix, full streak parity matrix, and `VAL-002`/`VAL-004`
  closure remain pending for later WP-004 pulses.

## 2026-05-31 WP-004 Pulse 03 Close Review

**Scope:** Selected completeness/skeleton Markdown export disclosure.

**Decision:** `passed_with_risk`

Pulse 03 expands the near-top Markdown export disclosure so incomplete, partial,
stale, missing, and skeleton source states are treated as evidence limits rather
than zero-value truth. Selected export evidence proves the disclosure remains
before report tables and directs readers to source, warning, and empty-state
sections before using rendered rows.

### Evidence inspected

- `icelines-cli/src/commands/export.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-03.md`
- `cargo test -p icelines-cli l0_export_leaders_discloses_methodology_limits_near_top --quiet`
- `cargo fmt --check`

### Accepted risks

- This covers selected Markdown export completeness/skeleton public-copy wording
  only; lockout, October rollover, ambiguous/Unicode/duplicate names, trade
  continuity, and GP thresholds remain open.
- Full report/export matrix and `VAL-002`/`VAL-004` closure remain pending for
  later WP-004 pulses.

## 2026-05-31 WP-004 Pulse 04 Close Review

**Scope:** Selected leaders export GP-threshold evidence.

**Decision:** `passed_with_risk`

Pulse 04 adds explicit fixture evidence for the first-class `--gp-min` leaders
export boundary. The selected fixture proves a below-threshold row is excluded
before rendering even when it would otherwise sort higher, and the threshold is
visible in front matter and active-filter result metadata before the table.

### Evidence inspected

- `icelines-cli/src/commands/export.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-04.md`
- `cargo test -p icelines-cli l0_export_leaders_gp_min_filters_rows_and_reports_threshold --quiet`

### Accepted risks

- This covers selected Markdown leaders `--gp-min` threshold behavior only;
  lockout, October rollover, ambiguous/Unicode/duplicate names, and trade
  continuity remain open.
- Full report/export matrix and `VAL-002`/`VAL-004` closure remain pending for
  later WP-004 pulses.

## 2026-05-31 WP-004 Pulse 05 Close Review

**Scope:** Selected duplicate and Unicode name Markdown export evidence.

**Decision:** `passed_with_risk`

Pulse 05 adds explicit fixture evidence for duplicate display names and Unicode
display names in Markdown leaders exports. The selected fixture proves duplicate
display names remain separate rendered rows and accented names survive report
rendering without being normalized away.

### Evidence inspected

- `icelines-cli/src/commands/export.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-05.md`
- `cargo test -p icelines-cli l0_export_leaders_preserves_unicode_and_duplicate_names_as_rows --quiet`

### Accepted risks

- This covers selected Markdown leaders duplicate/Unicode row rendering only;
  lockout, October rollover, and trade continuity remain open.
- Full report/export matrix and `VAL-002`/`VAL-004` closure remain pending for
  later WP-004 pulses.

## 2026-05-31 WP-004 Pulse 06 Close Review

**Scope:** Selected trade-continuity Markdown export evidence.

**Decision:** `passed_with_risk`

Pulse 06 adds explicit fixture evidence for traded-player continuity in Markdown
leaders exports. The selected fixture proves a traded player renders once with
aggregate GP/goals/assists/points totals and uses the last-stint team display
instead of splitting report rows by team stint.

### Evidence inspected

- `icelines-cli/src/commands/export.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-06.md`
- `cargo test -p icelines-cli l0_export_leaders_traded_player_renders_once_with_last_stint_team --quiet`

### Accepted risks

- This covers selected Markdown leaders traded-player row rendering only;
  lockout and October rollover remain open.
- Full report/export matrix and `VAL-002`/`VAL-004` closure remain pending for
  later WP-004 pulses.

## 2026-05-31 WP-004 Pulse 07 Close Review

**Scope:** Selected lockout and October rollover season-window Markdown export
evidence.

**Decision:** `passed_with_risk`

Pulse 07 adds explicit fixture evidence for historical season-window handling in
Markdown leaders exports. The selected fixture proves a 2012-13 shortened
lockout window and a 2025-26 October rollover window are preserved in front
matter, visible context, and rendered rows without falling back to the
default/current season.

### Evidence inspected

- `icelines-cli/src/commands/export.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-07.md`
- `cargo test -p icelines-cli l0_export_leaders_honors_lockout_and_october_rollover_windows --quiet`

### Accepted risks

- This covers selected Markdown leaders explicit season-window rendering only;
  full historical report/export matrix coverage remains open.
- Full-lockout skip evidence is recorded separately in pulse 08; full
  report/export matrix closure remains pending for WP-008 rehearsal.

## 2026-05-31 WP-004 Pulse 08 Close Review

**Scope:** Full-lockout season skip evidence and WP-004 package closeout.

**Decision:** `closed_with_risk`

Pulse 08 adds focused evidence that the fully cancelled 2004-05 season is not
offered as a fetchable historical season while adjacent seasons remain available.
This closes the remaining ambiguity in the `VAL-002` lockout-skip claim and
closes WP-004 with residual report/export matrix risk routed to WP-008.

### Evidence inspected

- `icelines-cli/src/commands/data.rs`
- `context/waves/2026-05-31-vtrace-wp004-reports/pulses/pulse-08.md`
- `cargo test -p icelines-cli l0_available_seasons_skip_full_lockout_and_keep_neighbors --quiet`

### Accepted risks

- This is selected season-inventory evidence for the full-lockout skip boundary;
  it does not prove every report/export surface against every historical edge.
- Broader ambiguous-name disambiguation breadth, broader active-streak parity,
  and full report/export matrix coverage remain WP-008 rehearsal residual risks.

## 2026-05-31 WP-003 Pulse 04 Close Review

**Scope:** Web scoring/outlook/tonight-intel GET cache-read boundary.

**Decision:** `closed_with_risk`

Pulse 04 keeps selected Rocket Richard scoring, outlook, and tonight-intel GET
and JSON routes from opening the writable data store when manifest state is
absent. Missing-cache requests render empty/missing-source state without
creating `~/.icelines/data`; existing cached reads remain available when a
manifest directory exists. This does not close full browser launch, no-JS,
viewport, host/bind, URL-before-open, broader JSON twin, or recovery inspection
for `VAL-003`.

### Evidence inspected

- `icelines-web/src/handlers/scoring.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-04.md`
- `cargo test -p icelines-web --test l1_router rocket -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers selected scoring/outlook/tonight-intel missing-cache route
  boundaries only, not the full Web route inventory.
- Full `VAL-003` launch, host/bind, viewport, focus/touch, JSON-twin, and
  recovery inspection remains open under WP-003.

## 2026-05-31 WP-003 Pulse 05 Close Review

**Scope:** Web Admin data-status GET cache-read boundary.

**Decision:** `closed_with_risk`

Pulse 05 keeps selected Admin data-status GET and JSON routes from opening the
writable data store when manifest state is absent. Missing-cache requests render
empty/missing-source state without creating `~/.icelines/data`; existing cached
manifest reads remain available when a manifest directory exists. This does not
close full browser launch, no-JS, viewport, host/bind, URL-before-open, broader
JSON twin, or recovery inspection for `VAL-003`.

### Evidence inspected

- `icelines-web/src/handlers/admin.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-05.md`
- `cargo test -p icelines-web --test l1_router admin -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers selected Admin data-status missing-cache route boundaries
  only, not the full Web route inventory.
- Full `VAL-003` launch, host/bind, viewport, focus/touch, JSON-twin, and
  recovery inspection remains open under WP-003.

## 2026-05-31 WP-003 Pulse 06 Close Review

**Scope:** Browser shell no-JS/viewport and unknown-route recovery boundary.

**Decision:** `closed_with_risk`

Pulse 06 adds explicit no-JS guidance to the shared HTML shell and records
route-level evidence that `/dashboard` exposes viewport metadata, a skip link,
global navigation, and server-rendered URL-addressable workspace copy. It also
records that unknown routes return a 404 with recovery copy, compare search, and
navigation links. This does not close full browser launch, host/bind,
URL-before-open, touch/focus interaction, or broader JSON-twin inspection for
`VAL-003`.

### Evidence inspected

- `icelines-web/templates/base.html`
- `icelines-web/templates/not_found.html`
- `icelines-web/static/style.css`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-06.md`
- `cargo test -p icelines-web --test l1_router l1_html_shell_exposes_no_js_viewport_and_recovery_navigation -- --nocapture`
- `cargo test -p icelines-web --test l1_router l1_unknown_route_returns_404 -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers route-level rendered HTML contracts, not a live browser
  screenshot/review.
- Full `VAL-003` launch, host/bind, URL-before-open, touch/focus, and broader
  JSON-twin inspection remains open under WP-003.

## 2026-05-31 WP-003 Pulse 07 Close Review

**Scope:** Serve launch safety and WP-003 closeout.

**Decision:** `closed_with_risk`

Pulse 07 records focused CLI serve launch evidence for URL-before-browser-open
output, `--no-open` browser gating, LAN bind warning copy, and bind resolution
behavior without opening a browser during tests. Combined with pulses 01-06,
WP-003 now has selected evidence for GET-read-only/cache-read boundaries,
no-JS/viewport/recovery shell behavior, and serve launch/bind behavior.

### Evidence inspected

- `icelines-cli/src/commands/serve.rs`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-07.md`
- `cargo test -p icelines-cli commands::serve -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --no-deps -- -D warnings`

### Accepted risks

- The serve evidence is unit-level launch-planning evidence and does not include a
  live browser screenshot/review.
- Touch/focus interaction and the full JSON-twin route matrix remain accepted
  residual risks for WP-008 before readiness claims.

## 2026-05-31 WP-003 Work Package Close Review

**Scope:** Web dashboard route and browser safety.

**Decision:** `closed_with_risk`

WP-003 closes after pulses 01-07. The evidence covers selected Web mutation
method boundaries, cache-read-only GET behavior, no-JS/viewport/recovery shell
contracts, unknown-route recovery, URL-before-open output, `--no-open` gating,
LAN bind warning copy, and bind resolution behavior. `VAL-003` is
`passed_with_risk`.

### Evidence inspected

- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-01.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-02.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-03.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-04.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-05.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-06.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-07.md`
- `icelines-web/tests/l1_router.rs`
- `icelines-cli/src/commands/serve.rs`
- `docs/vtrace/VERIFICATION.md`

### Accepted risks

- Live browser screenshot/review is not captured by the selected automated route
  and launch-planning evidence.
- Touch/focus interaction remains template/route-level rather than live-browser
  evidence.
- Full JSON-twin matrix inspection is deferred to WP-008 integration rehearsal.

## 2026-05-31 WP-006 Work Package Close Review

**Scope:** Fantasy read-model and local-state mutation safety.

**Decision:** `closed_with_risk`

WP-006 closes after pulses 01-04. The evidence covers selected fantasy JSON
missing-state reads, existing-FantasyDb read-only opens, poach
imported-availability read-only opens, shared fantasy ViewModels, CLI/TUI
handoffs, CLI L2 fantasy commands, Web dashboard mutation deferrals, Web fantasy
routes, and Web poach routes. `VAL-007` is `passed_with_risk`.

### Evidence inspected

- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-01.md`
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-02.md`
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-03.md`
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-04.md`
- `cargo test -p icelines-core fantasy -- --nocapture`
- `cargo test -p icelines-fetch fantasy_import -- --nocapture`
- `cargo test -p icelines-cli fantasy -- --nocapture`
- `cargo test -p icelines-web dashboard_command -- --nocapture`
- `cargo test -p icelines-web --test l1_router fantasy -- --nocapture`
- `cargo test -p icelines-web --test l1_router poach -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- Active-writer/concurrent-CLI SQLite coherence is not proven by immutable
  read-only Web route evidence and remains accepted for this package.
- Full interactive TUI rendering of every fantasy screen is deferred to broader
  parity/integration evidence; command parsing and handoff evidence is covered.
- Broader local-state preservation remains represented by focused FantasyDb,
  data-cache, import, and Web route preservation evidence rather than an
  exhaustive storage sweep.

## 2026-05-31 WP-006 Pulse 04 Close Review

**Scope:** VAL-007 fantasy decision-loop transcript.

**Decision:** `closed_with_risk`

Pulse 04 records the focused command/API transcript for poach, roster gaps,
simulation, Yahoo import dry-run/apply semantics, roster-shape validation, daily
and matchup read states, and Web mutation deferrals. The transcript closes
`VAL-007` with accepted active-writer SQLite and broader interactive-TUI risks.

### Evidence inspected

- `icelines-core` fantasy ViewModel tests
- `icelines-fetch` fantasy import tests
- `icelines-cli` fantasy command and TUI handoff tests
- `icelines-web` dashboard command tests
- `icelines-web` fantasy and poach route tests
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-04.md`

### Accepted risks

- The transcript is a focused automated evidence set, not a manual browser/TUI
  recording of every fantasy screen.
- Active-writer/concurrent-CLI SQLite semantics remain accepted risk.

## 2026-05-31 WP-006 Pulse 03 Close Review


**Scope:** Poach imported-availability read-only GET boundary.

**Decision:** `closed_with_risk`

Pulse 03 keeps selected Web poach imported-availability GET reads from opening
shared local SQLite state through writable connections. The poach availability
and watch read helpers now reuse the immutable read-only SQLite helper, and
route evidence proves selected `/api/v1/poach?availability=imported-available`
GET reads do not create `icelines.db-wal` or `icelines.db-shm` sidecar state.
This does not close the full `VAL-007` final transcript or
active-writer/concurrent-CLI database semantics.

### Evidence inspected

- `icelines-fetch/src/fantasy_db.rs`
- `icelines-web/src/handlers/poach.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-03.md`
- `cargo test -p icelines-web --test l1_router poach -- --nocapture`
- `cargo test -p icelines-web --test l1_router fantasy -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers selected poach imported-availability and watch read helpers,
  not all fantasy command/API flows.
- POST-backed watch mutations remain intentionally writable.
- Immutable read-only SQLite inspection assumes a closed local database; active
  writer/concurrent-CLI semantics remain pending.
- Full `VAL-007` final read/mutation-deferral transcript and broader local-state
  preservation evidence remain open under WP-006.

## 2026-05-31 WP-006 Pulse 02 Close Review


**Scope:** Fantasy existing-FantasyDb read-only GET boundary.

**Decision:** `closed_with_risk`

Pulse 02 keeps selected fantasy Web GET reads from opening an existing local
FantasyDb through the writable/migrating SQLite path. Existing database reads now
use a read-only SQLite URI mode after confirming `icelines.db` exists, and route
evidence proves selected fantasy gaps GET reads do not create `icelines.db-wal`
or `icelines.db-shm` sidecar state. This does not close the full `VAL-007`
fantasy read/mutation-deferral transcript or active-writer/concurrent-CLI
database semantics.

### Evidence inspected

- `icelines-fetch/src/fantasy_db.rs`
- `icelines-web/src/handlers/fantasy.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-02.md`
- `cargo test -p icelines-web --test l1_router fantasy -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers selected existing-FantasyDb GET reads only, not all fantasy
  command/API flows.
- Immutable read-only SQLite inspection assumes a closed local FantasyDb; active
  writer/concurrent-CLI semantics remain pending.
- Full poach/import/simulation read transcript and broader dashboard
  mutation-deferral inspection remain open under WP-006.

## 2026-05-31 WP-006 Pulse 01 Close Review

**Scope:** Fantasy JSON local-state and cache-read GET boundaries.

**Decision:** `closed_with_risk`

Pulse 01 keeps selected fantasy JSON GET routes from creating missing local
SQLite or data-cache state. Missing-DB fantasy reads now return an explicit
error without creating `~/.icelines`; daily and matchup missing-cache reads use
existing FantasyDb snapshots and source-state warnings without creating
`~/.icelines/data`. Existing local DB and cached manifest reads remain available.
This does not close the full `VAL-007` fantasy read/mutation-deferral transcript.

### Evidence inspected

- `icelines-web/src/handlers/fantasy.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp006-fantasy-state/pulses/pulse-01.md`
- `cargo test -p icelines-web --test l1_router fantasy -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers selected fantasy JSON missing-DB and missing-cache read
  boundaries only, not all fantasy command/API flows.
- Full poach/import/simulation read transcript and broader dashboard
  mutation-deferral inspection remain open under WP-006.

## 2026-05-31 WP-003 Pulse 03 Close Review

**Scope:** Web streaks GET cache-read boundary.

**Decision:** `closed_with_risk`

Pulse 03 keeps selected player/team streaks GET and JSON routes from opening the
writable data store when manifest state is absent. Missing-cache requests render
empty/recovery state without creating `~/.icelines/data`; existing cached reads
remain available when a manifest directory exists. This does not close full
browser launch, no-JS, viewport, host/bind, URL-before-open, broader JSON twin,
or recovery inspection for `VAL-003`.

### Evidence inspected

- `icelines-web/src/handlers/streaks.rs`
- `icelines-web/src/handlers/team.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-03.md`
- `cargo test -p icelines-web --test l1_router streaks -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers selected streaks missing-cache route boundaries only, not the
  full Web route inventory.
- Full `VAL-003` launch, host/bind, viewport, focus/touch, JSON-twin, and
  recovery inspection remains open under WP-003.

## 2026-05-31 WP-003 Pulse 02 Close Review

**Scope:** Web favorites GET cache-read boundary.

**Decision:** `closed_with_risk`

Pulse 02 removes the observed hidden live fetch/cache write from `GET
/favorites` by rendering favorite player stat lines from existing cached
boxscores only. The review accepts route-level evidence that a missing cache
does not create manifest or boxscore directories during page rendering. This
does not close full browser launch, no-JS, viewport, host/bind,
URL-before-open, JSON twin, or recovery inspection for `VAL-003`.

### Evidence inspected

- `icelines-web/src/handlers/favorites.rs`
- `icelines-web/tests/l1_router.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-02.md`
- `cargo test -p icelines-web --test l1_router l1_favorites_get_does_not_create_data_cache_when_missing -- --nocapture`
- `cargo test -p icelines-web --test l1_router favorites -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --no-deps -- -D warnings`

### Accepted risks

- The pulse covers the selected `/favorites` GET cache path only, not the full
  Web route inventory.
- JSON twin and browser/no-JS inspection for favorites remain part of broader
  WP-003.
- Full `VAL-003` launch, host/bind, viewport, focus/touch, JSON-twin, and
  recovery inspection remains open under WP-003.

## 2026-05-31 WP-003 Pulse 01 Close Review

**Scope:** Web season-type route mutation boundary.

**Decision:** `closed_with_risk`

Pulse 01 removes the observed GET mutation from `/season-type/:kind` by moving
the route to a POST-backed handler and rendering the global season-type toggle as
no-JS POST forms. The review accepts route-level evidence that rejected GET
requests return method-not-allowed and preserve the active season type. This does
not close full browser launch, no-JS, viewport, host/bind, URL-before-open, JSON
twin, or recovery inspection for `VAL-003`.

### Evidence inspected

- `icelines-web/src/lib.rs`
- `icelines-web/templates/base.html`
- `icelines-web/static/style.css`
- `icelines-web/tests/l1_router.rs`
- `icelines-web/tests/ted_lindsay_route_inventory.rs`
- `design/specs/surface-parity.md`
- `context/waves/2026-05-31-vtrace-wp003-web-safety/pulses/pulse-01.md`
- `cargo test -p icelines-web --test l1_router season_type -- --nocapture`
- `cargo test -p icelines-web --test ted_lindsay_route_inventory -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --test l1_router --test ted_lindsay_route_inventory --no-deps -- -D warnings`

### Accepted risks

- The pulse covers the selected season-type route only, not the full Web route
  inventory.
- The no-JS proof is route/template-level evidence, not a complete browser
  screenshot review.
- Full `VAL-003` launch, host/bind, viewport, focus/touch, JSON-twin, and
  recovery inspection remains open under WP-003.

## 2026-05-31 WP-001 Package Close Review

**Scope:** Source-state, query, and ViewModel parity foundation.

**Decision:** `closed_with_risk`

WP-001 has enough focused leaders evidence to close the foundation slice with
risk: shared query/result/source semantics are now carried through CLI JSON,
CLI text, CSV, Web JSON, Web HTML, Markdown export, and selected TUI Stats
rendering, including selected active-filter parity against the canonical query
JSON envelope. The close review does not promote broader parent requirements to
fully passed; it packages the remaining risks into successor work:

- WP-003 owns broader browser route/accessibility and Web safety proof.
- WP-004 owns full historical report/export and public-copy methodology proof.
- WP-005 owns broader source provenance, offline/fetch, and data-depth failure
  proof.
- WP-008 owns final cross-surface validation rehearsal and readiness alignment.

### Evidence inspected

- `context/waves/2026-05-30-vtrace-wp001-parity/WAVE.md`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-01.md` through
  `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-34.md`
- `docs/vtrace/TRACE.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/WORK_PACKAGES.md`
- `docs/vtrace/STAGE_EXECUTION.md`
- Focused Pulse 34 validation: `cargo fmt --check`; `cargo test -p icelines-cli
  --test system_tests
  l2_cmd_tui_stats_active_filter_result_state_matches_query_envelope --
  --nocapture`; `cargo test -p icelines-cli cli::tests:: -- --nocapture`;
  `cargo test -p icelines-cli l0_tui -- --nocapture`; `cargo clippy -p
  icelines-cli --test system_tests --no-deps -- -D warnings`;
  `C:\src\proof\target\debug\proof.exe check
  C:\src\ICELINES\docs\vtrace --errors-only`; `git --no-pager diff --check`.

### Accepted risks

- WP-001 remains selected leaders evidence, not exhaustive query-planner coverage
  for every grammar edge or renderer.
- Full interactive TUI transcript evidence remains open and is deferred to WP-008
  rehearsal unless a later TUI-specific package reopens it.
- Full report/export methodology and public-copy proof remain open under WP-004.
- Broader browser/accessibility route proof remains open under WP-003.
- Broader source provenance/fetch/offline failure proof remains open under
  WP-005.
- Full workspace clippy remains blocked by unrelated existing lint debt outside
  WP-001; affected-slice clippy is accepted for this close review.

## 2026-05-31 WP-001 Pulse 34 Close Review

**Scope:** TUI leaders active query-filter L2 snapshot parity evidence.

**Decision:** `closed_with_risk`

Pulse 34 adds a hidden non-interactive TUI snapshot seam and L2 subprocess
evidence that selected TUI Stats leaders output applies `country=CAN`, renders
the same active-filter result metadata and rows as the query JSON envelope, and
excludes the unfiltered non-CAN goals leader `Leon Draisaitl`. The review accepts
focused TUI snapshot-vs-envelope evidence, but does not close full interactive
TUI parity, full query-planner parity, or broader `WP-001`.

### Evidence inspected

- `icelines-cli/src/tui/mod.rs`
- `icelines-cli/src/tui/screens/queries.rs`
- `icelines-cli/src/cli.rs`
- `icelines-cli/src/main.rs`
- `icelines-cli/tests/system_tests.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-34.md`
- `cargo test -p icelines-cli --test system_tests l2_cmd_tui_stats_active_filter_result_state_matches_query_envelope -- --nocapture`
- `cargo test -p icelines-cli cli::tests:: -- --nocapture`
- `cargo test -p icelines-cli l0_tui -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test system_tests --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- The fixture covers selected `country=CAN` active-filter result behavior, not the
  full filter grammar or broader query-planner parity.
- This is a hidden non-interactive TUI snapshot seam, not a full interactive TUI
  transcript.
- Full workspace clippy remains blocked by unrelated existing lint debt outside
  this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 33 Close Review

**Scope:** Markdown leaders export active query-filter L2 evidence.

**Decision:** `closed_with_risk`

Pulse 33 adds additive CLI controls for `export md leaders --filter`, `--season`,
and `--type`, then proves the selected Markdown report path applies the parsed
free-form filter `country=CAN`, renders active-filter metadata in the report and
front matter, includes each filtered row from the query JSON envelope, and
excludes the unfiltered non-CAN goals leader `Leon Draisaitl`. The review accepts
focused report-vs-envelope evidence, but does not close full report/export
parity, full query-planner parity, or broader `WP-001`.

### Evidence inspected

- `icelines-cli/src/cli.rs`
- `icelines-cli/src/commands/export.rs`
- `icelines-cli/src/commands/query.rs`
- `icelines-cli/tests/system_tests.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-33.md`
- `cargo test -p icelines-cli export::tests::l0_export_leaders_free_form_filter_reports_and_filters_rows`
- `cargo test -p icelines-cli --test system_tests l2_cmd_export_md_leaders_active_filter_result_state_matches_query_envelope -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test system_tests --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- The fixture covers selected `country=CAN` active-filter result behavior, not the
  full filter grammar or broader query-planner parity.
- This is Markdown export subprocess evidence, not full report/export parity
  across every report shape.
- Full workspace clippy remains blocked by unrelated existing lint debt outside
  this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 32 Close Review

**Scope:** Default CLI text leaders active query-filter L2 evidence.

**Decision:** `closed_with_risk`

Pulse 32 adds L2 subprocess evidence that selected default CLI leaders text
output applies the parsed free-form filter `country=CAN`, renders
`active_filters country=CAN` in the visible result metadata line, includes each
row from the same command's JSON envelope, and excludes the unfiltered non-CAN
goals leader `Leon Draisaitl`. The review accepts focused CLI text-vs-envelope
evidence, but does not close full query-planner parity, full cross-surface
parity, or broader `WP-001`.

### Evidence inspected

- `icelines-cli/tests/system_tests.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-32.md`
- `cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_text_active_filter_result_state_matches_json_envelope -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test system_tests --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- The fixture covers selected `country=CAN` active-filter result behavior, not the
  full filter grammar or broader query-planner parity.
- This is CLI subprocess evidence, not an interactive TUI/Web/browser transcript.
- Full workspace clippy remains blocked by unrelated existing lint debt outside
  this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 31 Close Review

**Scope:** TUI leaders active query-filter rendered-results evidence.

**Decision:** `closed_with_risk`

Pulse 31 adds L0 evidence that the selected TUI leaders Stats result path applies
the parsed free-form filter `country=CAN`, renders `active_filters country=CAN`
in the visible result metadata line, keeps the matching row visible, and hides
the filtered-out row. The review accepts focused rendered-results evidence, but
does not close full interactive TUI parity, full query-planner parity, or broader
`WP-001`.

### Evidence inspected

- `icelines-cli/src/tui/screens/queries.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-31.md`
- `cargo test -p icelines-cli tui::screens::queries::tests::l0_tui_leaders_results_render_active_filter_result_state --bin icelines -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This is a focused render/unit fixture, not an interactive terminal transcript.
- The fixture covers selected `country=CAN` active-filter result behavior, not the
  full filter grammar or broader query-planner parity.
- Full workspace clippy remains blocked by unrelated existing lint debt outside
  this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 30 Close Review

**Scope:** Web leaders active query-filter UI route-level parity against CLI
JSON.

**Decision:** `closed_with_risk`

Pulse 30 adds L2 evidence that the selected Web leaders route preserves active
query-filter state in visible HTML UI matching the CLI JSON envelope for the
same filtered `/leaders` query. The review accepts focused route-level parity
evidence for the visible token, preserved filter input, and clear link, but does
not close full browser/accessibility route proof, full query-planner parity, or
broader `WP-001`.

### Evidence inspected

- `icelines-cli/tests/goalies_web_cli_parity.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-30.md`
- `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_json_and_web_html_active_filter_ui_match -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This is route-rendered HTML evidence, not a browser screenshot or assistive
  technology transcript.
- The fixture covers selected active filter UI state, not the full filter grammar
  or broader query-planner parity.
- Full Web all-targets clippy and full workspace clippy remain blocked by
  unrelated existing lint debt outside this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 29 Close Review

**Scope:** Web leaders active query-filter route-level parity against CLI JSON.

**Decision:** `closed_with_risk`

Pulse 29 adds L2 evidence that the selected Web leaders route preserves active
query-filter state in HTML result metadata matching the CLI JSON envelope for the
same filtered `/leaders` query. The review accepts focused route-level parity
evidence, but does not close full browser/accessibility route proof, full
query-planner parity, or broader `WP-001`.

### Evidence inspected

- `icelines-cli/tests/goalies_web_cli_parity.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-29.md`
- `cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_json_and_web_html_active_filter_state_match -- --nocapture`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This is route-rendered HTML evidence, not a browser screenshot or assistive
  technology transcript.
- The fixture covers selected active filter state, not the full filter grammar or
  broader query-planner parity.
- Full Web all-targets clippy and full workspace clippy remain blocked by
  unrelated existing lint debt outside this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 28 Close Review

**Scope:** Web leaders active position-chip route-level parity against CLI JSON.

**Decision:** `closed_with_risk`

Pulse 28 adds L2 evidence that the selected Web leaders active position chip
matches the CLI JSON envelope `position_filter` for the same `/leaders` route,
including the chip label, href, and exactly one `aria-current="true"` marker.
The review accepts focused route-level parity evidence, but does not close full
browser/accessibility route proof or broader `WP-001`.

### Evidence inspected

- `icelines-cli/tests/goalies_web_cli_parity.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-28.md`
- `cargo test -p icelines-cli l2_query_leaders_cli_json_and_web_html_active_position_chip_match --test goalies_web_cli_parity`
- `cargo fmt --check`
- `cargo clippy -p icelines-cli --test goalies_web_cli_parity --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This is route-rendered HTML evidence, not a browser screenshot or assistive
  technology transcript.
- Full Web all-targets clippy and full workspace clippy remain blocked by
  unrelated existing lint debt outside this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 27 Close Review

**Scope:** Web leaders active position-chip accessibility/current-route state.

**Decision:** `closed_with_risk`

Pulse 27 adds `aria-current="true"` to the selected Web leaders position chip so
the active filter state is not communicated by visual styling alone. The review
accepts focused Web L0 evidence plus affected-slice L1 evidence, but does not
close full browser/accessibility route proof or broader `WP-001`.

### Evidence inspected

- `icelines-web/templates/leaders.html`
- `icelines-web/src/handlers/leaders.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-27.md`
- `cargo test -p icelines-web l0_web_leaders_active_position_chip_exposes_current_route_state --lib`
- `cargo test -p icelines-web l0_web_leaders_position_chips_include_goalie_recovery_filter --lib`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This is rendered-template evidence, not a browser screenshot or assistive
  technology transcript.
- Full Web all-targets clippy and full workspace clippy remain blocked by
  unrelated existing lint debt outside this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 26 Close Review

**Scope:** Web leaders goalie position-chip recovery affordance for the existing
`pos=G` empty/warning/recovery path.

**Decision:** `closed_with_risk`

Pulse 26 adds a visible `G` chip to the Web leaders position-chip strip and
keeps the existing ViewModel-backed goalie-filter empty/warning/recovery state
as the behavior behind that affordance. The review accepts focused Web L0 and
affected-slice L1 evidence, but does not close full Web browser/accessibility
proof or broader `WP-001`.

### Evidence inspected

- `icelines-web/src/handlers/leaders.rs`
- `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-26.md`
- `cargo test -p icelines-web l0_web_leaders_position_chips_include_goalie_recovery_filter --lib`
- `cargo fmt --check`
- `cargo clippy -p icelines-web --lib --no-deps -- -D warnings`
- `C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only`

### Accepted risks

- This is L0 Web handler evidence for a visible affordance, not a browser
  screenshot, accessibility, or no-JS route proof.
- Full Web all-targets clippy and full workspace clippy remain blocked by
  unrelated existing lint debt outside this pulse.
- Full `WP-001` and WP-008 integration rehearsal remain open.

## 2026-05-30 R2 CONOPS Review

**Scope:** IceLines repo-baseline VTRACE adoption, focused on
`docs/vtrace/CONOPS.md` after the mission refresh.

**Gate type:** Gate 1 specification review, second-round role review.

**Decision:** `pass_with_risk`

The CONOPS is specific enough to proceed into `REQUIREMENTS.md`,
`VALIDATION.md`, `VERIFICATION.md`, and `TRACE.md`. It names realistic operating
flows, degraded paths, outputs, and handoffs for the main IceLines personas. The
remaining risks are not blockers for requirements drafting, but they must stay
visible because several scenarios currently name intended validation evidence
rather than completed evidence.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/MISSION.md`
- `docs/vtrace/CONOPS.md`
- `.roles/ROLE.md` and role files for the lenses above
- `COMMANDS.md`
- `Cargo.toml`
- VTRACE `docs/framework/vtrace-process.md`
- VTRACE `docs/framework/review-process.md`

### Findings

| ID | Severity | Lens | Finding | Required follow-up |
|---|---|---|---|---|
| R2-CONOPS-01 | Major | BENCH | `CON-001` through `CON-009` correctly reserve `VAL-###` IDs, but `VALIDATION.md` does not exist yet. These are planned validation records, not completed evidence. | Create `VALIDATION.md` with one row per `VAL-###`: scenario, acceptance method, fixture or demo setup, expected observable result, and evidence pointer. |
| R2-CONOPS-02 | Major | KEEL, BENCH | `CON-004` states cross-surface parity, but the comparison artifact is not yet controlled. TUI, CLI, Web HTML, JSON API, and `export md` need one canonical row/envelope comparison rule. | In `REQUIREMENTS.md` and `VERIFICATION.md`, define the parity fixture, the shared ViewModel/envelope to compare, and which surfaces are automated tests versus demonstration/inspection. |
| R2-CONOPS-03 | Major | HART, KEEL, WIRE | The scenarios use related but not-yet-frozen data-state words: `source_state`, `MissingSource`, `partial`, `stale`, `bundled`, `offline`, and freshness/provenance. Without one vocabulary, renderers can truthfully degrade in different ways and still drift. | Freeze a common data-state vocabulary in `INTERFACES.md` or `REQUIREMENTS.md`, then require every renderer and export artifact to carry it without recomputing source status locally. |
| R2-CONOPS-04 | Major | KEEL, FORGE | `CON-009` correctly marks lean/standalone build and FLETCH/SLICE removal as targets, not facts. `Cargo.toml` still contains cross-repo FLETCH/SLICE dependencies, so the CONOPS claim boundary is honest but fragile. | Split the target into traceable requirements: remove cross-repo deps, feature-gate `web`/`tui`/`net`, document removed FLETCH/SLICE command surfaces, and define rollback. |
| R2-CONOPS-05 | Minor | EDGE, TAPE, PACE | `CON-002` carries the right hazards for public historical claims: lockout skip, October season rollover, ambiguous names, trade continuity, and skeleton-season completeness. The scenario is too broad to verify as one assertion. | Break the stat-in-perspective workflow into requirement rows and fixtures so each hazard has an expected output or disclosure. |
| R2-CONOPS-06 | Minor | broadcast, GLASS, CREST | `CON-003` names web cold-start, bookmarkable state, and a first-impression check. Browser accessibility, mobile/narrow viewport, 404 recovery, color-not-alone, and 0.0.0.0 warning behavior should not be left as taste-only review. | Carry these into `VALIDATION.md` as observable browser checks, including the 5-second/screenshot review and active context above the fold. |
| R2-CONOPS-07 | Minor | SCOUT, PACE | The mission and CONOPS correctly say "perspective" means historical ranking, not era-adjusted or predictive value. That limitation must survive into exported post/report copy. | Add a requirement that public report/export artifacts disclose descriptive scope and avoid implying era normalization, betting prediction, or deployment-quality adjustment. |
| R2-CONOPS-08 | Note | WIRE, BENCH | Operational commands cited in the CONOPS (`serve`, `fetch boxscore`, `fetch sync`, `data install`, `snapshot verify`) are discoverable in `COMMANDS.md`. | No CONOPS change required; later verification should cite the exact command examples and expected exit/output shape. |

### Accepted risks

- The VTRACE adoption is mid-slice. `MISSION.md` and `CONOPS.md` are present;
  requirements, trace, verification, validation, and final review artifacts are
  still to be written.
- Static site/export language is intentionally qualified: Web, CLI, and TUI are
  active surfaces; mkdocs static site is deferred, while `export md` remains a
  first-class artifact renderer.
- Shift-level tracking and lean/offline CLI-only build are targets with known
  constraints, not shipped capability claims.
- Some validation will remain demonstration/inspection evidence rather than
  automated tests. That is acceptable only if `VALIDATION.md` labels it honestly.

### Required next step

Draft `REQUIREMENTS.md` from the nine `CON-###` scenarios, then draft
`VALIDATION.md` so the reserved `VAL-###` identifiers become concrete evidence
records before any Gate 3 readiness claim.

### Validation command

```powershell
git diff --check
```

## 2026-05-30 Implementation Readiness Review

**Scope:** IceLines VTRACE implementation-management adoption after pulling the
VTRACE implementation-control update.

**Gate type:** Implementation Readiness Review.

**Decision:** `pass_with_risk`

The baseline is ready to move from documentation review into controlled
implementation work packages. This decision does not close any implementation
evidence row; it approves the management artifacts that prevent implementation
from becoming untraceable slices.

### Review lenses

BENCH, Jim Gregory, HART, KEEL, FORGE, GLASS, CREST, broadcast, WIRE, TAPE,
PACE, and SCOUT.

### Evidence inspected

- VTRACE `docs/framework/implementation-management.md`
- VTRACE `docs/framework/execution-control.md`
- VTRACE `docs/framework/assurance-security-review.md`
- VTRACE adoption templates for implementation plan, work packages, change
  control, and integration plan
- `docs/vtrace/MISSION.md`
- `docs/vtrace/CONOPS.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/TRACE.md`

### Findings

| ID | Severity | Lens | Finding | Required follow-up |
|---|---|---|---|---|
| IRR-001 | Major | BENCH / Jim Gregory | VTRACE now requires implementation work to be controlled through `IMPLEMENTATION_PLAN.md` and `WORK_PACKAGES.md`; ICELINES did not have those artifacts. | Added both artifacts and mapped every requirement to a work package or target disposition. |
| IRR-002 | Major | GLASS / CREST / broadcast | The leading target slice, named layout persistence, crosses local-state schema, TUI, Web, URL/bookmark, and validation boundaries. | Added `INTEGRATION_PLAN.md`, `CHANGE_CONTROL.md`, `WP-002`, and `INT-002` before coding. |
| IRR-003 | Major | KEEL / FORGE | Dependency and lean CLI work remains too risky to mix with layout or browser work. | Isolated `WP-007` as target-not-met pending command-surface inventory, dependency inspection, and lean build evidence. |
| IRR-004 | Minor | HART / WIRE / TAPE | Source-state, parity, and fetch/offline evidence are foundational for later surface claims. | Sequenced `WP-001` before broad surface validation and `WP-005` before data-depth/freshness claims. |

### Accepted risks

- Work packages are proposed planning controls, not implementation evidence.
- Assurance/security lane decisions remain pending until each package reaches its
  review gate.
- `REQ-WB-003`, `REQ-DEP-001`, and `REQ-LEAN-001` remain unproven target posture
  until their named evidence passes.
- `WP-008` is blocked until predecessor packages produce evidence or explicit
  blocked/deferred/target-not-met dispositions.

### Required next step

Start `WP-002` only after selecting the layout storage/schema/migration rule and
recording any required `CHANGE_CONTROL.md` entry. Keep canonical ICELINES work
separate from TRACKER submodule pointer updates.

### Validation command

```powershell
git -C C:\src\ICELINES diff --check
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
```

## 2026-05-30 R2 Closure Addendum

**Scope:** Close all R2 CONOPS findings by adding traceable VTRACE companion
artifacts. No production code was changed.

**Decision:** `closed_for_gate_1`

**Round 3 decision:** A separate R3 CONOPS review is **not required**. The fixes
did not materially change the CONOPS semantics; they converted R2 follow-ups into
controlled `REQUIREMENTS.md`, `INTERFACES.md`, `VALIDATION.md`,
`VERIFICATION.md`, and `TRACE.md` records. The next review should be a Gate 2
requirements/trace review after evidence begins moving from `pending` to
verified/validated.

### Closure evidence

| Finding | Closure |
|---|---|
| R2-CONOPS-01 | `VALIDATION.md` now defines VAL-001 through VAL-009 with actor, need, workflow, success criteria, evidence pointer, and result state. |
| R2-CONOPS-02 | `REQUIREMENTS.md` REQ-PARITY-001, `INTERFACES.md` IF-VIEW-001, `VALIDATION.md` VAL-004, `VERIFICATION.md`, and `TRACE.md` define the cross-surface comparison rule. |
| R2-CONOPS-03 | `INTERFACES.md` IF-DATA-001 freezes the source/completeness vocabulary, including `MissingSource`, `partial`, `stale`, `bundled`, provenance, and freshness semantics. |
| R2-CONOPS-04 | `REQUIREMENTS.md` REQ-DEP-001 and REQ-LEAN-001 split the standalone/lean targets and mark them as target/not-met until FLETCH/SLICE removal and feature gating are implemented. |
| R2-CONOPS-05 | `REQUIREMENTS.md` REQ-STAT-002 and `VALIDATION.md` VAL-002 split stat-in-perspective hazards into discrete expected observations and evidence IDs. |
| R2-CONOPS-06 | `REQUIREMENTS.md` REQ-WEB-002 and `VALIDATION.md` VAL-003 carry first-impression, accessibility, viewport, 404, empty-state, and `0.0.0.0` warning checks. |
| R2-CONOPS-07 | `REQUIREMENTS.md` REQ-REPORT-001 and `INTERFACES.md` IF-REPORT-001 require exported/public artifacts to disclose descriptive scope and avoid era-adjusted, betting, predictive, or deployment-adjusted implications. |
| R2-CONOPS-08 | `VERIFICATION.md` and `VALIDATION.md` keep the operational commands as evidence targets; no CONOPS text change was required. |

### Remaining risks carried forward

- Many rows are still `pending` evidence, not passing tests or demos.
- `REQ-DEP-001` and `REQ-LEAN-001` remain explicit targets because `Cargo.toml`
  still contains FLETCH/SLICE dependency wiring and feature-gating work remains.
- Gate 2 should review requirement quality, trace completeness, and evidence
  readiness before any Gate 3 readiness claim.

## 2026-05-30 R1 Architecture Review

**Scope:** `docs/vtrace/ARCHITECTURE.md` after R2 CONOPS closure.

**Gate type:** Gate 2 architecture review, first-round role review.

**Decision:** `pass_to_design_with_risk`

The architecture artifact is sufficient to proceed one file at a time into
`DESIGN.md`. It names the system shape, component boundaries, data flow,
dependency boundaries, and failure modes without over-claiming standalone,
lean-build, static-site, or validation status.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/MISSION.md`
- `docs/vtrace/CONOPS.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/ARCHITECTURE.md`
- `design/ARCHITECTURE.md`
- `design/specs/platform-contracts.md`
- `design/specs/surface-parity.md`
- `design/specs/viewmodels.md`
- `.roles/*.md`
- `Cargo.toml`

### Persona findings

| ID | Severity | Lens | Finding | Required follow-up |
|---|---|---|---|---|
| R1-ARCH-01 | Note | HART, KEEL | The architecture preserves the `(player_id, season, season_type)` axis, distinguishes TUI long-lived state from one-shot/read adapters, and avoids a fake universal source fallback chain. | Carry cache key and invalidation decisions into `DESIGN.md`, especially any TUI workbench state split. |
| R1-ARCH-02 | Note | WIRE, TAPE, EDGE | Source failure modes are correctly treated as typed state or loud errors: `MissingSource`, integrity mismatch, schema drift, newer snapshot refusal, season leakage, and abbrev drift are named. | Convert the named failure modes into explicit design rows or fixture references in `DESIGN.md` and later evidence in `VERIFICATION.md`. |
| R1-ARCH-03 | Minor | FORGE, KEEL, BENCH | Standalone and lean CLI build are honestly marked as targets, but the architecture cannot yet prove the Cargo feature map. | `DESIGN.md` must define exact `web`/`tui`/`net`/`cli` feature boundaries and the FLETCH/SLICE replacement or removal path. |
| R1-ARCH-04 | Minor | GLASS, CREST, broadcast | Active context, color-not-alone, no-JS/browser recovery, URL state, and report disclosure are present at the architecture boundary. | `DESIGN.md` should allocate these to concrete templates/widgets/adapters so they are not left as review taste. |
| R1-ARCH-05 | Note | PACE, SCOUT | The architecture keeps historical perspective descriptive and avoids claiming era adjustment, betting prediction, or deployment adjustment. | Keep formula and public-copy assumptions visible in `DESIGN.md`; do not bury them in renderer prose. |

### Accepted risks

- No production code changed; architecture evidence remains mostly review and
  documentation evidence.
- FLETCH/SLICE dependency removal and lean feature gating remain target work.
- TUI workbench decomposition is not solved by this file; it is a design and
  implementation concern.
- Gate 3 readiness is not implied until validation/verification rows collect
  real command, fixture, or demo evidence.

### Required next step

Draft `DESIGN.md` as the next single file. It should turn this architecture into
file/module-level design choices, especially Cargo features, ViewModel adapters,
source-state propagation, browser/template allocation, and TUI state boundaries.

## 2026-05-30 R2 Architecture Review

**Scope:** `docs/vtrace/ARCHITECTURE.md` after the R1 Architecture review.

**Gate type:** Gate 2 architecture review, second-round role review.

**Decision:** `pass_to_design_r2_closed`

R2 found no architecture blocker and no need for a separate R3 Architecture
review. Two architecture-level clarity gaps were fixed in the same round:
current-versus-target posture and per-source fallback obligations. Remaining
items are design allocation and evidence work, not architecture changes.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/REVIEW.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/TRACE.md`
- `design/specs/surface-parity.md`
- `.roles/*.md`

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R2-ARCH-01 | Major | KEEL, HART, WIRE, TAPE | The architecture was directionally correct about source-specific fallback, but the obligations were embedded in prose. That is risky because KEEL's known failure mode is a future doc or implementation reintroducing a fake universal fallback chain. | Closed in `ARCHITECTURE.md` by adding `Source obligations`, with current read path, absent/failed behavior, and design obligation per source/domain. |
| R2-ARCH-02 | Major | KEEL, FORGE, BENCH | Current-state and target-state claims needed a single scan point so standalone, lean build, static-site, and evidence maturity cannot be over-claimed later. | Closed in `ARCHITECTURE.md` by adding `Current vs Target Posture`. |
| R2-ARCH-03 | Minor | broadcast, GLASS, CREST | Web/browser and visual-review duties were present, but design must still allocate them to concrete templates, route behavior, state placement, and empty/error surfaces. | Carry to `DESIGN.md`; not an architecture blocker after R2 because the architecture now names active surfaces and target posture. |
| R2-ARCH-04 | Minor | BENCH, EDGE | Failure modes are well named, but most remain pending evidence. The next file must avoid turning them into vague "test later" notes. | Carry to `DESIGN.md` as explicit fixture/test/demo hooks, then to `VERIFICATION.md` evidence rows. |
| R2-ARCH-05 | Note | PACE, SCOUT | Historical perspective remains correctly scoped as descriptive context, not era adjustment, betting, prediction, deployment adjustment, or scouting truth. | No architecture change required; keep the limitation visible in report/export design. |

### Round 3 decision

A separate R3 Architecture review is **not required** unless `DESIGN.md` changes
architecture semantics. The next file remains `docs/vtrace/DESIGN.md`, followed
by a design role review.

## 2026-05-30 R1 Design Review

**Scope:** `docs/vtrace/DESIGN.md` initial draft after Architecture R2 closure.

**Gate type:** Gate 2 design review, first-round role review.

**Decision:** `pass_to_code_rigor_with_risk`

The design is sufficient to proceed to `CODE_RIGOR.md`. It converts the
architecture into implementation-facing decisions, module allocation, source
fallback obligations, TUI state slices, web route boundaries, report/export
rules, parity evidence hooks, and target Cargo/dependency work. The review found
no reason to return to architecture.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/TRACE.md`
- `design/ARCHITECTURE.md`
- `design/specs/platform-contracts.md`
- `design/specs/surface-parity.md`
- `design/specs/viewmodels.md`
- workspace and member `Cargo.toml` files
- current source tree module layout

### Persona findings

| ID | Severity | Lens | Finding | Required follow-up |
|---|---|---|---|---|
| R1-DES-01 | Note | HART, WIRE, TAPE | Source-state projection is now concrete enough to prevent missing data from becoming zero-shaped output. | `CODE_RIGOR.md` must require tests or assertions that preserve `LoadOutcome.missing` through ViewModels, JSON, reports, and browser states. |
| R1-DES-02 | Minor | KEEL, FORGE, BENCH | The target feature map is useful, but it is intentionally not implemented. Current `Cargo.toml` still pulls web/TUI/network dependencies through the default CLI path and still has FLETCH/SLICE seams. | Keep REQ-DEP-001 and REQ-LEAN-001 as target-not-met until dependency inspection and lean build evidence pass. |
| R1-DES-03 | Minor | GLASS, CREST, broadcast | Browser and visual-state duties are allocated to routes/templates/adapters, but the exact tests are still evidence hooks rather than proof. | `CODE_RIGOR.md` should require no-JS, active-context, color-not-alone, recovery, and GET-read-only route checks before web changes claim closure. |
| R1-DES-04 | Minor | BENCH, EDGE | Historical-perspective hazards are split into design rows, but the fixture names and commands are still TBD. | Carry the six VAL-002 observations into verification evidence rows when fixtures or demos are selected. |
| R1-DES-05 | Note | PACE, SCOUT | Report/export design keeps perspective descriptive and disclosure-forward. | Report snapshot checks should include anti-overclaim copy and source/scope disclosure. |
| R1-DES-06 | Note | HART, FORGE | TUI state is logically split without forcing a risky immediate code refactor. | `CODE_RIGOR.md` should prevent new semantic logic from being added to renderer-only paths while implementation catches up. |

### Accepted risks

- This remains docs/design work; validation evidence is not yet Gate 3 evidence.
- Target feature names are design targets, not implemented Cargo features.
- FLETCH/SLICE dependency independence remains unresolved.
- TUI decomposition is a logical state contract until an implementation wave
  changes files.

### Required next step

Draft `CODE_RIGOR.md` as the next single file. It should formalize constraints
from the design review: ViewModel purity, source-state preservation, shared query
intent, TUI state/cache discipline, web GET/mutation safety, report disclosure,
fetch integrity handling, feature/dependency claims, and evidence-led gates.

## 2026-05-30 R2 Design Review

**Scope:** `docs/vtrace/DESIGN.md` after the R1 Design review.

**Gate type:** Gate 2 design review, second-round role review.

**Decision:** `pass_to_code_rigor_r2_closed`

R2 found no need to return to Architecture and no blocker for the next
one-file step, `CODE_RIGOR.md`. Five design clarity gaps were closed in the
same round so CODE_RIGOR can enforce concrete constraints rather than vague
review notes.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/DESIGN.md`
- `docs/vtrace/REVIEW.md`
- `.roles/hart.md`
- `.roles/keel.md`
- `.roles/bench.md`
- `.roles/wire.md`
- `.roles/glass.md`
- `.roles/crest.md`
- R1 Design findings and Architecture R2 closure notes

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R2-DES-01 | Major | HART, KEEL | The draft named `(player_id, season, season_type)` and cache invalidation, but did not enumerate key shapes for identity, season stats, stints, rosters, schedules, source notices, query results, and user state. | Closed in `DESIGN.md` by adding `Domain key-shape design`, including key shapes, season-switch stability, `PlayerView`/ViewModel accessor rules, and cache-key omission rules. |
| R2-DES-02 | Major | KEEL, FORGE | The draft split TUI from one-shot surfaces, but web/server/report cache placement could still be over-read as allowing hidden semantic state outside the URL/source context. | Closed in `DESIGN.md` by adding `Surface lifetime and cache placement`, explicitly limiting long-lived semantic state to TUI unless a later design defines cache keys and invalidation. |
| R2-DES-03 | Major | WIRE, EDGE, BENCH | Fetch/integrity behavior was present but too compressed; CODE_RIGOR needs a state machine for integrity, schema, retry/backoff, partial writes, and `LoadOutcome` construction. | Closed in `DESIGN.md` by adding `Fetch boundary state machine`. |
| R2-DES-04 | Minor | BENCH, FORGE | Evidence hooks were useful but not tiered, which risks turning Gate 3 into one broad "test later" bucket. | Closed in `DESIGN.md` by adding `Evidence tier allocation` for L0/L1/L2, route/browser inspection, parity fixtures, and lean-build evidence. |
| R2-DES-05 | Minor | GLASS, CREST, broadcast | Visual/source-state duties were allocated, but the design did not specify where active context, primary decision path, source/status carriers, and recovery live per surface. | Closed in `DESIGN.md` by adding `Surface context and visual hierarchy`. |
| R2-DES-06 | Note | PACE, SCOUT, TAPE | Historical report/export scope remains descriptive and avoids era-adjusted, betting, predictive, deployment-adjusted, or linemate-adjusted claims. | No design change required beyond preserving R1 report/export rules. |

### Round 3 decision

A separate R3 Design review is **not required** unless `CODE_RIGOR.md` or a
later implementation wave changes design semantics. The next file remains
`docs/vtrace/CODE_RIGOR.md`, followed by a code-rigor role review.

## 2026-05-30 R1 Code Rigor Review

**Scope:** `docs/vtrace/CODE_RIGOR.md` initial draft after Design R2 closure.

**Gate type:** Gate 2 code-rigor review, first-round role review.

**Decision:** `pass_to_verification_with_risk`

The code-rigor file is sufficient to proceed to the next one-file step. It
formalizes project-specific constraints for ViewModel purity, source-state
preservation, shared query intent, key-shape and cache discipline, TUI state,
web mutation safety, browser-visible context, report disclosure, fetch integrity,
upstream failures, local-state preservation, feature/dependency claims, and
evidence-led closure.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/DESIGN.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/VERIFICATION.md`
- R2 Design findings and closure notes

### Persona findings

| ID | Severity | Lens | Finding | Required follow-up |
|---|---|---|---|---|
| R1-CR-01 | Note | HART, WIRE | Source-state and fetch/integrity constraints are specific enough to prevent silent zero-shaped failures when code work starts. | Verification must add or link concrete fixtures for missing source, stale source, integrity mismatch, newer schema, 429, 503, and schema drift. |
| R1-CR-02 | Note | KEEL, FORGE | Cargo and dependency claims remain guarded as target-not-met, which is the correct current posture. | Do not change REQ-DEP-001 or REQ-LEAN-001 status until dependency inspection and lean build evidence pass. |
| R1-CR-03 | Minor | BENCH, EDGE | Evidence rows are well scoped but remain pending because this is a docs baseline, not an implementation PR. | The next verification pass should reconcile `VERIFICATION.md` with the expanded `CR-*` evidence map. |
| R1-CR-04 | Minor | GLASS, CREST, broadcast | Browser context, no-JS, and color-not-alone requirements are enforceable in review, but need route/browser evidence before closure. | Carry to VAL-003 and route/browser checks. |
| R1-CR-05 | Note | PACE, SCOUT, TAPE | Report/export constraints correctly prevent unsupported historical, predictive, betting, or deployment-adjusted claims. | Snapshot/text review should be required before public report claims close. |
| R1-CR-06 | Note | HART, BENCH | TUI state concentration is treated as a risk rather than forced into an immediate broad refactor. | Future TUI changes should add semantic state behind named helpers/state slices and cache-key tests. |

### Accepted risks

- `CODE_RIGOR.md` defines implementation constraints but does not itself prove
  implementation compliance.
- The verification evidence map in `VERIFICATION.md` still needs a follow-up pass
  to align with the expanded `CR-*` rows.
- FLETCH/SLICE removal and lean CLI build remain target-not-met.
- Browser and route claims remain pending until VAL-003 evidence exists.

### Required next step

Refresh `VERIFICATION.md` as the next single file. It should align verification
methods and evidence rows to `CODE_RIGOR.md`, preserve target-not-met status for
dependency/lean claims, and keep validation/demo evidence separate from code
verification evidence.

## 2026-05-30 R2 Code Rigor Review

**Scope:** `docs/vtrace/CODE_RIGOR.md` after the R1 Code Rigor review.

**Gate type:** Gate 2 code-rigor review, second-round role review.

**Decision:** `pass_to_verification_r2_closed`

R2 found no blocker and no need to return to Design. Six review findings were
closed in `CODE_RIGOR.md` so the next one-file step, `VERIFICATION.md`, can map
evidence to enforceable constraints rather than broad review notes.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/DESIGN.md`
- R1 Code Rigor findings
- `.roles` persona guidance

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R2-CR-01 | Major | HART, FORGE | Rust model-safety constraints were implied but not explicit: repository sharing, `spawn_local`/`LocalSet`, `PlayerView<'_>` lifetimes, broad `Arc<Mutex<LoadOutcome>>`, and core I/O-free boundaries. | Closed by adding CR-024 plus tailoring, checklist, evidence, and open-risk rows. |
| R2-CR-02 | Major | TAPE, WIRE, EDGE | Data-edge rigor for season ID leakage, ESPN/team-abbrev drift, Unicode/duplicate names, GP threshold boundaries, rollover, lockout/skeleton seasons, and trade multi-stint preservation was not enforceable enough. | Closed by adding CR-025 plus data-edge tailoring, checklist, evidence, and open-risk rows. |
| R2-CR-03 | Minor | BENCH, FORGE | Validation command wording was too generic for implementation claims. | Closed by tightening CR-005 and adding a verification command matrix with workspace defaults and affected-slice rules. |
| R2-CR-04 | Minor | PACE, BENCH | Formula and methodology rigor needed threshold, optionality, known-value, and measured-vs-labeled complexity rules. | Closed by adding CR-026 plus methodology tailoring, checklist, evidence, and open-risk rows. |
| R2-CR-05 | Minor | broadcast, GLASS, CREST | Browser HTTP and local-operability constraints needed host/CORS/cache/MIME/auto-open/bind/viewport/touch/focus coverage. | Closed by adding CR-027 plus web-operability tailoring, checklist, evidence, and open-risk rows. |
| R2-CR-06 | Note | SCOUT, PACE, TAPE | Depth-chart/report copy should explicitly treat deployment, line chemistry, special-teams, injury, and linemate context as annotations or limits unless modeled. | Closed by adding CR-028 and related evidence/open-risk rows. |

### Accepted risks

- The R2 closures define code-rigor requirements but do not provide implementation
  evidence by themselves.
- The verification file still needs alignment with new evidence rows
  EVID-CR-011 through EVID-CR-014.
- Lean CLI, FLETCH/SLICE removal, web route evidence, data-edge fixtures, and
  formula/report snapshots remain pending or target-not-met as applicable.

### Round 3 decision

A separate R3 Code Rigor review is **not required** unless `VERIFICATION.md` or
a later implementation wave changes these constraints. A later optional R3 was
run before `VERIFICATION.md`; the R3 section below is now the controlling
Code Rigor closure record. The next file remains `docs/vtrace/VERIFICATION.md`,
followed by a verification role review.

## 2026-05-30 R3 Code Rigor Review

**Scope:** `docs/vtrace/CODE_RIGOR.md` after R2 closure.

**Gate type:** Optional Gate 2 code-rigor review, third-round role review.

**Decision:** `pass_to_verification_r3_closed`

R3 found no blocker and no reason to return to Architecture or Design. It did
find four implementation-contract gaps that were sharp enough to close before
moving to `VERIFICATION.md`.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/CODE_RIGOR.md`
- R2 Code Rigor closure
- `.roles` persona guidance

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R3-CR-01 | Major | FORGE, WIRE | Rust boundary hygiene was under-specified for library panics, typed crate errors, external schema drift, and dependency direction. | Closed by adding CR-029 plus tailoring, checklist, evidence, and open-risk rows. |
| R3-CR-02 | Major | HART, BENCH, EDGE | HART invariants and compile-time fences were referenced indirectly but not named as code-rigor gates. | Closed by adding CR-030 plus model-invariant tailoring, checklist, evidence, and open-risk rows. |
| R3-CR-03 | Minor | WIRE, TAPE, EDGE | External-source reliability policy needed explicit 429/503, Retry-After/backoff, partial-write, schema-version, CSV encoding, and column-drift constraints. | Closed by adding CR-031 plus upstream reliability tailoring, checklist, evidence, and open-risk rows. |
| R3-CR-04 | Minor | GLASS, CREST, broadcast, KEEL | Shared visual/accessibility contracts needed stronger implementation language for centralized tokens, active context, semantic structure, and recovery states. | Closed by adding CR-032 plus visual-contract tailoring, checklist, evidence, and open-risk rows. |

### Accepted risks

- R3 closures remain constraints; implementation evidence is still pending.
- `VERIFICATION.md` must now account for evidence rows EVID-CR-015 through
  EVID-CR-018 in addition to the R2 additions.
- Static-site/mkdocs role language remains treated as historical/deferred unless
  a later design reactivates that surface.

### Round 4 decision

No R4 Code Rigor review is needed before `VERIFICATION.md`. Reopen Code Rigor
only if verification mapping exposes a missing evidence class or a later
implementation wave changes model, source, Rust, or surface semantics.

## 2026-05-30 R1 Verification Review

**Scope:** `docs/vtrace/VERIFICATION.md` initial refresh after Code Rigor R3
closure.

**Gate type:** Gate 3 verification review, first-round role review.

**Decision:** `pass_to_trace_with_risk`

The verification file is sufficient to proceed to the next one-file step,
`TRACE.md`. It now maps each requirement to verification evidence, separates
validation scenarios from implementation proof, preserves target-not-met posture
for dependency and lean CLI claims, and carries all Code Rigor R2/R3 evidence
rows through EVID-CR-018.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- Code Rigor R2 and R3 closure notes

### Persona findings

| ID | Severity | Lens | Finding | Required follow-up |
|---|---|---|---|---|
| R1-VER-01 | Note | BENCH, FORGE | The command matrix correctly distinguishes docs-only checks from implementation closure and allows affected-slice commands only with rationale. | `TRACE.md` should link EVID-CODE-001 and EVID-CR-002 without treating docs validation as code evidence. |
| R1-VER-02 | Note | HART, WIRE, EDGE | Data-edge, model-invariant, and upstream-reliability evidence is now explicit enough to prevent broad "works on current data" closure. | `TRACE.md` should preserve links from REQ-STAT-002, REQ-DATA-001, REQ-FRESH-001, and CR-025/CR-030/CR-031 to the new evidence rows. |
| R1-VER-03 | Minor | GLASS, CREST, broadcast | Web/browser verification now includes local-operability and accessibility checks, but implementation evidence remains pending. | Keep VAL-003, EVID-CR-006, EVID-CR-014, and EVID-CR-018 pending until route/browser proof exists. |
| R1-VER-04 | Minor | PACE, SCOUT, TAPE | Public-copy verification correctly blocks unsupported deployment, injury, special-teams, line-chemistry, and linemate claims. | `TRACE.md` should connect REQ-STAT-001, REQ-REPORT-001, CR-017, CR-026, and CR-028 to snapshot/text evidence. |
| R1-VER-05 | Note | KEEL, FORGE | FLETCH/SLICE and lean CLI claims remain target-not-met with explicit evidence gates. | Do not advance REQ-DEP-001 or REQ-LEAN-001 status until dependency and lean-build evidence pass. |

### Accepted risks

- Verification is an evidence plan; most implementation evidence remains pending.
- Demonstration/inspection rows are honest but should become fixtures or tests
  when implementation waves touch those surfaces.
- `TRACE.md` still needs to reconcile the new EVID-CR-011 through EVID-CR-018
  rows with requirements, interfaces, validation scenarios, and review records.

### Required next step

Draft or refresh `TRACE.md` as the next single file. It should provide the
end-to-end map from `CONOPS.md`, `REQUIREMENTS.md`, `INTERFACES.md`,
`VALIDATION.md`, `VERIFICATION.md`, `CODE_RIGOR.md`, and `REVIEW.md` without
claiming implementation evidence that is still pending.

## 2026-05-30 R2 Verification Review

**Scope:** `docs/vtrace/VERIFICATION.md` after R1 Verification review.

**Gate type:** Gate 3 verification review, second-round role review.

**Decision:** `pass_to_trace_r2_closed`

R2 found no blocker and no need to return to Code Rigor. It did identify three
traceability-contract fixes that should be closed before `TRACE.md`, because the
next file will rely on exact status and evidence-pointer semantics.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/INTERFACES.md`
- R1 Verification review notes

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R2-VER-01 | Major | BENCH, FORGE | Verification statuses were used consistently but not defined, leaving room for `pending`, `passed`, `target-not-met`, `blocked`, or `waived` to drift in `TRACE.md`. | Closed by adding a Status Semantics section. |
| R2-VER-02 | Major | HART, WIRE, EDGE | Several Code Rigor evidence pointers referenced validation scenarios (`VAL-*`) where closure should point at evidence rows (`EVID-VAL-*`). | Closed by converting those pointers to evidence IDs and adding a pointer-semantics rule. |
| R2-VER-03 | Minor | KEEL, broadcast | The closing readiness note still said the file was ready for Verification R1 even though R1 had completed. | Closed by replacing it with Trace Readiness language for the next file. |

### Accepted risks

- R2 improves evidence semantics but does not create implementation evidence.
- Dependency and lean CLI rows remain `target-not-met`.
- Most implementation rows remain `pending` until fixtures, commands, snapshots,
  route/browser checks, or inspections are recorded.

### Round 3 decision

No R3 Verification review is needed before `TRACE.md`. Reopen Verification only
if `TRACE.md` exposes an unmapped evidence class or changes status semantics.

## 2026-05-30 R1 Trace Review

**Scope:** `docs/vtrace/TRACE.md` initial trace refresh after Verification R2.

**Gate type:** end-to-end VTRACE traceability review.

**Decision:** `pass_with_r2_optional`

R1 found that the trace is complete enough to serve as the baseline
cross-document map. It preserves the key evidence-risk posture: most
implementation evidence remains `pending`, dependency/lean claims remain
`target-not-met`, and contextual IDs are not treated as proof.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/TRACE.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R1-TRACE-01 | Major | BENCH, FORGE | `TRACE.md` needed canonical status semantics so rows cannot mix prose such as pending plus target-not-met in one status cell. | Closed by using Verification R2 status vocabulary and normalizing `VAL-009` to `pending`; dependency/lean target failure remains on requirement/evidence rows. |
| R1-TRACE-02 | Major | HART, WIRE, EDGE | The trace needed to distinguish contextual IDs from proof IDs across the whole matrix. | Closed by adding trace rules that require `EVID-*` rows or explicit target-not-met rows for closure. |
| R1-TRACE-03 | Major | KEEL, PACE | Dependency and lean-build risk could be diluted if only validation scenario status was read. | Closed by keeping `REQ-DEP-001`, `REQ-LEAN-001`, `EVID-DEP-001`, `EVID-LEAN-001`, and `EVID-CR-009` as `target-not-met`. |
| R1-TRACE-04 | Minor | GLASS, CREST | Browser, accessibility, and visual-context claims needed to stay tied to pending route/browser evidence. | Closed by linking those rows to `EVID-CR-006`, `EVID-CR-014`, and `EVID-CR-018` and keeping them pending. |

### Accepted risks

- The trace baseline is documentation-complete, not implementation-complete.
- VTRACE docs validation does not close command, fixture, browser, snapshot, or
  dependency evidence rows.
- A Trace R2 review is optional; run it if the next step is to harden the
  baseline before portfolio snapshotting or implementation planning.

## 2026-05-30 R2 Trace Review

**Scope:** `docs/vtrace/TRACE.md` after the R1 Trace review.

**Gate type:** end-to-end VTRACE traceability review, second-round role review.

**Decision:** `pass_to_snapshot_r2_closed`

R2 found no implementation-readiness blocker and no need for a separate Trace R3.
Two traceability hardening gaps were fixed in the same round: named architecture
decisions are now directly traceable, and validation scenario references in trace
summary tables now carry the corresponding `EVID-VAL-*` evidence rows where
those rows exist.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/TRACE.md`
- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/CODE_RIGOR.md`
- `.roles/*.md`

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R2-TRACE-01 | Major | HART, KEEL, BENCH | `TRACE.md` linked `ADR-VT-004` through dependency/lean rows, but the other accepted architecture decisions were only indirectly covered by prose and design links. That made the architecture bridge less auditable than requirements, interfaces, and evidence rows. | Closed by adding an Architecture Decision Trace for `ADR-VT-001` through `ADR-VT-005`, preserving the `target-not-met` status for `ADR-VT-004`. |
| R2-TRACE-02 | Major | BENCH, FORGE, WIRE | Interface and Code Rigor trace tables often cited `VAL-*` scenario IDs without their corresponding `EVID-VAL-*` evidence rows. The trace rules already said contextual IDs are not proof, but the summary tables could still be read too quickly as evidence closure. | Closed by renaming the affected columns to Validation / Evidence Coverage and adding corresponding `EVID-VAL-*` links where validation evidence rows exist. |
| R2-TRACE-03 | Minor | GLASS, CREST, broadcast | Browser, report/export, and visual-context rows remained pending as intended; the review confirmed no static-site or mkdocs row was promoted to active implementation evidence. | No patch required beyond the ADR and evidence-link hardening. |
| R2-TRACE-04 | Note | KEEL, FORGE | Dependency and lean CLI risks remain visible and correctly stay `target-not-met`. | No patch required; `REQ-DEP-001`, `REQ-LEAN-001`, `EVID-DEP-001`, `EVID-LEAN-001`, and `EVID-CR-009` remain `target-not-met`. |

### Accepted risks

- This closes the documentation trace baseline only; it does not create command,
  fixture, browser, snapshot, dependency, or lean-build implementation evidence.
- Most implementation rows remain `pending`.
- Dependency and lean CLI rows remain `target-not-met`.
- `EVID-DOC-001` remains available for docs-only validation evidence but does not
  close implementation rows.

### Round 3 decision

No R3 Trace review is required before snapshotting the VTRACE baseline or moving
into implementation planning. A later optional R3 was run to harden review-ID
auditability; the R3 section below is now the controlling Trace closure record.
Reopen Trace only if a later implementation wave adds or removes requirements,
interfaces, validation rows, evidence classes, or architecture/design decisions.

## 2026-05-30 R3 Trace Review

**Scope:** `docs/vtrace/TRACE.md` after the R2 Trace review.

**Gate type:** end-to-end VTRACE traceability review, third-round role review.

**Decision:** `pass_to_snapshot_r3_closed`

R3 found one auditability gap, not an implementation-readiness blocker. Source
requirements, interfaces, validation rows, evidence rows, code-rigor constraints,
architecture decisions, and design decisions were already covered. Prior review
finding IDs, however, were summarized by review round in the Review Closure Trace
instead of being directly enumerated.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, and
broadcast.

### Evidence inspected

- `docs/vtrace/TRACE.md`
- `docs/vtrace/REVIEW.md`
- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/CODE_RIGOR.md`
- `.roles/*.md`

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R3-TRACE-01 | Major | BENCH, HART, KEEL | The Review Closure Trace had source coverage for requirement/interface/evidence IDs, but prior review findings from Architecture, Design, Code Rigor, Verification, and Trace R2 were summarized by round rather than enumerated as `R2-*`/`R3-*` IDs. That weakened auditability for persona-review closure. | Closed by replacing round-level summary rows with direct closure rows for R2 Architecture, R2 Design, R2/R3 Code Rigor, R2 Verification, R2 Trace, and this R3 Trace finding. |
| R3-TRACE-02 | Note | FORGE, WIRE, EDGE | The trace still preserves evidence honesty: implementation rows remain pending, dependency/lean rows remain target-not-met, and contextual IDs are not treated as proof. | No patch required beyond R3-TRACE-01 closure. |
| R3-TRACE-03 | Note | GLASS, CREST, broadcast | Static site and mkdocs remain deferred/historical; active Web/report/export evidence still requires pending route/browser/snapshot proof. | No patch required. |

### Accepted risks

- The VTRACE trace baseline is audit-complete but not implementation-complete.
- New implementation waves must update this trace when evidence moves from
  `pending` or `target-not-met` to `passed`, `blocked`, or `waived`.
- Dependency and lean CLI rows remain `target-not-met` until their explicit
  evidence gates pass.

### Round 4 decision

No R4 Trace review is needed before snapshotting or implementation planning.
Reopen Trace only when controlled IDs or evidence status actually change.

## 2026-05-30 VTRACE Baseline Snapshot Review

**Scope:** `docs/vtrace/` after Trace R3 closure and correction of the
self-referential review-ledger layer.

**Gate type:** scoped Gate 3 review for the VTRACE documentation baseline only.

**Decision:** `pass_with_risk`

The VTRACE documentation baseline is ready to snapshot or use as the control
surface for implementation planning. This is not an implementation-readiness
claim: validation demos, parity fixtures, route/browser checks, command
transcripts, dependency removal, and lean-build proof remain open evidence rows.

### Review lenses

BENCH, FORGE, KEEL, WIRE, HART, EDGE, GLASS, CREST, and broadcast.

### Evidence inspected

- `docs/vtrace/MISSION.md`
- `docs/vtrace/CONOPS.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/ARCHITECTURE.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/TRACE.md`
- `docs/vtrace/REVIEW.md`

### Findings

No new blocking findings. The only status change is `EVID-DOC-001`, which is now
passed for the docs-only VTRACE markdown baseline. All implementation,
validation, dependency, and lean-build evidence boundaries remain unchanged.

### Accepted risks

- This pass closes documentation evidence only.
- Most implementation and validation rows remain `pending`.
- FLETCH/SLICE removal and lean CLI feature-boundary rows remain
  `target-not-met`.
- Future implementation waves must update `VERIFICATION.md`, `VALIDATION.md`,
  `TRACE.md`, and this review record when evidence status changes.

### Validation commands and results

```powershell
git -C C:\src\ICELINES diff --check
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
```

Result: passed on 2026-05-30; `proof` checked 11 files with 0 errors and 0
warnings.

### Required follow-up

Before implementation evidence work, finish first-class review coverage for the
foundational VTRACE files. The 2026-05-30 R1 Foundational Files Review below is
the controlling follow-up for that correction.

## 2026-05-30 R1 Foundational Files Review

**Scope:** first-class review of `MISSION.md`, `REQUIREMENTS.md`,
`INTERFACES.md`, and `VALIDATION.md` after the trace-baseline pass.

**Gate type:** VTRACE artifact coverage review using `.roles` persona lenses.

**Decision:** `pass_with_risk_after_patch`

The review found no need to reopen CONOPS, Architecture, Design, Code Rigor,
Verification, or Trace for broad redesign. It did find one mission target that
was still too prose-only: durable personalized workbench layouts. The patch
carries that target through requirements, interface, design, validation,
verification, code-rigor, and trace rows while keeping it pending rather than
implemented.

### Review lenses

HART, KEEL, TAPE, FORGE, PACE, BENCH, EDGE, WIRE, SCOUT, GLASS, CREST, Jack
Adams, Ted Lindsay, Campbell, and broadcast.

### Evidence inspected

- `docs/vtrace/MISSION.md`
- `docs/vtrace/REQUIREMENTS.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/CODE_RIGOR.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/TRACE.md`
- `.roles/*.md`

### Findings and closure

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| R1-FOUND-01 | Major | Jack Adams, GLASS, BENCH | `MISSION.md` names "personalized workbench layouts" as a success criterion, but the target had no controlled requirement, interface, validation scenario, or trace row. | Closed by adding `REQ-WB-003`, `IF-LAYOUT-001`, `VAL-010`, `EVID-VAL-010`, and corresponding `TRACE.md` rows. |
| R1-FOUND-02 | Major | HART, KEEL, FORGE | Layout persistence affects local user state, but code-rigor and design coverage did not name layouts as a migration/preservation concern. | Closed by adding `DES-015` and extending `CR-020` / `EVID-CR-008` coverage for named layouts. |
| R1-FOUND-03 | Minor | BENCH, WIRE | Verification target-claim wording covered dependency/lean targets but not named layout target posture, risking accidental implementation overclaim. | Closed by updating `VERIFICATION.md` target-claim wording and gap ledger; named layout evidence remains pending. |

### Accepted risks

- Named layout persistence is now traceable, but still has no implementation,
  storage schema, migration/refusal rule, or restore evidence.
- The documentation baseline remains docs-complete, not implementation-complete.
- Dependency/lean rows remain `target-not-met`; most implementation evidence
  remains `pending`.

### Round 2 decision

No R2 Foundational Files review is required before implementation planning. Reopen
this review only if another mission or foundational-file target is discovered
without controlled requirement, interface, validation, verification, and trace
coverage.

## 2026-05-30 WP-002 S4 Execution Checkpoint

**Scope:** `WP-002` named workbench layout persistence after shared schema, CLI,
TUI, and Web restore implementation.

**Gate type:** S4 work-package execution checkpoint, not final close review.

**Decision:** `continue_with_risk`

The implementation is coherent enough to continue toward WP-002 close. Focused
L0 checks pass for the shared schema/store, CLI layout round trip, TUI persisted
layout application, and Web dashboard layout restore. At this checkpoint the
package was not yet closed; the close review below records the later L2 evidence
and accepted L1 risk.

### Review lenses

KEEL, HART, BENCH, FORGE, GLASS, CREST, broadcast, EDGE, WIRE, and Jim Gregory.

### Evidence inspected

- `icelines-core/src/workbench_layout.rs`
- `icelines-cli/src/commands/layout.rs`
- `icelines-cli/src/config.rs`
- `icelines-cli/src/main.rs`
- `icelines-cli/src/tui/mod.rs`
- `icelines-cli/src/tui/mdi.rs`
- `icelines-web/src/config.rs`
- `icelines-web/src/handlers/dashboard.rs`
- `docs/vtrace/CHANGE_CONTROL.md`
- `docs/vtrace/INTERFACES.md`
- `docs/vtrace/DESIGN.md`
- `docs/vtrace/WORK_PACKAGES.md`
- `docs/vtrace/VERIFICATION.md`
- `docs/vtrace/VALIDATION.md`
- `docs/vtrace/TRACE.md`
- `context/waves/2026-05-30-vtrace-wp002-layout/pulses/pulse-01.md`

### Findings and disposition

| ID | Severity | Lens | Finding | Disposition |
|---|---|---|---|---|
| WP002-S4-01 | Note | KEEL / HART | The layout record keeps semantic state to center workbench, left/right pane bindings, optional experience, active-context policy, and version. It does not persist query results or hockey-derived cache state. | Accept for S4; close review later marks `IF-LAYOUT-001` passed_with_risk for WP-002. |
| WP002-S4-02 | Note | FORGE / BENCH | Focused L0 tests pass and the previous unrelated CLI bin-test compile blocker was fixed with a test-local manifest helper. | Accept for S4; still require broad L1 clippy/workspace confidence or documented affected-slice equivalent. |
| WP002-S4-03 | Minor | GLASS / broadcast / CREST | Web restore through `layout=<name>` and TUI restore through `--layout <name>` are explicit and bookmark/local-state safe, but no user-facing restart/reload transcript is recorded yet at this checkpoint. | Close review later records CLI/Web L2 evidence and accepts TUI bin-test evidence. |
| WP002-S4-04 | Minor | Jim Gregory | VTRACE now requires repo-local pulse evidence. The WP-002 wave and pulse record exist and are linked from implementation docs. | Continue using the pulse as the execution record through close review. |

### Validation commands and results

```powershell
cargo fmt --check
cargo test -p icelines-core workbench_layout --lib
cargo test -p icelines-web dashboard --lib
cargo test -p icelines-cli l0_layout_cli_save_and_show_round_trip --bin icelines
cargo test -p icelines-cli l0_mdi_applies_persisted_workbench_layout --bin icelines
```

Result: passed on 2026-05-30 for the focused WP-002 slice.

### Accepted risks

- Full workspace clippy/tests were not recorded at this checkpoint.
- `VAL-010` still needed durable restart/reload demo evidence and stored-record
  inspection at this checkpoint.
- `EVID-CR-008` remains broader than WP-002 layout storage; FantasyDb,
  favorites, watch rules, reports, snapshots, and cache preservation are not
  globally closed by this slice.

### Required next step

Finish WP-002 close by recording L1 posture, L2 `VAL-010` evidence, final
trace/evidence updates, and a Work Package Close Review.

## 2026-05-30 WP-002 Work Package Close Review

**Scope:** `WP-002` named workbench layout persistence only.

**Decision:** `close_with_risk`

`WP-002` is accepted as a controlled implementation slice: shared layout schema
and store, CLI layout management, TUI restore hook, Web `layout=<name>` restore,
stage/pulse evidence, and trace/validation rows are present. The close is
limited to named layout persistence and does not close broader workbench,
browser, dependency, or local-state rows.

### Close evidence

```powershell
cargo fmt --check
cargo test -p icelines-core workbench_layout --lib
cargo test -p icelines-web dashboard --lib
cargo test -p icelines-cli l0_layout_cli_save_and_show_round_trip --bin icelines
cargo test -p icelines-cli l0_mdi_applies_persisted_workbench_layout --bin icelines
cargo test -p icelines-cli l0_verify --bin icelines
cargo clippy -p icelines-core --lib --tests -- -D warnings
target\debug\icelines.exe --no-setup layout save/show tonight
icelines serve --no-live --no-open; GET /dashboard?layout=tonight
git -C C:\src\ICELINES diff --check
C:\src\proof\target\debug\proof.exe check C:\src\ICELINES\docs\vtrace --errors-only
```

Result: WP-002 focused tests, core clippy, durable CLI reload/store inspection,
Web restore demo, formatting, diff, and VTRACE proof passed.

### Accepted close risks

| Risk | Disposition |
|---|---|
| Full `cargo clippy --workspace --all-targets -- -D warnings` is blocked by unrelated existing lint debt in `icelines-fetch`, `icelines-web` tests, and non-layout CLI commands. | Accepted for WP-002 close; carry as broader `EVID-CODE-001` debt and do not use this close to claim workspace lint cleanliness. |
| TUI restore has bin-test evidence but no interactive transcript. | Accepted for WP-002 close because the restore hook is covered by `l0_mdi_applies_persisted_workbench_layout`; future visual/browser-style rehearsal remains in WP-008. |
| `EVID-CR-008` covers more than layout persistence. | Accepted only for the WP-002 layout-store portion; FantasyDb, favorites, watch rules, reports, snapshots, and cache preservation remain open. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Cross-surface semantics converge on the shared record. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence is sufficient for the slice; broader L1 risk is named. |
| FORGE | pass_with_risk | Core clippy passed; unrelated workspace lint debt blocks an unrestricted pass. |
| GLASS / broadcast | pass_with_risk | Web restore demo passed; TUI restore is automated evidence only. |
| CREST / WIRE | pass_with_risk | Local file and URL state are explicit, versioned, and non-secret. |
| Jim Gregory | pass_with_risk | `CHG-001`, stage record, pulse record, and trace rows are updated. |

### Close outcome

`WP-002` closes with risk. `REQ-WB-003`, `IF-LAYOUT-001`, `VAL-010`, and
`EVID-VAL-010` may be treated as `passed_with_risk` for named layout
persistence. `EVID-CODE-001`, broader `EVID-CR-008`, `VAL-001`, `VAL-003`, and
WP-008 integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 01 Close Review

**Scope:** `WP-001` leaders stable identity parity slice only.

**Decision:** `close_with_risk`

The pulse is accepted as a narrow parity improvement: shared `LeadersView`
stable `player_id` now survives into active CLI leaders JSON and Web leaders
JSON as additive `nhl_id` fields, with `team_abbrev` available for direct
cross-surface comparison. The close is limited to leaders CLI/Web JSON identity
and does not close full source-state, query planner, TUI, Web HTML,
report/export, warning, or active-context parity.

### Close evidence

```powershell
cargo fmt --check
cargo clippy -p icelines-web --lib --no-deps -- -D warnings
cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings
cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib
cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_json_export
cargo test -p icelines-cli --test system_tests l2_cmd_query_leaders_json_csv_row_identity_match
cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_stable_identity_match
cargo clippy --workspace --all-targets -- -D warnings
```

Result: affected formatting, Web clippy, CLI clippy, L0 JSON projection, CLI JSON
row-shape checks, and L2 CLI/Web leaders parity passed. Full workspace clippy
remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`.

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders identity parity. | Accepted for pulse 01 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-011`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders JSON identity slice; do not claim unrestricted L1 or workspace cleanliness. |
| TUI, Web HTML, report/export, source/completeness state, warnings, and active context were not compared in this pulse. | Carry these to later WP-001 pulses or WP-008 integration rehearsal. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Stable row identity remains sourced from shared `LeadersView`. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI/Web JSON slice. |
| FORGE | pass_with_risk | Affected Web and CLI clippy passed; workspace clippy blocker is unrelated and recorded. |
| WIRE / broadcast | pass_with_risk | JSON additions are additive and preserve existing fields. |
| Jim Gregory | pass_with_risk | `CHG-002`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 01 closes with risk. `EVID-WP001-L0`, `EVID-WP001-L1`, and
`EVID-WP001-L2` may be treated as passed or passed_with_risk for leaders
identity parity. Broad `WP-001`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-011`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008 integration rehearsal
remain open.

## 2026-05-30 WP-001 Pulse 02 Close Review

**Scope:** `WP-001` leaders Web HTML stable identity slice only.

**Decision:** `close_with_risk`

The pulse is accepted as a narrow parity improvement: active Web leaders HTML
rows now expose stable row identity through additive `data-*` attributes sourced
from the same `LeadersView` projection used by the JSON path. The close is
limited to CLI JSON versus Web HTML leaders row identity and does not close full
source-state, query planner, TUI, report/export, warning, source/completeness, or
active-context parity.

### Close evidence

```powershell
cargo fmt --check
cargo clippy -p icelines-web --lib --no-deps -- -D warnings
cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings
cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib
cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_html_stable_identity_match
```

Result: affected formatting, Web clippy, CLI clippy, L0 Web template identity,
and L2 CLI/Web HTML leaders identity parity passed. Full workspace clippy remains
blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`.

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than Web HTML leaders identity parity. | Accepted for pulse 02 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-011`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Web HTML leaders identity slice; do not claim unrestricted L1 or workspace cleanliness. |
| TUI, report/export, source/completeness state, warnings, and active context were not compared in this pulse. | Carry these to later WP-001 pulses or WP-008 integration rehearsal. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Stable row identity remains sourced from shared `LeadersView`. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI/Web HTML slice. |
| FORGE | pass_with_risk | Affected Web and CLI clippy passed; workspace clippy blocker is unrelated and recorded. |
| broadcast / CREST | pass_with_risk | HTML additions are additive `data-*` attributes and preserve visible layout. |
| Jim Gregory | pass_with_risk | `CHG-003`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 02 closes with risk. `EVID-WP001-HTML-L2` may be treated as
passed for leaders Web HTML identity parity. Broad `WP-001`, `VAL-004`,
`EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-011`, `EVID-CR-018`, `EVID-CODE-001`,
and WP-008 integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 03 Close Review

**Scope:** `WP-001` leaders CLI/Web JSON source-state parity slice only.

**Decision:** `close_with_risk`

The pulse is accepted as a narrow source-state improvement: active leaders CLI
JSON and Web JSON now expose typed `complete` / `roster` source state from the
leaders `ViewContext`, with the existing CLI top-level row array and Web v1
envelope preserved. The close is limited to bundled roster source/completeness
state for the leaders JSON paths and does not close full provenance,
missing/partial/stale paths, warnings, active context, TUI, Web HTML source
badges, or report/export parity.

### Close evidence

```powershell
cargo fmt --check
cargo clippy -p icelines-web --lib --no-deps -- -D warnings
cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings
cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib
cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_source_state_match
```

Result: affected formatting, Web clippy, CLI clippy, L0 Web ViewContext source
state, and L2 CLI/Web JSON source-state parity passed. Full workspace clippy
remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`.

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders JSON source-state parity. | Accepted for pulse 03 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-011`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders JSON source-state slice; do not claim unrestricted L1 or workspace cleanliness. |
| Missing, partial, stale, provenance, warning, active-context, TUI, Web HTML, and report/export states were not compared in this pulse. | Carry these to later WP-001 pulses, WP-005 source reliability work, or WP-008 integration rehearsal. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| HART / KEEL | pass_with_risk | Leaders source state is carried by `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI/Web JSON source-state slice. |
| FORGE | pass_with_risk | Affected Web and CLI clippy passed; workspace clippy blocker is unrelated and recorded. |
| WIRE / broadcast | pass_with_risk | JSON additions are additive and preserve existing CLI/Web response shapes. |
| Jim Gregory | pass_with_risk | `CHG-004`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 03 closes with risk. `EVID-WP001-SOURCE-L2` may be treated as
passed for leaders CLI/Web JSON source-state parity. Broad `WP-001`, `VAL-004`,
`VAL-005`, `EVID-VAL-004`, `EVID-VAL-005`, `EVID-CR-003`, `EVID-CR-011`,
`EVID-CR-018`, `EVID-CODE-001`, and WP-008 integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 04 Close Review

**Scope:** `WP-001` leaders CLI/Web JSON active-context parity slice only.

**Decision:** `close_with_risk`

The pulse is accepted as a narrow active-context improvement: active leaders CLI
JSON now exposes `season` and `season_type` from the leaders `ViewContext`
window, and the L2 fixture compares those row fields with the existing Web JSON
`meta.season` and `meta.season_type`. The close preserves the existing CLI
top-level row array and Web v1 envelope. It does not close active context across
TUI, Web HTML, reports/exports, browser visual hierarchy, warning states, or
recovery/empty paths.

### Close evidence

```powershell
cargo fmt --check
cargo clippy -p icelines-web --lib --no-deps -- -D warnings
cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings
cargo test -p icelines-web l0_web_leaders_view_round_trips_template_and_json_rows --lib
cargo test -p icelines-cli --test goalies_web_cli_parity l2_query_leaders_cli_and_web_active_context_match
```

Result: affected formatting, Web clippy, CLI clippy, L0 Web ViewContext active
window, and L2 CLI/Web JSON active-context parity passed. Full workspace clippy
remains blocked by unrelated `icelines-fetch/src/fletch.rs` `too_many_arguments`.

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders JSON active-context parity. | Accepted for pulse 04 only; keep `WP-001`, `REQ-WB-002`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders JSON active-context slice; do not claim unrestricted L1 or workspace cleanliness. |
| TUI, Web HTML visible context, report/export context, warnings, empty/recovery states, and broader active-context presentation were not compared in this pulse. | Carry these to later WP-001 pulses, WP-003 browser route work, WP-004 report/export work, or WP-008 integration rehearsal. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| HART / KEEL | pass_with_risk | Active window is carried from `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI/Web JSON active-context slice. |
| FORGE | pass_with_risk | Affected Web and CLI clippy passed; workspace clippy blocker is unrelated and recorded. |
| WIRE / broadcast | pass_with_risk | JSON additions are additive and preserve existing CLI/Web response shapes. |
| Jim Gregory | pass_with_risk | `CHG-005`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 04 closes with risk. `EVID-WP001-CONTEXT-L2` may be treated as
passed for leaders CLI/Web JSON active-context parity. Broad `WP-001`,
`VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and
WP-008 integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 05 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-05.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/tests/goalies_web_cli_parity.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive CLI leaders JSON row fields: `total`, `returned`, `top`, `active_filters`. |
| Parity fixture | `l2_query_leaders_cli_and_web_result_state_match` compares CLI row result state with Web JSON meta result state. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders JSON result-state parity. | Accepted for pulse 05 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders JSON result-state slice; do not claim unrestricted L1 or workspace cleanliness. |
| Empty-result CLI JSON still cannot carry row-level result state because the compatibility-preserved shape is a top-level row array. | Carry to a later explicit CLI JSON envelope/versioning decision instead of changing shape in this pulse. |
| TUI, report/export, warnings, browser recovery states, and broader parity were not compared in this pulse. | Carry these to later WP-001 pulses, WP-003 browser route work, WP-004 report/export work, or WP-008 integration rehearsal. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / BENCH | pass_with_risk | Result-state fields are additive and parity-tested against the existing Web JSON meta surface. |
| FORGE | pass_with_risk | Affected Web and CLI clippy passed; workspace clippy blocker is unrelated and recorded. |
| WIRE / broadcast | pass_with_risk | JSON additions preserve existing CLI row-array and Web v1 envelope shapes. |
| Jim Gregory | pass_with_risk | `CHG-006`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 05 closes with risk. `EVID-WP001-RESULT-L2` may be treated as
passed for leaders CLI/Web JSON result-state parity. Broad `WP-001`, `VAL-004`,
`EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008
integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 22 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-22.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/tests/goalies_web_cli_parity.rs` |
| Interface change | Additive default CLI leaders text warning, empty-state detail, and recovery guidance sourced from `LeadersView.warnings` and `LeadersView.empty_state`. |
| Fixture | `l0_query_leaders_warning_empty_lines_report_recovery` asserts renderer helper output, and `l2_query_leaders_cli_text_renders_empty_warning_recovery_state` compares default text output with the CLI JSON envelope metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected CLI text empty/warning recovery presentation. | Accepted for pulse 22 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full cross-surface empty/warning parity remains broader than this slice. | Accepted for pulse 22 only; do not claim full `VAL-004`, full TUI/Web/report parity, or broad provenance closure. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the CLI text leaders empty/warning recovery slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | CLI text recovery is rendered from `LeadersView` empty/warning state, not recomputed from table rows. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover selected default text output against the JSON envelope source. |
| FORGE | pass_with_risk | Affected CLI bin and parity-test clippy passed; broader lint blockers are unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-023`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The text block is additive and preserves JSON, CSV, TUI, Web, Markdown export, context/result, and non-empty table contracts. |

### Close outcome

`WP-001` pulse 22 closes with risk. `EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L0`
and `EVID-WP001-QUERY-TEXT-EMPTY-WARNING-L2` may be treated as passed for
selected default CLI leaders text empty/warning recovery guidance. Broad
`WP-001`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`,
`EVID-CODE-001`, full cross-surface parity, broader source provenance, and
WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 23 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-23.md` |
| Code | `icelines-cli/src/commands/export.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive Markdown leaders export report-body warning, empty-state detail, and recovery guidance sourced from `LeadersView.warnings` and `LeadersView.empty_state`. |
| Fixture | `l0_export_leaders_warning_empty_summary_reports_recovery` asserts renderer helper output, and `l2_cmd_export_md_leaders_empty_warning_matches_query_envelope` compares Markdown export output with the CLI JSON envelope metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Markdown export empty/warning recovery presentation. | Accepted for pulse 23 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `REQ-REPORT-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full report/export parity remains broader than this slice. | Accepted for pulse 23 only; do not claim full `VAL-002`, full `VAL-004`, full export/front-matter parity, or broad provenance closure. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Markdown leaders empty/warning recovery slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Markdown recovery is rendered from `LeadersView` empty/warning state, not recomputed from table rows. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover selected Markdown export output against the JSON envelope source. |
| FORGE | pass_with_risk | Affected CLI bin and system-test clippy passed; broader lint blockers are unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-024`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The Markdown sections are additive and preserve JSON, CSV, TUI, Web, CLI text, front-matter, context/result, and non-empty table contracts. |

### Close outcome

`WP-001` pulse 23 closes with risk. `EVID-WP001-EXPORT-EMPTY-WARNING-L0`
and `EVID-WP001-EXPORT-EMPTY-WARNING-L2` may be treated as passed for selected
Markdown leaders export empty/warning recovery guidance. Broad `WP-001`,
`VAL-002`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`,
`EVID-CR-018`, `EVID-CODE-001`, full report/export parity, broader source
provenance, and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 24 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-24.md` |
| Code | `icelines-cli/src/commands/export.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive Markdown leaders export front-matter warning, empty-state detail, and recovery guidance sourced from `LeadersView.warnings` and `LeadersView.empty_state`. |
| Fixture | `l0_export_leaders_front_matter_reports_empty_warning_state` asserts front-matter helper output, and `l2_cmd_export_md_leaders_front_matter_empty_warning_matches_query_envelope` compares Markdown export front matter with the CLI JSON envelope metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Markdown export front-matter empty/warning recovery metadata. | Accepted for pulse 24 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `REQ-REPORT-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full report/export parity remains broader than this slice. | Accepted for pulse 24 only; do not claim full `VAL-002`, full `VAL-004`, full export parity, or broad provenance closure. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Markdown leaders front-matter empty/warning recovery slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Front-matter recovery metadata is rendered from `LeadersView` empty/warning state, not recomputed from table rows. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover selected Markdown export front matter against the JSON envelope source. |
| FORGE | pass_with_risk | Affected CLI bin and system-test clippy passed; broader lint blockers are unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-025`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The front-matter `state` block is additive and preserves report body, JSON, CSV, TUI, Web, CLI text, context/result, and non-empty table contracts. |

### Close outcome

`WP-001` pulse 24 closes with risk. `EVID-WP001-EXPORT-FM-EMPTY-WARNING-L0`
and `EVID-WP001-EXPORT-FM-EMPTY-WARNING-L2` may be treated as passed for
selected Markdown leaders export front-matter empty/warning recovery guidance.
Broad `WP-001`, `VAL-002`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-007`, `EVID-CR-018`, `EVID-CODE-001`, full report/export parity,
broader source provenance, and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 25 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-25.md` |
| Code | `icelines-cli/src/tui/screens/queries.rs` |
| Interface change | Additive TUI Stats leaders warning, empty-state detail, and recovery guidance sourced from `LeadersView.warnings` and `LeadersView.empty_state` for the selected goalie-filter empty result. |
| Fixture | `l0_tui_leaders_goalie_filter_reports_empty_warning_recovery_state` asserts the state, and `l0_tui_leaders_results_render_goalie_filter_empty_warning_recovery_state` asserts the rendered TUI result lines. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected TUI empty/warning recovery rendering. | Accepted for pulse 25 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full interactive TUI parity remains broader than this slice. | Accepted because this pulse uses focused L0 state/render evidence only; do not claim full TUI parity or WP-008 closure. |
| TUI warning/empty lines change vertical space before table rows. | Accepted because selected-row indexing was adjusted to account for the inserted state lines and the focused render fixture covers the selected empty result. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the TUI empty/warning recovery slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | TUI recovery state is sourced from `LeadersView` empty/warning state, not renderer-local hockey inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 state and render evidence cover the selected TUI Stats leaders empty result slice. |
| FORGE | pass_with_risk | Affected CLI bin formatting, focused tests, and clippy passed; broader lint blockers are unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-026`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The TUI state block is additive and preserves JSON, CSV, Web, CLI text, Markdown export, context/result, and non-empty table contracts. |

### Close outcome

`WP-001` pulse 25 closes with risk. `EVID-WP001-TUI-EMPTY-WARNING-L0` may be
treated as passed for selected TUI Stats leaders empty/warning recovery guidance.
Broad `WP-001`, `VAL-001`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-018`, `EVID-CODE-001`, full interactive TUI parity, broader source
provenance, and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 13 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-13.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive CSV leaders identity/context/source-state metadata sourced from `LeadersView.context`. |
| Fixture | `l0_query_leaders_csv_header_reports_identity_context_and_source_state` and `l2_cmd_query_leaders` assert selected leaders CSV identity/context/source-state metadata and JSON/CSV row identity. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected CLI CSV identity/context/source-state metadata. | Accepted for pulse 13 only; keep `WP-001`, `REQ-WB-002`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full query planner parity and broad cross-surface parity remain broader than this slice. | Accepted for pulse 13 only; do not claim full `VAL-004` or full query-surface closure. |
| CSV consumers may depend on existing leading metric columns. | Accepted because the change is additive trailing metadata and preserves existing leading metric columns and row order. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the CLI CSV leaders metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | CSV metadata is sourced from shared ViewModel row identity and `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover the selected CSV output slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-014`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The CSV change is additive and preserves existing leading metric columns and row order. |

### Close outcome

`WP-001` pulse 13 closes with risk. `EVID-WP001-QUERY-CSV-METADATA-L0` and
`EVID-WP001-QUERY-CSV-METADATA-L2` may be treated as passed for selected CLI CSV
leaders identity/context/source-state metadata. Broad `WP-001`, `VAL-004`,
`EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008
integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 14 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-14.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive CSV leaders query-result metadata for `total`, `returned`, `top`, `sort`, and `active_filters`. |
| Fixture | `l0_query_leaders_csv_header_reports_identity_context_and_source_state` and `l2_cmd_query_leaders_json_csv_row_identity_match` assert selected leaders CSV query-result metadata matches the JSON result/query state. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected CLI CSV query-result metadata. | Accepted for pulse 14 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full query planner parity and broad cross-surface parity remain broader than this slice. | Accepted for pulse 14 only; do not claim full `VAL-004` or full query-surface closure. |
| CSV consumers may depend on existing leading metric and pulse 13 metadata columns. | Accepted because the change is additive trailing metadata and preserves existing leading metric columns, pulse 13 metadata columns, and row order. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the CLI CSV query-result metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | CSV query-result metadata is sourced from leaders query execution state and active filters, not renderer-local recomputation. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover the selected filtered CSV output slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-015`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The CSV change is additive and preserves existing leading metric columns and pulse 13 metadata columns. |

### Close outcome

`WP-001` pulse 14 closes with risk. `EVID-WP001-QUERY-CSV-RESULT-L0` and
`EVID-WP001-QUERY-CSV-RESULT-L2` may be treated as passed for selected CLI CSV
leaders query-result metadata. Broad `WP-001`, `VAL-004`, `EVID-VAL-004`,
`EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008 integration rehearsal
remain open.

## 2026-05-31 WP-001 Pulse 15 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-15.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive default CLI leaders text query-result metadata for `total`, `returned`, `top`, `sort`, and `active_filters`. |
| Fixture | `l0_query_leaders_result_line_reports_query_result_metadata` and `l2_cmd_query_leaders_exits_zero` assert selected leaders text query-result metadata appears before the table. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected CLI text query-result metadata. | Accepted for pulse 15 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full query planner parity and broad cross-surface parity remain broader than this slice. | Accepted for pulse 15 only; do not claim full `VAL-004` or full query-surface closure. |
| CLI text consumers may depend on existing context and table output. | Accepted because the change is an additive `Result:` line after the existing `Context:` line and preserves the table semantics. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the CLI text query-result metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Text query-result metadata is sourced from leaders query execution state and active filters, not renderer-local recomputation. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover the selected filtered text output slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-016`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The text change is additive and preserves existing context/source disclosure and table semantics. |

### Close outcome

`WP-001` pulse 15 closes with risk. `EVID-WP001-QUERY-TEXT-RESULT-L0` and
`EVID-WP001-QUERY-TEXT-RESULT-L2` may be treated as passed for selected default
CLI text leaders query-result metadata. Broad `WP-001`, `VAL-004`,
`EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008
integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 16 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-16.md` |
| Code | `icelines-cli/src/tui/screens/queries.rs` |
| Interface change | Additive TUI Stats leaders query-result metadata for `total`, `returned`, `top`, `sort`, and `active_filters`. |
| Fixture | `l0_tui_leaders_active_filters_label_reports_query_intent` and `l0_tui_leaders_results_render_active_context_and_source_state` assert selected leaders TUI result metadata appears after the context line. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected TUI query-result metadata. | Accepted for pulse 16 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full interactive TUI/system rehearsal remains broader than this slice. | Accepted because this pulse uses focused L0 render/unit evidence only; do not claim full TUI parity or WP-008 closure. |
| Full query planner parity and broad cross-surface parity remain broader than this slice. | Accepted for pulse 16 only; do not claim full `VAL-004` or full query-surface closure. |
| TUI vertical space changes can affect selection affordance. | Accepted because the render path reserves space for the added result line and preserves row selection/table behavior in the focused fixture. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the TUI query-result metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | TUI query-result metadata is sourced from leaders query execution state and active filters, not renderer-local recomputation. |
| BENCH / EDGE | pass_with_risk | Focused L0 render/unit evidence covers the selected filtered TUI output slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-017`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The TUI change is additive and preserves existing context/source disclosure and table semantics. |

### Close outcome

`WP-001` pulse 16 closes with risk. `EVID-WP001-TUI-RESULT-L0` may be treated
as passed for selected TUI Stats leaders query-result metadata. Broad `WP-001`,
`VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, full
interactive TUI/system parity, and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 17 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-17.md` |
| Code | `icelines-cli/src/commands/export.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive Markdown leaders export report-body query-result metadata for `Total`, `Returned`, `Top`, `Sort`, and `Active filters`. |
| Fixture | `l0_export_leaders_reports_result_and_query_intent` and `l2_cmd_export_md_leaders_to_stdout` assert selected Markdown export result metadata appears after the context/source-state section and before the table. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Markdown export query-result metadata. | Accepted for pulse 17 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-REPORT-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-013`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Front-matter result metadata remains outside this slice. | Accepted because this pulse intentionally updates report-body disclosure only while preserving existing front matter and table contracts. |
| Full report/export parity remains broader than this slice. | Accepted for pulse 17 only; do not claim full `VAL-002C`, full `VAL-004`, or broad report/export closure. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Markdown export query-result metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Markdown export result metadata is sourced from the filtered leaders result set and requested query intent, not a renderer-local planner. |
| BENCH / EDGE | pass_with_risk | Focused L0 and L2 evidence covers selected Markdown stdout output with the canonical leaders table preserved. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-018`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The change is additive and preserves existing front matter, context/source disclosure, and table semantics. |

### Close outcome

`WP-001` pulse 17 closes with risk. `EVID-WP001-EXPORT-RESULT-L0` and
`EVID-WP001-EXPORT-RESULT-L2` may be treated as passed for selected Markdown
leaders export query-result metadata. Broad `WP-001`, `VAL-002C`, `VAL-004`,
`EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-013`, `EVID-CR-018`,
`EVID-CODE-001`, full report/export parity, and WP-008 integration rehearsal
remain open.

## 2026-05-31 WP-001 Pulse 18 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-18.md` |
| Code | `icelines-cli/src/commands/export.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive Markdown leaders export front-matter `result` metadata for `total`, `returned`, `top`, `sort`, and `active_filters`. |
| Fixture | `l0_export_leaders_has_required_front_matter`, `l0_export_leaders_reports_result_and_query_intent`, and `l2_cmd_export_md_leaders_to_stdout` assert selected Markdown export front-matter result metadata is emitted from the same result metadata used by the report body. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Markdown export front-matter query-result metadata. | Accepted for pulse 18 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-REPORT-001`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-013`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full report/export parity remains broader than this slice. | Accepted for pulse 18 only; do not claim full `VAL-002C`, full `VAL-004`, or broad report/export closure. |
| Front-matter consumers may depend on existing context/source metadata. | Accepted because the `result` block is additive and preserves existing `context`, `sources`, report body, and table semantics. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Markdown export front-matter result metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Front-matter result metadata is sourced from the same filtered leaders result metadata used by the body summary, not separate renderer-local inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 and L2 evidence covers selected Markdown stdout front matter with the canonical leaders table preserved. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-019`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The change is additive and preserves existing front matter, context/source disclosure, result-body disclosure, and table semantics. |

### Close outcome

`WP-001` pulse 18 closes with risk. `EVID-WP001-EXPORT-FM-RESULT-L0` and
`EVID-WP001-EXPORT-FM-RESULT-L2` may be treated as passed for selected Markdown
leaders export front-matter query-result metadata. Broad `WP-001`, `VAL-002C`,
`VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-013`,
`EVID-CR-018`, `EVID-CODE-001`, full report/export parity, and WP-008
integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 19 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-19.md` |
| Code | `icelines-web/src/templates.rs`; `icelines-web/src/handlers/leaders.rs`; `icelines-web/templates/leaders.html`; `icelines-cli/tests/goalies_web_cli_parity.rs` |
| Interface change | Additive Web leaders HTML `data-result-total`, `data-result-returned`, `data-result-top`, `data-result-sort`, and `data-result-active-filters` metadata on the existing result meta line. |
| Fixture | `l0_web_leaders_view_round_trips_template_and_json_rows` asserts the HTML attributes, and `l2_query_leaders_cli_json_and_web_html_result_state_match` compares them with CLI JSON result-state metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Web HTML query-result metadata. | Accepted for pulse 19 only; keep `WP-001`, `REQ-QUERY-001`, `REQ-WEB-002`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-006`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full browser route/accessibility parity remains broader than this slice. | Accepted for pulse 19 only; do not claim full `VAL-003`, full `VAL-004`, or broad browser route/accessibility closure. |
| HTML consumers may depend on existing active-context attributes. | Accepted because the `data-result-*` attributes are additive and preserve existing `data-active-*`, human-readable meta, table, empty-state, and recovery semantics. |
| Full workspace clippy still fails in `icelines-fetch`, and Web all-targets clippy still fails in an unrelated Web route test. | Accepted as unrelated to the Web HTML leaders result metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | HTML result metadata is rendered from the selected leaders result context, not recomputed from renderer-local table inspection. |
| BENCH / EDGE | pass_with_risk | Focused L0 and L2 evidence covers selected Web HTML attributes and CLI JSON parity for the canonical leaders result. |
| FORGE | pass_with_risk | Affected Web lib and CLI parity-test clippy passed; broader lint blockers are unrelated and recorded. |
| broadcast / CREST | pass_with_risk | The Web HTML change is read-only, additive metadata on an existing page; no new script or mutation path was introduced. |
| WIRE | pass_with_risk | The change preserves existing JSON, active-context, empty-state, recovery, and table contracts while adding machine-readable result metadata. |

### Close outcome

`WP-001` pulse 19 closes with risk. `EVID-WP001-HTML-RESULT-L2` may be treated as
passed for selected leaders Web HTML query-result metadata parity. Broad
`WP-001`, `VAL-003`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-006`,
`EVID-CR-018`, `EVID-CODE-001`, full browser route/accessibility parity, and
WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 20 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-20.md` |
| Code | `icelines-web/src/templates.rs`; `icelines-web/src/handlers/leaders.rs`; `icelines-web/templates/leaders.html`; `icelines-cli/tests/goalies_web_cli_parity.rs` |
| Interface change | Additive Web leaders HTML `data-source-kind` and `data-source-completeness` metadata on the existing leaders meta line. |
| Fixture | `l0_web_leaders_view_round_trips_template_and_json_rows` asserts the HTML attributes, and `l2_query_leaders_cli_json_and_web_html_source_state_match` compares them with CLI JSON source-state metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Web HTML source-state metadata. | Accepted for pulse 20 only; keep `WP-001`, `REQ-DATA-001`, `REQ-WEB-002`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-006`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full browser route/accessibility parity remains broader than this slice. | Accepted for pulse 20 only; do not claim full `VAL-003`, full `VAL-004`, full `VAL-005`, or broad browser route/accessibility/source provenance closure. |
| HTML consumers may depend on existing active-context and result attributes. | Accepted because the `data-source-*` attributes are additive and preserve existing `data-active-*`, `data-result-*`, human-readable meta, table, empty-state, and recovery semantics. |
| Full workspace clippy still fails in `icelines-fetch`, and Web all-targets clippy still fails in an unrelated Web route test. | Accepted as unrelated to the Web HTML leaders source-state metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | HTML source metadata is rendered from `LeadersView.context.source_state`, not recomputed from renderer-local table inspection. |
| BENCH / EDGE | pass_with_risk | Focused L0 and L2 evidence covers selected Web HTML attributes and CLI JSON parity for the canonical leaders source state. |
| FORGE | pass_with_risk | Affected Web lib and CLI parity-test clippy passed; broader lint blockers are unrelated and recorded. |
| broadcast / CREST | pass_with_risk | The Web HTML change is read-only, additive metadata on an existing page; no new script or mutation path was introduced. |
| WIRE | pass_with_risk | The change preserves existing JSON, active-context, result-state, empty-state, recovery, and table contracts while adding machine-readable source metadata. |

### Close outcome

`WP-001` pulse 20 closes with risk. `EVID-WP001-HTML-SOURCE-L2` may be treated as
passed for selected leaders Web HTML source-state metadata parity. Broad
`WP-001`, `VAL-003`, `VAL-004`, `VAL-005`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-006`, `EVID-CR-018`, `EVID-CODE-001`, full browser
route/accessibility parity, broader source provenance, and WP-008 integration
rehearsal remain open.

## 2026-05-31 WP-001 Pulse 21 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-21.md` |
| Code | `icelines-web/src/templates.rs`; `icelines-web/src/handlers/leaders.rs`; `icelines-web/templates/leaders.html`; `icelines-cli/tests/goalies_web_cli_parity.rs` |
| Interface change | Additive Web leaders HTML `data-empty-kind`, `data-warning-count`, `data-warning-kinds`, and per-warning `data-warning-kind` metadata on the existing leaders meta, warning, and empty-state markup. |
| Fixture | `l0_web_leaders_template_renders_empty_warning_recovery` asserts the HTML attributes, and `l2_query_leaders_cli_json_and_web_html_empty_warning_metadata_match` compares them with CLI JSON envelope empty/warning metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Web HTML empty/warning metadata. | Accepted for pulse 21 only; keep `WP-001`, `REQ-DATA-001`, `REQ-WEB-001`, `REQ-WEB-002`, `REQ-PARITY-001`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-006`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full browser route/accessibility parity remains broader than this slice. | Accepted for pulse 21 only; do not claim full `VAL-003`, full `VAL-004`, or broad browser route/accessibility/source provenance closure. |
| HTML consumers may depend on existing active-context, result, and source attributes. | Accepted because the empty/warning attributes are additive and preserve existing `data-active-*`, `data-result-*`, `data-source-*`, human-readable meta, table, empty-state, and recovery semantics. |
| Full workspace clippy still fails in `icelines-fetch`, and Web all-targets clippy still fails in an unrelated Web route test. | Accepted as unrelated to the Web HTML leaders empty/warning metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | HTML empty/warning metadata is rendered from `LeadersView.empty_state` and `LeadersView.warnings`, not recomputed from renderer-local table inspection. |
| BENCH / EDGE | pass_with_risk | Focused L0 and L2 evidence covers selected Web HTML attributes and CLI JSON envelope parity for the canonical leaders empty result. |
| FORGE | pass_with_risk | Affected Web lib and CLI parity-test clippy passed; broader lint blockers are unrelated and recorded. |
| broadcast / CREST | pass_with_risk | The Web HTML change is read-only, additive metadata on an existing page; no new script or mutation path was introduced. |
| WIRE | pass_with_risk | The change preserves existing JSON, active-context, result-state, source-state, recovery, and table contracts while adding machine-readable empty/warning metadata. |

### Close outcome

`WP-001` pulse 21 closes with risk. `EVID-WP001-HTML-EMPTY-WARNING-L2` may be
treated as passed for selected leaders Web HTML empty/warning metadata parity.
Broad `WP-001`, `VAL-003`, `VAL-004`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-006`, `EVID-CR-018`, `EVID-CODE-001`, full browser
route/accessibility parity, broader source provenance, and WP-008 integration
rehearsal remain open.

## 2026-05-30 WP-001 Pulse 06 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-06.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/src/cli.rs`; `icelines-cli/src/main.rs`; `icelines-web/src/handlers/leaders.rs`; `icelines-cli/tests/goalies_web_cli_parity.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Opt-in CLI leaders `--json-envelope` plus additive Web leaders JSON `meta.empty_state` and `meta.warnings`. |
| Parity fixture | `l2_query_leaders_cli_and_web_empty_warning_state_match` compares CLI envelope data/meta with Web JSON meta for the goalie-filter empty leaders result. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders JSON empty/warning parity. | Accepted for pulse 06 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Existing CLI `query leaders --json` remains a top-level row array and cannot carry empty-result metadata. | Accepted as compatibility posture; use the new opt-in `--json-envelope` for envelope metadata rather than changing existing clients. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders JSON empty/warning slice; do not claim unrestricted L1 or workspace cleanliness. |
| TUI, report/export, browser HTML recovery, and broader provenance/context behavior were not compared in this pulse. | Carry these to later WP-001 pulses, WP-003 browser route work, WP-004 report/export work, or WP-008 integration rehearsal. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Empty/warning state is carried by ViewModel context and recovery actions, not renderer-local guesswork. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI/Web JSON empty/warning slice. |
| FORGE | pass_with_risk | Affected Web and CLI clippy passed; workspace clippy blocker is unrelated and recorded. |
| WIRE / broadcast | pass_with_risk | The compatibility-preserved CLI row array remains available; envelope metadata is opt-in. |
| Jim Gregory | pass_with_risk | `CHG-007`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 06 closes with risk. `EVID-WP001-EMPTY-WARNING-L2` may be treated
as passed for leaders CLI/Web JSON empty/warning parity. Broad `WP-001`,
`VAL-004`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and
WP-008 integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 07 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-07.md` |
| Code | `icelines-web/src/templates.rs`; `icelines-web/src/handlers/leaders.rs`; `icelines-web/templates/leaders.html`; `icelines-cli/tests/goalies_web_cli_parity.rs` |
| Interface change | Additive Web leaders HTML empty-state/warning recovery rendering from the ViewModel template data. |
| Parity fixture | `l2_query_leaders_cli_json_and_web_html_recovery_match` compares CLI envelope metadata with Web HTML recovery text and `/goalies` link semantics for the goalie-filter empty leaders result. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders browser HTML recovery parity. | Accepted for pulse 07 only; keep `WP-001`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders Web HTML recovery slice; do not claim unrestricted L1 or workspace cleanliness. |
| The browser route/accessibility proof is broader than this selected HTML recovery fixture. | Carry no-JS, viewport, focus/touch, color-not-alone, 404, host/CORS, launch, TUI, and report/export evidence to later WP-003, WP-004, or WP-008 work. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Empty/warning recovery remains sourced from ViewModel metadata and is rendered without template-local hockey inference. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI envelope/Web HTML recovery slice. |
| FORGE | pass_with_risk | Affected Web and CLI parity-test clippy passed; workspace clippy blocker is unrelated and recorded. |
| broadcast / CREST | pass_with_risk | HTML additions provide visible recovery text and link semantics while preserving the non-empty leaders table path. |
| Jim Gregory | pass_with_risk | `CHG-008`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 07 closes with risk. `EVID-WP001-HTML-RECOVERY-L2` may be treated
as passed for selected leaders browser HTML recovery parity. Broad `WP-001`,
`VAL-003`, `VAL-004`, `EVID-VAL-003`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-006`, `EVID-CR-014`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008
integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 08 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-08.md` |
| Code | `icelines-web/src/templates.rs`; `icelines-web/src/handlers/leaders.rs`; `icelines-web/templates/leaders.html`; `icelines-cli/tests/goalies_web_cli_parity.rs` |
| Interface change | Additive Web leaders HTML `data-active-season` and `data-active-season-type` attributes sourced from the leaders `ViewContext`. |
| Parity fixture | `l2_query_leaders_cli_json_and_web_html_active_context_match` compares CLI JSON active context with Web HTML active-context attributes. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than leaders browser HTML active-context parity. | Accepted for pulse 08 only; keep `WP-001`, `REQ-WB-002`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the leaders Web HTML active-context slice; do not claim unrestricted L1 or workspace cleanliness. |
| The browser route/accessibility proof is broader than this selected HTML active-context fixture. | Carry no-JS, viewport, focus/touch, color-not-alone, 404, host/CORS, launch, TUI, report/export, broader active-context, and provenance evidence to later WP-003, WP-004, or WP-008 work. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Active context remains sourced from `ViewContext`, not template-local inference. |
| BENCH / EDGE | pass_with_risk | L0/L2 evidence covers the selected leaders CLI JSON/Web HTML active-context slice. |
| FORGE | pass_with_risk | Affected Web and CLI parity-test clippy passed; workspace clippy blocker is unrelated and recorded. |
| broadcast / CREST | pass_with_risk | HTML additions expose machine-readable context attributes while preserving visible meta text. |
| Jim Gregory | pass_with_risk | `CHG-009`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 08 closes with risk. `EVID-WP001-HTML-CONTEXT-L2` may be treated
as passed for selected leaders browser HTML active-context parity. Broad
`WP-001`, `VAL-003`, `VAL-004`, `EVID-VAL-003`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-006`, `EVID-CR-014`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008
integration rehearsal remain open.

## 2026-05-30 WP-001 Pulse 09 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-09.md` |
| Code | `icelines-cli/src/tui/screens/queries.rs` |
| Interface change | Additive TUI Stats leaders context/source-state presentation sourced from `LeadersView.context`. |
| Fixture | `l0_tui_leaders_*` tests assert selected leaders ViewContext source-state and rendered `Context: 20242025 regular | source roster complete`. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected TUI leaders context/source presentation. | Accepted for pulse 09 only; keep `WP-001`, `REQ-WB-002`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the TUI leaders context/source-state slice; do not claim unrestricted L1 or workspace cleanliness. |
| The selected TUI fixture is L0 render evidence, not a full interactive TUI rehearsal or report/export parity check. | Carry interactive TUI, report/export, query planner, broader provenance/context, browser route/accessibility, and WP-008 rehearsal evidence forward. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | TUI display is sourced from `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 evidence covers the selected TUI Stats leaders result slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| WIRE | pass_with_risk | The context/source line is additive and preserves existing results table content. |
| Jim Gregory | pass_with_risk | `CHG-010`, stage record, pulse record, and trace/evidence rows are updated. |

### Close outcome

`WP-001` pulse 09 closes with risk. `EVID-WP001-TUI-CONTEXT-L0` may be treated
as passed for selected leaders TUI context/source-state presentation. Broad
`WP-001`, `VAL-001`, `VAL-004`, `EVID-VAL-001`, `EVID-VAL-004`, `EVID-CR-003`,
`EVID-CR-018`, `EVID-CODE-001`, and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 10 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-10.md` |
| Code | `icelines-cli/src/commands/export.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive Markdown leaders export context/source-state presentation sourced from `LeadersView.context`. |
| Fixture | `l0_export_leaders_reports_context_and_source_state` and `l2_cmd_export_md_leaders_to_stdout` assert selected leaders export context/source-state before the table. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Markdown leaders export context/source presentation. | Accepted for pulse 10 only; keep `WP-001`, `REQ-WB-002`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full report/export and public historical reporting evidence remain broader than this slice. | Accepted for pulse 10 only; keep `REQ-REPORT-001`, `VAL-002`, `EVID-VAL-002C`, `EVID-CR-007`, and `EVID-CR-013` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Markdown leaders export context/source-state slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Export display is sourced from `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover the selected Markdown leaders report slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-011`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The context/source section is additive and preserves existing leaders table columns. |

### Close outcome

`WP-001` pulse 10 closes with risk. `EVID-WP001-EXPORT-CONTEXT-L0` and
`EVID-WP001-EXPORT-CONTEXT-L2` may be treated as passed for selected leaders
Markdown export context/source-state presentation. Broad `WP-001`, `VAL-002`,
`VAL-004`, `EVID-VAL-002C`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`,
`EVID-CR-013`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008 integration rehearsal
remain open.

## 2026-05-31 WP-001 Pulse 11 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-11.md` |
| Code | `icelines-cli/src/commands/export.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive Markdown leaders export front-matter metadata sourced from `LeadersView.context`. |
| Fixture | `l0_export_leaders_has_required_front_matter`, `l0_export_leaders_reports_context_and_source_state`, and `l2_cmd_export_md_leaders_to_stdout` assert selected leaders export front-matter context/source metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected Markdown leaders export front-matter metadata. | Accepted for pulse 11 only; keep `WP-001`, `REQ-WB-002`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full report/export and public historical reporting evidence remain broader than this slice. | Accepted for pulse 11 only; keep `REQ-REPORT-001`, `VAL-002`, `EVID-VAL-002C`, `EVID-CR-007`, and `EVID-CR-013` open overall. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the Markdown leaders export front-matter metadata slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | Export metadata is sourced from `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover the selected Markdown leaders report metadata slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-012`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The front-matter metadata is additive and preserves the existing body context section and leaders table columns. |

### Close outcome

`WP-001` pulse 11 closes with risk. `EVID-WP001-EXPORT-METADATA-L0` and
`EVID-WP001-EXPORT-METADATA-L2` may be treated as passed for selected leaders
Markdown export front-matter metadata. Broad `WP-001`, `VAL-002`, `VAL-004`,
`EVID-VAL-002C`, `EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-007`, `EVID-CR-013`,
`EVID-CR-018`, `EVID-CODE-001`, and WP-008 integration rehearsal remain open.

## 2026-05-31 WP-001 Pulse 12 Close Review

### Scope reviewed

| Item | Reviewed Evidence |
|---|---|
| Pulse | `context/waves/2026-05-30-vtrace-wp001-parity/pulses/pulse-12.md` |
| Code | `icelines-cli/src/commands/query.rs`; `icelines-cli/tests/system_tests.rs` |
| Interface change | Additive default CLI leaders text context/source-state disclosure sourced from `LeadersView.context`. |
| Fixture | `l0_query_leaders_context_line_reports_context_and_source_state` and `l2_cmd_query_leaders_exits_zero` assert selected leaders query text context/source metadata. |

### Accepted close risks

| Risk | Disposition |
|---|---|
| `WP-001` is broader than selected CLI text query context/source-state presentation. | Accepted for pulse 12 only; keep `WP-001`, `REQ-WB-002`, `REQ-DATA-001`, `REQ-QUERY-001`, `REQ-PARITY-001`, `EVID-CR-003`, `EVID-CR-018`, and `EVID-CODE-001` open overall. |
| Full query planner parity and broad cross-surface parity remain broader than this slice. | Accepted for pulse 12 only; do not claim full `VAL-004` or full query-surface closure. |
| Full workspace clippy still fails in `icelines-fetch`. | Accepted as unrelated to the CLI text leaders context/source-state slice; do not claim unrestricted L1 or workspace cleanliness. |

### Role decision

| Lane | Decision | Note |
|---|---|---|
| KEEL / HART | pass_with_risk | CLI text context is sourced from `ViewContext`, not renderer-local inference. |
| BENCH / EDGE | pass_with_risk | Focused L0 and CLI subprocess evidence cover the selected default leaders text output slice. |
| FORGE | pass_with_risk | Affected CLI bin clippy passed; workspace clippy blocker is unrelated and recorded. |
| Jim Gregory | pass_with_risk | `CHG-013`, stage record, pulse record, and trace/evidence rows are updated. |
| WIRE | pass_with_risk | The context/source line is additive and preserves JSON, CSV, Web, TUI, Markdown export, and leaders table semantics. |

### Close outcome

`WP-001` pulse 12 closes with risk. `EVID-WP001-QUERY-TEXT-CONTEXT-L0` and
`EVID-WP001-QUERY-TEXT-CONTEXT-L2` may be treated as passed for selected default
CLI leaders text context/source-state presentation. Broad `WP-001`, `VAL-004`,
`EVID-VAL-004`, `EVID-CR-003`, `EVID-CR-018`, `EVID-CODE-001`, and WP-008
integration rehearsal remain open.
