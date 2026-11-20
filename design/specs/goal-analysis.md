# IceLines Analyze Goal — first native feature contract

Goal: point to one NHL Goal Visualizer link or game/event pair and inspect the
complete published sequence with source-aware Rinkcraft questions for both teams.
This belongs in IceLines, not a PUCK-only script or separate research application.

## Surfaces and ownership

Shared Rust service: `icelines-fetch::goal_analysis`; shared descriptive audit:
`icelines-fetch::goal_sequence_audit`. The existing offline example delegates to
the same audit. Native UI: `/goal-analysis`, game-page picker and global nav.
JSON: `/api/v1/goal-analysis?target=GAME/EVENT`. CLI: `icelines analyze-goal` exports
portable HTML and structured JSON through the same service/rendering surface.
Dated team studies remain in PUCK; evidence rules remain in Rinkcraft.

## Evidence and behavior

Accept official HTTPS Goal Visualizer or sprite URLs, or numeric game/event pair.
Reject unrelated/ambiguous URLs and mismatched source identities. Reuse existing
NHL discovery, typed frame parsing, source URL allowlist and FLETCH replay fetch.
Read only the chosen goal's tracking; game metadata may contain the whole game.
Missing tracking returns an explicit unavailable state, never a fabricated sequence.

Preserve raw-native positions, timestamps and source hashes. Show official goal/
assists and keeper context, frame-level nearest-opposing-skater separation, grouped
nearest-point runs, and targeted questions for empty-net, short-handed or redirect
contexts. These are observations and prompts, not automatic passes, possession,
active-ice membership, called assignment, failure grades or player attributes.
Body/stick/height/vision and scoring-frame alignment remain unverified. Native
Euclidean separation is not certified physical reach. Render bounds from observed
coordinates rather than a presumed physical rink or hardcoded coordinate scale.

Cache complete captures per game/event, serialize concurrent requests through
existing fetch locks, write atoms and expose source-capture hashes. Respect live-
feed policy: offline/cached requests do not silently fetch. Unit availability and
source correction/history are not inferred from missing observations or old caches.

## Reviewable acceptance

- Goal URL and ID forms converge; unsafe/ambiguous selectors fail.
- Single-goal selection, scorer/game/URL/count consistency, keeper unknown versus
  recorded empty-net, and missing scorer observations retain their correct states.
- Offline capture imports, cached rereads and concurrent requests are coherent.
- Shared report JSON cannot close its script element; labels use text content and
  external source links are restricted to official HTTPS hosts.
- Existing four-goal audit remains byte-identical after extraction.
- CLI, native page, JSON twin, game picker and exported report work against the
  pilot; one different historical goal is fetched through the live feature.
- Full sequence playback/end frame, desktop/mobile geometry and zero page errors.

No full-app/WASM acceptance, automatic tactical causality, calibrated NHL rates or
observed duty/blame system is implied. Browser/WASM clients can consume the JSON
boundary; a standalone WASM source-fetch/analysis surface is not claimed here.

## Same-player geometry comparisons

The additive `geometry_changes` object uses schema
`icelines.goal-geometry-change.v0.1`. Each frame compares against ten frames
earlier, shortened at the start. Frame zero has no earlier comparison. Rust
computes puck/scorer endpoint displacement and matches player IDs across both
endpoints, retaining each player's before/after separation from the scorer.
Nearest-opponent identity changes are explicit; subtracting two different
opponents' distances must not be presented as one defender's movement.

Missing scorer observations produce null scorer movement/separation changes.
Endpoint presence changes are listed, and matched players indicate whether they
were observed in every intervening frame. Neither establishes active ice status
or a substitution. Distances are native Euclidean endpoint distances, not path
length, speed, physical reach, possession, coverage or causation. There is no
automatic open-lane verdict. Post-goal movement can occur anywhere in this
unaligned clip. The native and exported surface display the shared Rust values
and optionally connect matching endpoint observations on the plot.

## Fixed attacking-goal reference

The additive `goal_geometry` object uses shared Rust schema
`icelines.goal-relative-geometry.v0.1`. Official-renderer dimensions and regulation
rink geometry support a fixed goal-line reference and derived feet. Recorded
home defending side and scoring-team identity select the attacking goal;
unknown orientation remains unavailable. A goalie's changing position never
defines the net location. Preserve the original raw audit and its units.

Show puck distance, goal-relative bearing and front/behind-goal-line state.
The fixed-reference dashed line is a distance guide, not a shot trajectory.
Exact contact/release/goal frames and independently calibrated physical accuracy
remain separate tasks. Do not call this bearing a release-aligned shot angle
or infer goal probability. The shared study report can use the same geometry
to inspect candidate credited-touch intervals.
