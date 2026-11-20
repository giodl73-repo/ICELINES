# Bounded event episode fixtures

These neutral, authored game inputs exercise `view_model::rinkcraft_events`.
Run `cargo run -p icelines-core --example rinkcraft_events --locked -- INPUT.json OUTPUT.json`.
No NHL player ratings or observed event frequencies are encoded.

Request `rinkcraft.event-episode.0.1` starts one period at a declared clock,
5v5 plus goalies, zero score and one controller. All players are active initially;
the supported minor removes its offender from the pool. IDs identify actors,
not player attributes. Probabilities are authored until a separate task-contest
adapter is defined. All choice weights must be positive and total 10000.

Each flight/collection lists the author's reachable outcome set; all listed
players must be active, even on unselected branches. Positive step seconds are
elapsed live time before an action resolves. A contact is evaluated at that
endpoint; penalty expiration is logged at its exact earlier clock when necessary.
There is no intermediate spatial interpolation. A restart consumes zero live
seconds and declares the adjudicated faceoff winner. No substitutions are made.

The chain `release(shot=true) → post → deflect → goal` has one attempt and one goal.
A goalie touch without a hold leaves a loose puck. A collected rebound followed
by a new deliberate shot creates a new attempt with a prior same-team attempt
link. A hold whistles; a new action requires an explicit restart. Legal/illegal
challenge adjudication and puck separation are separate, jointly sampled facts.

The ordinary-minor branch assumes all special exceptions already excluded by
declared adjudication. It supports one skater minor lifecycle, delayed call,
service, expiry and nonoffending goal termination. It does not automatically
recognize tripping geometry, penalty-shot cases or all NHL faceoff exceptions.
Unknown exits and icing candidates stop unresolved. A sampled terminal outcome
may make a later fixed action invalid; this script API rejects that action rather
than inventing a compatible continuation. Adaptive episode policies are future work.

Result JSON embeds the input, selected draws, event/cause IDs, state snapshots,
attempt/parent links, minor state and a reconciled goal ledger. It reports raw
contacts, not official player SOG/save/scorer/assist attribution. No continuous
match, empty-net transition, empirical physics or full rulebook implementation
is claimed. Prior possession/tournament versions remain separate.
