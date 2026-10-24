# Loaded player season history review

Seven review lenses applied by one agent. Scope: player-ID reads across the
already resident season/type windows, presented inside player detail. No full
career claim, bulk acquisition, persistence or workflow change.

- **HART — PASS:** Each history entry preserves season/type, exact revision and
  package provenance. Missing player records stay null and distinct from zero GP.
  Regular and playoff rows are separate and never summed into a career total.
- **FORGE — PASS:** Safe Rust only. Keyed repository views project owned small
  rows without cloning repositories or packages. Query and history share one
  projection helper. Both default and sparse storage pass 20 engine tests and
  strict all-target Clippy. Detail gains an additive history field; worker
  operations are unchanged. The UI handles older responses without that field.
- **KEEL — PASS:** History reads do not change active selection or LRU order.
  Existing eight-window/input budgets remain authoritative. Refresh, eviction and
  unload affect the next read. The dialog closes when its dataset/generation
  changes and releases its DOM when closed. No new storage or network ownership.
- **PACE — PASS:** Existing Rust pace/goalie fields and display precision are
  reused. No new formulas, era adjustment, complete-career or memory claims.
- **BENCH — PASS:** Default/sparse tests cover empty state, missing records,
  zero-game playoffs, source/revision preservation, query equality, read-only
  residency, refresh, unload and eviction. All 178 browser units pass. Chromium
  matches three real window projections to query exports, reads offline without
  package requests, preserves active selection and shows missing/goalie records.
  The initial all-empty browser fixture was rejected as invalid; corrected to a
  valid single-other-player fixture without relaxing engine validation. Initial
  failure and corrected reports remain in local target/history-browser-*.
  Native-loader/distributed-WASM parity passes all 270 cases (33,613 full rows,
  90 empty results), covering both season types, nine sorts and five floors.
- **EDGE — PASS:** Stale in-flight detail responses are ignored. Dataset changes
  close an already-open dialog. An old queued close event cannot erase a newly
  opened dialog. Missing dates remain unknown; source/names use textContent.
- **GLASS — PASS, desktop emulation:** Named history section and explicit list,
  readable per-window cards, regular/playoff labels and a partial-history note.
  All three themes checked at 390px; no horizontal dialog overflow. Physical
  Android/TalkBack acceptance remains deferred.

Release boundary: local preview only until separately published. The prior
250 MiB browser target is still unmet; this feature does not waive it.
