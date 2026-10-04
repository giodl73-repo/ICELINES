# Browser WASM pulse 26 — retained-artifact rollback preparation

Date: 2026-10-04. REQ-BROWSER-001 / WP-BW-06. Previous turn was progress:
full checked build and project-subpath observations. Goal remains active.

Added a manual master-only `browser-rollback.yml` workflow. It reads a selected
previous successful browser-build artifact rather than rebuilding an old commit.
Run ID, full commit and reviewed shell SHA-256 are explicit dispatch inputs.
The downloader accepts only successful same-repository master runs of
browser-pages.yml triggered by push/workflow_dispatch. PR/fork, mismatched
commit, failed and unrelated workflow runs are refused. Arguments use structured
subprocess calls and environment fields, not interpolated shell code.

`verify-browser-artifact.py` checks source commit/clean status, manifest identity,
allowlisted shell paths, sizes/digests, WASM identity, embedded worker manifest
and template digest, catalog package paths/sizes/digests, no unexpected files or
symlinks, and nojekyll. It does not execute retained code. Checksums establish
byte identity, not release approval; selecting a reviewed previous-good build
remains an operator responsibility.

The workflow preserves current gh-pages docs while replacing workbench, retains
preservation evidence and uses github-pages review. Normal publication and
rollback share master concurrency. Workflow path triggers cover browser workflow
changes. The checked build runs the new tests. Pages source migration remains
explicit; no remote settings were changed.

## Evidence

`python -m unittest discover -s scripts/tests -p test_browser_artifact.py`
passed seven tests. Synthetic bytes cover accepted clean identity, wrong
commit/hash, dirty source, changed shell/package/worker and unexpected files.
Source selection covers invalid inputs and failed/fork/PR/wrong-workflow or
wrong-commit runs. An initial test found dot-path repository names accepted;
the repository-name boundary was tightened and all tests then passed.

The validator also refused the actual dirty local distribution with its recorded
commit/hash as intended. These foundation bytes are not claimed to be a clean
production rollback artifact. No remote download, dispatch or deployment ran.

## Decision and remaining gates

User selected **Decide after deployed-origin testing** for relay hosting.
Probe the exact HTTPS Pages origin first; allocate provider/owner if blocked.
No relay provider is inferred. Actual retained-artifact rollback, saved-data
compatibility, remote Linux execution, Pages migration/deployment, live access,
offline reopening and representative mobile/resource acceptance remain open.
This adds the rollback path, not production recovery proof.
