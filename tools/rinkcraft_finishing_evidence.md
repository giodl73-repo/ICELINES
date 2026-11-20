# Optional finishing evidence in player dossiers

The shared numerical-history selector can attach an experimental finishing
readout. Both native and WASM consumers call that selector; TypeScript renders
its results without calculating a model or assigning a score.

Add optional `finishing_research` to an explicitly supplied numerical-history
asset. The value follows `rinkcraft_finishing_evidence::ResearchInput`:

- schema `icelines.finishing-research-input.v1`;
- bounded model ID, model-config SHA256 and fixed finishing Model;
- interval probability and declared minimum target games/minutes;
- source records with season, URL, source SHA256 and byte count;
- unique target player/season records, each with Target and earlier HistoryRows.

The input contains no target-goal outcomes. It is authored research data, not an
automatically fitted production model. The code validates consistency with the
supplied statistical history; provenance must still establish the fitting method
and authentic source lineage. A config hash alone does not validate a model.

Without this optional input, the previous numerical card is unchanged. Malformed
research is represented by an unavailable readout and leaves ordinary statistics
available. Existing malformed *whole history JSON* still fails normal parsing;
this is not recovery from an unreadable statistical asset.

## Selection gates

The selector requires an exact selected player/season, five-on-five skater scope,
matching position group and declared target qualification. No independent
finishing fallback to a different season or player is permitted. A source
fingerprint must match the corresponding statistical-history source. Selected
target xG and every earlier goals/xG row must agree with those exports. Available
earlier source rows cannot silently be omitted. Target goals are never read by
the estimator; the Target type has no such field.

It then calls the verified Rust posterior and interval functions. The readout is
position-relative: baseline1, posterior mean and equal-tailed conditional interval.
It records history/calibration seasons, historical goals/xG, source fingerprints,
model identity and limits. The normalized-input SHA256 hashes the typed research
input serialized by serde_json, not the original asset's whitespace or JSON
number spelling. File hashes are separately recorded in study provenance.

States: `experimental`, `baseline_only`, `missing`, `unavailable`, and
`not_applicable`. No informative earlier exposure returns a visibly labeled
model baseline with uncertainty, not a low skill grade. Goalies receive explicit
non-applicability for this skater model. No complete Shooting,1–20 attribute,
role classification or goalie estimate is inferred.

## Consumer boundary

The browser puts the component in the closed dossier, outside the compact
attribute rail. Native Askama uses the same readout. Print keeps the compact
single-page sheet and omits expanded dossier contents. Sources and model
fingerprints remain inspectable on screen. Headline attributes stay unrated.

Native can receive the asset through existing `ICELINES_RINKCRAFT_HISTORY`.
Browser/WASM receives an explicitly supplied history asset; old data still work.
No new WASM binding was needed. A native offline projection is also available:

```powershell
cargo run -p icelines-core --example rinkcraft_profile_snapshot --locked -- HISTORY.json PLAYER_ID SEASON OUTPUT.json
cargo test -p icelines-core --test rinkcraft_finishing_evidence --locked
```

The dated pilot supplies the frozen2023–24 model with10GP/300 five-on-five-minute
target qualification and90% intervals. These are inherited research settings,
not independently established reliability thresholds. Prior/provider/context and
game/team uncertainty remain outside the conditional interval. No hosted build,
model-policy replacement or numerical ability release follows from integration.


## Inspectable posterior support

Optional `support` exposes a sorted ledger of earlier-season goals/provider xG,
historical position/factor and adjusted exposure. Shared Rust also supplies the
position-residual prior/posterior shape and **rate**, adjusted history exposure,
and history weight in the posterior mean. Old readouts without support still
load. Invalid/missing research and goalie non-applicability have no support ledger.

For residual prior shape/rate `a,a`, earlier goals `G` and adjusted exposure `E`:
posterior is Gamma(`a+G`,`a+E`), mean `(a+G)/(a+E)`; history weight is `E/(a+E)`.
For `E>0`, the mean equals `(1-weight)*1 + weight*(G/E)`. For no exposure the
weight is zero and the baseline remains1. This is an algebraic model weight,
not an empirical reliability/confidence measure, independent evidence fraction,
validated talent estimate, whole Shooting score or1–20 mapping. The global
Gamma rate is not the position-residual prior rate when they differ. Each
historical position determines its own factor; target goals remain excluded.

The collapsed ledger is in the closed dossier in both native and browser
presentations. It does not change the compact front, original prediction or
interval calculation. Frozen/model/context uncertainty and release gates remain.
