# Two-stage defended passing — 2026-11-14

Explicit draft/episode/result 0.3 adds `pass_under_pressure`. Existing profile
verification and contest rules remain shared. Delivery is Passing versus Reading;
only a strict win opens a separately timed Handling-versus-Defending receipt.
Ties/denials leave the puck loose; no automatic defender control or touch.

41 event tests pass (31 existing, ten new passing tests). Passing checks exhaust
400 pairs per task, verify correct capabilities and boosted counts, replay 400
episode seeds covering all five terminal branches, and compare 200 matched seeds
each for receiver and passer upgrades. Checks also cover unused Skating,
schema/profile/actor/duration/side/goalie guards, serving defenders, a receipt
period-boundary stop and exact live penalty expiry during receipt.

Existing possession (12), contest (six) and game-character (eight) checks pass:
67 distinct focused tests. Run `cargo test -p icelines-core --lib rinkcraft_events
--locked`, plus neighboring test filters `rinkcraft_possession`,
`rinkcraft_contest`, `rinkcraft_game`. Build the existing JSON evaluator with
`cargo build -p icelines-core --example rinkcraft_events --locked`.

Neutral drafts/instructions live in
`icelines-core/examples/fixtures/rinkcraft-events-passing/`. They isolate passer
Passing, receiver Handling, lane Reading and receipt Defending at 10/16 with a
fixed seed/context. Half-offset exact joint chances are 22.5625%, 29.33125%,
29.33125%, 16.15%, 16.15%. These are game mathematics for two contested stages,
not observed NHL percentages. Context admission/calibration is still required
before generalizing to routine passes or full games.

Formatter/whitespace checks pass. No app/WASM acceptance, hosted build, publication,
push or merge is claimed. Shot/goalie capability mappings and autonomous policies
remain separate work.
