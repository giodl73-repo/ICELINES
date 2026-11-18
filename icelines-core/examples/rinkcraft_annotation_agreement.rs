//! Method demonstration only; intentionally refuses real-observation datasets.
use icelines_core::view_model::rinkcraft_annotation_agreement::{compare, Annotation};
use serde::Deserialize;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    dataset_kind: String,
    first: Vec<Annotation>,
    second: Vec<Annotation>,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err(
            "Usage: rinkcraft_annotation_agreement SYNTHETIC_INPUT_JSON OUTPUT_JSON".into(),
        );
    }
    if std::fs::metadata(&args[0])?.len() > 1024 * 1024 {
        return Err("Synthetic input exceeds1MiB".into());
    }
    let input: Input = serde_json::from_str(&std::fs::read_to_string(&args[0])?)?;
    if input.dataset_kind != "synthetic_method_test" {
        return Err(
            "Only synthetic method tests accepted; real observation import is not implemented"
                .into(),
        );
    }
    std::fs::write(
        &args[1],
        serde_json::to_vec_pretty(&compare(&input.first, &input.second)?)?,
    )?;
    Ok(())
}
