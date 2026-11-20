# Two-stage defended pass

Five fictional drafts isolate passer Passing, receiver Handling, lane defender
Reading and receipt defender Defending. Same players, seed, access context and
timing. Each upgrade changes one capability from 10 to 16; all others stay 10.

Use the existing CLI: `rinkcraft_events build DRAFT REQUEST`, then
`rinkcraft_events REQUEST RESULT`. Matching draft/episode 0.3 opts into the new
`pass_under_pressure` action; 0.1 and 0.2 reject it. Verified character profiles
and score rules use the existing `rinkcraft.event-profiles.0.1` contract.

The actual skater controller releases to a distinct teammate through a declared
lane. Passing faces the lane defender's Reading. A strict win opens a separate
timed arrival/collection stage: receiver Handling versus opposing Defending.
Every named actor must be active and appropriately sided at delivery; recipient
and pressure actor are checked again at receipt. The two defenders may be the
same player in a declared context. Goalies cannot substitute skater capabilities.

Delivery loss/tie leaves a loose puck, without automatically recording an
interception or defender touch. It consumes neither receipt time nor receipt RNG.
Delivery success keeps the puck in flight until receipt resolution. Receipt
records a receiver contact; a win establishes control, and other outcomes leave
it loose. Last deliberate play remains the passer's release. This is a pass, not
a shot or automatic goal. Live penalty clocks advance through both actual stages;
an unsupported period boundary stops before inventing a receipt.

Step seconds are delivery time; `receive_seconds` (1..1200) is additional live
time only when delivery succeeds. There is no speed-to-distance conversion.
Each reached task records actors, scores, capability IDs, modifiers, exact
400-pair counts, dice and outcome in `pass_task`. Other legacy fields/RNG paths
are unchanged. No context adjustments or class bonuses are introduced.

Under half-offset scoring, neutral control requires two wins: 190²/400² =
22.5625%. Better Passing or Handling gives 29.33125%; stronger lane Reading or
receipt Defending gives 16.15%. Equal endpoint probabilities can have different
stage exposures and time costs. These are exact game mathematics for a **twice
contested pass**, not NHL completion rates. Do not apply two defended gates to
every routine pass. Access/pressure-context selection and calibration remain
separate work before continuous games.
