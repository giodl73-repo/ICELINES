//! Local publication catalog for sealed UI-neutral card documents.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    time::Duration,
};

use chrono::{DateTime, Utc};
use icelines_core::{parse_card_document, CardDocumentError, CardDocumentView, CardKind};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::atomic_write::write_bytes_atomic;

pub const CARD_PUBLICATION_CATALOG_SCHEMA: &str = "card_publication_catalog.v1";
pub const CARD_PUBLICATION_CATALOG_JSON_SCHEMA: &str =
    include_str!("../../design/schemas/card_publication_catalog.v1.schema.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPublicationKey {
    pub card_kind: CardKind,
    pub season: u32,
    pub game_id: u64,
    pub focus_team: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPublicationEntry {
    pub key: CardPublicationKey,
    pub document_id: String,
    pub fingerprint: String,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CardPublicationCatalog {
    pub schema: String,
    pub updated_at: DateTime<Utc>,
    pub entries: Vec<CardPublicationEntry>,
    pub fingerprint: String,
}

#[derive(Debug, Error)]
pub enum CardPublicationStoreError {
    #[error("card publication I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("card publication JSON failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("card publication contract failed: {0}")]
    Card(#[from] CardDocumentError),
    #[error("only player-line matchup cards can be published in v1")]
    UnsupportedCardKind,
    #[error("player-line matchup publication identity is incomplete or inconsistent")]
    InvalidIdentity,
    #[error("publication time predates the sealed card")]
    PublicationBeforeCard,
    #[error("publication time predates the active entry for this card key")]
    StalePublication,
    #[error("card publication catalog is missing")]
    MissingCatalog,
    #[error(
        "published player-line matchup card is missing for season {season} game {game_id} team {focus_team}"
    )]
    MissingEntry {
        season: u32,
        game_id: u64,
        focus_team: String,
    },
    #[error("published card blob is missing for fingerprint {0}")]
    MissingBlob(String),
    #[error("card publication catalog fingerprint mismatch")]
    CatalogFingerprintMismatch,
    #[error("published card fingerprint or identity mismatch")]
    PublishedCardMismatch,
    #[error("card publication lock failed: {0}")]
    LockFailed(String),
}

#[derive(Debug, Clone)]
pub struct CardPublicationStore {
    root: PathBuf,
}

impl CardPublicationStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn under_data_root(data_root: impl AsRef<Path>) -> Self {
        Self::new(data_root.as_ref().join("cards"))
    }

    pub fn default_root() -> PathBuf {
        if let Some(root) = std::env::var_os("ICELINES_DATA_ROOT") {
            return root.into();
        }
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .unwrap_or_else(|| ".".into());
        PathBuf::from(home).join(".icelines").join("data")
    }

    pub fn publish(
        &self,
        card: &CardDocumentView,
        published_at: DateTime<Utc>,
    ) -> Result<CardPublicationEntry, CardPublicationStoreError> {
        card.validate()?;
        let key = publication_key(card)?;
        if card
            .context
            .evidence_at
            .into_iter()
            .chain(card.context.view.generated_at)
            .any(|timestamp| timestamp > published_at)
        {
            return Err(CardPublicationStoreError::PublicationBeforeCard);
        }
        let _guard = crate::fetch_lock::acquire(&self.root, Duration::from_secs(5))
            .map_err(|error| CardPublicationStoreError::LockFailed(error.to_string()))?;
        let entry = CardPublicationEntry {
            key,
            document_id: card.document_id.clone(),
            fingerprint: card.fingerprint.clone(),
            published_at,
        };
        let blob_path = self.blob_path(&entry.fingerprint);
        let bytes = serde_json::to_vec_pretty(card)?;
        if blob_path.exists() {
            let existing = std::fs::read_to_string(&blob_path)
                .map_err(|source| io_error(blob_path.clone(), source))?;
            let existing = parse_card_document(&existing)?;
            if existing.fingerprint != card.fingerprint {
                return Err(CardPublicationStoreError::PublishedCardMismatch);
            }
        } else {
            write_bytes_atomic(&blob_path, &bytes)
                .map_err(|source| io_error(blob_path.clone(), source))?;
        }

        let mut catalog = match self.load_catalog() {
            Ok(catalog) => catalog,
            Err(CardPublicationStoreError::MissingCatalog) => CardPublicationCatalog {
                schema: CARD_PUBLICATION_CATALOG_SCHEMA.to_owned(),
                updated_at: published_at,
                entries: Vec::new(),
                fingerprint: String::new(),
            },
            Err(error) => return Err(error),
        };
        if catalog
            .entries
            .iter()
            .any(|existing| existing.key == entry.key && existing.published_at > published_at)
        {
            return Err(CardPublicationStoreError::StalePublication);
        }
        catalog.entries.retain(|existing| existing.key != entry.key);
        catalog.entries.push(entry.clone());
        catalog.entries.sort_by(|left, right| {
            (
                left.key.season,
                left.key.game_id,
                left.key.focus_team.as_str(),
            )
                .cmp(&(
                    right.key.season,
                    right.key.game_id,
                    right.key.focus_team.as_str(),
                ))
        });
        catalog.updated_at = catalog.updated_at.max(published_at);
        catalog.fingerprint = catalog_fingerprint(&catalog)?;
        let catalog_path = self.catalog_path();
        write_bytes_atomic(&catalog_path, &serde_json::to_vec_pretty(&catalog)?)
            .map_err(|source| io_error(catalog_path, source))?;
        Ok(entry)
    }

    pub fn load_player_line_matchup(
        &self,
        season: u32,
        game_id: u64,
        focus_team: &str,
    ) -> Result<CardDocumentView, CardPublicationStoreError> {
        let focus_team = focus_team.trim().to_ascii_uppercase();
        let key = CardPublicationKey {
            card_kind: CardKind::PlayerLineMatchup,
            season,
            game_id,
            focus_team: focus_team.clone(),
        };
        let catalog = self.load_catalog()?;
        let entry = catalog
            .entries
            .iter()
            .find(|entry| entry.key == key)
            .ok_or(CardPublicationStoreError::MissingEntry {
                season,
                game_id,
                focus_team,
            })?;
        let path = self.blob_path(&entry.fingerprint);
        let json = match std::fs::read_to_string(&path) {
            Ok(json) => json,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(CardPublicationStoreError::MissingBlob(
                    entry.fingerprint.clone(),
                ));
            }
            Err(source) => return Err(io_error(path, source)),
        };
        let card = parse_card_document(&json)?;
        if card.fingerprint != entry.fingerprint
            || card.document_id != entry.document_id
            || publication_key(&card)? != entry.key
        {
            return Err(CardPublicationStoreError::PublishedCardMismatch);
        }
        Ok(card)
    }

    pub fn load_catalog(&self) -> Result<CardPublicationCatalog, CardPublicationStoreError> {
        let path = self.catalog_path();
        let bytes = match std::fs::read(&path) {
            Ok(bytes) => bytes,
            Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
                return Err(CardPublicationStoreError::MissingCatalog);
            }
            Err(source) => return Err(io_error(path, source)),
        };
        let catalog: CardPublicationCatalog = serde_json::from_slice(&bytes)?;
        let mut keys = BTreeSet::new();
        if catalog.schema != CARD_PUBLICATION_CATALOG_SCHEMA
            || catalog.entries.is_empty()
            || catalog.entries.iter().any(|entry| {
                entry.key.card_kind != CardKind::PlayerLineMatchup
                    || entry.key.season < 20_000_000
                    || entry.key.game_id == 0
                    || entry.key.focus_team.len() != 3
                    || !entry
                        .key
                        .focus_team
                        .bytes()
                        .all(|byte| byte.is_ascii_uppercase())
                    || entry.document_id.trim().is_empty()
                    || !valid_fingerprint(&entry.fingerprint)
                    || entry.published_at > catalog.updated_at
                    || !keys.insert((
                        entry.key.season,
                        entry.key.game_id,
                        entry.key.focus_team.clone(),
                    ))
            })
            || catalog.fingerprint != catalog_fingerprint(&catalog)?
        {
            return Err(CardPublicationStoreError::CatalogFingerprintMismatch);
        }
        Ok(catalog)
    }

    fn catalog_path(&self) -> PathBuf {
        self.root.join("catalog.json")
    }

    fn blob_path(&self, fingerprint: &str) -> PathBuf {
        self.root
            .join("blobs")
            .join("sha256")
            .join(format!("{fingerprint}.json"))
    }
}

