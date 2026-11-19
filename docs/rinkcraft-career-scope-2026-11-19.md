# Reconciled NHL career record preview

The five local studio examples now support Season, Current and Career alternatives.
Career means **NHL regular-season career through its capture endpoint**, including
the available current-season partial row. It excludes playoffs, minors, juniors
and international games. This is not an all-hockey lifetime aggregate.

## Source and aggregation contract

22 official raw responses cover five landing documents and matching stats reports,
49 NHL regular-season player-season rows. Landing career GP/core totals, the NHL
landing season ledger and REST season summaries reconcile exactly. Multiple team
stints may sum to a season total; a duplicated combined-total-plus-stints fails.
These endpoints belong to the same provider, not independent corroboration.

The Rust example verifies HTTPS routes, actor/game type, raw hashes, source/assembly
fields, seasonal coverage and GP, and capture dates. Career reports may span only
the full actor/game-type filter, not a hidden single-year slice. Exact seasonal
TOI is pooled, with EV+PP+SH accounting for skaters. Rounded average TOI is never
multiplied by GP. Rates pool numerators/denominators; seasonal percentages or
percentiles are never averaged. Additional RT/penalty counts retain per-field
coverage and are not falsely presented as independently published career totals.

The first regular-season row is not necessarily the first NHL game: playoff debuts
can precede regular-season careers. Historical documentary identity/biography
retains its evidence dates and meaning. Scope choices do not reassess classes.

## One discovered field conflict

The goalie REST season saves sum to8840, while reconciled shots against9643 minus
goals against804 gives8839. The difference appears in2023–24: reported saves1466,
shots against1606 and goals against141. The published save fraction is also
retained. No cause, definition resolution or provider correction is invented.

The career save percentage is therefore unavailable with `conflicting_sources`,
not silently calculated from a chosen numerator. GP/starts/wins/shutouts/GA/TOI
remain available. A field conflict does not discard the rest of the player's
record. The primary cell shows Source conflict; expandable reconciliation retains
both counts and the affected season. No missing value is replaced by zero.

## Presentation and schema boundary

The scope switch replaces the entire primary frame. Career labels appear on the
front, source panel, evidence brief and workload. Legacy `season` fields identify
the last observed endpoint for local component binding, **not a one-season window**;
the career kind/range/label is mandatory. The aggregate has a distinct preview
schema and must not be imported as a single-season StatsRepository row.

All career percentiles and annual feat reuse are withheld. There is no validated
career-stage/era reference cohort. Core career counts reconciled does not mean
all possible task/skill measurements are available. Thirty headline abilities
remain unrated. The existing34 annual rules and three experimental candidates
are not converted into career achievements. Historical/Current numeric frames
and documentary assertions remain exactly preserved.

The prototype requires complete matching TOI for these five modern careers.
Historical eras with missing time need a separate partial-coverage representation;
this pass does not claim arbitrary retired-career support. Native/WASM loaders,
hosted deployment, automatic refresh and simulator integration are unchanged.

```text
powershell -NoProfile -File tools/capture_rinkcraft_career.ps1 -InputProfiles INPUT_PROFILES -OutputDirectory NEW_CAPTURE_DIRECTORY
cargo test -p icelines-sources --example rinkcraft_career_frame --locked
cargo run -p icelines-sources --example rinkcraft_career_frame --locked -- HISTORICAL_FIXTURES CAPTURE_INPUT_JSON OUTPUT_CAREER_PATCHES
node tools/bundle_rinkcraft_career.mjs EXISTING_SCOPE_BUNDLE OUTPUT_CAREER_PATCHES OUTPUT_BUNDLE
node tools/prepare_rinkcraft_theme_studio.mjs OUTPUT_BUNDLE SITE_DIRECTORY
node icelines-browser/e2e/rinkcraft-career-scope.mjs
```

Five Rust tests, TypeScript compilation and24 focused Node tests pass. Independent
checks verify56 raw-derived core reconciliations,556 season-field comparisons,
49 season GP rows and30 displayed-measure decisions (29 numeric, one withheld),
22 raw hashes, six malformed-input rejections and byte-identical replay. Five
three-scope round trips, career deep link,30career desktop/30mobile theme cases,
five one-page Letter exports and existing Current/Season/theme regressions pass.
The goalie screenshot was inspected. No full native/WASM/hosted acceptance claimed.

Rinkcraft owns the definition, source/conflict record and producer review. PUCK
owns dated/player outputs, raw captures, exact commands/hashes and an immutable
packet retaining the complete site and replay inputs. Reusable Rust/CLI/browser
code belongs here. Source discrepancies remain explicit research work, not a
reason to fabricate a complete capability card.
