# Compact Rinkcraft profile

The primary profile is an original US Letter sheet: identity, class, specialty,
secondary class, raw measured values and percentiles, short method and trait
names, and documentary coverage. Existing authored labels are retained, without
truncating names or introducing game mechanics. The full dossier, numerical
evidence, context, comparisons, style and history remain in a closed disclosure.
This changes presentation only; documentary selection and numerical computation
remain in the shared Rust implementation.

Both the native Askama template and browser renderer use the shared front-sheet
stylesheet. At desktop size the front is 816 by 1056 CSS pixels (8.5 by 11 inches).
Mobile reflows the fields and value boxes. Print uses Letter with 0.35-inch
margins, hides the surrounding application and evidence, and preserves values.
Unknown documentary coverage stays unknown rather than becoming zero.

## Local validation

- TypeScript compilation passed with the installed pinned dependencies.
- Five focused browser tests passed, including preservation of detailed evidence,
  literal text handling, zero versus missing values, and unknown dossier coverage.
- `cargo check -p icelines-web --locked` passed, compiling the Askama templates.
  No Rust computation or classification code changed.
- Headless Edge rendered the new browser component using existing native API
  data for defense, forward, goalie and an unscouted historical profile. The
  native application stylesheet and browser dialog stylesheet were checked. No page errors, no
  horizontal overflow at 390 pixels, and disclosures opened successfully.
- All eight exported PDFs (native-style page and browser-style dialog) were independently checked with pypdf: one Letter page
  each. Desktop and mobile screenshots were retained locally for visual review.
- Existing acceptance scripts now open the evidence disclosure before interacting
  with its contents; native visibility assertions target the new front sheet.
  These scripts passed syntax checks; their full native/WASM runs were not repeated
  against the unchanged preview servers.
- `git diff --check` passed.

Replay the focused layout check after TypeScript compilation with
`node icelines-browser/e2e/rinkcraft-compact-sheet.mjs`. Set `ICELINES_TEST_URL`
if the existing native API uses a different loopback address.

Local artifacts are under `target/compact-sheet/` in the isolated F: worktree;
they are not repository payloads. The pre-existing preview servers were read
for data only; they were not restarted or replaced. No hosted build,
publication, main merge or desktop control was performed.


## Attribute-column revision

The six measured signals now form a single vertical column at the left of the
main form. Their raw values remain prominent; percentiles sit in smaller framed
capsules. Four neighboring boxes show Methods, Traits, Role and Open Questions.
Goalies retain their four measured signals; the legacy seven-attribute fallback
retains all seven. No fabricated signals fill missing slots.

`design/previews/rinkcraft-section-map.html` is the local side-by-side comparison
shell. Copy it to `target/compact-sheet/comparison.html` beside `defense.html`
and a rendered `dnd-reference-page1.png`. The reference is page 1 of Wizards of
the Coast's official 2014 character-sheet PDF; reference bytes remain outside
Git. Six keyboard-accessible controls highlight the proposed matching sections
on both pages. Supporting-record correspondence is explicitly a layout proposal.

The original native data server was unavailable during this revision. Local
fixtures were projected by the existing shared WASM engine from the existing
catalogs, with a deliberately absent dossier variant for missing-data coverage.
The focused layout acceptance also accepts an explicit `ICELINES_PROFILE_FIXTURES`
JSON path instead of using the live API. TypeScript, five focused tests, native
Askama compilation and the revised stacked-column/mobile/print checks passed.
Four print exports remain one Letter page each. The comparison check verified
that the reference image loads, all six controls update both panes, the six
signals stack without overlap, and the pair stays within a horizontally
scrollable workspace on a narrow screen. Screenshots were inspected locally.
Replay it with `node icelines-browser/e2e/rinkcraft-section-map.mjs` after serving
the prepared comparison directory (default loopback port 8771).
