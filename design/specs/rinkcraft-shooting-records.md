# Shooting records and candidate calibration

`rinkcraft_shooting_records` tests a second observational family after seasonal
skating. It consumes the frozen EDGE-enriched preview and the checksummed exposure
inventory. Every profile is verified before use; actor/window/group/GP and
all-strength scope must bind. Goalies are outside this analysis.

## Three separate constructs

- Peak shot speed: positive maximum in mph from the matched EDGE response.
- Shot volume: official shots on goal × 60 / official all-strength TOI minutes.
- Raw conversion: official goals × 100 / official shots on goal, only with
  nonzero shots and goals <= shots. Missing/invalid ratios remain unknown;
  zero goals remain zero. This is an audit outcome, not a finishing estimate.

The existing Rust finishing experiment is retained. Its conditional historical
model requires verified provider xG, historical exposure, calibration metadata
and uncertainty. Official goals/SOG and a maximum velocity do not supply those
inputs. This new analysis neither replaces that model nor completes Shooting.

## Observational family

| Candidate | Peak shot-speed midrank | SOG/60 midrank |
| --- | --- | --- |
| Shot Power | >= P90 | < P90 |
| Shot Volume | < P90 | >= P90 |
| Power & Volume | >= P90 | >= P90 |

These are authored research thresholds and provisional names. All-strength
comparisons stay within season and position group, using midrank ties including
the subject. Both measured metrics and >=20 measured peers per metric are required
for the exclusive family, with >=10 GP and >=100 minutes. Missing is Unassessed;
insufficient exposure is distinct. At most one candidate occurs per player/window.
No production awards, ability scores, negative feats or simulation bonuses occur.

SOG are not all attempts; TOI is not equivalent shooting opportunity time; maximum
velocity is not release timing, placement, decision quality or repeatable power.
A high percentile is relative to its own measured reference, not interchangeable
between positions or with NHL-provided percentiles. These observations can overlap
existing shot-generation feats; later presentation must avoid double-counting the
same underlying shots as independent corroboration or additive game bonuses.

## Conversion sensitivity and source consistency

Conversion is compared separately at minimum 10, 25 and 50 SOG, retaining the same
GP/TOI eligibility. These are sensitivity settings, not validated reliability
thresholds. The conversion audit reports small peer sets rather than hiding their
collapse; the candidate family's >=20-peer floor does not certify conversion ranks.
Its thresholds never award this family or a finishing feat. Classifications are
verified unchanged across the three conversion gates.

Official counts remain the source for volume and conversion. The EDGE all-location
counts and capture time are carried separately and cross-checked without altering
either source. Matching GP does not prove identical capture clocks, events or
provider coverage. Count differences are explicit and need reconciliation before
any claim of fully synchronized joint observations. No causal interpretation or
permanent player trait follows from these provisional combinations.

Consecutive-season overlap includes only the same players and position groups
assessed in both snapshots. It is descriptive overlap, not prospective validation,
corrected reliability or evidence of ability change. The current season is partial.

## Replay and validation

```text
cargo test -p icelines-sources --example rinkcraft_shooting_records --locked
cargo run -p icelines-sources --example rinkcraft_shooting_records --locked -- BASE_DIRECTORY EXPOSURE_JSON EXPOSURE_SHA256 NEW_REPORT_JSON
python tools/verify_rinkcraft_shooting_records.py BASE_DIRECTORY EXPOSURE_JSON REPORT_JSON NEW_VERIFICATION_JSON
```

The Rust tests cover zero/missing/invalid conversion, count domains/units, midrank
ties, P90 boundaries, exclusive combinations and minimum peer requirements.
The independent Python oracle recomputes all source counts, rates, positional
reference membership, midranks, classifications, census, sensitivity, snapshot
overlap and source hash linkage. Copied display floats use 1e-12 round-trip
tolerance. Three focused tests passed; 3,941 input hashes and 24,289 ranks were
independently verified over 3,565 skater records and three conversion gates.
Dated findings and player examples are retained in PUCK and the research packet.
The final report adds both provider counters and EDGE capture timestamps to the
initial analysis. Previous development reports remain outside the deliverable.
No native, browser, WASM or hosted preview assets changed in this step.
