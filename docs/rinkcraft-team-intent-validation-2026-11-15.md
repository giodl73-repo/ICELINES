# Prospective team-intent validation — 2026-11-15

Rust protocol 0.11 adds optional scene team intent, schema 0.1: local six-player
legend, declared trigger, prospective named cover permission and outlet inventory.
Only a permitted chaser's Intent changes. Access/collection uses the shared engine;
no capability, class, pass chance or goalie domain is rewritten. Cover execution
is always unconfirmed; no actual covering trajectory/handoff is implemented.

## Local checks

175 distinct focused checks pass: events 149 (14 new team-intent checks), possession 12,
contest 6, characters 8. Tests cover active/inactive/unknown triggers, timely/late/
unknown/unavailable cover, unavailable partner/goalie, permission changing chase/access and actual control,
forecasts distinct from executed motion, outlet occupancy/proximity versus lanes,
goalie pressure explicitly unmapped, local labels independent of identity,
validation of unreached branches, old protocol equality, replay and goal ledger.

```powershell
$env:TEMP='F:/src/ICELINES-worktrees/rinkcraft-sheet/target/windows-tmp'
$env:TMP=$env:TEMP
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo test -p icelines-core --lib rinkcraft_events --locked
cargo build -p icelines-core --example rinkcraft_events --locked
```

Eight neutral drafts under `icelines-core/examples/fixtures/rinkcraft-team-intent/`.
Dated PUCK `rinkcraft-team-intent` run requires clean source, retains its evaluator
by commit, records source/binary/input/output hashes and seed 20261115, replays each
case exactly and compares all 132 previous frozen artifacts (70 results/62 requests).
`inputs/verify.py` separately recomputes forecast timing, permission, outlet status,
actual spatial arrivals, dice/probabilities, zones, elapsed costs and hashes. Python
validates artifacts; gameplay stays Rust. Actual completed run results are in its
provenance, verification, findings and hash seal.

## Scope

A forecast gates a chase; it does not execute a rotation. No handoff confirmation,
exposed-lane consequence or counter-defense shield is claimed. Outlet proximity is
not pass-lane openness. This is an authored local permission variant inspired by
the Rinkcraft systems atlas, not a complete named coach/club system. All geometry,
deadlines, rates, policy and difficulty remain authored, with empirical estimates
null. Target 1 branch cases and fictional scores of 10 do not establish NHL effectiveness.
No real-player grades, observed assignment errors, continuous game, complete NHL
law, full app/WASM acceptance or independent expert review is claimed.

Final checks after the complete-unit availability guard: 175 focused tests pass, including absent coverer, partner and goalie. Locked CLI build passes.
