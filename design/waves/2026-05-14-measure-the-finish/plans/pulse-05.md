# Pulse 05 - Surface Parity and Wave Closeout

## Goal

Review the Measure the Finish surfaces, close any parity/documentation gaps, and
close the active wave if the player scoring trend and streak outputs are
discoverable across the intended surfaces.

## Governing roles

- **scout**: copy must stay descriptive: recent volume, conversion, inside
  looks, shot pressure, and streaks. Do not introduce betting or projection
  claims.
- **edge**: missing play-by-play, loaded zero-event games, missing coordinates,
  and missing player IDs must remain visible in JSON/source-state fields.
- **bench**: add only parity/regression tests for surfaces touched in this wave;
  do not expand scope into unrelated route redesigns.
- **wire**: all cache warming must remain CLI/Admin POST/fetch backed. No GET
  route may fetch live game data.

## Owned scope

1. Verify web/API, CLI, and TUI player streak/profile surfaces expose the new
   ViewModel fields where already wired.
2. Update user-facing docs for any changed output labels or cache requirements.
3. Add or amend focused parity tests for JSON field presence and no-live-fetch
   behavior.
4. Update `WAVE.md` / `PHASES.md` and close the wave if all Pulse 02-04 outputs
   are shipped and CI is green.

## Non-goals

- No new scoring model or projection math.
- No third-party xG parity claims.
- No new unrelated web/TUI redesign.

## Gates

- [x] `cargo fmt --check`
- [x] `cargo test -p icelines-core --quiet`
- [x] `cargo test -p icelines-fetch --quiet`
- [x] `cargo test -p icelines-web --quiet`
- [x] `cargo clippy -- -D warnings`
- [ ] `C:\src\proof\target\debug\proof.exe check . --errors-only`

## Gate Notes

- `proof check . --errors-only` remains unchecked because the repo-wide proof
  baseline fails on pre-existing ASCII-box diagnostics in `SPEC.md`,
  `design/ARCHITECTURE.md`, `design/IceLines.md`, and older plans. The changed
  wave docs pass targeted proof checks.
