# /review-specs — IceLines Role-Based Spec Review

Run a structured review of the IceLines specs using all 8 domain roles.
Each role applies its lens to find gaps, inconsistencies, and improvements.

## Usage

```
/review-specs [--role <NAME>]   # single role (default: all)
/review-specs --quick           # summary table only
```

## What It Does

1. Reads all specs in `docs/specs/`
2. Reads all role definitions in `.roles/`
3. For each role, applies its `lens.verify` questions to the specs
4. Reports: findings, severity (BLOCKER / WARNING / NOTE), and suggested fix
5. Produces a reconciliation table: which specs reference which plans

## Role Tiebreaker (review order)

1. TAPE   — data accuracy first (wrong data = wrong everything)
2. FORGE  — architecture must hold before we build on it
3. PACE   — methodology assumptions must be explicit
4. EDGE   — failure modes must be catalogued
5. BENCH  — testability requirements must be stated
6. SCOUT  — hockey domain sense check
7. GLASS  — UX and output clarity
8. WIRE   — API/pipeline reliability

## Severity Levels

- **BLOCKER** — spec is inconsistent, contradictory, or missing a critical invariant.
  Cannot proceed to implementation without resolving.
- **WARNING** — spec is incomplete or ambiguous in a way that will cause problems.
  Should resolve before implementation.
- **NOTE** — observation or suggestion. Does not block.

## Instructions for Claude

When this skill is invoked:

1. Read every file in `docs/specs/` and `docs/plans/`
2. Read every role file in `.roles/`
3. Check: do the plans reference all the specs? Flag any spec with no plan coverage.
4. For each role, apply its verify questions systematically:
   - Quote the specific spec text that triggers each finding
   - State whether it is a BLOCKER / WARNING / NOTE
   - Suggest the specific fix
5. Check cross-spec consistency:
   - Does `rust-cli.md` data model match `player-analysis.md` data model?
   - Does `dashboard-engine.md` query model match `data-sources.md` data tiers?
   - Does `player-analysis.md` Phase column match `rust-cli.md` command list?
6. Produce a final summary: total findings by severity, top 3 blockers to fix first.
7. End with: "Review complete. N blockers, M warnings, K notes."
