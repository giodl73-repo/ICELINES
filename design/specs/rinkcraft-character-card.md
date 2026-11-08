# Rinkcraft character card (CHG-128)

Owner correction: a D&D-readable identity card is the primary view; statistical
curves/history/time travel belong in expanders. The official sheet and D&D Beyond
section guide were inspected as layout precedents. No third-party artwork, rules,
ability modifiers or character-level mechanics are copied.

Input schema `rinkcraft.character-catalog.v1`, protocol
`rinkcraft.documentary-class.0.1.0-experimental`. Authored player assessments live
in the independent Rinkcraft research repository. IceLines owns generic types,
validation, dated selection and rendering. Class identity is source-based curated
interpretation, never a threshold over statistical bands.

Records separate class definition/analogy/purpose, primary and secondary assignment,
specialty, summary, source-linked features/methods, explicit evidence types,
publication-season cutoffs, dates, sources/locators and unknowns. Select only
supported features/secondary classes no later than the requested publication
season. Return unscouted before supported primary evidence or for an unassessed
player. This conservative cutoff is not a dated behavioral change detector.

The native server reads an optional explicitly configured input:
`ICELINES_RINKCRAFT_SHEETS=PATH_TO_AUTHORED_CATALOG`. It never scans sibling repos.
The existing profile JSON gains nullable/defaulted `character_sheet`; `season`
selects its period, otherwise selected last/latest season is used. CLI statistical
JSON keeps this field null; catalog attachment is currently a web/browser adapter.
Inputs are limited to 128 KiB. Invalid references, classes, duplicate identities,
dates or non-HTTPS source links reject. Fingerprint is SHA-256 of the typed
catalog's canonical serde serialization, not its original whitespace bytes.

WASM adds stateless `character_sheet(catalog_json, player_id, season)` using the
same core selector. Worker operation `characterSheet` receives explicit catalog
input; browser data acquisition is an optional same-origin public artifact,
`rinkcraft-characters.json`. No dossier means unscouted, not a class inferred from
the statistics. The active prepared package selects documentary period; the
interior statistical-history controls inspect measurements without rewriting the
identity. Loading another package selects a new period and closes stale dialogs.

The generic browser build copies an optional operator-provided catalog from ignored
`public/`, includes it and the shared original CSS in the authenticated offline
shell, and retains explicit release/publication authority. The research snapshot
is not committed in IceLines. With no supplied input this remains a statistical
app with honest unscouted cards.

Primary card: name, roster position independently from class, hockey class and
fantasy analogy, specialty, compact measured attributes with period/GP, signature
features, methods, multiclass, unknowns and visible provisional/evidence dates.
Source dossier and trait evolution are separate expanders. Old attribute-anchor
links target the character card so existing bookmarks do not reopen the old
dashboard. Original D&D-inspired design is shared by native and browser surfaces.

No NHL-wide automatic classifier, proficiency bonus, synergy model, player
personality inference or validated skill/role level is introduced.
