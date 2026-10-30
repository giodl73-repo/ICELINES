# Browser WASM pulse 10 — saved package integrity

Date: 2026-10-03. Parent: WP-BW-03 / REQ-BROWSER-001 / CHG-119.

## Implementation

The library validates saved storage envelopes before returning their records or
committing a new snapshot. IDs, digest shape, byte type/size, policy, required
package fields and schema version are checked. SHA-256 integrity is checked
before UTF-8 decoding and JSON parsing. Stored header metadata must match the
package bytes. Player arrays and display metadata are reconstructed from those
verified bytes rather than trusting duplicate stored metadata. Domain rows,
units, season validity and hockey invariants remain the shared Rust engine's
responsibility before activation.

Validation is sequential across library records to avoid parallel decoding of
all historical packages. Failed validation rejects access without deleting,
resetting or silently migrating original records. Failed replacements cannot
change the old package or active pointer. The active pointer itself has a shape
check. Denied BroadcastChannel construction degrades to no cross-tab messaging
instead of preventing application module startup.

## Evidence

The TypeScript build and all 66 browser Node tests passed after the final source
change. Two subsequent test improvements exercised invalid UTF-8 with a matching
digest and a corrupt active pointer; all 17 library tests passed with those
changes. Distribution verification checked 16 shell assets and loaded/queried
all 75 catalog packages through actual WASM.

New fixtures cover byte/digest corruption, metadata-context substitution,
untrusted duplicate arrays, future package schema, unknown package fields,
malformed policy, failed replacement preserving old state, original corrupt
records remaining in the database, and denied messaging construction.

The local 8053 storage harness exercised actual IndexedDB and WASM:

- The validation build restored the previously saved 20242025 regular package.
- A harness control changed only the saved record's digest. Retry refused the
  record and removed saved-state claims; all 905 active rows and package export
  remained available.
- Restoring the original record and retrying recovered Saved locally without
  reimporting or resetting the database.

[Screenshot](browser-pulse-10.png). The corruption controls and backup are local
test harness code, outside the published dist. The screenshot precedes the final
integrity-before-decoding reorder, which retained the same UI behavior and passed
the final deterministic suite. Harness index changes intentionally fail offline
shell integrity, so the screenshot does not establish offline behavior.

## Remaining acceptance

This establishes envelope integrity and refusal, not all migration/storage
gates. There is no supported older saved schema needing migration yet; future
versions are refused. Interrupted migration/transactions, actual blocked opens
and version-change coordination, storage-loss recovery, and library memory
measurements remain open. A malformed saved record currently makes the saved
library unavailable as a whole; the application continues using memory and
public packages. Recovery/quarantine for individually invalid records is not
claimed. Live stats and scores/schedule, deployed live access, offline opening,
mobile/keyboard acceptance, performance and Pages deployment/rollback also remain
required by the plan.
