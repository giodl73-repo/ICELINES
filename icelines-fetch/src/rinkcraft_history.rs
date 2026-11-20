use icelines_core::view_model::rinkcraft_history::{self, NumericalHistory};
use std::path::Path;
pub fn load(path: &Path) -> anyhow::Result<NumericalHistory> {
    anyhow::ensure!(
        std::fs::metadata(path)?.len() <= 8 * 1024 * 1024,
        "history exceeds 8 MiB"
    );
    rinkcraft_history::parse_source(&std::fs::read_to_string(path)?).map_err(anyhow::Error::msg)
}
