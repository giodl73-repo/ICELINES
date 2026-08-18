# Hockey film blueprints

IceLines owns hockey-specific story grammar and the data requirements needed to
instantiate it. These blueprints are inputs to an editorial workflow; they are
not rendered media and are not substitutes for a conformed REEL manifest.

- `player-history.yaml` structures a public-career biography.
- `team-hype.yaml` structures a roster-centered anticipation or game film.
- `animated-moment.yaml` reconstructs a verified goal, celly, save, hit,
  penalty, faceoff, or bench sequence as a short animation. When available, it
  can use timestamped NHL EDGE Goal Visualizer player/puck paths discovered from
  Gamecenter landing without claiming undocumented rink units.
- `hockey-sprite-profile-v0.1.yaml` maps explicit hockey role, action, phase,
  facing, handedness, and possession selectors onto REEL's generic layered-pose
  library. It treats unknown handedness as an explicit state and refuses to
  imply that the starter profile covers unlisted hockey movement.

The customer project owns its brief, chosen facts, media inventory, generated
assets, provenance ledger, and final manifest. REEL owns the provider-neutral
timeline, score, rendering, and verification contracts.

Each blueprint ends with a `reel_handoff` describing which generic REEL
artifacts should be bound into the review or release package. Hockey-fact,
rights/provenance, editorial, and accessibility gates remain explicit human
decisions; a valid render never implies their approval.

The starter sprite profile is deliberately measured from one reconstructed
sequence. Pose counts are coverage observations, not promises: IceLines should
grow the hockey vocabulary from reviewed plays instead of manufacturing a
nominal grid of every possible combination.
