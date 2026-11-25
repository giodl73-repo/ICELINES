# Deployment from admitted snapshots — 2026-11-24

Sparse Task Skills do not automatically justify more feats. Alex Killorn's
selected-season snapshot earns penalty drawing but does not cross the existing
production thresholds. It also contains substantial shorthanded workload that
the preview's missing deployment object concealed. Workload belongs in Role
Coverage and Deployment, independently of task qualification.

## Shared recovery

`rinkcraft_deployment::from_stat_profile` reuses the existing deployment model:

- Total seconds = all-strength minutes × 60.
- PP seconds = full-precision PP share × total seconds.
- SH seconds = full-precision SH seconds per appearance × GP.
- EV seconds = total − PP − SH.

All three observed inputs must exist. Every recovered count must be within
1e-6 seconds of a nonnegative whole second and fit in u32. Overflow, impossible
strength sums, incomplete inputs, zero GP and goalie profiles produce no
recovered deployment. Zero observed PP/SH time remains zero. The maximum
fractional-second error among the 787 current snapshots was 3.64e-12.

Existing deployment formatting supplies per-appearance time and shares of the
player's own TOI. It rounds time to the nearest second. EV includes more than
5v5; OT must not be added again. This is workload, not efficiency, team-opportunity
share, line assignment, PP unit, station, isolated PK quality or a capability.
The legacy generic attribute-context SH row truncates fractional seconds, so it
can differ by one displayed second from the rounded Deployment field.

The catalog attachment already validates actor/season/group/GP. It now recovers
deployment for the selected admitted window when a matching deployment is absent;
matching direct-source deployment remains preferred. Native and browser rendering
also reject mismatched GP. The WASM attachment response includes deployment and
the normal browser load path applies it; no player-specific runtime patch exists.
Original source hashes are not fabricated for reconstructed records.

## Frozen local result

Input: `target/shooting-card-study/enriched/rinkcraft-stat-attributes.json`.
SHA-256: `9fc4ee1625e407c88592bffe80c11bc0b6cc566105d282d4ae080cb86a574e77`.
Existing 2026–27 snapshot, NHL fetched 2026-11-23; no new source fetch.

| Player | GP | PP / appearance | SH / appearance |
|---|---:|---:|---:|
| Alex Killorn | 24 | 0:50 | 2:30 |
| Nathan MacKinnon | 20 | 4:11 | 0:05 |
| Adam Fox | 23 | 3:40 | 1:25 |
| Sebastian Aho | 24 | 3:48 | 1:39 |

All 787 skater profiles receive coherent workload. Projection checks preserve
each original achievement object and every integer attribute grade across all
retained seasons. No new feat, class, assignment or simulation bonus is awarded.

## Validation

- Native CLI/Askama and WASM builds passed.
- 17 focused Rust tests passed: exact recovery, zero/missing distinction,
  fractional-second rejection, impossible sums, selected-window attachment,
  direct-source preservation, existing input contracts and task selection.
- Browser suite passed 246 tests, including stale-GP workload rejection.
- Four live cards passed Role Coverage summary, four workload rows, actual worker
  recovery from a missing deployment object, task-repertoire retention, mobile
  overflow and no page errors.
- Four single-page Letter PDFs independently checked with pypdf for PP/SH summary
  and every selected task. Print spacing was tightened around workload and record
  rows. Desktop screenshot inspected.

Retained ignored projection helper: `target/deployment-card-study/project.mjs`.
Acceptance: `node icelines-browser/e2e/rinkcraft-deployment-snapshot.mjs` against
the existing loopback preview. It does not start a server.

Preview: <http://127.0.0.1:8780/?player=8473986>.
