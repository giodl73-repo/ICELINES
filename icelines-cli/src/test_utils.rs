//! Test-only helpers shared across modules.
//!
//! Keeps cross-module test state (notably the process-global HOME /
//! USERPROFILE env vars used to isolate file-cache code paths) in one
//! place so unrelated test suites can't race each other.

#![cfg(test)]

/// Acquire the process-wide env-mutation lock. Tests that mutate
/// `HOME` / `USERPROFILE` (e.g. headshot disk cache, user-scheme
/// loader, group/fantasy DB tests) must hold this guard for the
/// duration of the env mutation so concurrent tests in other modules
/// see a consistent value.
///
/// One mutex for all such tests in the binary — earlier this lived
/// per-module with separate `static Mutex` declarations, which let
/// `headshot::tests` race `scheme::tests` and produced flaky failures
/// when both ran in the same `cargo test` worker.
pub fn home_env_lock() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    // A poisoned mutex still gives us serialised access — the panic
    // already happened in the contending test, no reason to compound.
    LOCK.lock().unwrap_or_else(|p| p.into_inner())
}
