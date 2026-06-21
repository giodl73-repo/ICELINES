# Phase Flyers Career Pulse 03 - Matrix Wording

**Date:** 2026-06-21
**Result:** Passed

## Work Completed

- Tightened `GET /career` wording around read-only cohort query shape,
  `CareerView` projection, shared shell rendering, `top` capping, bad-request
  guidance, missing-store fetch instruction, and no-live-fetch/no-store-create
  non-claims.
- Tightened `GET /api/v1/career` wording around JSON twin query validation,
  `CareerView` projection, data/meta success envelopes, shared bad-request
  envelopes, missing-store fetch instruction, and non-claims.

## Validation

- `git diff --check`

## Outcome

The Career cohort route rows now carry scoped local-store wording.
