# Browser WASM pulse 44 — additional remote test failures

Date: 2026-10-04. Previous turn made progress by pushing the locally accepted
toolchain/TLS fixes as `aff632e158be0b749e2cb47ee3bde98f15f39e6e` to draft PR #74.
Managed worktree was clean after that push. Fresh browser run 37213784874 and
repository matrix 37213784873 were authoritatively live; browser was in progress
at this turn's final inspection. No retained artifact or remote success claimed.

The original matrix exposed two additional completed failures. Fetch job
111468620452 passed all 585 library tests and browser-native parity but failed
the URL date test because its fixture supplied an empty `gameWeek`. Shared
schedule validation requires the requested date to be represented, even when
that date has no games. The fixture now supplies the requested day with an empty
games array, preserving its URL/empty-result purpose. All three tests in
`foster1_date_fetch` passed (session 27692 exited 0).

Native TUI job 111468620478 failed only
`official_live_roster_manifest_is_required_for_authority` (1458 other tests
passed). The CLI file is unchanged from base master. Its unproven snapshot uses
creation time, now after the test's fixed September 29 opening boundary, so it
returns unavailable before testing missing provenance. A test-only adjustment
uses the day after creation as the second case's boundary. Production authority
logic is unchanged. An initial attempted observation-time adjustment was
corrected because unproven sources deliberately ignore that observation time.

Terminal 40743 is compiling the targeted CLI test, log
`target/browser-pulse-44-authority.txt`. Source was corrected during dependency
compilation; finish this handle and then rerun the corrected targeted test so its
evidence cannot be confused with the initial attempt. Do not restart merely on
an observation timeout. The CLI edit exists only in the managed worktree and
must be mirrored by a narrow exact hunk, never by copying the entire native file.
Fetch fixture is also present in the original checkout. The two changes remain
uncommitted. Next action: verify the corrected CLI test, inspect remaining remote
failures, then commit/push these narrowly scoped test corrections to PR #74.

No merge, Pages settings change or deployment occurred. Full browser composition,
device/resources and deployed/live/offline/update/rollback acceptance stay open.
