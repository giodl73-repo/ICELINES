# Approved sheet reconciliation — 2026-11-09

CHG-142 / WP-RINKCRAFT-18. Inputs: common base `6402e5e`, C: alternative
`cfc4337`, F: approved form `66f3cbe`. Both branches were compared in their shared
Git object store. The C: checkout remains clean at its original commit. No merge,
blanket cherry-pick, push, hosted build or publication was performed.

## Disposition

Keep the F: Letter-form layout and approved Rinkcraft names. Its six capability
slots remain Unrated; raw values/percentiles remain in Measured Record. Experience
is separate from quality; assignments/links/limitations do not inherit generic
class purposes or open questions. Retain full evidence in its dossier disclosure.
Carry over only the C: branch's explicit per-measure interpretation/peer-count
explanation. Do not carry over its raw-stat attribute rail or 650px contract.

Correct the front's unavailable-catalog status, including the footer. Preserve
Unscouted for a supported catalog without a dossier. Native front position now
falls back to numerical source group when the ordinary season record is absent,
so goalie-specific capability/task vocabulary is retained. This adds a template
projection field; no core numerical/classification code or source catalogs change.

## Reproduced checks

- Pinned TypeScript compilation and eight focused profile/sheet Node tests pass.
  Coverage includes zero/null, literal source content, unknown evidence, unrated
  capabilities, goalie vocabulary and unavailable versus unscouted dossiers.
- `cargo check -p icelines-web --locked` passes, compiling native Askama templates.
  The existing handoff build cache at `F:/src/ICELINES/target` was reused through
  `CARGO_TARGET_DIR`; the unrelated checkout's source files were not edited.
- Six retained fixture cases pass under each native/browser host stylesheet:
  defense, legacy forward, goalie, unscouted, unavailable catalog and source-only
  goalie. The last two are explicit fixture perturbations. Each has six vertically
  stacked nonoverlapping unrated capability slots, separate measured records,
  closed/reachable dossier, no 390px horizontal overflow and one-page print.
- pypdf independently verifies all twelve exports are one-page 612×792pt Letter
  with both Attributes and Measured Record. Visual inspection covered the whole
  defense form. The reference comparison passes all seven highlight controls,
  image loading, desktop/mobile geometry and absence of page errors.
- `git diff --check` passes. Source/catalog/classification data remain untouched.

Commands:

```powershell
node icelines-browser/node_modules/typescript/bin/tsc --project icelines-browser/tsconfig.json
node --test icelines-browser/tests/rinkcraft-profile.test.mjs icelines-browser/tests/rinkcraft-sheet.test.mjs
$env:CARGO_TARGET_DIR='F:/src/ICELINES/target'
cargo check -p icelines-web --locked
$env:ICELINES_PROFILE_FIXTURES='F:/src/ICELINES-worktrees/rinkcraft-sheet/target/compact-sheet/fixtures.json'
node icelines-browser/e2e/rinkcraft-compact-sheet.mjs
$env:ICELINES_SHEET_SURFACE='browser'
node icelines-browser/e2e/rinkcraft-compact-sheet.mjs
Remove-Item Env:ICELINES_SHEET_SURFACE
node icelines-browser/e2e/rinkcraft-section-map.mjs
```

## Review surface and limits

Existing loopback preview `http://127.0.0.1:8771/defense.html?layout=rinkcraft-v2`
serves the regenerated snapshot; `comparison.html` pairs it with the reference.
Artifacts/fixtures/PDFs remain ignored under `target/compact-sheet`. Snapshots use
the browser renderer with retained shared-engine projections. Native template
compilation and host-CSS fixture checks are not a full native runtime comparison.
No full native/browser application rebuild, new WASM engine, hosted acceptance or
replay of all historical live-app E2Es is claimed. Earlier C: E2E results apply to
that alternative build, not this form. Capability scales remain a presentation
proposal without scored assessments, a validated rubric or an exceptional tier.
