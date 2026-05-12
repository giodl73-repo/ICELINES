# Phase Jim Gregory - release and operations hardening

**Date**: 2026-05-09
**Status**: Implemented - latest CI pending
**Trophy**: Jim Gregory General Manager of the Year Award. Fit: this phase manages the whole organization: CI, release discipline, data freshness, packaging, and operational trust.
**Estimated**: 2-4 days

---

## Why

IceLines is no longer a small CLI. It is a bundled-data Rust workspace with CLI,
TUI, web, site, snapshots, live fetches, visual-system expectations, and release
artifacts. That needs an operations plan:

- what blocks a release,
- how data freshness is checked,
- how current season rolls over,
- how screenshot/golden visual checks fit into release confidence,
- what CI guarantees,
- what a downloaded binary is expected to do cold.

Jim Gregory turns release quality from habit into checklist.

---

## Role review gates

| Role | Gate |
|---|---|
| HART | Release checks include current season/type and snapshot schema compatibility. |
| KEEL | CI and release smoke cover CLI/TUI/web/site enough to protect surface convergence. |
| TAPE | Bundled data freshness/provenance is recorded for each release. |
| FORGE | CI uses stable Rust commands, avoids broad shell magic, and blocks unsafe dependency drift where chosen. |
| PACE | Performance budgets are measured only where gates require them; otherwise marked advisory. |
| BENCH | Test tiers are explicit; no live-network tests in default CI. |
| EDGE | Release checklist includes season rollover, empty data, stale snapshots, and corrupted snapshot cases. |
| WIRE | External API changes fail loudly in fetch tests; network failures have clear errors. |
| SCOUT | Release smoke includes at least one known hockey sanity path: player, team, goalie, playoff/schedule. |
| GLASS/Broadcast | Release smoke includes `icelines serve --no-open` and verifies URL/banner behavior. |

---

## Platform contracts enforced

Jim Gregory turns `design/specs/platform-contracts.md` into release discipline:

- **Data context**: smoke tests assert season/type/source state appears in major
  outputs.
- **Query/filter intent**: representative CLI/TUI/web filters share expected row
  identity on fixtures.
- **ViewModel**: contract fixtures build and serialize without renderer
  dependencies.
- **Surface parity**: the matrix has no undocumented `stub` or stale `done`
  claims before release.
- **Visual language**: screenshot/golden checks protect agreed TUI/web/CLI
  semantic tokens.

---

## Sub-phase ordering

```text
JimGregory.1  CI gate policy
JimGregory.2  Release checklist and versioning rules
JimGregory.3  Data freshness and season rollover
JimGregory.4  Binary smoke and packaging checks
JimGregory.5  Docs and closeout
```

---

## JimGregory.1 - CI gate policy

Define required and advisory checks.

