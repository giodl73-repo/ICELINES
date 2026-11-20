# Sourced factual identity

`CharacterAssessment.card_facts` is an optional array, selected by shared Rust
into `CharacterSheet.card_facts`. Absent/empty arrays remain omitted in serialized
legacy catalogs, preserving existing fingerprints. This extends the v1 catalog
without changing its documentary-class protocol or the capability model.

Each fact has one unique `field` (`development` or `experience`), a visible
`context`, the flattened `CharacterFeature` source/date fields, and an optional
inclusive `through_season`. Blank context/name/detail/evidence type, duplicate
fields, unsupported source IDs, sources later than `from_season`, and reversed
validity ranges reject the catalog. Facts before their availability or after
expiry are omitted; only selected fact sources enter the sheet's source union.
The current catalog allows one fact per field, not multiple historical versions
of that field. A dated new catalog revision preserves earlier assessments.

The front shows a short value and context. A closed `Record source` disclosure
holds the explanatory detail, source link, publication precision and locator.
Missing fields display `Undocumented`. Native and browser views use the same
selected data. Print omits the disclosure, retaining the value/context and the
existing source coverage/dossier route.

Development is a selected pre-NHL pathway, not inferred ancestry or an exhaustive
history. Experience can show a sourced NHL debut with regular-season/postseason
context; it does not imply quality level, years of full seasons, or career GP.
Facts never populate capability ratings, assigned roles or known limitations.

Research inputs remain authored in Rinkcraft; named-player runs belong in PUCK.
The local theme studio is a generated preview, not a full application deployment.
