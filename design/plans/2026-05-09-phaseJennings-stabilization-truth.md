# Phase Jennings - stabilization and truth pass

**Date**: 2026-05-09
**Status**: Draft - immediate pre-Messier gate
**Trophy**: William M. Jennings Trophy. Fit: defensive excellence. This phase is about allowing fewer goals: broken tests, stale docs, drifted plans, and hidden assumptions.
**Target release**: pre-v0.24.0, before Phase Messier begins
**Estimated**: 0.5-1.5 days

---

## Why

IceLines is now large enough that feature velocity depends on trust in the
baseline. A prior narrow check appeared green, but the command was not recorded;
Jennings must re-run and record exact commands before treating any baseline as
true. The known full-suite failure class is that
`icelines-cli/tests/foster_capability_matrix.rs` constructs `Config` without the
newer `ai` field. That specific failure is small; the defect class is bigger.

Jennings restores the build-green invariant and makes docs/plans match the
codebase before the next feature phase.

---

## Role review gates

| Role | Gate |
|---|---|
| HART | Any cache/model statement must name its `(player_id, season, season_type)` coupling or say why it is identity-stable. |
| KEEL | README, ARCHITECTURE, plans index, COMMANDS, and actual crates/routes must agree on surface status. |
| TAPE | Data-source truth claims must distinguish bundled, snapshot, installed, and live fetch paths. |
| FORGE | Add a stable config constructor/default so tests do not hand-build drifting struct literals. |
| PACE | Any test count, timing, or complexity number is either measured from this baseline or marked estimate. |
| BENCH | Full workspace tests compile and run; baseline command outputs and test count are recorded. |
| EDGE | Add a PITFALLS entry for config literal drift and any new failure found during the pass. |
| WIRE | External API and JSON-route claims are marked implemented/partial/deferred, not aspirational. |
| SCOUT | No new hockey-methodology claims. Existing depth/goalie claims are left untouched unless verified. |
| GLASS/Broadcast | User-facing docs must not advertise routes/commands as done when they are stubs or coming-soon. |

---

## Scope

### Jennings.1 - Build-green repair

- Fix the current `cargo test --workspace --no-fail-fast` compile failure.
- Add one of:
  - `impl Default for Config`, or
  - `Config::test_default()`, or
  - a small test-fixture builder in `icelines-cli` that owns config defaults.
- Replace hand-built `Config { ... }` test literals where the drift risk is
  obvious.

Acceptance:
- `cargo check --workspace` green.
- `cargo test --workspace --no-fail-fast` compiles and runs.
- The config drift class has a structural fix, not only a one-line patch.

### Jennings.2 - Baseline ledger

Record the current baseline in this plan's closeout block:

- command used,
- exact working tree state before the command,
- pass/fail status,
- test count if available,
- known warnings intentionally left out of scope,
- dirty files at start/end.

Acceptance:
- Future plans stop carrying stale counts like "803 green" or "1051 green"
  unless they were measured after Jennings.
- Any claim that a workspace check "passes" names the exact command and date.

### Jennings.3 - Plan and doc truth pass

Update:

- `design/plans/INDEX.md`
- `design/phases.md`
- `README.md` architecture summary if still stale
- `design/ARCHITECTURE.md` only where it makes current-state claims
- `COMMANDS.md` only for obviously stale route/command status

Rules:

- Do not rewrite historical docs for style.
- Mark work as `Implemented`, `Active`, `Planned`, `Partial`, or `Deferred`.
- If a web route is a stub/coming-soon, say so.
- If a plan depends on a green baseline, name Jennings as the precondition.

Acceptance:
- The plans index has a "Roadmap from Jennings" section.
- Phase Messier names Jennings as a hard preflight.
- Phase Campbell is named as the ViewModel/platform-contract stage between
  Jennings and Messier.
- `design/specs/platform-contracts.md` exists and is linked by forward plans.
- Stale crate-count claims are corrected or filed as a follow-up.

### Jennings.4 - Role-review checklist template

Add or repeat this checklist in every active forward plan:

```markdown
## Role Review Gates

- HART:
- KEEL:
- TAPE:
- FORGE:
- PACE:
- BENCH:
- EDGE:
- WIRE:
- SCOUT:
- GLASS/Broadcast:
```

Acceptance:
- Messier, Lester Patrick, Ted Lindsay, and Jim Gregory all include the gate.
- Campbell, Selke, and Prince of Wales also include the gate once those plans
  are active in the forward roadmap.

---

## Out of scope

- No new product features.
- No web route implementation.
- No broad refactors beyond the config-test drift fix.
- No dependency churn unless required to restore the build.

---

## Verification

Run at phase exit:

```bash
cargo check --workspace
cargo test --workspace --no-fail-fast
cargo fmt --check
```

Clippy policy for Jennings: run `cargo clippy --workspace --no-deps` if feasible,
but do not expand the phase into a full lint-burn-down unless the user chooses
to make Jim Gregory active immediately.

---

## Exit criteria

- The workspace test suite builds again.
- The drift class behind the failure is structurally addressed.
- Roadmap docs point to the right next phase.
- Messier can begin from a known-green baseline.

---

## Closeout ledger

**Status**: Partially complete - Jennings.1 and baseline measurement landed.

**Structural fix**:

- Added `impl Default for Config`.
- Added `Config::test_default()` as the single test/fallback constructor.
- Replaced obvious drifting `Config { ... }` literals in:
  - `icelines-cli/tests/foster_capability_matrix.rs`
  - `icelines-cli/src/commands/setup.rs`
  - `icelines-cli/src/tui/screens/player.rs`
  - `icelines-cli/src/tui/app.rs`

**Measured commands**:

| Command | Result | Notes |
|---|---|---|
| `cargo check -p icelines-cli` | PASS | Warnings only. |
| `cargo test -p icelines-cli --test foster_capability_matrix` | PASS | 24 passed. |
| `cargo check --workspace` | PASS | Warnings only. |
| `cargo test --workspace --no-fail-fast` | PASS | Full workspace green. |
| `cargo test --workspace -- --list` | PASS | Counted 4620 `: test` entries. |
| `cargo fmt --check` | FAIL | Broad pre-existing formatting drift across unrelated files. Touched Rust files were formatted with `rustfmt --edition 2021`. |

**Measured baseline**:

- Test inventory: 4620 `: test` entries from `cargo test --workspace -- --list`.
- Full workspace test command: green.
- Formatting gate: not yet green repo-wide; defer broad formatting policy to
  Jim Gregory or a dedicated fmt cleanup to avoid noisy unrelated churn.

**Known warnings left out of scope**:

- Existing unused/dead-code warnings in `icelines-query`, `icelines-cli`,
  `icelines-fetch`, and external local `mdpath`.
- Existing repo-wide rustfmt drift outside touched files.
