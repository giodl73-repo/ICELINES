# Shooting/Tracking shot encounter — 2026-11-15

Episode/draft/result 0.5 adds `resolve_shot` for an existing in-flight attempt.
Verified skater Shooting opposes verified goalie Tracking; conditional authored
finish and containment distributions stay separate. The shared goal reducer
preserves existing goal/penalty semantics. Other goalie attributes are unmapped.

63 event tests pass (51 existing plus 12 shot tests). Checks exhaust all 400 pairs
for baseline/Shooting16/Tracking16; replay 500 seeds covering all six terminal
branches; compare 200 matched upgrade seeds; confirm unmapped Rebounds has no
effect across 50 seeds; reject malformed unreached distributions, wrong version,
profile branch, live-attempt and origin/goalie identity. Additional checks cover
post-to-goal in one attempt, original-shooter identity through a tip, rebound/new
attempt links, pending/serving minor termination and goalie touch versus hold
during a delayed call.

Existing possession (12), contest (six) and character (eight) checks pass:
89 distinct focused checks. Run `cargo test -p icelines-core --lib rinkcraft_events
--locked` and the neighboring `rinkcraft_possession`, `rinkcraft_contest`,
`rinkcraft_game` filters. Build the existing JSON evaluator with
`cargo build -p icelines-core --example rinkcraft_events --locked`.

Six neutral authored drafts/instructions:
`icelines-core/examples/fixtures/rinkcraft-shot-contest/`. Exact branch
probabilities sum to one. Fixed conditional finish 50/25/25 and hold/rebound
40/60 are game parameters, not NHL frequencies. More Tracking can create more
rebound events through greater contact opportunity with unchanged conditional
containment. No official save/SOG/scorer/assist credit follows from raw contacts.

Formatter/whitespace checks pass. Shot geometry, encounter admission, remaining
goalie tasks, conditional-rate calibration and full-game policies remain open.
No app/WASM acceptance, hosted build, publication, merge or push is claimed.
