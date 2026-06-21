# Phase Predators Snapshots Pulse 03 - Matrix Wording

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Tightened admin snapshot JSON activate wording around sealed-only
  `SnapshotMutationIntent::activate`, active pointer updates, store guards, and
  `MutationResultView`.
- Tightened admin snapshot HTML activate wording around sealed inactive form
  controls, shared intent, `/admin` redirects, and creation/sealing non-claims.
- Tightened admin snapshot JSON delete wording around inactive-only
  `SnapshotMutationIntent::delete`, active snapshot rejection, and
  `MutationResultView`.
- Tightened admin snapshot HTML delete wording around inactive-row controls,
  shared intent, `/admin` redirects, and broad-maintenance non-claims.

## Validation

- `git diff --check`

## Outcome

The admin snapshot mutation route rows now carry scoped sealed/inactive wording.
