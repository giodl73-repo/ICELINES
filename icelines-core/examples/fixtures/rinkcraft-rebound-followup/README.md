# A conditional attacking rebound follow-up

Matching episode/draft/result 0.7 adds `rebound_followup` immediately after an
explicit `resolve_shot` step. It is admitted only when that source actually ends
with a loose `goalie_rebound`. Otherwise it records `followup_skipped` without
time/RNG or altering the source result, including goals/holds. Unknown source
adjudication still stops the episode normally.

Declared access/basis names one reachable attacking collector and opposing
collection pressure; it does not detect arrival, distance or puck geometry.
Step seconds are the collection window. Handling vs Defending must strictly win
to establish actual collector control. Loss/tie stays loose; pressure does not
automatically own it. Only a successful collection opens the goalie Recovery vs
collector Skating race, assessed within that window, not as an extra time cost.

This reuses the earlier Reset task pairing, but introduces a new explicit game
consequence: goalie win is reset (Tracking adjustment 0), attacker win is late
(required declared negative adjustment, example −2), tie stops unresolved while
the attacker controls. No Tracking grade is rewritten. Readiness supplies no
automatic save or goal. Release and shot times are charged only when reached.
A new deliberate shot links attempt 2 to parent 1; the modifier applies only to
that one contest. Next finish/containment/whistle rules remain shared.

Neutral Recovery 10 vs Skating 10 counts are 190/20/190. Recovery 16 gives
247/17/136; Skating 16 gives 136/17/247. At declared −2 late adjustment, a neutral
next shooter wins 229/400 versus 190/400 when reset. With existing 50% goal finish,
the conditional goal-event chances are 28.625% versus 23.75%. These are authored
game mathematics, not NHL effects. Recovery ties contribute explicit unresolved
mass; do not treat them as no-goal outcomes when aggregating.

Seven drafts: neutral, Recovery 16, collector Handling 16, collector Skating 16,
stronger late consequence −4, containment Rebounds 16, unused Movement 16. Context
selection, difficulty/consequence calibration and remaining physics remain open.

Illustrations select the first baseline loose-rebound seed >=20261115, before
examining collection, recovery or the second shot: 20261119. First shot dice 1/12,
containment die 11. Earlier candidates are retained. Rebounds 16 may prevent that
same illustration's follow-up by holding the source puck. These conditional
examples are not unconditional frequency estimates. Old event 0.1–0.6 artifacts
must remain byte-identical.
