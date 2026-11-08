//! Optional, explicitly configured Rinkcraft research catalog. No discovery or network fetch.
use icelines_core::view_model::rinkcraft_character::CharacterCatalog;
use std::path::Path;
pub fn load_character_catalog(path: &Path) -> Result<CharacterCatalog, String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 128 * 1024 {
        return Err("character catalog exceeds 128 KiB".into());
    }
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let c: CharacterCatalog = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    c.validate()?;
    Ok(c)
}