fn publication_key(
    card: &CardDocumentView,
) -> Result<CardPublicationKey, CardPublicationStoreError> {
    if card.card_kind != CardKind::PlayerLineMatchup {
        return Err(CardPublicationStoreError::UnsupportedCardKind);
    }
    let [game_id] = card.context.joins.game_ids.as_slice() else {
        return Err(CardPublicationStoreError::InvalidIdentity);
    };
    let game_id = game_id
        .parse::<u64>()
        .map_err(|_| CardPublicationStoreError::InvalidIdentity)?;
    let focus_team = card
        .theme
        .team_abbreviation
        .as_deref()
        .map(str::trim)
        .filter(|team| !team.is_empty())
        .map(str::to_ascii_uppercase)
        .ok_or(CardPublicationStoreError::InvalidIdentity)?;
    if !card.context.joins.team_ids.contains(&focus_team) {
        return Err(CardPublicationStoreError::InvalidIdentity);
    }
    let key = CardPublicationKey {
        card_kind: CardKind::PlayerLineMatchup,
        season: card.context.view.window.season.0,
        game_id,
        focus_team,
    };
    if key.season < 20_000_000
        || key.game_id == 0
        || key.focus_team.len() != 3
        || !key.focus_team.bytes().all(|byte| byte.is_ascii_uppercase())
    {
        return Err(CardPublicationStoreError::InvalidIdentity);
    }
    Ok(key)
}