Required candidates:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace --no-fail-fast
```

Conditional/advisory candidates:

```bash
cargo clippy --workspace --no-deps -- -D warnings
cargo audit
cargo test --workspace --features snapshot-tests
cargo bench -p icelines-cli --bench filter_chain
```

Acceptance:

- `.github/workflows` matches the chosen policy.
- If clippy is not fully blocking yet, the doc says why and lists the burn-down path.
- If `cargo audit` is blocking, CI installs `cargo-audit`, documents ignored
  advisories with expiry dates, and distinguishes runtime from dev-only issues.
- Snapshot tests are blocking only if deterministic fixtures are checked in.
- Criterion benches are advisory unless a specific performance regression gate
  is documented.
- No default CI job depends on live NHL/ESPN/MoneyPuck network access.

Progress:

- 2026-05-12: Updated `.github/workflows/ci.yml` so release workflow,
  release-smoke script, release checklist, rollover doc, README, and command
  doc changes trigger CI. The CI release job now runs
  `scripts/release-smoke.ps1` instead of only compiling the optimized binary.

---

## JimGregory.2 - Release checklist and versioning rules

Add `docs/release-checklist.md` or `design/release-checklist.md`.

Checklist must include:

- version bump locations,
- changelog entry,
- plans index status update,
- README command sanity,
- bundled-data manifest/provenance,
- smoke commands,
- tag command,
- artifact names,
- rollback notes.

Acceptance:

- One release checklist exists and is linked from `CLAUDE.md` or `README.md`.
- Versioning guidance distinguishes patch, minor, and data-only updates.

Progress:

- 2026-05-12: Added `design/release-checklist.md` with release type/version
  rules, version/doc touchpoints, current-season/data sanity, required gates,
  binary smoke commands, artifact names, tag flow, rollback notes, and advisory
  gate status.
- 2026-05-12: Linked the checklist from `CLAUDE.md`, `README.md`, and
  `COMMANDS.md`.

---

## JimGregory.3 - Data freshness and season rollover

Document and test where possible:

- `CURRENT_SEASON` update process,
- October rollover checklist,
- playoff season-type availability,
- bundled season list verification,
- snapshot schema/version compatibility,
- what "weekly refreshed" means and where it is recorded.

Acceptance:

- There is a written current-season rollover procedure.
- A test or script verifies bundled season IDs include the expected current season.
- Data freshness expectations in README match the actual automation.

Progress:

- 2026-05-12: Added `design/current-season-rollover.md` with the October
  rollover procedure, bundled-data invariants, required gates, and the exact
  data freshness contract: GitHub data tarballs refresh weekly, while embedded
  binary data changes only when refreshed `data/seasons/` files are committed
  before a release build.
- 2026-05-12: Added a release rollover fence in
  `icelines-fetch/src/bundled.rs` asserting `CURRENT_SEASON_STR` is the newest
  bundled season, current-season regular/goalie data is embedded, and the
  2004-05 lockout remains excluded.
- 2026-05-12: Updated README, data guide, data-bundle spec, and the data
  bundle workflow wording so freshness claims match the actual automation.

---

## JimGregory.4 - Binary smoke and packaging checks

Define release smoke commands for a fresh binary:

```bash
icelines --help
icelines query leaders --top 5
icelines query player McDavid --percentiles
icelines team EDM
icelines goalies --top 5
icelines serve --no-open --port 0
```

Adjust exact commands to real CLI support after Lester Patrick.

Acceptance:

- A smoke script exists or a manual checklist is documented.
- Smoke does not require a live fetch.
- Web serve smoke verifies URL printing before browser open behavior.

Progress:

- 2026-05-12: Added `scripts/release-smoke.ps1`. The script builds the
  optimized CLI by default, then smokes `--version`, `--help`,
  `query leaders`, `query goalies`, `tui --help`, `serve --help`, `docs`,
  `export md leaders`, `poach`, and a short-lived `serve --no-open` URL check.
  It also supports `-SkipBuild` for validating an already-built
  `target/release/icelines.exe`.
- 2026-05-12: Updated the local `ci-release` test slice to run the release
  smoke script instead of only compiling the release binary.
- 2026-05-12: Added release-workflow archive verification before artifact
  upload, asserting each zip/tarball exists and contains the expected
  platform binary name.

---

## JimGregory.5 - Docs and closeout

Update:

- `README.md`
- `COMMANDS.md`
- `CHANGELOG.md`
- `design/plans/INDEX.md`
- `design/phases.md`

Acceptance:

- Release gates are documented.
- CI status and release process are no longer tribal knowledge.

Progress:

- 2026-05-12: Updated `README.md`, `COMMANDS.md`, `CHANGELOG.md`,
  `design/plans/INDEX.md`, `design/phases.md`, `design/ARCHITECTURE.md`, and
  data specs so release gates, data freshness, bundled season scope, and
  artifact expectations are explicit.
- 2026-05-12: Local closeout gates passed:
  `cargo test -p icelines-fetch bundled`, `cargo fmt --check`,
  `scripts/test-slice.ps1 ci-release`, and `git diff --check`.
- 2026-05-12: Remote CI for the latest Jim Gregory commits is still pending;
  close the phase after GitHub Actions reports success for the head commit.

---

## Out of scope

- Changing product behavior.
- Public hosted deployment.
- Auth/TLS.
- Replacing GitHub Actions with another CI.
