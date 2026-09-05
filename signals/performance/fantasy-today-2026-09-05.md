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

## League-aware v2 follow-up

The in-process `fantasy_today.v2` implementation was measured on the same
private local league only as runtime evidence; no roster or player names are
recorded here. The dated command used a full remaining fantasy week so the
bounded transaction path executed rather than returning no candidate.

Release-mode ten-process series (milliseconds):

`6143.3, 5540.2, 372.7, 368.4, 377.5, 388.9, 386.3, 363.7, 370.2, 370.3`

The first two processes were cold filesystem/OS-cache samples. For the eight
warm samples:

- warm release p50: **370.3 ms**
- warm release p95 (nearest rank): **388.9 ms**
- bounded candidates considered: **12**
- candidate evaluation elapsed: **3 ms** release (**20 ms** dev)
- disclosed population truncation: **yes**
- supported candidate ceiling: **12 candidates / 250 ms**

The v2 warm p95 remains well inside the two-second interaction gate. Relative
to the earlier v1 warm p95 of 239.7 ms, the measured in-process league-aware
assembly adds about 149 ms at p95; this regression is documented and accepted
for saved matchup, current-roster, legality, quiet-night, and transaction
composition. The candidate evaluator itself is not the dominant cost.

The fantasy database SHA-256 was checked immediately before and after a
successful v2 assembly and was byte-identical. The `icelines.db-wal` and
`icelines.db-shm` inventories were empty before and after. Source inspection
also found zero `std::process::Command` or `OnceLock` adapters in the TUI and
Web Fantasy Today paths.
