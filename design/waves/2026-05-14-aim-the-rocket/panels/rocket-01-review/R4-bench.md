# R4 Review - bench

## Findings

### F-01 - WARN: Parser expansion needs source-shaped fixtures
File: `icelines-fetch/src/nhl_api.rs`
Finding: The next pulse will add shot-event parsing from official play-by-play shapes.
Consequence: Without known-shape fixtures, a parser could accidentally count goals twice, drop blocked shots, or require optional fields.
Fix: Add L0 tests for `goal`, `shot-on-goal`, `missed-shot`, `blocked-shot`, and missing-coordinate variants, with expected Corsi/Fenwick/SOG counts calculated in comments.

### F-02 - WARN: Cache projection needs an L1 round-trip
File: `icelines-fetch/src/datastore.rs`
Finding: The scoring provider will read raw `DataKind::PlayByPlay` from the manifest-backed store.
Consequence: Unit-only parser tests would miss manifest path/key regressions.
Fix: Add an L1 tempdir test that writes raw play-by-play JSON, manifests it as `DataKind::PlayByPlay`, and projects typed scoring events through the same DataStore path used by web/admin loading.
