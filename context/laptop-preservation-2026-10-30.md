# Legacy laptop checkout preservation — 2026-10-30

The owner requested preservation of the remaining work before retiring this
laptop checkout. This commit captures the legacy fantasy-market, browser/WASM,
native and documentation working tree. It is an archival WIP checkpoint.
It does not adopt these old implementations over the newer default branch,
qualify the unadopted experiments, or waive any VTRACE/resource/release gate.
No build or full native/browser qualification is claimed for this legacy tree.

The current default branch and all locally retained experiment histories are
integrated separately for archival reachability. Ignored snapshots, private
media, benchmark captures and generated build outputs need separate custody
review before deleting the local checkout or its worktrees.
