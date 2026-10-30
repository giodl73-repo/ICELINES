# Pulse 16 — Opt-in polling and cross-tab live acquisition

Status: implemented with deterministic and local browser evidence; release gates remain open.

## Contract

Automatic checks default off and last only in the current tab. Season statistics
use the native 24-hour freshness interval; scores/schedule use a provisional
60-second interval. Manual refresh remains available. Enabling checks waits a
complete interval, with 0–10% positive jitter. Each subsequent interval starts
after completion, including failures. Busy operations defer; there are no
overlapping polls or catch-up bursts. Dataset/date changes and page exit stop
automatic checks. Hidden/offline transitions cancel automatic acquisition and
pause checks; resuming waits another complete interval. Polling does not enable
the independent saved-copy policy.

A deployment-scoped Web Lock serializes live acquisition across tabs. The
120-second deadline includes lock waiting; cancellation removes queued reads.
Within acquisition, the existing two-request bound remains. No response bytes,
private queries or persistence preferences are shared through coordination.
The lock covers acquisition, not later Rust projection or saved transactions;
those retain generation/revision guards. This serializes requests rather than
deduplicating them. Without Web Locks, manual reads serialize within one page
and automatic checks are unavailable.

## Evidence

`npm test` passed all **95 tests**, including five polling and five coordination
cases. Coverage includes initial delay/jitter, context reset, hidden/offline
cancellation, busy/slow reads, failure cadence, unavailable coordination,
queued cancellation, lock-wait deadlines and release despite an unresponsive
adapter. No live upstream requests occur in these tests.

`node scripts/verify-browser-dist.mjs` verified **19 shell assets**, four
hand-audited count sorts and **75 public packages** using actual WASM loading
and queries. Largest shell plus one-package gzip sum: **395,663 bytes**.

Actual browser evidence used `http://127.0.0.1:8056/`, production UI/worker/WASM,
real IndexedDB and native Web Locks. The unpublished harness supplies fixed
schedule responses, accelerates the minute timer to one second, and synthesizes
visibility/offline predicates. It does not establish live access or native
visibility/offline browser behavior.

- Saved schedule restoration issued zero schedule requests. Enabling polling
  displayed its waiting state; subsequent fixture refreshes ran and respected
  the already enabled refresh-save policy.
- After five requests, simulated hidden then offline states retained count five
  across observations and displayed the paused state.
- Changing the date stopped polling and unchecked the control, preserving the
  prior loaded schedule. Neither date change implicitly fetched another week.
- Tab A held its sixth request. Tab B displayed loading but its request counter
  remained zero. Releasing A allowed B's first request to run.
- A committed revision `27323663edeef7c2228fcb53139e080cb3d9af91c1e03d790a1404b77a09d982`.
  B's save detected the prior-revision conflict, retained fresh data in memory,
  and reported that the earlier saved copy remained intact. Acquisition
  coordination correctly does not bypass transactional save conflicts.

[Polling pause screenshot](browser-pulse-16.png).
[Cross-tab save conflict screenshot](browser-pulse-16-conflict.png).
Both tabs ended with automatic polling disabled and no held request.

## Remaining acceptance

Deployed live source access or an allocated relay, real visibility/offline
transitions, offline reopen, browser file interchange round-trip, populated
legacy migration, mobile/resource acceptance and Pages preview/deploy/rollback
remain required. Cross-tab acquisition and the displayed save conflict now have
local browser evidence; that does not close every multi-tab lifecycle gate.
