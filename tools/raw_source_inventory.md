# Bounded raw-source receipts

`icelines_sources::raw_inventory::inventory(format, raw)` records observed JSON
paths or CSV columns without typed profile projection. Native offline usage:

```text
raw_source_inventory json|csv INPUT_FILE OUTPUT_JSON
```

Inputs are explicit UTF8 files, at most32MiB. No network, cache/config discovery,
recursive directory scan or metadata-path opening is performed by this tool.
Output includes raw-byte SHA256/length, structural counts and literal event-tag
labels; other field values are not copied into the receipt. Caller selects appropriate public inputs and keeps
source context/rights/provenance separately.

JSON paths use JSON-quoted bracket keys and normalized array `[*]` segments;
object keys cannot collide with wildcard notation. Occurrences, explicit nulls
and JSON kinds remain separate. Absent paths do not gain synthetic null/zero
counts. JSON traversal is bounded to64 levels,2M nodes and10K distinct paths.
If a `plays` array exists, observed `typeDescKey` tags and missing/non-string tag
counts are reported as literal structure, not interpreted hockey ontology.

CSV retains header order, row count and per-column literal empty/nonempty
strings. Numeric zero and the word `null` remain nonempty values. Duplicate/
blank headers and inconsistent records fail rather than silently collapse fields.
No numerical statistics, units, opportunity rate or actor role is inferred.

```powershell
cargo test -p icelines-sources --test raw_inventory --locked
cargo build -p icelines-sources --example raw_source_inventory --locked
```

Inventory success is not semantic coverage, source authenticity or completeness.
An observed timestamp/event ID does not establish a complete attempt universe;
an absent example key is not proof that every provider/version lacks that concept.
Use full key/tag receipts and direct source inspection before concluding whether
a task input is present. Different source windows and retrospective selection
must not silently enter a prospective pilot or complete-career claim.

Named retained source audits, hashes and generated reports belong to PUCK.
Authored task/source requirements belong to Rinkcraft. Only neutral parsing,
validation and fixtures live here. No real observation importer, automatic
possession/duty/class/capability inference, source refresh or simulator change.
