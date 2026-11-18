# Explicit repertoire catalog projection

`tools/project_rinkcraft_repertoires.mjs` selects documentary sheets through an
existing shared Rust WASM package, preserving all numerical profile fields.

```text
node tools/project_rinkcraft_repertoires.mjs WASM_PACKAGE_DIRECTORY CATALOG_JSON BASE_FIXTURES_JSON OUTPUT_DIRECTORY
```

The inputs must be explicit local paths. The directory contains the generated
`icelines_wasm.js`/`icelines_wasm_bg.wasm` pair. The caller owns source selection,
access rights and authored interpretation. No network refresh, repository scan,
automatic classification, skill scoring or game-model update occurs here.

Each base fixture supplies matching player identities and an explicit documentary
as-of season. Missing dates, crossed identities or unsupported catalogs reject;
no latest-season or nearest-player fallback. The Rust selector validates support
and date cutoffs. Existing class, specialty, summary, traits and methods must match
the base snapshot; the tool is deliberately scoped to repertoire revisions.
It writes new fixtures and a receipt containing input hashes and selected counts.
Catalog revision metadata/source union/unknowns may change; statistical fields do
not. A historical source cutoff is distinct from the numerical evidence season.

The local theme studio can consume these fixtures through its existing explicit
preparation command. Historical repertoire claims are labeled separately from
current-season measurements. This is a generated local preview, not a full native
server rebuild, application publication or a validated assessment instrument.

Validation2026-11-18: five positive projections preserve numbers/accepted identity;
four negative cases (missing source, missing date, wrong player, empty fixture map)
reject without a completed fixture output. Twelve actual curated feature cutoffs
omit each entry before its documentary start. Existing Rust selector is reused,
not reimplemented. TypeScript and48 desktop/24 mobile cases pass, as do12 source
expanders and five independently checked single-page real-profile Letter exports.

Named authored catalogs/review records belong in Rinkcraft; dated profile outputs,
fixtures and provenance belong in PUCK. No named-player evidence is embedded in
this generic projection tool.
