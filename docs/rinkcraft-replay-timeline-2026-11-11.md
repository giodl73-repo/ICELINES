# Rust replay inspection and branches

The possession lab now offers moment selection, previous/next navigation, read-only
history review, prefix branches, separate branch provenance and immediate-source
restore. Rust owns reconstruction, validation and content fingerprints. JS selects
and renders snapshots; it never computes earlier game state.

Core APIs: `inspect_replay`, `inspect_replay_from_json`, `branch_replay`, and
`branch_replay_from_json`. Each verifies the source v1 replay first. Inspection
replays the selected prefix, including stations/participants. Branching truncates
the request, preserves seed/hero/rules/prior actions and rejects terminal or
out-of-range moments. Support may change only at step zero. Saved v1 session
fields, gameplay strings and rules are unchanged.

Native POST routes `/api/v1/rinkcraft/possession/inspect` (`{replay,step}`) and
`/api/v1/rinkcraft/possession/branch` (`{replay,fork}`) delegate to the same Rust
functions as WASM. The WASM boundary checks finite integral steps rather than
coercing arbitrary JavaScript numbers into unsigned indices. Fork requests deny
unknown fields, including attempts to replace the seed.

Branch result schema `rinkcraft.possession-branch.v1` contains a content-based
parent fingerprint, fork step, discarded actions, support-change flag and initial
child replay. It is a provenance sidecar, not a replay-import root. The usual
replay download saves the current continuation; export while reviewing saves the
full live sequence. The UI retains one immediate parent, not a durable branch tree.
Content fingerprints are not author signatures or historical NHL evidence.

Rust CLI:

```powershell
cargo run -p icelines-core --example rinkcraft_replay_inspect --locked -- replay.json 1
cargo run -p icelines-core --example rinkcraft_replay_inspect --locked -- replay.json 0 commit
cargo run -p icelines-core --example rinkcraft_replay_inspect --locked -- replay.json 0 commit --replay
```

The last command emits the ordinary child replay instead of the branch record.
All modes only read the input. Retroactive support changes fail with nonzero exit.

Validation: twelve possession/replay/timeline tests, eight creation and six
arithmetic regressions; ten-template native/WASM inspection/branch parity;
seventeen selected prefix snapshots; source fingerprint stability under key
reordering; invalid numeric indices/retroactive support/terminal forks; actual
read-only UI and unchanged exports; prefix continuation with identical next dice;
source restoration and branch records; keyboard focus/navigation, 390-pixel layout,
offline browser operation and one-page character printing. Existing import,
possession and creator E2E plus 26 focused Node tests pass. CLI inspection/branch
parity, importable child output and invalid-plan exit were checked. Screenshots
were inspected. Artifacts: ignored `target/replay-timeline/`.

Correction found during verification: seed 7 completes via eight actions with
recoveries, rather than the previously simplified clean three-action description.
The clean Conductor/standard-array path is seed 45. The engine and previous
`completeSeed:7` result remain valid. A neutral-fixture test was corrected to
exercise its legal continuation instead of assuming the showcase's action path.

Replay using existing owned native/browser servers and supplied operator assets:

```powershell
node icelines-browser/e2e/rinkcraft-replay-timeline.mjs
node icelines-browser/e2e/rinkcraft-replay-import.mjs
node icelines-browser/e2e/rinkcraft-possession.mjs
node icelines-browser/e2e/rinkcraft-game-studio.mjs
```

No new tactical actions, shots, goals, fatigue, clock, NHL abilities or game-balance
claim. Rinkcraft retains the governed design and executed checkpoint. Local only.
