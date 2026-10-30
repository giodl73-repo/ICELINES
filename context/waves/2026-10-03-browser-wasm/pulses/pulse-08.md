# Browser WASM pulse 08 — private bookmarks and explicit public links

Date: 2026-10-03. Parent: WP-BW-01 / REQ-BROWSER-001 / CHG-119.

## Implementation

Added a bounded allowlisted hash-view contract for leaders: season/type,
skater/goalie kind, compatible sort, default or explicit GP, public/local data
reference and explicitly shared public filter. Unsupported paths, duplicate or
unknown parameters, malformed seasons, lockout, wrong player sort and oversized
values are rejected. Arbitrary network URLs cannot enter acquisition through
the route. Public package URLs still come only from the verified catalog.

Startup and hash navigation restore all view controls and load the requested
public season; an unrelated saved-active dataset cannot override it. A matching
saved public package may avoid the download, but its bytes must match the public
catalog digest. Local view links use only a local marker and season/type: no
record IDs, package names, private hashes, data or filters. Restoration selects
the compatible active saved local copy, or a single unambiguous matching record.
Otherwise the UI offers loading/importing without substituting public data.

Ordinary successful queries omit filter text from bookmarks. Share this view
creates a reviewable URL and defaults to omitting filter text. Public filter
inclusion requires its explicit checkbox; local filters cannot be included.
Generated links use the current deployment path and omit unrelated URL search
parameters. An incoming public filter remains in its link across reload; an
ordinary later query returns to filter-free bookmarks. Link state captures the
applied query rather than unsubmitted control edits. Changing queries clears
the previous generated link and its filter-sharing choice.

## Verification

`npm test --prefix icelines-browser` passed 55 Node tests (eight new route/privacy
tests). TypeScript build passed. The final distribution verifier loaded/queried
all 75 public packages through real WASM and verified 16 shell assets.

Actual browser at `http://127.0.0.1:8052/ICELINES/` proved:

- Explicit 20242025 playoff / goalies / GAA / GP zero loaded 27 rows, and all
  controls and context survived reload under the project subpath.
- Default generated link omitted the applied filter. Explicit inclusion created
  a URL-encoded public filter; following/reloading `goalie-games>=10` restored
  the exact filter and eight qualifying goalie rows.
- With the public playoff season saved, a same-context `data=local` link showed
  the missing-local load/import message, unloaded results and disabled exports.
  It did not substitute the saved public dataset.
- A subsequent explicit regular-season link took precedence over that saved
  playoff pointer, returning six `p>=100` skaters. Its default generated share
  URL omitted filter text. [Browser evidence](browser-pulse-08.png).

Session 92672 serves the project-path preview; browser tab 9 is marked for
continuation. Final static goalie-language help was added after that screenshot
and passed the subsequent build/test/verification run.

Acceptance exposed an existing shared-query convention: generic `gp` is the
skater Games stat and is inapplicable to goalies; the native evaluator accepts
inapplicable stat atoms rather than applying them. `goalie-games` correctly
filters goalie GP. The browser now explains this convention; no independent
JavaScript rewrite or native engine semantic change was introduced.

## Open gates

A full local imported-data deep-link round-trip and competing hash/acquisition
transitions still need browser evidence. Deployed live paths, offline/update
acceptance, complete worker payload contracts, storage failures/migrations,
native web/CLI ordering reconciliation, Pages execution/publication/rollback
and measured performance budgets remain open. The full goal remains active.
