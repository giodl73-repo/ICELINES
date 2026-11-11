# Rust paired-seed experiment

`view_model::rinkcraft_plan_experiment` evaluates both support plans through their
own legal paths under `sole_legal_action_v1`. It delegates every contest/state to
the existing possession engine, chooses its unique legal action, and stops at a
terminal phase or the existing 16-roll cap. Multiple choices fail rather than
silently introducing an undeclared policy. No old gameplay/schema fields change.

The bounded request embeds character/scene rules, a verified fictional hero,
master seed and 2–4096 paired trials. The unique trial-seed schedule uses the
existing SplitMix64 high-u32 generator with duplicate rejection. A shared seed
couples dice by roll ordinal; after paths diverge, that ordinal may resolve
different actions. Both actual action paths and endpoints are retained per trial.

The result includes a canonical request hash, five-by-five paired terminal-phase matrix,
all marginal counts, commit-minus-hold differences and paired Monte Carlo SE.
SE uses the sample variance of paired {-1,0,1} indicators, with the u32 seed-space
finite-population correction. It approximates sampling precision under a uniform
seed/PRNG assumption; it does not quantify model uncertainty or empirical NHL
probabilities. No discordance in a sample does not prove population equality.
Equal terminal phases do not imply identical control, stations or histories;
retained paths can reconstruct full final states. The endpoints receive no overall utility or ranking. Completion is a received
pass, not a shot or goal. Action and hero-capability exposures count contest uses.

Read-only Rust CLIs:

```powershell
cargo run -p icelines-core --example rinkcraft_plan_experiment --locked -- REQUEST.json
cargo run -p icelines-core --example rinkcraft_plan_study --locked -- STUDY.json
```

Study rule paths resolve relative to its input file. The batch validates all case
inputs before emitting one JSON record per case; it allows 1–16 uniquely named
cases and at most 8192 total paired trials. Standard-array heroes are built in
Rust. Input files are size bounded, and no CLI writes files or accesses a network.

All 66 Rinkcraft library tests pass. Five experiment tests check deterministic
unique schedules, paired marginal conservation, zero-bonus control, changing
unused hero attributes without altering paths, actual stored-path reconstruction,
ordinal dice coupling, hand-computed paired SE, bounds and invalid inputs.

Rinkcraft decision 0031 owns the authored 13-case protocol and interpretation;
full outputs stay ignored under `target/paired-seed-study/`. This is CLI analysis.
The existing native/WASM studio remains at its separately recorded 3e3d3c3 build.

Executed study: all 13 authored cases, 512 pairs each (6656 pairs / 13312 scenes),
completed. Single CLI reproduces the batch prefix and is byte-deterministic on
repeat; invalid batch IDs/templates/trial counts/bonuses fail before partial
stdout. Zero-bonus control paths match, as do Vanguard forward/defense paths.
Post-run coverage inspection also finds identical Setkeeper/Scrambler paths:
their shared Puck Play 9 is the only goalie capability exercised by this scene.
Core statistics remain in Rust; JSON projection/provenance helpers only retain
and format the output. Approximate SE is per case; shared schedules do not make
the total workload a pooled independent sample. No native/WASM rebuild or new
UI integration is claimed for this CLI feature.
