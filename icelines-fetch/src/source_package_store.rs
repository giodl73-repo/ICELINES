//! Immutable raw-capture and normalized source-package persistence.
//!
//! `icelines-sources` remains pure. This fetch-owned store persists captured
//! bytes by content hash, validates normalized packages before writing, and
//! changes the active pointer only for complete packages.

use crate::atomic_write::{write_bytes_atomic, write_json_atomic};
use icelines_core::source_facts::{ContentHash, PackageId, SourceContractError, SourcePackage};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

const ACTIVE_POINTER_SCHEMA: &str = "icelines_source_package_active.v1";

#[derive(Debug, thiserror::Error)]
pub enum SourcePackageStoreError {
    #[error("source package I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("source package JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("source package contract failed: {0}")]
    Contract(#[from] SourceContractError),
    #[error("source package {0} does not exist")]
    PackageNotFound(String),
    #[error("source capture {0} does not exist")]
    CaptureNotFound(String),
    #[error("source capture integrity mismatch: expected {expected}, got {actual}")]
    CaptureIntegrity { expected: String, actual: String },
    #[error("source package {0} is incomplete and cannot become active")]
    IncompletePackage(String),
    #[error("active source-package pointer is invalid")]
    InvalidActivePointer,
    #[error("active pointer fingerprint does not match package {0}")]
    ActiveFingerprintMismatch(String),
}

#[derive(Debug, Clone)]
pub struct SourcePackageStore {
    root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ActiveSourcePackagePointer {
    schema: String,
    package_id: PackageId,
    fingerprint: ContentHash,
}

impl SourcePackageStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_owned());
        PathBuf::from(home)
            .join(".icelines")
            .join("source-packages")
    }

    /// Stores source bytes content-addressably and returns their canonical
    /// SHA-256. Repeated storage of identical bytes is idempotent.
    pub fn store_capture(&self, bytes: &[u8]) -> Result<ContentHash, SourcePackageStoreError> {
        let hash = content_hash(bytes)?;
        let path = self.capture_path(&hash);
        if path.exists() {
            let existing = std::fs::read(&path)?;
            verify_capture(&hash, &existing)?;
        } else {
            write_bytes_atomic(&path, bytes)?;
        }
        Ok(hash)
    }

    pub fn read_capture(&self, hash: &ContentHash) -> Result<Vec<u8>, SourcePackageStoreError> {
        let path = self.capture_path(hash);
        if !path.exists() {
            return Err(SourcePackageStoreError::CaptureNotFound(hash.to_string()));
        }
        let bytes = std::fs::read(path)?;
        verify_capture(hash, &bytes)?;
        Ok(bytes)
    }

    /// Writes a validated package whether complete or incomplete. Incomplete
    /// packages remain valuable audit artifacts but cannot be activated.
    pub fn store_package(&self, package: &SourcePackage) -> Result<(), SourcePackageStoreError> {
        package.validate()?;
        write_json_atomic(&self.package_path(&package.package_id), package)?;
        Ok(())
    }

    pub fn load_package(
        &self,
        package_id: &PackageId,
    ) -> Result<SourcePackage, SourcePackageStoreError> {
        let path = self.package_path(package_id);
        if !path.exists() {
            return Err(SourcePackageStoreError::PackageNotFound(
                package_id.to_string(),
            ));
        }
        let package: SourcePackage = serde_json::from_slice(&std::fs::read(path)?)?;
        package.validate()?;
        if &package.package_id != package_id {
            return Err(SourcePackageStoreError::PackageNotFound(
                package_id.to_string(),
            ));
        }
        Ok(package)
    }

    pub fn activate(&self, package_id: &PackageId) -> Result<(), SourcePackageStoreError> {
        let package = self.load_package(package_id)?;
        if !package.run_manifest.complete {
            return Err(SourcePackageStoreError::IncompletePackage(
                package_id.to_string(),
            ));
        }
        write_json_atomic(
            &self.root.join("active.json"),
            &ActiveSourcePackagePointer {
                schema: ACTIVE_POINTER_SCHEMA.to_owned(),
                package_id: package.package_id,
                fingerprint: package.fingerprint,
            },
        )?;
        Ok(())
    }

    pub fn load_active(&self) -> Result<SourcePackage, SourcePackageStoreError> {
        let path = self.root.join("active.json");
        if !path.exists() {
            return Err(SourcePackageStoreError::InvalidActivePointer);
        }
        let pointer: ActiveSourcePackagePointer = serde_json::from_slice(&std::fs::read(path)?)?;
        if pointer.schema != ACTIVE_POINTER_SCHEMA {
            return Err(SourcePackageStoreError::InvalidActivePointer);
        }
        let package = self.load_package(&pointer.package_id)?;
        if package.fingerprint != pointer.fingerprint {
            return Err(SourcePackageStoreError::ActiveFingerprintMismatch(
                pointer.package_id.to_string(),
            ));
        }
        Ok(package)
    }

    fn capture_path(&self, hash: &ContentHash) -> PathBuf {
        self.root
            .join("captures")
            .join(format!("{}.bin", hash.as_str()))
    }

    fn package_path(&self, package_id: &PackageId) -> PathBuf {
        // Logical package IDs are never treated as filesystem paths.
        let key = format!("{:x}", Sha256::digest(package_id.as_str().as_bytes()));
        self.root.join("packages").join(format!("{key}.json"))
    }
}

