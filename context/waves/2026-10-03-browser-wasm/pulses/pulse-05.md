# Browser WASM pulse 05 — build and Pages workflow

Date: 2026-10-03. Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Implementation

`scripts/build-browser.mjs` is the shared local/CI entry point. Browser toolchain
versions live in `icelines-browser/toolchain.json`: Rust 1.95.0, Node 22.22.3,
Python 3.14.2, wasm-bindgen 0.2.118. The pipeline enforces the crate/CLI binding
pin, generates packages and bindings, builds, verifies distribution integrity,
and optionally runs fixture tests. Build metadata records actual tool versions,
HEAD, dirty working-tree status and the WASM digest. HEAD alone does not identify
dirty source inputs. No reproducible-byte claim is made across operating systems.

Browser npm lock entries now use the public npm registry with SHA-512 integrity.
Workspace-downloaded tarballs were checked against the previous lock digests
before rewriting registry references. Clean offline `npm ci --ignore-scripts`
passed using the verified workspace cache.

The build cleans only its verified distribution directory, copies only catalog
packages, and rejects unexpected publication files or links. The verifier hashes
shell/catalog assets and loads/queries every package through actual generated
WASM bindings. Worker-template bytes now participate in the offline cache
identity, including when only worker logic changes.

`.github/workflows/browser-pages.yml` prepares Linux pull-request/master checks,
LFS checkout, immutable action pins, preview artifact retention for 30 days,
and a separate master-only manual publish path. It has not been pushed,
dispatched or deployed. Environment review enforcement depends on repository
settings. A retained artifact does not yet prove a working rollback deployment.

## Verification

`node scripts/build-browser.mjs --offline --check` passed:

- Actual WASM verification of 75 public catalog packages and 15 shell assets.
- Seven portable data/engine Rust tests, 42 native loader tests and two
  native/browser normalization parity tests.
- Seven Python archive/catalog tests and 37 Node acquisition, archive, storage
  and offline-shell tests.

The initial native loader run hit filesystem permission errors writing temporary
files outside this sandbox. The entry point now assigns child-process TMP,
TEMP and TMPDIR to `target/browser-test-tmp`; the full rerun passed without
escalation. Rust emitted incremental hard-link fallback warnings, with no failing
checks. Workflow YAML parsed successfully with PyYAML 6.0.3; this is syntax
evidence, not a GitHub Actions execution. Scoped whitespace checks passed.

## Remaining release gates

The goal remains active. Linux CI execution, deployed Pages/relay reachability,
rollback proof, offline reopening and update acceptance, worker recovery,
storage lifecycle/quota behavior, URL restoration, native sort/rate-floor
parity, resource budgets and remaining first-slice capabilities need evidence.
The .roles review remains a planning approval with conditions; this pulse does
not promote the application to shipped status or close those conditions.
