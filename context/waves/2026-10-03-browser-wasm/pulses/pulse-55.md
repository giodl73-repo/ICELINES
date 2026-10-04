# Browser WASM pulse 55 — deployment policy and current artifact composition

Date: 2026-10-04. Previous turn made progress: dated desktop failure evidence,
latest implementation CI/artifact verification, and scoped evidence commit
`711c6c129cd28016db40bd482ff20cfe88e45568`. Goal remains active.

Revalidated this evidence commit's browser run 37217923341 and repository run
37217923317 as in progress, with no completed failed jobs. Existing live runs
were not restarted. The implementation checkpoint's successful runs remain
separate from this new head's pending checks.

Read-only GitHub checks found:

- Pages still uses legacy `gh-pages:/` publishing.
- Master remains protected at `935136020140bd5b408d26cbb0777dd6f0fb5ef9`.
- The `github-pages` environment has custom deployment branch restrictions;
  its only rule is `gh-pages` (policy ID 47950940).
- No required-reviewer rule was returned. The master branch requires linear
  history; required PR reviews/status checks were null in the API response.
- `gh-pages` remains at `79dfba2eeef4aa478955d53558bbaef83f205016`.

The prepared Pages workflow's deploy job runs from master, so the existing
environment policy would reject it. Amended the rollout runbook to require an
approved additive master allowance, preserving gh-pages and branch restrictions.
No remote environment, branch protection or Pages configuration was changed.

Verified the existing documentation checkout is clean at that exact current
gh-pages commit. Composed it with the verified retained implementation artifact
from run 37216513390 into a fresh `target/browser-pages-site-55` tree. Hash checks
preserved all 95 documentation files and added 102 browser files under workbench.
Root index SHA-256 remains
`cb48f1f72ac9ae2c75a4362051da61b2260caaffb4946ccf11d366e3258b1628`.
Inventory: `target/browser-pages-preservation-55.json`. This local composition
is preparation evidence, not an actual master deployment or remote rollback.

Remaining rollout decision is concrete: after review/checks, merge draft PR #74
through the protected branch's normal process, switch Pages to Actions, add the
master environment branch allowance, and dispatch a validation publication.
The runbook requires deliberate approval for that deployment; none is inferred
from CI success or the relay timing answer. Exact-origin stats/schedule tests,
physical device/assistive acceptance, full peak memory and remote update/rollback
still remain. Relay host/owner selection follows deployed-origin testing.
