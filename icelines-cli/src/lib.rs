//! Phase Foster.0.7 — thin library facade so integration tests in
//! `icelines-cli/tests/*.rs` can access the typed modules. The `bin`
//! target (`main.rs`) re-uses these via `crate::*`.
//!
//! Only the modules currently consumed from outside the binary are
//! re-exported here. New modules can be added on demand without
//! disturbing the existing CLI dispatch.

pub mod config;
