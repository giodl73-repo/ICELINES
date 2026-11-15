# First timestamped local-grid access

Episode/draft/result 0.8 adds `loose_puck_access` to an actually loose puck. Use
the existing JSON CLI `build DRAFT REQUEST`, then `REQUEST RESULT`. Compatible
older actions remain available; prior versions must retain exact artifacts.

Require `rinkcraft.access-grid.0.1`, current snapshot clock, basis and exactly one
pose/intent for every active actor. Local integer cells −20..20 use x east/y north;
they are not physical rink units or a player's north/south versus lateral style.
Far nonparticipants share a coarse out-of-window cell in these neutral fixtures;
no real team formation or collision geometry is inferred.

Holding within reach is available immediately; holding outside reach does not
chase. Pursue uses authored reaction delay + Manhattan cells outside reach ×
seconds/cell + an initial-turn delay if facing no toward-target axis. No later
turns, obstacles, acceleration or Skating-to-meters/second mapping is supplied.
All arrivals and inclusion/exclusion reasons are retained. A unique earliest
skater gets access without a roll; coincident earliest skaters each roll Skating,
with a unique maximum needed. Tied top stops unresolved. Access is not control.

Collection pressure comes from all opposing projected arrivals by completion.
Open Handling uses the declared target. Contested Handling rolls once against
all reachable Defending rolls; strict win over all controls. Defense winning
does not own the puck. Exact neutral success: target 5 open 16/20; one pressure
190/400; two pressures 2470/8000. Independent game dice are not an empirical
independence claim; coordinated support is separate future work.

Step seconds are a maximum window. Charge only reached phases. An actor can
arrive at the deadline without enough collection time: record that case without
claiming nobody arrived. Missing/stale snapshots are rejected. A goalie reaching
first or providing pressure stops explicitly unmapped, not excluded or evaluated
through skater capabilities. A penalty expiry updates eligibility at its exact
clock and stops for a new snapshot instead of inventing the returning position.

Eight drafts: early attacker, turn-delayed attacker, early defender, two pressure
actors, tied access, Skating 16 in that tie, no arrival within the window, first
goalie. All seed 20261115. D-position characters participate directly in retrieval.
This derives loose-puck access only, not passing lanes, coaching intent, persistent
trajectories, real player grades or NHL-calibrated rates. Absence alone is not
proof that a player missed their assigned responsibility.
