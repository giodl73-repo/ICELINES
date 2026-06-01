use anyhow::{Context, Result};
use icelines_core::{
    parse_experience_id, parse_pane_binding_id, parse_workbench_id, WorkbenchLayoutRecord,
    WorkbenchLayoutStore,
};

use crate::{cli::LayoutSubcommand, config::Config};

pub fn run(cmd: LayoutSubcommand, cfg: &Config) -> Result<()> {
    let path = cfg.layout_store_path();
    match cmd {
        LayoutSubcommand::List => {
            let store = WorkbenchLayoutStore::load_from_path(&path)
                .with_context(|| format!("load layout store {}", path.display()))?;
            for layout in &store.layouts {
                let experience = layout.experience.as_deref().unwrap_or("-");
                println!(
                    "{}\tcenter={}\tleft={}\tright={}\texperience={}",
                    layout.name, layout.center, layout.left, layout.right, experience
                );
            }
            Ok(())
        }
        LayoutSubcommand::Show { name } => {
            let store = WorkbenchLayoutStore::load_from_path(&path)
                .with_context(|| format!("load layout store {}", path.display()))?;
            let layout = store.get(&name)?;
            println!("{}", serde_json::to_string_pretty(layout)?);
            Ok(())
        }
        LayoutSubcommand::Save {
            name,
            center,
            left,
            right,
            experience,
        }
        | LayoutSubcommand::Update {
            name,
            center,
            left,
            right,
            experience,
        } => {
            let mut store = WorkbenchLayoutStore::load_from_path(&path)
                .with_context(|| format!("load layout store {}", path.display()))?;
            let record = WorkbenchLayoutRecord::new(
                &name,
                parse_workbench_id(&center)?,
                parse_pane_binding_id(&left)?,
                parse_pane_binding_id(&right)?,
                experience.as_deref().map(parse_experience_id).transpose()?,
            )?;
            store.upsert(record)?;
            store
                .save_to_path(&path)
                .with_context(|| format!("save layout store {}", path.display()))?;
            println!("saved layout {name}");
            Ok(())
        }
        LayoutSubcommand::Delete { name } => {
            let mut store = WorkbenchLayoutStore::load_from_path(&path)
                .with_context(|| format!("load layout store {}", path.display()))?;
            let normalized = icelines_core::normalize_layout_name(&name)?;
            let before = store.layouts.len();
            store.layouts.retain(|layout| layout.name != normalized);
            if before == store.layouts.len() {
                anyhow::bail!(icelines_core::WorkbenchLayoutError::MissingLayout(
                    normalized
                ));
            }
            store
                .save_to_path(&path)
                .with_context(|| format!("save layout store {}", path.display()))?;
            println!("deleted layout {name}");
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn l0_layout_cli_save_and_show_round_trip() {
        let path = temp_layout_path("cli-round-trip");
        let mut store = WorkbenchLayoutStore::default();
        store
            .upsert(
                WorkbenchLayoutRecord::new(
                    "tonight",
                    parse_workbench_id("scores").unwrap(),
                    parse_pane_binding_id("favorites-left").unwrap(),
                    parse_pane_binding_id("schedule-right").unwrap(),
                    Some(parse_experience_id("tonight-bench").unwrap()),
                )
                .unwrap(),
            )
            .unwrap();
        store.save_to_path(&path).unwrap();

        let restored = WorkbenchLayoutStore::load_from_path(&path).unwrap();
        let layout = restored.get("tonight").unwrap();

        assert_eq!(layout.center, "scores");
        assert_eq!(layout.experience.as_deref(), Some("tonight-bench"));
        let _ = std::fs::remove_file(path);
    }

    fn temp_layout_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("icelines-layout-command-{label}-{nonce}.json"))
    }
}
