# Goalie Rebounds containment — 2026-11-15

Episode/draft/result 0.6 requires exclusive rebound context after a Tracking
contact; old 0.5 weighted containment is unchanged. Hold-eligible checks use
verified goalie Rebounds, one d20 and the existing score term. Unknown stops
only when reached. Holds whistle; rebounds stay loose. No added live-time
interval, phantom opposing player, possession award or official save credit.

72 event checks pass (63 prior plus nine new containment tests). New checks
exhaust 20 dice for scores 10/16, equality holds and direct-rule scaling;
replay 300 encounter seeds with goal reconciliation; compare 200 matched upgrade
seeds preserving Tracking and initial goals; verify contact opportunity versus
conditional terms; check Recovery invariance over 30 seeds; exercise unknown
reached/unreached contexts over 100 seeds; reject mixed/missing policies and bad
targets/bases; and preserve delayed-call behavior for holds versus loose rebounds.

Existing possession (12), contest (six) and character (eight) tests pass:
98 distinct focused checks. Run `cargo test -p icelines-core --lib rinkcraft_events
--locked`, plus neighboring filters `rinkcraft_possession`, `rinkcraft_contest`,
`rinkcraft_game`. Build the JSON evaluator with
`cargo build -p icelines-core --example rinkcraft_events --locked`.

Seven neutral drafts/instructions:
`icelines-core/examples/fixtures/rinkcraft-goalie-containment/`. Target13 bridges
the older authored 40% neutral holds; score16 yields55%. This is not NHL target
calibration. The first baseline-contact illustration seed >=20261115 is20261117;
selection is explicitly conditional on contact, before considering containment.
Rejected candidate results belong in the dated PUCK custody packet. Exact whole-
encounter probabilities separately include Tracking contact exposure.

Formatter/whitespace checks pass. Geometry/eligibility detection, target
calibration, safe/dangerous redirection, remaining goalie tasks and full-game
policies remain open. No app/WASM acceptance, hosted build, publication, merge
or push is claimed.
