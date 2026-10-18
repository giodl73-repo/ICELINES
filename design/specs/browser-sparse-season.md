# Opt-in sparse prepared-season ownership

CHG-124 / IF-BROWSER-SPARSE-001; REQ-PARITY-001 / REQ-BROWSER-001.
Depends on additive core/query reads in PR111 and PR112.

The browser's prepared-package owner can select `icelines-wasm/sparse-storage`.
Default builds retain StatsRepository ownership. Sparse builds use the separate
SparseSeasonRepository, scoped to prepared-package consumers: compact hot rows,
eight typed dense payload tables, checked NonZeroU32 slots and owner back-references.
Replacing/removing a row releases payloads and repairs swapped owners/slots. The
implementation ports the safe algorithm tested by MISER EXP006/009; no lab crate
dependency or unsafe packing is introduced. Payload widths/Option distinctions,
identity merge policy and contract references remain original types.

This is deliberately not a complete StatsRepository replacement: it has no roster
overlays, Tier2 JSON report cache or internal window LRU. BrowserEngine already owns
prepared-window refresh/eviction and does not consume those indexes/caches. Their
absence contributes to any measured browser-owner saving and must be disclosed.
Window identity is scoped by each ActivePackage's separate repository; identities
are not shared across package windows. The sparse owner is !Send + !Sync.

PlayerView remains a concrete-reference alias over generic ReadPlayerView. Existing `PlayerView<'a>` still
has `stats: &'a SeasonStats` and retains Copy/Clone behavior and concrete repository
APIs. New views carry a lightweight SeasonRowRead value borrowing actual rows/tables.
References implement SeasonStatsRead, so generic and legacy helpers share original
core formulas. Existing goalie/pace comparator signatures delegate to new read comparators with
unchanged comparison expressions. Borrowed JSON emits all original SeasonStats
fields in original order, resolving typed references without an assembled clone.

Both browser backends use read_league, the actual Constraint evaluator, shared
catalog/goalie/pace sorting and existing result projection. Source packages remain
owned for exports. Revision calculation, staging-before-activation, source exports,
player details, query/result schemas, selection, eight-window/64MiB input limits,
error handling and prepared-data/provider restrictions retain their contracts.
The feature is opt-in and does not enable/publish a new browser artifact by default.
`node scripts/build-browser.mjs --sparse-storage` builds an explicit candidate
distribution. Build-info records `wasm_features`; ordinary builds record an empty
list. The backend choice is a build property and changes no application controls.

Verification: frozen original catalog bits/order over 288 boundary rows, exact
typed/JSON payload parity, 1,200 mixed update/remove operations, slot invariants,
legacy concrete-reference/Copy and Box/mutable-reference coercion checks, and browser tests in both feature modes.
Native workspace compatibility, core/query regression suites and both WASM builds
remain required. Original-engine/default-branch/sparse-branch native replay uses all
75 packages, all eight browser sorts, 20 query profiles per window, every player
detail, canonical exports, two local imports, invalid refresh preservation and
resident reselection. Preserve source/feature/lock/executable hashes and raw samples.

Native requested retained/construction-peak/query-scratch measurements are separate
from process RSS, WASM linear memory and whole-browser peaks. Sparse tables can lose
at dense report occupancy (MISER EXP004). No resource criterion changes: the fixed
250MiB whole-browser target and separate Android gate remain. No merge/deployment or
production acceptance is implied by compiling or measuring this candidate.