fn content_hash(bytes: &[u8]) -> Result<ContentHash, SourceContractError> {
    ContentHash::try_new(format!("{:x}", Sha256::digest(bytes)))
}

fn verify_capture(expected: &ContentHash, bytes: &[u8]) -> Result<(), SourcePackageStoreError> {
    let actual = content_hash(bytes)?;
    if &actual != expected {
        return Err(SourcePackageStoreError::CaptureIntegrity {
            expected: expected.to_string(),
            actual: actual.to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use icelines_core::model::Season;
    use icelines_core::source_facts::{
        AdapterVersion, PolicyVersion, SourceObjectOutcome, SourceObjectState, SourceRunManifest,
    };
    use tempfile::TempDir;

    fn hash(character: char) -> ContentHash {
        ContentHash::try_new(character.to_string().repeat(64)).unwrap()
    }

    fn package(package_id: &str, complete: bool) -> SourcePackage {
        let state = if complete {
            SourceObjectState::Acquired { records: 0 }
        } else {
            SourceObjectState::Failed {
                reason: "fixture acquisition failed".to_owned(),
            }
        };
        SourcePackage::seal(
            PackageId::try_new(package_id).unwrap(),
            Season(20_262_027),
            Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 31, 0, 0, 0).single().unwrap(),
            AdapterVersion::try_new("registry.v1").unwrap(),
            PolicyVersion::try_new("reconcile.v1").unwrap(),
            hash('f'),
            SourceRunManifest {
                requested_scope: "fixture".to_owned(),
                source_catalog_version: "catalog.v1".to_owned(),
                objects: vec![SourceObjectOutcome {
                    object_id: "SEA:nhl_draft".to_owned(),
                    source_family: "nhl_draft".to_owned(),
                    organization: None,
                    terminal_pagination: complete,
                    state,
                }],
                complete,
            },
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
        .unwrap()
    }

    #[test]
    fn capture_round_trip_is_content_addressed_and_integrity_checked() {
        let directory = TempDir::new().unwrap();
        let store = SourcePackageStore::new(directory.path());
        let expected = store.store_capture(b"official source bytes").unwrap();
        assert_eq!(
            store.read_capture(&expected).unwrap(),
            b"official source bytes"
        );

        std::fs::write(store.capture_path(&expected), b"tampered").unwrap();
        assert!(matches!(
            store.read_capture(&expected).unwrap_err(),
            SourcePackageStoreError::CaptureIntegrity { .. }
        ));
    }

    #[test]
    fn only_complete_validated_packages_can_change_the_active_pointer() {
        let directory = TempDir::new().unwrap();
        let store = SourcePackageStore::new(directory.path());
        let incomplete = package("audit/incomplete", false);
        store.store_package(&incomplete).unwrap();
        assert!(matches!(
            store.activate(&incomplete.package_id).unwrap_err(),
            SourcePackageStoreError::IncompletePackage(_)
        ));
        assert!(!directory.path().join("active.json").exists());

        let complete = package("../../logical-not-a-path", true);
        store.store_package(&complete).unwrap();
        store.activate(&complete.package_id).unwrap();
        let active = store.load_active().unwrap();
        assert_eq!(active.package_id, complete.package_id);
        assert_eq!(active.fingerprint, complete.fingerprint);
    }
}
