# Rebound follow-up and goalie Recovery — 2026-11-15

Episode/draft/result 0.7 adds one conditional attacking follow-up after an explicit
shot resolution. Only an actual source loose goalie rebound admits collection;
other known source outcomes skip without time/RNG. Handling/Defending secures
the puck; Recovery/Skating assesses readiness within that window. Reset 0/declared
late adjustment applies to one new linked shot contest; ties stop unresolved.

79 event checks pass (72 prior plus seven follow-up tests). Checks exhaust 400
readiness pairs for neutral/Recovery 16/Skating 16, replay 700 complete bounded
episodes covering skips, collection denials, unknowns, reset/late and actual
second-shot goals, compare 250 paired Recovery-upgrade seeds, verify all 400
adjusted Tracking contests, reject plan/version/identity/cost/side errors even
when skipped, verify Movement invariance over 30 seeds and stop period boundaries
before inventing a second release. Existing possession 12, contest 6 and character 8
tests pass:105 distinct focused checks.

Run `cargo test -p icelines-core --lib rinkcraft_events --locked` plus neighboring
filters `rinkcraft_possession`, `rinkcraft_contest`, `rinkcraft_game`. Build the
existing JSON evaluator with `cargo build -p icelines-core --example rinkcraft_events --locked`.
Neutral drafts/instructions: `icelines-core/examples/fixtures/rinkcraft-rebound-followup/`.

The older Reset prototype supplies the task pairing, not evidence for the new
late consequence. Trial −2 and sensitivity −4 are authored contest adjustments,
not Tracking grade changes or calibrated NHL effects. Unknown readiness mass
must not be treated as eventual no-goal. Access remains declared, not inferred
from Skating or spatial geometry. Conditional source-rebound illustrations use
first seed 20261119 from a 20261115 start, before examining follow-up outcomes;
earlier candidates are retained in the dated PUCK packet.

Formatter/whitespace pass. Physics, modifier calibration, other task maps and
autonomous/full-game policies remain open. No app/WASM acceptance, hosted build,
publication, push or merge is claimed.
