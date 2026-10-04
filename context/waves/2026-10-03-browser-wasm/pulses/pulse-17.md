# Pulse 17 — Reproducible query exports and safe CSV text

Status: query export contract implemented and checked; full browser file
interchange and release gates remain open.

## Change

JSON query exports retain the existing result fields and add export schema 1,
the exact completed request (filter, sort, player kind, requested GP floor and
injected query date), and methodology disclosures. Existing result metadata
carries season/type, package revision, source, observation/fetch timestamps,
missing sources and effective/pace sample floors. Numeric values and engine row
order remain unchanged. Query exports describe a result; package exports remain
the importable season/schedule backups.

The browser snapshots the request together with the accepted query response.
Changing unsubmitted UI controls cannot change the metadata attached to exported
rows. CSV includes the same metadata as single-field comment records whose
values are JSON encoded. Source/filter line breaks cannot create extra records.
All row fields use CSV quoting and escaped quotes. Potential spreadsheet
formulas in strings, including leading whitespace/control cases, receive an
apostrophe; numeric values remain numeric, zero remains zero and null is empty.
JSON preserves the original text. Methodology records disclose that CSV text
transformation, descriptive pace, missing-value semantics, full precision and
absence of historical era adjustment.

## Evidence

`npm test` passed **99 tests**. Four new export cases use an independent CSV
record reader to verify field boundaries, quotes, commas, accents, embedded
line breaks, dangerous prefixes, null/zero/full precision, metadata completeness,
explicit-zero versus default floors, goalie context, empty output and ordering.

`node scripts/verify-browser-dist.mjs` passed **20 shell assets**, four count
goldens and all **75 packages** through actual generated WASM. Largest shell
plus one-package gzip sum is **396,659 bytes**. `git diff --check` passed.

Actual local browser at `http://127.0.0.1:8056/` used the shared worker/WASM to
run `p>=100`, Points, skaters, default GP. Six result rows were accepted. Controls
were then changed to `p>=999` and Goals without submitting. Both production
export buttons emitted the original six-row query with filter `p>=100`, sort
`points`, effective GP 0, pace floor 10, unknown observation time and revision
`ba4f8b9ab5e8882652099025360d8409b214a917d03ccc3c326a603a7ab86479`.

The unpublished harness observes the Blob passed to `URL.createObjectURL` and
retains its text in a read-only DOM control. It calls the original URL API and
does not replace production download generation. This proves UI-to-export
payload wiring, not a completed browser file download. Captured bytes:

- [JSON result](browser-pulse-17-query.json)
- [CSV result](browser-pulse-17-query.csv)
- [UI evidence](browser-pulse-17.png)

No live request or data upload was required. Query controls were restored after
verification. Existing save policies and package formats are unchanged.

## Remaining acceptance

Actual browser download/file-picker round-trip, deployed live acquisition or
allocated relay, offline reopen, native visibility/offline behavior, populated
legacy migration, mobile/resource acceptance and Pages deployment/rollback
remain open. This pulse addresses query export context and CSV injection;
it does not claim completion of the entire export/import release gate.
