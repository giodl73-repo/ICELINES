//! Deterministic provider parsing and normalization for IceLines.
//!
//! This crate accepts caller-supplied bytes or decoded payloads. It does not
//! perform network requests, discover files, select active snapshots, persist
//! data, calculate product scores, or render user interfaces.

#![deny(unsafe_code)]

pub mod adapter;
pub mod ahl;
pub mod bundled_artifact;
pub mod capwages;
pub mod compat;
pub mod contracts_csv;
pub mod current_state;
pub mod fragment;
pub mod identity_review;
pub mod moneypuck;
pub mod moneypuck_goalie_game;
pub mod moneypuck_team_game;
pub mod nhl;
pub mod playoffs_bundle;
pub mod prospect_population;
pub mod reconciliation;
pub mod schema;
pub mod teams;
pub mod transactions;
pub mod yahoo_eligibility;

pub use adapter::{
    AbsenceSemantics, AdapterDisposition, AdapterError, AdapterErrorCategory, AdapterId,
    AdapterVersion, AdditiveFieldPolicy, ContentHash, HistoricalAvailability, SourceAdapter,
    SourceDescriptor, SourceId, SourceInput, ValidationError,
};
