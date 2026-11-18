# Delivery-method pilot diagnostics

`rinkcraft_annotation_agreement::compare` compares two supplied candidate lists
using exact source-lineage/game/period/actor/component/event-ID keys. It does not
extract hockey events, infer possession, match nearby players/times or validate
an opportunity against a source. Currently supported periods are regulation1–3.
Duplicate coder keys fail; preserve originals/revisions rather than silently
reconciling them. Different lineages or components cannot be forced to match.

Two descriptive denominators stay separate:

- Detection overlap: `2 * matched / (first candidates + second candidates)`.
  An empty union is unavailable, not perfect agreement. Unmatched annotations
  remain in both original directions; neither coder is treated as ground truth.
- Outcome agreement: identical labels divided by matched pairs with both labels.
  Matched pairs without two labels are reported separately. A full9×9 confusion
  matrix preserves technical/choice/opponent/teammate/ambiguous/untested/censored/
  out-of-scope distinctions. No missing label is coerced to failure or censored.

The counterexample has four candidates per coder, two shared, and identical
labels on both shared items:50% detection overlap but100% matched-label agreement.
This is a synthetic method check, not an NHL finding or a reliability coefficient.
The helper does not compare start/end boundary timing, calculate chance-corrected
gamma/kappa, establish independent coders, set a pass threshold or release player
rates/grades. Shared provider anchors may carry shared bias.

```powershell
cargo test -p icelines-core --test rinkcraft_annotation_agreement --locked
cargo build -p icelines-core --example rinkcraft_annotation_agreement --locked
```

```text
rinkcraft_annotation_agreement SYNTHETIC_INPUT_JSON OUTPUT_JSON
```

The offline example requires `dataset_kind: synthetic_method_test`, bounded
to1MiB, with `first` and `second` annotation arrays. It intentionally rejects real
observation datasets; it is not the worksheet importer. Optional outcome values
use the existing nine snake_case labels or null. All numerical computation is
shared Rust. No native/browser/WASM runtime workflow integration is claimed.

Rinkcraft owns DELIVERY-FEAS-01: a prospective exploratory delivery-method design
with separate source/coder/manifest readiness. PUCK owns named participant choices
and dated outputs. Fixed design choices do not mean a ready or completed pilot.
The retained exports currently lack both selected full task denominators; this
is not a survey of all upstream APIs/vendors. No new footage, goal-animation
detector, player assessment, sample observation or simulation change is included.
