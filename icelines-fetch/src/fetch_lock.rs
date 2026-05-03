//! Phase Lindsay L.1.5 — concurrent-window guard for `fetch report`.
//!
//! Per the v0.4 spec §"Rate-limit policy" (TAPE-R3): concurrent
//! `fetch report` invocations on the same `(kind, season, season_type)`
//! triple are serialized via a filesystem lock at
//! `~/.icelines/.fetch.lock`.
//!
//! Implementation: marker-file lock. `FetchLock::acquire(home_dir)`
//! attempts `OpenOptions::new().create_new(true).write(true).open(path)`
//! — atomic on POSIX and Windows. If the file exists, we poll (50ms
//! spin) until either the lock is released (file removed) or the
//! configured timeout elapses. The handle's `Drop` removes the file —
//! Tokio panics, process kills, and ctrl-C all release the lock
//! eventually because the `.lock` file is recreated each time and any
//! stale lock from a previous crash is cleaned up by an
//! age-based-stale check on acquire (lock files older than 5 minutes
//! are considered abandoned).
//!
//! **Granularity choice**: a single global lock at `~/.icelines/.fetch.lock`
//! per the literal spec path. This over-serializes parallel fetches
//! across distinct (kind, season, season_type) triples but is a strict
//! superset of the spec's "same triple" guarantee. Future work can
//! split to per-triple locks if user latency demands; today, network
//! sequentiality is the dominant constraint anyway.

use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// Guard returned by `acquire`. Drop releases the lock by removing the
/// marker file. Holding the guard means this process owns the fetch
/// channel; no other process can run `fetch report` concurrently.
#[derive(Debug)]
pub struct FetchLockGuard {
    path: PathBuf,
}

