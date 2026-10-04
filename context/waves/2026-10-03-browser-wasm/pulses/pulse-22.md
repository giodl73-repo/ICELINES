# Browser WASM pulse 22 — actual file interchange

Dates: 2026-10-03 through 2026-10-04. REQ-BROWSER-001 / WP-BW-03/05
selected acceptance evidence. Previous turn was progress: bounded package
transport, tests and browser proof. Goal remains active.

## Authoritative observations

The pulse-21 server handle 23699 was polled and remained running. In-app browser
tab 22 at `http://127.0.0.1:8060/` had the real 2023-24 regular package loaded
and the `p >= 100` query showing nine players. The disposable transport fixture
does not alter production export/import, worker, package bytes or local storage.

1. Clicked production **Export package**. The browser download-event observer
   timed out after 10 seconds; nevertheless, filesystem inspection found the
   newly written `C:/Users/giodl/Downloads/icelines-20232024-regular.json`.
   It contains 909,591 bytes, schema 1, season 20232024, type regular, source
   repository:20232024. SHA-256 matches the catalog and published bytes exactly:
   `84adc3c1c4791483adcb2ca9c8758334c56059aa95585ee2880773f9feb03adc`.
2. Copied those unchanged downloaded bytes into
   `target/browser-pulse-22-exported-package.json` for the file-picker test.
3. Clicked **Unload from memory**. Header changed to Choose a dataset and
   status confirmed saved copies remain in the library.
4. Opened the production import file chooser and selected that exported file.
   The chooser tool took an extended interval to return across the date change;
   no claim of prompt unattended picker automation is made. Its returned UI
   showed **Imported into memory. Save locally to keep it.**, season 20232024,
   and the same nine-row query as before unload. Saved library stayed at one
   (the earlier 2024-25 baseline); the imported season was not auto-saved.
5. Clicked production **Export JSON** and **Export CSV**. Newly written files
   in Downloads were 4,167 and 1,932 bytes respectively, dated 2026-10-04.

`scripts/verify-browser-file-roundtrip.py` independently parses the actual
downloaded JSON/CSV with Python's JSON and CSV readers. It verifies package
length/digest against the catalog, result season/type/revision/filter/sort,
GP and pace floors, unknown observation timestamps, the ordered nine-player
name/point fixture, every CSV field against JSON (including null and full
numeric precision), and CSV provenance/filter metadata. It passed:

```text
python scripts/verify-browser-file-roundtrip.py C:\Users\giodl\Downloads icelines-browser/dist/catalog.json
package_bytes=909591; query_rows=9; json_csv_agree=true
```

[Imported session and export controls](../screenshots/pulse-22-package-imported.png).

## Scope and remaining acceptance

This closes the actual download and package reimport evidence gap for this
regular-season slice, beyond pulse 17's Blob wiring. No production code changed;
the new verification script reads explicit local files and performs no network
requests. It does not claim arbitrary archive/playoff/schedule interchange,
mobile file-picker coverage, permanent browser storage, offline reopening,
deployed live CORS, migration runtime, full scoped native parity/resources, or
Pages deploy/update/rollback. Those broader release gates remain open.
