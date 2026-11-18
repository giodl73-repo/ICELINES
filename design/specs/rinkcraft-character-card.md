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

## Feature evidence and showcase extension (CHG-129)

The optional external catalog can name a showcase and mark featured assessments;
the application does not select or rank its members. Native `/rinkcraft` and a
browser atlas expose dossiers. Missing active-package players yield an explicit
unavailable message. Old catalogs remain valid through serde defaults.

Pathfinder is the carrier class alias; methods remain in features/specialties.
Feature stat links resolve existing season attributes, using their Rust-computed
values, percentiles and peer counts. Relation labels distinguish a measured aspect
from activity, outcome or deployment context. Missing measurements stay visible.
No UI recomputes ranks or infers class membership. Raw season lines preserve nulls
and percentage scales. Day/year publication precision is retained; research owns
the conservative season cutoff for year-only sources. All classes remain provisional.


## Rinkcraft category form — 2026-11-09

Owner accepted the semantic section mapping, using Rinkcraft headings throughout
the player form. The front sheet now separates six capability attributes from
Under Pressure, Task Skills and Signature Methods. Playing Tendencies, Assigned
Role, Unit Links, Known Limitations, Distinctive Features and Open Questions are
separate right-column boxes. Supporting boxes retain Read & React, Threat Denial,
Response Timing, Skating/Movement Detail, Role Coverage, Deployment, Record
Counters and Measured Record. Identity retains class, specialty, secondary class,
development and experience; provenance remains below the form.

Skater capability labels are Skating, Handling, Passing, Shooting, Reading and
Defending. Goalies use Tracking, Positioning, Movement, Rebounds, Recovery and
Puck Play. The intended display is 1–20; every capability is currently Unrated.
No scoring rubric, numerical assessment contract, derived modifier or 20+ tier
is introduced by this presentation change. Statistical percentile bands do not
populate these fields. Unsupported tasks and response measures stay Unassessed;
unsupported biography, duties and relationships stay Undocumented.

The primary class purpose does not establish an Assigned Role. Unknowns do not
populate Known Limitations. The class, methods and tendencies still come from
the dated documentary selector. Raw numerical values and percentiles are retained
in Measured Record; the complete source-linked dossier remains expandable.
Native Askama and browser rendering share the stylesheet and category semantics.

## Player repertoires — 2026-11-18

Owner clarified that Under Pressure and Task Skills should differentiate players
through named characteristics. They now use optional source/date-bound repertoire
arrays rather than fixed task rows or numeric scores. Pressure entries name a
conditional response; skills name a task specialty. First3/5 entries are shown,
with extras expandable. Missing curation remains unknown. Attributes, methods,
style and supporting measurements retain their separate meanings. See
[repertoire contract](rinkcraft-player-repertoires.md); this refines presentation,
not the attribute rubric, action taxonomy or simulator.
