# Three-zone transition validation â€” 2026-11-15

Rust event/draft/result 0.9 in the owned compact-sheet branch. Adds bounded
retrieval/exit/neutral/regroup/entry execution using both teams' declared routes
and a fixed physical frame. Reuses spatial access/collection; old versions retain
optional-field omission and numerical behavior. No real-player grade changes.

## Checks and reproduction

Focused event suite plus possession, contest and character slices; existing Rust
CLI built with locked dependencies. Exact final totals and source revision are
recorded in the dated PUCK manifest and the completion record below.

```powershell
$env:TEMP='F:/src/ICELINES-worktrees/rinkcraft-sheet/target/windows-tmp'
$env:TMP=$env:TEMP
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-core --lib rinkcraft_events --locked
cargo build -p icelines-core --example rinkcraft_events --locked
```

Neutral fixtures: `icelines-core/examples/fixtures/rinkcraft-three-zone/`.
PUCK: `scenarios/nhl/2026-27/rinkcraft-three-zone-transition/inputs/run.ps1`,
then `verify.py`, `seal.py` and `seal.py --verify`. Source must be clean before
the recorded trial. Runner stores source/binary/input/output hashes and seed;
older frozen inputs/outputs remain untouched. Binary retained by commit on F:.

## Acceptance boundaries

- Fixed physical bands and both relative zone directions; reversal never flips coordinates.
- Controlled exit versus clearance; carry/pass entry versus dump and actual retrieval.
- Pass crossing in flight versus secured receipt; failed receipt remains loose.
- Dump lost: opponent retrieves in its defensive end, exits and attacks the other end.
- Clear lost: opponent starts neutral progression; no invented defensive exit.
- Backward regroup creates another exit; actual holder cannot pass to itself.
- Neutral entry contest uses Handling/Defending, exact 190/400; Handling16 247/400.
- Explicit unknown crossing, goalie access, period boundary, maximum time/transition
  limits and exact penalty expiry do not fabricate completion or possession.
- Existing 0.7 shot/rebound behavior composes with 0.9, and earlier artifacts are
  compared byte-for-byte. Goal ledger stays empty in these transport-only cases.

Cells, endpoint routes, future actor projections, times, choice policies, open
thresholds and task pairing are authored game hypotheses. Target1 fixtures are
guaranteed neutral-score branch tests, not estimated completion rates. Geography
exists within this macro, not a persistent player-tracking world. Offside/icing
law, source-defined team cover/lanes, loose-failure retrieval continuations,
goalie outlets, continuous matches, NHL calibration and full app/WASM acceptance
are outside this slice. Rinkcraft decision0061/review/packet preserve those limits.

## Local completion record

144 distinct focused checks pass: events118, possession12, contest6, characters8.
A test setup initially tried restarting before the delayed-call whistle; corrected
to actual offending control, whistle, restart and serving interval. The exact-expiry
case then passes without changing engine penalty semantics. One fixture include
path was also corrected before the successful build/test run.

Locked Rust CLI build also passes. Dated replay/legacy audit results are recorded in the PUCK manifest and findings linked above.
