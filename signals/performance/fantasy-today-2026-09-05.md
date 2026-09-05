# Fantasy Today performance and read-only evidence

**Date:** 2026-09-05

**Command:** `target/debug/icelines.exe fantasy today --json`

**Fixture:** local 16-team fantasy league, 16-player user roster, cached
2026-27 NHL schedule, sealed 2025-26 player-rate sample

**Machine/profile:** Windows development workstation, unoptimized `dev` profile

## Default-path boundary

- The command opens the existing fantasy SQLite database through the immutable
  read-only connection and does not run migrations or create sidecars.
- Schedule reads are cached-only. A missing cache returns the explicit
  `fantasy schedule-edge --refresh` recovery command; it never falls through to
  the 32-team NHL fetch loop.
- Weekly pickup and sleeper searches remain outside the default cockpit and are
  exposed as drill-down recovery actions.
- Quiet-night coverage is derived from the already-built daily lineup and the
  cached schedule, avoiding a third player-pool load.

## Measurements

Database SHA-256 was identical before and after each measurement series.
Contract output reported `fantasy_today.v1`, and quiet-night coverage was
present.

First seven-process series (milliseconds):

`8047.4, 6006.8, 2200.3, 722.9, 701.1, 673.9, 678.9`

Second warm ten-process series (milliseconds):

`2318.9, 680.9, 694.8, 673.6, 674.2, 673.9, 681.3, 680.9, 2175.4, 671.3`

- warm p50: **680.9 ms**
- observed nearest-rank p95: **2318.9 ms**
- steady-state cluster: **671-695 ms**

The steady-state and median meet the two-second interaction budget. Two
outliers on the unoptimized Windows binary left that debug-only p95 319 ms
above the target, so a release-mode series was also measured.

Release-mode 20-process series began with two cold OS/cache samples at 7655.8
ms and 3334.2 ms. The following 18 warm samples were all between 219.2 ms and
239.7 ms:

- warm release p50: **233.7 ms**
- warm release p95 (nearest rank): **239.7 ms**
- database SHA-256 unchanged: **yes**

The measured warm release p95 therefore clears the two-second default-path
gate with substantial headroom; cold process and filesystem initialization is
recorded separately rather than mislabeled as warm latency.

## Surface smoke evidence

A local no-live/no-cache server on loopback returned:

- `GET /api/v1/fantasy/today`: HTTP 200, schema `fantasy_today.v1`, typed
  `provisional` state, and a populated primary decision;
- `GET /fantasy/today`: HTTP 200 with the semantic `Fantasy Today` heading and
  no script dependency.

The server was then stopped cleanly. Hermetic route tests separately verify
that missing local state returns typed JSON and semantic HTML 503 degradation
without creating `~/.icelines`.
