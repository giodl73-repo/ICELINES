# Rinkcraft measured playing methods — CHG-132

Owner requests numerical distinctions in how players play. Extend numerical
profiles and projected evidence cards with optional `style_evidence`, preserving
existing catalogs, 15/5 measurements, class hypotheses, source/reference cohorts
and temporal/competition guards. Protocol:
`rinkcraft.playing-methods.0.1.0-experimental`. Rinkcraft owns research definitions,
source registry, access/coverage record and authored cohort; IceLines owns code.

Core derives five-on-five high-danger unblocked attempt share from existing
high-danger count / unblocked attempt count and reuses hits / ice time * 3600.
An optional verified movement capture supplies all-strength maximum skating mph,
20+ mph burst count / reported GP, and miles / reported GP. Each measure carries
value, displayed unit, explicit numerator/denominator, scope, meaning and source
URLs/hashes. For contact the stored numerator is the hit count multiplied by 3600;
the denominator remains seconds. No tracking percentile/cohort is inferred.

Source ingestion: bounded local `manifest.json` and numeric-ID response filenames.
Manifest schema `icelines.edge-capture.v1`, requested season, capture date and
records (ID, pinned public regular-season endpoint URL, byte count, SHA256).
One MiB per manifest/response, whole numerical display catalog remains two MiB.
Validate bytes, identity, skater position, reported games, finite/nonnegative
optional fields, season compatibility and expected endpoint. No network, cookie,
provider-cache mutation or automatic acquisition. Missing data is not zero. Raw
response has no echoed season; requested URL custody is not independent response-
season verification. `profile-style --numbers FILE --edge-captures DIR` emits a
new enriched numerical JSON; original inputs remain untouched.

Native/browser render a separate Playing style section with measured tendencies
and explicit open axes: carry/pass/dump, independent direct/lateral movement,
pursuit/recovery, passing/deception; goalie depth/movement, rebound decisions and
puck exchange. Goalies never receive skater measures. Defender shot mix is not a
retrieval/gap/coverage grade. Existing WASM numerical projection transports the
same optional style evidence and shared validation; no new worker operation.
Prepared enriched catalog uses the existing optional authenticated shell asset.

No binary finesse/physical slider, threshold label, automatic class assignment,
overall style score, with-puck speed, acceleration, intention, chemistry or skill
estimate. Own source scopes stay visible; unsupported axes say Unmeasured.
The source batch failure limits tracking to one retained player, while all 22
skaters have shot/contact data and all three goalies have explicit method gaps.

Acceptance: formula/missingness and source identity/hash/season tests; original
345 measurements unchanged; actual 25-card native/WASM parity; temporal/playoff
guards, no-JS/offline/mobile rendering. See matching validation report. These
checks validate software behavior, not persistent playing styles or class validity.
