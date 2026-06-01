use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    workbench_entry, workbench_experience, workbench_pane_binding, WorkbenchExperienceId,
    WorkbenchId, WorkbenchPaneBindingId, WorkbenchSurface, WorkbenchZone, WORKBENCH_CATALOG,
    WORKBENCH_EXPERIENCES, WORKBENCH_PANE_BINDINGS,
};

pub const WORKBENCH_LAYOUT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, thiserror::Error)]
pub enum WorkbenchLayoutError {
    #[error("layout name cannot be empty")]
    EmptyName,
    #[error(
        "layout name '{0}' contains unsupported characters; use letters, numbers, '-', '_', or '.'"
    )]
    InvalidName(String),
    #[error(
        "unsupported workbench layout store version {found}; supported version is {supported}"
    )]
    UnsupportedStoreVersion { found: u32, supported: u32 },
    #[error(
        "unsupported workbench layout record version {found}; supported version is {supported}"
    )]
    UnsupportedRecordVersion { found: u32, supported: u32 },
    #[error("unknown workbench slug '{0}'")]
    UnknownWorkbench(String),
    #[error("workbench '{0}' is not layout-restorable")]
    UnsupportedWorkbench(String),
    #[error("unknown pane binding slug '{0}'")]
    UnknownPaneBinding(String),
    #[error("pane binding '{slug}' is not supported in {zone:?}")]
    UnsupportedPaneZone { slug: String, zone: WorkbenchZone },
    #[error("pane binding '{slug}' does not support {surface:?}")]
    UnsupportedPaneSurface {
        slug: String,
        surface: WorkbenchSurface,
    },
    #[error("unknown experience slug '{0}'")]
    UnknownExperience(String),
    #[error("experience '{slug}' does not support {surface:?}")]
    UnsupportedExperienceSurface {
        slug: String,
        surface: WorkbenchSurface,
    },
    #[error("experience '{experience}' centers on '{expected}' but layout centers on '{actual}'")]
    ExperienceCenterMismatch {
        experience: String,
        expected: String,
        actual: String,
    },
    #[error("layout '{0}' not found")]
    MissingLayout(String),
    #[error("cannot read layout store {}: {source}", path.display())]
    ReadStore {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid layout store JSON at {}: {source}", path.display())]
    CorruptStore {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("cannot create layout store directory {}: {source}", path.display())]
    CreateStoreDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("cannot write layout store {}: {source}", path.display())]
    WriteStore {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("cannot replace layout store {}: {source}", path.display())]
    ReplaceStore {
        path: PathBuf,
        source: std::io::Error,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchLayoutStore {
    pub version: u32,
    #[serde(default)]
    pub layouts: Vec<WorkbenchLayoutRecord>,
}

impl Default for WorkbenchLayoutStore {
    fn default() -> Self {
        Self {
            version: WORKBENCH_LAYOUT_SCHEMA_VERSION,
            layouts: Vec::new(),
        }
    }
}

impl WorkbenchLayoutStore {
    pub fn load_from_path(path: impl AsRef<Path>) -> Result<Self, WorkbenchLayoutError> {
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self::default());
        }
        let text =
            std::fs::read_to_string(path).map_err(|source| WorkbenchLayoutError::ReadStore {
                path: path.to_path_buf(),
                source,
            })?;
        let store: Self =
            serde_json::from_str(&text).map_err(|source| WorkbenchLayoutError::CorruptStore {
                path: path.to_path_buf(),
                source,
            })?;
        store.validate()?;
        Ok(store)
    }

    pub fn save_to_path(&self, path: impl AsRef<Path>) -> Result<(), WorkbenchLayoutError> {
        let path = path.as_ref();
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| {
                WorkbenchLayoutError::CreateStoreDir {
                    path: parent.to_path_buf(),
                    source,
                }
            })?;
        }
        let text =
            serde_json::to_string_pretty(self).expect("layout store serialization is infallible");
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text).map_err(|source| WorkbenchLayoutError::WriteStore {
            path: tmp.clone(),
            source,
        })?;
        std::fs::rename(&tmp, path).map_err(|source| WorkbenchLayoutError::ReplaceStore {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    }

    pub fn validate(&self) -> Result<(), WorkbenchLayoutError> {
        if self.version != WORKBENCH_LAYOUT_SCHEMA_VERSION {
            return Err(WorkbenchLayoutError::UnsupportedStoreVersion {
                found: self.version,
                supported: WORKBENCH_LAYOUT_SCHEMA_VERSION,
            });
        }
        for layout in &self.layouts {
            layout.validate_for_surface(WorkbenchSurface::Tui)?;
            layout.validate_for_surface(WorkbenchSurface::Web)?;
        }
        Ok(())
    }

    pub fn upsert(&mut self, record: WorkbenchLayoutRecord) -> Result<(), WorkbenchLayoutError> {
        record.validate_for_surface(WorkbenchSurface::Tui)?;
        record.validate_for_surface(WorkbenchSurface::Web)?;
        if let Some(existing) = self
            .layouts
            .iter_mut()
            .find(|existing| existing.name == record.name)
        {
            *existing = record;
        } else {
            self.layouts.push(record);
            self.layouts.sort_by(|a, b| a.name.cmp(&b.name));
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Result<&WorkbenchLayoutRecord, WorkbenchLayoutError> {
        let normalized = normalize_layout_name(name)?;
        self.layouts
            .iter()
            .find(|layout| layout.name == normalized)
            .ok_or(WorkbenchLayoutError::MissingLayout(normalized))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkbenchLayoutRecord {
    pub version: u32,
    pub name: String,
    pub center: String,
    pub left: String,
    pub right: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub experience: Option<String>,
    #[serde(default)]
    pub active_context_policy: WorkbenchLayoutContextPolicy,
}

impl WorkbenchLayoutRecord {
    pub fn new(
        name: impl AsRef<str>,
        center: WorkbenchId,
        left: WorkbenchPaneBindingId,
        right: WorkbenchPaneBindingId,
        experience: Option<WorkbenchExperienceId>,
    ) -> Result<Self, WorkbenchLayoutError> {
        let record = Self {
            version: WORKBENCH_LAYOUT_SCHEMA_VERSION,
            name: normalize_layout_name(name.as_ref())?,
            center: center.slug().to_owned(),
            left: left.slug().to_owned(),
            right: right.slug().to_owned(),
            experience: experience.map(|id| id.slug().to_owned()),
            active_context_policy: WorkbenchLayoutContextPolicy::PreserveActiveContext,
        };
        record.validate_for_surface(WorkbenchSurface::Tui)?;
        record.validate_for_surface(WorkbenchSurface::Web)?;
        Ok(record)
    }

    pub fn center_id(&self) -> Result<WorkbenchId, WorkbenchLayoutError> {
        parse_workbench_id(&self.center)
    }

    pub fn left_id(&self) -> Result<WorkbenchPaneBindingId, WorkbenchLayoutError> {
        parse_pane_binding_id(&self.left)
    }

    pub fn right_id(&self) -> Result<WorkbenchPaneBindingId, WorkbenchLayoutError> {
        parse_pane_binding_id(&self.right)
    }

    pub fn experience_id(&self) -> Result<Option<WorkbenchExperienceId>, WorkbenchLayoutError> {
        self.experience
            .as_deref()
            .map(parse_experience_id)
            .transpose()
    }

    pub fn validate_for_surface(
        &self,
        surface: WorkbenchSurface,
    ) -> Result<(), WorkbenchLayoutError> {
        if self.version != WORKBENCH_LAYOUT_SCHEMA_VERSION {
            return Err(WorkbenchLayoutError::UnsupportedRecordVersion {
                found: self.version,
                supported: WORKBENCH_LAYOUT_SCHEMA_VERSION,
            });
        }
        normalize_layout_name(&self.name)?;
        let center = self.center_id()?;
        let entry = workbench_entry(center)
            .ok_or_else(|| WorkbenchLayoutError::UnknownWorkbench(self.center.clone()))?;
        if entry.default_zone != WorkbenchZone::Center {
            return Err(WorkbenchLayoutError::UnsupportedWorkbench(
                center.slug().to_owned(),
            ));
        }
        validate_pane_surface(self.left_id()?, WorkbenchZone::LeftPane, surface)?;
        validate_pane_surface(self.right_id()?, WorkbenchZone::RightPane, surface)?;
        if let Some(experience_id) = self.experience_id()? {
            let experience = workbench_experience(experience_id).ok_or_else(|| {
                WorkbenchLayoutError::UnknownExperience(experience_id.slug().to_owned())
            })?;
            if !experience.supported_surfaces.contains(&surface) {
                return Err(WorkbenchLayoutError::UnsupportedExperienceSurface {
                    slug: experience_id.slug().to_owned(),
                    surface,
                });
            }
            if experience.center != center {
                return Err(WorkbenchLayoutError::ExperienceCenterMismatch {
                    experience: experience_id.slug().to_owned(),
                    expected: experience.center.slug().to_owned(),
                    actual: center.slug().to_owned(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkbenchLayoutContextPolicy {
    #[default]
    PreserveActiveContext,
}

pub fn normalize_layout_name(raw: &str) -> Result<String, WorkbenchLayoutError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(WorkbenchLayoutError::EmptyName);
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Err(WorkbenchLayoutError::InvalidName(trimmed.to_owned()));
    }
    Ok(trimmed.to_owned())
}

pub fn parse_workbench_id(slug: &str) -> Result<WorkbenchId, WorkbenchLayoutError> {
    WORKBENCH_CATALOG
        .iter()
        .find(|entry| entry.id.slug() == slug)
        .map(|entry| entry.id)
        .ok_or_else(|| WorkbenchLayoutError::UnknownWorkbench(slug.to_owned()))
}

pub fn parse_pane_binding_id(slug: &str) -> Result<WorkbenchPaneBindingId, WorkbenchLayoutError> {
    WORKBENCH_PANE_BINDINGS
        .iter()
        .find(|binding| binding.id.slug() == slug)
        .map(|binding| binding.id)
        .ok_or_else(|| WorkbenchLayoutError::UnknownPaneBinding(slug.to_owned()))
}

pub fn parse_experience_id(slug: &str) -> Result<WorkbenchExperienceId, WorkbenchLayoutError> {
    WORKBENCH_EXPERIENCES
        .iter()
        .find(|experience| experience.id.slug() == slug)
        .map(|experience| experience.id)
        .ok_or_else(|| WorkbenchLayoutError::UnknownExperience(slug.to_owned()))
}

fn validate_pane_surface(
    id: WorkbenchPaneBindingId,
    zone: WorkbenchZone,
    surface: WorkbenchSurface,
) -> Result<(), WorkbenchLayoutError> {
    let binding = workbench_pane_binding(id)
        .ok_or_else(|| WorkbenchLayoutError::UnknownPaneBinding(id.slug().to_owned()))?;
    if binding.zone != zone {
        return Err(WorkbenchLayoutError::UnsupportedPaneZone {
            slug: id.slug().to_owned(),
            zone,
        });
    }
    if !binding.supported_surfaces.contains(&surface) {
        return Err(WorkbenchLayoutError::UnsupportedPaneSurface {
            slug: id.slug().to_owned(),
            surface,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn l0_workbench_layout_record_round_trips_typed_ids() {
        let record = WorkbenchLayoutRecord::new(
            "tonight",
            WorkbenchId::Scores,
            WorkbenchPaneBindingId::FavoritesLeft,
            WorkbenchPaneBindingId::ScheduleRight,
            Some(WorkbenchExperienceId::TonightBench),
        )
        .expect("valid layout");

        let json = serde_json::to_string(&record).expect("serialize record");
        let restored: WorkbenchLayoutRecord = serde_json::from_str(&json).expect("parse record");

        assert_eq!(restored.center_id().unwrap(), WorkbenchId::Scores);
        assert_eq!(
            restored.left_id().unwrap(),
            WorkbenchPaneBindingId::FavoritesLeft
        );
        assert_eq!(
            restored.right_id().unwrap(),
            WorkbenchPaneBindingId::ScheduleRight
        );
        assert_eq!(
            restored.experience_id().unwrap(),
            Some(WorkbenchExperienceId::TonightBench)
        );
    }

    #[test]
    fn l0_workbench_layout_store_refuses_unsupported_version() {
        let store = WorkbenchLayoutStore {
            version: WORKBENCH_LAYOUT_SCHEMA_VERSION + 1,
            layouts: Vec::new(),
        };

        let err = store.validate().expect_err("future store must fail");

        assert!(matches!(
            err,
            WorkbenchLayoutError::UnsupportedStoreVersion { .. }
        ));
    }

    #[test]
    fn l0_workbench_layout_store_refuses_incomplete_or_corrupt_records() {
        let path = temp_layout_path("incomplete");
        std::fs::write(&path, r#"{"version":1,"layouts":[{"name":"bad"}]}"#).unwrap();

        let err = WorkbenchLayoutStore::load_from_path(&path).expect_err("record is incomplete");

        assert!(matches!(err, WorkbenchLayoutError::CorruptStore { .. }));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn l0_workbench_layout_store_upsert_preserves_valid_existing_store() {
        let path = temp_layout_path("upsert");
        let mut store = WorkbenchLayoutStore::default();
        store
            .upsert(
                WorkbenchLayoutRecord::new(
                    "tonight",
                    WorkbenchId::Scores,
                    WorkbenchPaneBindingId::FavoritesLeft,
                    WorkbenchPaneBindingId::ScheduleRight,
                    Some(WorkbenchExperienceId::TonightBench),
                )
                .unwrap(),
            )
            .unwrap();
        store.save_to_path(&path).unwrap();

        let mut restored = WorkbenchLayoutStore::load_from_path(&path).unwrap();
        restored
            .upsert(
                WorkbenchLayoutRecord::new(
                    "tonight",
                    WorkbenchId::Stats,
                    WorkbenchPaneBindingId::FavoritesLeft,
                    WorkbenchPaneBindingId::ScheduleRight,
                    None,
                )
                .unwrap(),
            )
            .unwrap();

        assert_eq!(restored.layouts.len(), 1);
        assert_eq!(restored.layouts[0].center, "stats");
        assert_eq!(restored.layouts[0].left, "favorites-left");
        assert_eq!(restored.layouts[0].right, "schedule-right");
        let _ = std::fs::remove_file(path);
    }

    fn temp_layout_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("icelines-layout-{label}-{nonce}.json"))
    }
}
