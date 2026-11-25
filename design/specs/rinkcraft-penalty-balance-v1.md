# Penalty balance v1

Playing Tendencies gains a compact expandable **Penalty balance**. It describes
recorded calls drawn and taken, separately from earned feats and documentary
tendencies. It does not award a class, feat, attribute point or simulation bonus.

The selected skater's existing all-strength NHL context supplies `drawn` and
`taken` rates. Each is credited calls × 60 / official all-strength minutes.
Balance is drawn minus taken, computed before rounding each display to two
decimals. Positive means more drawn; negative means more taken. Values rounding
to zero use +0.00 rather than negative zero. Penalty minutes are a different unit.

Require positive GP and finite positive minutes with all-strength scope; no
ten-GP gate applies. A missing rate stays Unavailable and prevents a balance.
Measured zero stays zero. No inputs means no record. Goalies are withheld from
this skater contract. Actor, season, group, GP and minutes bind the derived record
to its parent; catalog admission rejects stale or altered records.

Rust derives the record from existing context during native/WASM profile
projection. Normal refresh needs no additional fetch or enrichment stage.
TypeScript displays Rust strings without computing hockey rates. Older catalogs
remain valid, and already serialized old cards without the field simply omit it.

The closed disclosure shows the balance. Open it for both component rates,
exposure, formula and interpretation. Existing feats and sourced tendencies stay
in place. Print hides this supplemental disclosure to preserve the compact sheet.

Calls drawn do not equal power plays created. Coincidental penalties, majors and
other calls can have different manpower consequences. These rates do not measure
decision quality, legality of uncalled contact, goals gained or isolated skill.
Possession, deployment, opponents and officiating can affect the observed rates.
