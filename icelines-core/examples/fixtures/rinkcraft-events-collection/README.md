# Capability-driven pressure collection

Three neutral fictional drafts isolate Handling 10/16 versus Defending 10/16.
The actor IDs, reachable context, timing and seed are identical. One skater has
declared access to a loose puck; one opposing skater can pressure collection.
This does not determine who arrives first or simulate spatial access.

```powershell
cargo run -p icelines-core --example rinkcraft_events --locked -- build neutral.draft.json request.json
cargo run -p icelines-core --example rinkcraft_events --locked -- request.json result.json
```

Draft 0.2 builds and verifies fictional characters from the existing game rules.
Episode 0.2 requires `profiles` (`rinkcraft.event-profiles.0.1`), and 0.1 rejects
that opt-in. Bindings can be partial, but every task participant needs a verified
profile. Every supplied profile is checked, including unused ones. Goalies must
have goalie profiles and cannot be passed through this skater collection task.

`collect_under_pressure` requires a loose puck and opposing active skaters.
Handling versus Defending uses the existing selected score rule, with zero
context adjustments. `half_offset` uses floor((score−10)/2); adjacent scores can
therefore share a modifier. `direct` is an explicit alternative. No class bonus.

Sample one of 400 equiprobable ordered d20 pairs. The ledger records pair index,
dice, score IDs/values, rule, modifiers, totals and exact win/tie/loss counts.
Actor wins establish control through the ordinary controller path, including a
pending-minor whistle where applicable. Ties and pressure wins leave the puck
loose; the pressuring player does not automatically collect it.

Half-offset exact chances: 10v10 = 190/400 control (47.5%), 16v10 = 247/400
(61.75%), 10v16 = 136/400 (34%). These are authored game mathematics, not observed
NHL probabilities or validated capability grades. Flight contact weights, shot
outcomes, foul rulings, reachability and timing remain declared. Saved 0.1 inputs,
RNG paths and serialized result fields are unchanged.
