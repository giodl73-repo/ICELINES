# Bounded event episode validation — 2026-11-14

New Rust module `view_model::rinkcraft_events`; request/result 0.1; example
`rinkcraft_events`; eight authored neutral fixtures under
`icelines-core/examples/fixtures/rinkcraft-events/`.

Local checks: 20 event tests, 12 existing possession tests, six contest tests and
eight game-character tests pass (46 distinct tests). The event suite includes
100-seed exact replay/one-controller checks; post/tip/goal identity; rebound/new
attempt links; hold/whistle; delayed call and service; PP versus short-handed
goals; pending-minor wash; exact in-step penalty expiry; inactive/unselected
candidate rejection; clocks, scoring and unknown-law stops.

One newly added test initially used the already penalized actor for a second
foul. The engine correctly rejected that actor before the intended lifecycle
check. Corrected the fixture to use an active teammate; the limit test now
exercises the intended one-minor guard. No production rule was relaxed.

Reproduce with `cargo test -p icelines-core --lib rinkcraft_events --locked`;
neighboring test filters are `rinkcraft_possession`, `rinkcraft_contest` and
`rinkcraft_game`. `cargo build -p icelines-core --example rinkcraft_events --locked`
builds the JSON evaluator. `rustfmt` was applied to the three new Rust files;
`git diff --check` passes.

Fixtures are branch/state tests, not hockey frequency estimates. Contact weights,
reachability, timing, ordinary-minor adjudication and faceoff winners are authored.
No capability mapping, full-match controller, NHL empirical ratings or official
SOG/save/scorer attribution is introduced. No existing tournament, possession,
character or contest implementation changed. No native/browser app rebuild,
WASM acceptance, hosted build or publication is claimed.
