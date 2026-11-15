# Declared pass pressure — 2026-11-14

Episode/draft/result 0.4 adds an explicit lane/receipt context: open, defended or
unknown. Open checks use one d20 plus the existing score term against an authored
target; defended checks reuse the original opposed task. Unknown is never open
and stops only if reached. Profile contract stays 0.1 and all supplied characters
are verified. Shared `ScoreRule::term` preserves direct/half-offset arithmetic.

51 event checks pass (41 prior plus ten context tests). New checks exhaust all
20 open dice, target equality, a higher score, an explicit direct rule and all
400 defended pairs. They also check no phantom opponent/binding, 100-seed replay,
100 unknown-receipt reached/unreached seeds, 200-seed clocks/ownership coverage,
unknown lane without release, schema/target/basis/side guards and receipt period
boundaries. Existing possession (12), contest (six, including all-score exact
arithmetic) and character (eight) tests pass: 77 distinct focused checks.

Commands: `cargo test -p icelines-core --lib rinkcraft_events --locked`; existing
neighbor filters `rinkcraft_possession`, `rinkcraft_contest`, `rinkcraft_game`.
Build `cargo build -p icelines-core --example rinkcraft_events --locked`.
Neutral drafts and instructions:
`icelines-core/examples/fixtures/rinkcraft-pass-context/`.

Open target 5 under half-offset 10 gives 80% per open stage; this is an authored
parameter, not NHL calibration. Neutral joint control is 64%, 38%, 38%, 22.5625%
for the four pressure combinations. Two open-stage skill upgrades give 76%.
Unknowns receive no imputed probability. Targets are score-rule dependent;
one-die and two-die mechanisms do not share identical actor rolls just because
their seed is the same.

Formatter and whitespace checks pass. Context detection/admission frequencies,
target calibration, shot/goalie tasks and autonomous game policies remain open.
No app/WASM acceptance, hosted build, publication, merge or push is claimed.
