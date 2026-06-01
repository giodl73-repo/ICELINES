# VTRACE WP-005 Offline/Fetch/Data-Depth Wave

## Scope

Work package: `WP-005` - offline, fetch, and data-depth reliability evidence.

Primary requirements: `REQ-OFFLINE-001`, `REQ-DATA-DEPTH-001`,
`REQ-FRESH-001`, and `REQ-DATA-001`.

Primary validation scenarios: `VAL-005`, `VAL-006`, and `VAL-008`.

## Objective

Prove that ICELINES does not hide missing or unsafe source state with
opportunistic live fetches, unchecked snapshot bytes, or vague data-depth
failure states. Each pulse should close one small boundary with repeatable
evidence and leave broader offline/fetch claims pending until command transcripts
and fixture coverage exist.

## Pulse Log

| Pulse | Scope | Evidence | Status |
|---|---|---|---|
| 01 | Snapshot seal/refusal boundary | `cargo test -p icelines-fetch l0_snapshot_read_named_refuses_unsealed_snapshot --quiet`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --tests -- -D warnings -A clippy::too_many_arguments`; `EVID-WP005-SNAPSHOT-SEAL-L0`; `CHG-057` | passed_with_risk |
| 02 | Offline query smoke | `cargo test -p icelines-cli --test system_tests l2_cmd_no_live -- --nocapture`; `cargo test -p icelines-cli commands::tonight --bin icelines -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-cli --test system_tests --bin icelines --no-deps -- -D warnings`; `EVID-WP005-OFFLINE-SMOKE-L2`; `CHG-058` | passed_with_risk |
| 03 | Shift capability lock/refusal | `cargo test -p icelines-cli --test foster_capability_matrix shifts -- --nocapture`; `cargo test -p icelines-cli --test system_tests l2_foster08_config_set_shifts_favorites_rejected -- --nocapture`; `EVID-WP005-SHIFTS-LOCK-L1`; `CHG-059` | passed_with_risk |
| 04 | Upstream retry failure fixtures | `cargo test -p icelines-fetch --test fetch_retry_l15 -- --nocapture`; `EVID-WP005-FETCH-RETRY-L1`; `CHG-060` | passed_with_risk |
| 05 | Data/fetch command transcript boundaries | `cargo test -p icelines-cli --test system_tests l2_wp005 -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-cli --test system_tests --bin icelines --no-deps -- -D warnings`; `EVID-WP005-DATA-TRANSCRIPT-L2`; `CHG-061` | passed_with_risk |
| 06 | Snapshot integrity/missing-file fixtures | `cargo test -p icelines-fetch snapshot::tests::l0_snapshot --lib -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`; `EVID-WP005-SNAPSHOT-INTEGRITY-L0`; `CHG-062` | passed_with_risk |
| 07 | Chunked snapshot schema drift fixtures | `cargo test -p icelines-fetch snapshot::tests::l0_lindsay_chunked_manifest --lib -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`; `EVID-WP005-CHUNKED-SCHEMA-L0`; `CHG-063` | passed_with_risk |
| 08 | MoneyPuck CSV drift fixtures | `cargo test -p icelines-fetch moneypuck::tests::l0_parse_csv_checked --lib -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`; `cargo clippy -p icelines-cli --bin icelines --no-deps -- -D warnings`; `EVID-WP005-MONEYPUCK-CSV-L0`; `CHG-064` | passed_with_risk |
| 09 | FLETCH cache/refresh fallback fixtures | `cargo test -p icelines-fetch fletch::tests::fetch_generic_http_bytes_uses_cached_object_when_source_unavailable --lib -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --no-deps -- -D warnings`; `EVID-WP005-CACHE-REFRESH-L0`; `CHG-065` | passed_with_risk |
| 10 | Upstream player landing schema drift fixture | `cargo test -p icelines-fetch --test career_landing_mock l1_fetch_player_career_history_surfaces_schema_error -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --test career_landing_mock --no-deps -- -D warnings`; `EVID-WP005-UPSTREAM-SCHEMA-L1`; `CHG-066` | passed_with_risk |
| 11 | ESPN/NHL abbreviation drift fixtures | `cargo test -p icelines-fetch teams::tests::l0_espn_to_nhl --lib -- --nocapture`; `cargo test -p icelines-fetch transactions::convert::tests::l0_convert --lib -- --nocapture`; `cargo test -p icelines-fetch --test transactions_storage l1_bundled_team_abbrevs_all_canonical -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --test transactions_storage --no-deps -- -D warnings`; `EVID-WP005-ABBREV-DRIFT-L1`; `CHG-067` | passed_with_risk |
| 12 | Player landing missing-source fixture | `cargo test -p icelines-fetch --test career_landing_mock l1_fetch_all_career_histories_collects_and_skips -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --test career_landing_mock --no-deps -- -D warnings`; `EVID-WP005-MISSING-SOURCE-L1`; `CHG-068` | passed_with_risk |
| 13 | Career-history partial refresh resume/flag fixture and closeout | `cargo test -p icelines-fetch career_landing::tests::l0_store_partial_refresh_preserves_existing_histories --lib -- --nocapture`; `cargo test -p icelines-fetch --test career_landing_mock l1_fetch_all_career_histories_collects_and_skips -- --nocapture`; `cargo fmt --check`; `cargo clippy -p icelines-fetch --lib --test career_landing_mock --no-deps -- -D warnings`; `EVID-WP005-PARTIAL-RESUME-L0`; `CHG-069` | passed_with_risk |

## Current Status

`WP-005` is `closed_with_risk`. Pulse 01 proves the selected partial-write boundary:
named snapshot reads refuse unsealed snapshots before deserializing existing file
bytes as trusted source state. Pulse 02 proves the selected offline/query
boundary: global `--no-live` blocks live-only schedule fetches while bundled
leaders queries still answer without local cache writes. Pulse 03 records the
existing shift capability lock/refusal evidence for unsupported per-shift data.
Pulse 04 records selected httpmock retry/failure evidence for 429, 503, generic
5xx, non-retryable 4xx, retry budget, and backoff cap behavior. Pulse 05 records
selected command transcript evidence for lockout data install, data-status,
fetch sync, snapshot verify, and no-live fetch boxscore/play-by-play refusals.
Pulse 06 records selected snapshot integrity mismatch and missing-file fixtures.
Pulse 07 records selected chunked snapshot manifest schema compatibility and
newer-schema refusal fixtures. Pulse 08 records selected MoneyPuck CSV
required-column and malformed-row drift fixtures, with the user-facing fetch path
failing loudly before snapshot creation. Pulse 09 records selected generic FLETCH
HTTP cache/refresh fallback evidence: non-forced unavailable-source fetches reuse
verified cached bytes, while forced refresh still fails loudly. Pulse 10 records
selected player landing upstream schema-drift evidence: malformed `200` JSON
without the required `seasonTotals` shape surfaces a schema-related error. Pulse
11 records selected abbreviation-drift evidence: ESPN shorthand, relocation, and
unknown team abbreviations are normalized or surfaced explicitly, and bundled
transaction rows stay canonical across covered seasons with named historical
exceptions. Pulse 12 records selected missing-source evidence: a `404` player
landing response is skipped without panicking or converting absence into trusted
career-history data, while adjacent valid player histories are still collected.
Pulse 13 records selected partial refresh resume/flag evidence: a career-history
partial refresh preserves existing cached player histories, merges successful new
histories, and stamps the refreshed blob while skipped player IDs remain
reported by the batch fetch path.

## Remaining Work

- Broader `data install`, `fetch boxscore --for-favorites`, `data-status`, and
  `snapshot verify` transcript breadth beyond the selected pulse 05 cases is
  accepted WP-008 integration rehearsal residual risk.

## Validation Notes

Affected-slice validation is acceptable only when the pulse records why the
slice covers all touched invariants. Full `VAL-005`, `VAL-006`, and `VAL-008`
remain pending until broader command transcript breadth and broader failure
fixtures are recorded. WP-005 closes with risk after pulse 13; remaining breadth
is carried to WP-008 rather than blocking the fetch/source-state package.

Pulse 01 clippy used an affected-slice command with the pre-existing
`clippy::too_many_arguments` lint in `icelines-fetch/src/fletch.rs` allowed at
the command line. The unallowed package clippy command reaches only that
unrelated historical fetch-helper lint; snapshot read/refusal code and tests pass
under the affected-slice command.
