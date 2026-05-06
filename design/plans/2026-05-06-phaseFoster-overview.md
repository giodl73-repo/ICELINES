# Phase Foster — Implementation orchestrator

**Specs**: `foster-overview.md`, `foster-data-architecture.md`,
`foster-favorites-dashboard.md`, `foster-time-and-timeframes.md`
**Status**: Plan — orchestrator
**Date**: 2026-05-06

---

## Sub-phase ordering

```
Foster.0 (data architecture) ─────────────┐
                                            │
Foster.1 (time axis) ←─ depends on F.0 ─────┤
                                            │
Foster.2 (favorites view)  ←─ F.0 + F.1 ────┤
                                            │
Foster.3 (boxscores + EventStream) ←─ F.2 ──┤
                                            │
Foster.4 (sync engine) ←─ F.0 + F.3 ────────┤
                                            │
Foster.5 (timeframes) ←─ F.1 + F.2 ─────────┤
                                            │
Foster.6 (setup wizard polish + docs + persona pass) ←─ all
```

Critical path: **F.0 → F.1 → F.2 → F.3**. F.4 / F.5 land in parallel
once F.3 lands. F.6 closes.

## Per-sub-phase plans

| Sub-phase | Plan | Test budget |
|---|---|---|
| Foster.0 | `2026-05-06-phaseFoster-data.md` | 35 + 24 capability matrix |
| Foster.1 | `2026-05-06-phaseFoster-time.md` (shared with F.5) | 12 |
| Foster.2 | `2026-05-06-phaseFoster-favorites.md` (shared with F.3) | 21 + 6 personas |
| Foster.3 | `2026-05-06-phaseFoster-favorites.md` | 12 |
| Foster.4 | `2026-05-06-phaseFoster-data.md` (shared with F.0 — sync engine) | 15 |
| Foster.5 | `2026-05-06-phaseFoster-time.md` | 9 |
| Foster.6 | `2026-05-06-phaseFoster-closeout.md` | 4 unit + 30 personas |
| **Total** | — | **~168 tests** |

## Pre-flight (before Foster.0 starts)

- [x] NHL API probe (`/v1/score/{date}` works arbitrarily back) — done 2026-05-06
- [x] Two rounds of 8-role review complete; 19 blockers captured + addressed in revised specs
- [x] Groups+teams committed (`ab8903bf`) — Foster.0 collapses `MemberKind` into `EntityRef` via migration 006
- [x] User sign-off on locked decisions (bundle stays 38, eager non-blocking, capability matrix, shifts deferred)
- [ ] Final read-through of all four specs by user
- [ ] Foster.0 spawn

## Top-level risks

| Risk | Severity | Mitigation |
|---|---|---|
| `shifts` capability surfaces misleading data | High → Eliminated | Locked: `shifts="off"` is the only valid mode until real shift parsing ships in a separate phase |
| Eager auto-refresh blocks alt-screen | High → Eliminated | Locked: non-blocking via tokio::spawn + one-shot channel |
| Manifest O(n) at 50k entries | Med → Eliminated | Locked: sharded by kind, HashMap-indexed in OnceLock |
| L3 golden tests break under auto-refresh | Med → Eliminated | Locked: `MockClock` injection + `ICELINES_TEST_MODE=1` |
| Timeframe × `--filter` ambiguity | High → Eliminated | Locked: namespaced grammar (`g.week>=10`); bare aliases bind to active timeframe |
| Snapshot read-shim mutating user data | High → Eliminated | Locked: `~/.icelines/snapshots/` is **immutable read-only input**; manifest rebuilt on open |

## Rollback strategy per sub-phase

Foster.0 is the only sub-phase that touches existing data on disk
(via the snapshot read-shim). Recovery: delete `~/.icelines/data/manifest/`
— DataStore rebuilds from bundle + snapshots dir on next open.

Other sub-phases are additive (new files, new routes, new tabs);
rollback = revert the commit.

## Roles checkpoint cadence

- **Foster.0** — checkpoint before .0 ships: TAPE + FORGE + BENCH on
  the data layer artifacts (manifest format, EntityRef, Freshness).
- **Foster.2** — checkpoint: SCOUT (favorites correctness) + GLASS
  (Favorites tab UX).
- **Foster.4** — checkpoint: PACE (background refresh latency
  measurements against the existing retry policy).
- **Foster.5** — checkpoint: EDGE (timeframe × filter grammar
  acceptance).
- **Foster.6** — final closeout: 8-role pass on the full phase
  ("did we forget anything").

## Out-of-plan items (deferred to post-Foster)

- Real per-shift parsing (Phase Maurice Richard candidate)
- Pre-2014 historical scores (depends on API probe extension)
- League-wide boxscore bulk-fetch (`boxscores=league` capability)
- Multi-group selector for Favorites surface (Foster.6 polish item)
- Multi-user / cloud sync (explicit IceLines.md non-goal)
- Live websocket / push notifications (explicit non-goal)
