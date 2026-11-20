# Visible measured assessment — CHG-135

Owner requests continued numerical assessment. Add optional `assessment` to the
shared HistoryCard, schema `icelines.rinkcraft-measured-assessment.v1`. Archive,
measurement and class schemas remain unchanged. Research policy:
Rinkcraft `docs/research/measured-assessment-protocol-01.md`, decision 0015.

Fixed skater shortlist: primary assists/60, SOG/60, individual xG/60, hits/60.
Goalies: GSAx/60, save fraction, SOG faced/60. This is an editorial subset, not a
complete role vector. Core operates only after the history's temporal filtering.

Repeated headlines require >=2 supplied snapshots, the player reported in every
snapshot, and all metric points ranked/comparable with P >=80. Select at most two
by greatest minimum annual percentile; ties follow shortlist order. Carry key,
label/unit, count, minimum/maximum P and latest value/P. Incomplete coverage or
unranked/incompatible data cannot become a low grade or a qualifying repetition.
Full history's available-year counts remain unchanged and accessible.

Recent headline uses only the latest two supplied, consecutive seasons with
matching group/situation. Require per-metric ranked values, same unit/formula;
choose greatest absolute percentile-point difference from the shortlist, retaining
sign, raw difference/values, ranks and dates. Exact ties use shortlist order. Never
substitute an older pair when the newest player record is missing. No compatible
pair => explicit reason. No statistical significance, improvement label or cause.

Native/browser render the shared object as an always-visible assessment on the
primary D&D card above its closed full-history expander. Each summary carries its
own dates and reported/supplied coverage. Selection rules expand separately.
Correlated measurements are not independent evidence; no overall score, class
inference, validated ability, persistent style or goalie technique is assigned.
Legacy HistoryCard objects without assessment retain the history table.

CLI `profile-assess --history FILE --player ID [--first-season YYYYZZZZ
--last-season YYYYZZZZ]` uses the same bounded local archive reader/projection.
Explicit CLI file is unbounded in time unless bounds are supplied; product native
and browser retain their display/active-season ceilings and regular-season guards.
Invalid CLI season IDs reject; unknown/no-window players return JSON null.
No network, source mutation or quote/class inputs.

Original 74 player-seasons, 1,020 annual measurements, 345 primary measurements
and 47 style readouts remain unchanged. Existing WASM history method/worker
operation expose the additive field; no new prepared data asset. Acceptance:
threshold/ties/cutoff/missing/incompatible group, actual CLI/WASM and native/WASM
parity, raw-source preservation, legacy rendering and no-JS/offline/mobile checks.
Evidence: `docs/rinkcraft-measured-assessment-validation-2026-11-08.md`.
