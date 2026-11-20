# Capability-driven event collection — 2026-11-14

Explicit episode/result 0.2 adds `collect_under_pressure` and verified fictional
character bindings. Draft 0.2 builds characters through the existing game module;
profile exports are checked by rebuilding their rules/method/provenance. The
task uses skater Handling versus opposing Defending with the selected existing
d20 score rule and no contextual/class bonuses.

31 event tests pass: prior 20 plus 11 adapter tests. Three selected cases exhaust
all 400 ordered dice pairs; exact outcomes are 190/20/190, 247/17/136 and
136/17/247. A 100-seed probe checks replay and ownership; another 100 matched-seed
probe checks unchanged dice/prefixes and monotone capability changes. Tests also
cover unused-capability/class-label invariance, provenance tampering, schema
opt-in, missing/duplicate/unknown bindings, branch/side/puck eligibility,
penalized pressure actors, pending-call control, draft building and direct rules.

Existing possession (12), contest (six) and character (eight) tests pass: 57
distinct focused tests total. The Rust CLI example builds with `--locked`.
`rustfmt` applied to changed/new Rust files; whitespace checks pass.

Reproduce: `cargo test -p icelines-core --lib rinkcraft_events --locked`, then
neighboring filters `rinkcraft_possession`, `rinkcraft_contest`, `rinkcraft_game`.
Build: `cargo build -p icelines-core --example rinkcraft_events --locked`.
Neutral drafts and instructions are in
`icelines-core/examples/fixtures/rinkcraft-events-collection/`.

Only collection uses capability scores. Access/arrival, contact mixtures, timing,
shots, goalie contacts, fouls and faceoffs remain declared. Ties and pressure wins
leave a loose puck. Mathematical/game consistency does not establish NHL rates,
real-player ratings, spatial physics or full-game validity. No app/WASM build,
hosted build, push or merge is claimed.