fn catalog_fingerprint(catalog: &CardPublicationCatalog) -> Result<String, serde_json::Error> {
    let mut canonical = catalog.clone();
    canonical.fingerprint.clear();
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn io_error(path: PathBuf, source: std::io::Error) -> CardPublicationStoreError {
    CardPublicationStoreError::Io { path, source }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::{Arc, Barrier};
    use tempfile::TempDir;

    fn card() -> CardDocumentView {
        parse_card_document(include_str!(
            "../../examples/player-line-matchup-card-nyr-vs-sea-2026-27.json"
        ))
        .unwrap()
    }

    #[test]
    fn publication_round_trip_is_cataloged_and_content_addressed() {
        let directory = TempDir::new().unwrap();
        let store = CardPublicationStore::new(directory.path());
        let published_at = Utc.with_ymd_and_hms(2026, 10, 10, 18, 0, 0).unwrap();

        let entry = store.publish(&card(), published_at).unwrap();
        let loaded = store
            .load_player_line_matchup(20262027, 2026020001, "nyr")
            .unwrap();

        assert_eq!(loaded.fingerprint, entry.fingerprint);
        assert_eq!(store.load_catalog().unwrap().entries, [entry]);
    }

    #[test]
    fn tampered_blob_and_early_publication_fail_closed() {
        let directory = TempDir::new().unwrap();
        let store = CardPublicationStore::new(directory.path());
        let published_at = Utc.with_ymd_and_hms(2026, 10, 10, 18, 0, 0).unwrap();
        let mut invalid = card();
        invalid.context.joins.game_ids = vec!["0".to_owned()];
        invalid = invalid.seal().unwrap();
        assert!(matches!(
            store.publish(&invalid, published_at),
            Err(CardPublicationStoreError::InvalidIdentity)
        ));
        let entry = store.publish(&card(), published_at).unwrap();
        std::fs::write(store.blob_path(&entry.fingerprint), b"{}").unwrap();
        assert!(store
            .load_player_line_matchup(20262027, 2026020001, "NYR")
            .is_err());

        let early = Utc.with_ymd_and_hms(2026, 10, 10, 14, 0, 0).unwrap();
        assert!(matches!(
            store.publish(&card(), early),
            Err(CardPublicationStoreError::PublicationBeforeCard)
        ));
    }

    #[test]
    fn older_publication_cannot_silently_replace_the_active_entry() {
        let directory = TempDir::new().unwrap();
        let store = CardPublicationStore::new(directory.path());
        let current = Utc.with_ymd_and_hms(2026, 10, 10, 18, 0, 0).unwrap();
        store.publish(&card(), current).unwrap();
        let stale = Utc.with_ymd_and_hms(2026, 10, 10, 17, 0, 0).unwrap();

        assert!(matches!(
            store.publish(&card(), stale),
            Err(CardPublicationStoreError::StalePublication)
        ));
    }

    #[test]
    fn concurrent_publications_preserve_both_catalog_entries() {
        let directory = TempDir::new().unwrap();
        let root = directory.path().to_path_buf();
        let published_at = Utc.with_ymd_and_hms(2026, 10, 10, 18, 0, 0).unwrap();
        let first = card();
        let mut second = card();
        second.theme.team_abbreviation = Some("SEA".to_owned());
        second = second.seal().unwrap();
        let barrier = Arc::new(Barrier::new(3));

        let publish = |card: CardDocumentView, barrier: Arc<Barrier>, root: PathBuf| {
            std::thread::spawn(move || {
                barrier.wait();
                CardPublicationStore::new(root)
                    .publish(&card, published_at)
                    .unwrap();
            })
        };
        let first_handle = publish(first, Arc::clone(&barrier), root.clone());
        let second_handle = publish(second, Arc::clone(&barrier), root.clone());
        barrier.wait();
        first_handle.join().unwrap();
        second_handle.join().unwrap();

        assert_eq!(
            CardPublicationStore::new(root)
                .load_catalog()
                .unwrap()
                .entries
                .len(),
            2
        );
    }
}