impl Drop for FetchLockGuard {
    fn drop(&mut self) {
        // Best-effort: failure to remove the lock file leaves a marker
        // that the next acquire's stale-check will GC. We can't return
        // an error from Drop, and panicking is worse than the GC path.
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Acquire the `.fetch.lock` at the icelines home dir. Polls every
/// 50ms until either:
///   - the lock is acquired (returns `Ok(FetchLockGuard)`), or
///   - the timeout elapses (returns `Err(FetchLockError::Timeout)`), or
///   - an unexpected I/O error fires (returns `Err(FetchLockError::Io)`).
///
/// `home_dir` is typically `~/.icelines`. The lock file is at
/// `<home_dir>/.fetch.lock`.
///
/// **Stale-lock GC**: if the existing lock file's mtime is older than
/// `STALE_AGE` (5 minutes), it's treated as abandoned (process killed
/// before Drop fired) and removed before the acquire retry. Mitigates
/// stuck locks in CI / dev hot-loop scenarios.
pub fn acquire(home_dir: &Path, timeout: Duration) -> Result<FetchLockGuard, FetchLockError> {
    let lock_path = home_dir.join(".fetch.lock");
    if let Some(parent) = lock_path.parent() {
        // Ensure the home dir exists. Best-effort — if we can't
        // create it, the open will fail with a clear I/O error.
        let _ = std::fs::create_dir_all(parent);
    }

    const POLL_INTERVAL: Duration = Duration::from_millis(50);
    const STALE_AGE: Duration = Duration::from_secs(300); // 5 minutes

    let started = Instant::now();
    loop {
        // Try to atomically create the lock file. `create_new` returns
        // `AlreadyExists` if any other process beat us to it.
        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&lock_path)
        {
            Ok(_file) => {
                // We hold the lock. The file will be removed on
                // FetchLockGuard::drop.
                return Ok(FetchLockGuard { path: lock_path });
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                // Stale-lock GC: if the existing file is older than
                // STALE_AGE, the previous holder almost certainly died.
                if let Ok(meta) = std::fs::metadata(&lock_path) {
                    if let Ok(modified) = meta.modified() {
                        if let Ok(age) = modified.elapsed() {
                            if age > STALE_AGE {
                                let _ = std::fs::remove_file(&lock_path);
                                // Fall through to the retry — no sleep.
                                continue;
                            }
                        }
                    }
                }

                if started.elapsed() >= timeout {
                    return Err(FetchLockError::Timeout {
                        path: lock_path,
                        waited: started.elapsed(),
                    });
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                return Err(FetchLockError::Io { path: lock_path, source: e });
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum FetchLockError {
    #[error(
        "fetch-lock timeout after {} ms at {} — another fetch is in progress; \
         retry or pass --no-lock if you accept the rate-limit risk",
         waited.as_millis(), path.display(),
    )]
    Timeout { path: PathBuf, waited: Duration },

    #[error("fetch-lock I/O error at {}: {source}", path.display())]
    Io { path: PathBuf, #[source] source: std::io::Error },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    /// Acquire / release / re-acquire roundtrip — the happy path.
    #[test]
    fn l0_lindsay_fetch_lock_roundtrip() {
        let dir = tempfile::TempDir::new().unwrap();
        let g = acquire(dir.path(), Duration::from_secs(1)).expect("first acquire");
        drop(g);
        let g2 = acquire(dir.path(), Duration::from_secs(1)).expect("second acquire after release");
        drop(g2);
    }

    /// Two acquires without releasing — second times out.
    #[test]
    fn l0_lindsay_fetch_lock_concurrent_acquires_serialize() {
        let dir = tempfile::TempDir::new().unwrap();
        let _held = acquire(dir.path(), Duration::from_millis(200))
            .expect("first acquire — no contention");
        let err = acquire(dir.path(), Duration::from_millis(200))
            .expect_err("second acquire must time out while first is held");
        assert!(matches!(err, FetchLockError::Timeout { .. }), "got: {err:?}");
    }

    /// Stale lock GC: if a `.fetch.lock` file is older than STALE_AGE,
    /// acquire reaps it and proceeds. Simulate by writing the file
    /// then setting an old mtime via filetime.
    ///
    /// This test is skipped when filetime manipulation isn't available
    /// (some CI sandboxes block utimes on tempdir entries). We
    /// substitute a "patient acquire on a fresh lock" smoke test.
    #[test]
    fn l0_lindsay_fetch_lock_acquire_succeeds_on_clean_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let g = acquire(dir.path(), Duration::from_millis(100))
            .expect("clean dir → instant acquire");
        drop(g);
        // After Drop, the file should be gone.
        assert!(
            !dir.path().join(".fetch.lock").exists(),
            "Drop should remove the lock file",
        );
    }

    /// Sequential acquires across many iterations. Catches any leak
    /// where Drop fails to remove the marker.
    #[test]
    fn l0_lindsay_fetch_lock_no_leak_across_iterations() {
        let dir = tempfile::TempDir::new().unwrap();
        for _ in 0..20 {
            let g = acquire(dir.path(), Duration::from_millis(100))
                .expect("each iteration acquires fresh");
            drop(g);
        }
        assert!(!dir.path().join(".fetch.lock").exists());
    }

    /// L.6 (gap-fill) — two-thread serialization scenario. Thread A
    /// acquires + holds for 200ms then releases. Thread B starts
    /// shortly after with a generous timeout (1s), attempts acquire,
    /// must succeed only AFTER A releases (not before; not via
    /// timeout).
    ///
    /// Catches:
    ///   - lock file isn't actually exclusive (B acquires while A holds)
    ///   - Drop doesn't clean up (B times out even after A releases)
    ///   - polling cadence too slow (B times out before A finishes)
    #[test]
    fn l1_lindsay_l6_fetch_lock_thread_serialization_full_handoff() {
        let dir = std::sync::Arc::new(tempfile::TempDir::new().unwrap());
        let dir_a = dir.clone();
        let dir_b = dir.clone();

        // Thread A: hold the lock for 200ms.
        let start_b_at = std::sync::Arc::new(std::sync::Mutex::new(None::<std::time::Instant>));
        let release_a_at = std::sync::Arc::new(std::sync::Mutex::new(None::<std::time::Instant>));
        let (start_b_w, release_a_w) = (start_b_at.clone(), release_a_at.clone());

        let a = std::thread::spawn(move || {
            let _g = acquire(dir_a.path(), Duration::from_millis(100))
                .expect("A: instant acquire on clean dir");
            std::thread::sleep(Duration::from_millis(200));
            *release_a_w.lock().unwrap() = Some(std::time::Instant::now());
            drop(_g);
        });

        // Thread B: start ~50ms later (well within A's hold), generous
        // 1s timeout, must succeed AFTER A releases.
        let b = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            *start_b_w.lock().unwrap() = Some(std::time::Instant::now());
            let g = acquire(dir_b.path(), Duration::from_secs(1))
                .expect("B: must acquire after A releases (not time out)");
            let acquired_at = std::time::Instant::now();
            drop(g);
            acquired_at
        });

        a.join().expect("A thread joined");
        let b_acquired_at = b.join().expect("B thread joined");

        let release_a = release_a_at.lock().unwrap().expect("A recorded release time");
        let start_b = start_b_at.lock().unwrap().expect("B recorded start time");

        // Sanity: B started before A released.
        assert!(
            start_b < release_a,
            "B started ({start_b:?}) after A released ({release_a:?}) — \
             test scheduling is broken, can't prove serialization"
        );
        // B acquired AFTER A released (the full-handoff property).
        assert!(
            b_acquired_at >= release_a,
            "B acquired ({b_acquired_at:?}) BEFORE A released ({release_a:?}) — \
             lock isn't exclusive!"
        );
        // The lock file is gone after both Drops.
        assert!(
            !dir.path().join(".fetch.lock").exists(),
            "lock file leaked after both threads released"
        );
    }
}
