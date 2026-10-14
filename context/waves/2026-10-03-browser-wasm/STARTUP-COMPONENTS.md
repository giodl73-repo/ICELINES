# Startup component diagnostics — 2026-10-14

Published5ef015c4 remains unchanged. This local investigation measures the exact
retained master artifact before starting the engine. These are nonfunctional
attribution controls, not release candidates or full-workload acceptance.

| Pair / run order | Startup private peak MiB | Blank MiB |
|---|---:|---:|
| Normal offline preparation first |284.27|239.12|
| Offline registration suppressed second |269.52|236.68|
| Offline registration suppressed first |266.55|233.52|
| Normal offline preparation second |277.10|230.22|
| Modules loaded, application initialization removed first |260.89|234.21|
| Scripts removed from styled markup second |249.67|229.59|
| Scripts removed from styled markup first |252.82|236.18|
| Modules loaded, application initialization removed second |258.70|232.38|

All runs use fresh isolated Chromium, original launch flags and OS sampler,
five seconds idle and no forced GC. Normal controls await verified offline
readiness; suppressed/module/scriptless controls cannot provide the complete
application. All reports explicitly set the250 MiB acceptance comparison to null.
They do not run75 packages, imports, warm queries or restart.

Both opposite-order pairs point toward offline preparation and module loading
contributing to startup allocation. Their differing blank baselines prevent
exact causal byte estimates. Suppressing offline preparation still reaches
266.55–269.52 MiB, so a shell-only optimization has not demonstrated enough
benefit. The rejected verified-shell-body experiment separately failed to
reduce full-workload memory; fewer apparent copies are insufficient evidence.

A supplemental normal startup inventory peaks279.22 MiB, baseline234.68. After
measured phases, CDP reports the workbench page, its service worker and two
browser_ui targets at chrome://omnibox-popup.top-chrome/. SystemInfo identifies
three renderer processes, but does not map targets to their PIDs. No engine
worker exists in any startup run. Do not equate target count to an exact process
allocation, exclude those processes or change acceptance flags.

Next investigate individual renderer/target allocation and the initialization
cost beyond loading modules. A future fix must preserve offline functionality,
the original full workload and the250 MiB whole-browser definition. Physical
Android remains explicitly deferred. Full validation remains incomplete.

Byte-preserved reports and explicit sibling SHA references are in
evidence/startup-components-provenance-20261014.json. The ignored local helper
and sample-file hashes allow attribution scope to be checked separately from
production source. No PR or publication is proposed for these controls.
