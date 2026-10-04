# Browser WASM pulse 14 — shared scores and schedule slice

Date: 2026-10-03. Parent: WP-BW-04 / WP-BW-05 / REQ-BROWSER-001 / CHG-119.

## Implemented boundary

The existing source-owned `ScheduledGame` and `parse_game` remain the hockey
projection. New `parse_game_week` validates the whole game-week envelope before
returning rows: dated game arrays, seven-day/128-game bounds, valid starts,
duplicate identity/date rejection, consistent game dates, teams and game type,
and scores that fit the model without integer wrapping. Missing scores remain
`None`; explicit zero remains zero. Series context uses the existing parser.
Unknown source state strings remain visible rather than inferred as final.

Native `fetch_schedule_url` now delegates to that complete-week validator,
returning `SchemaChanged` for malformed/incomplete envelopes instead of quietly
producing empty or partial success. Club-season parsing remains separately scoped.
WASM exposes a stateless 2 MiB-limited schedule projection; worker operation
`schedule` does not load or replace season stats.

Browser acquisition uses the fixed NHL `/v1/schedule/YYYY-MM-DD` endpoint and
existing bounded HTTP reader (timeouts, safe retries, deadline, no credentials,
no cache, redirect refusal). Rust validates the returned game-week payload.
The schedule controller stages bytes, digest and projection before replacing
its last good snapshot. Cancellation/date changes discard late acquisition or
projection results. Failed refreshes retain previous data with a stale warning.

The UI provides an independently labelled date, refresh/cancel, score table,
NHL state, UTC start, series label, requested context, fetched timestamp and
revision. Source observation time stays unknown. This slice is session-only;
durable schedule snapshots, portable backups and opt-in polling are still open.

## Verification

`node scripts/build-browser.mjs --offline --check` passed:

- Five source schedule tests, including a hand-audited shared JSON fixture.
- 88 core catalog tests, two portable-data tests and eight WASM engine tests.
- 42 native loader tests and two native/browser parity suites (90 combinations).
- Six native web leaders tests, seven Python catalog tests, 74 Node browser tests.
- Distribution verification: 17 shell assets, four count-order goldens,
  all 75 packages through real WASM, and distributed schedule projection.

Six native `nhl_api::parse_game_tests` passed separately. WASM/source clippy with
warnings denied, TypeScript check, Rust formatting and whitespace checks passed.
After refining the initial date-change message to avoid claiming nonexistent
retained data, `npm test` rebuilt the shell and passed all 75 browser tests.
Final distribution verification passed again; shell plus largest single package
measures 389,778 bytes gzip, below the provisional 10 MiB transfer bound.
Native library test compilation exposed a missing test-only `GoalieStats` import
from the loader extraction; restored that import without changing runtime logic.

The distributed schedule golden proves playoff series mapping, zero versus
missing scores, malformed/oversized refusal and that projection can run before
stats activation and after loading stats without replacing them.

## Browser evidence and open gates

`scripts/prepare-browser-schedule-preview.mjs` creates a fixture harness outside
publishable dist. At `http://127.0.0.1:8056/`, Codex in-app browser displayed
NYR–WSH final 3–0 and MTL–TBL future scores as unavailable. An invalid complete-week
response left both rows visible with a stale warning. Loaded 2024–25 regular
stats with `p>=100` (six players); valid and invalid schedule refreshes both
preserved those results. [Screenshot](browser-pulse-14.png).

This is deterministic fixture evidence, not browser CORS or deployed live proof.
The harness modifies the index, so its shell integrity failure is expected;
offline shell behavior is not assessed here. Live access/relay disposition,
schedule persistence/polling, offline reopening, full memory/mobile acceptance,
Pages publication and rollback gates remain open. No shipped claim is advanced.
