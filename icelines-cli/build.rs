//! Build-time configuration for the `icelines` binary.
//!
//! ## Windows main-thread stack size
//!
//! The `Commands` enum has 25+ top-level variants; clap-derive generates a
//! large `match` for parsing. On Windows, the OS gives the main thread a
//! 1 MB default stack — debug builds (no inlining) blow it during arg
//! parsing before `main` runs.
//!
//! Fix: bump the linker `/STACK:` reserve to 8 MB on Windows MSVC.
//! Zero runtime cost. Affects only the binary's main thread; tokio worker
//! threads use their own stack config.
//!
//! Linux + macOS use 8 MB by default and need no patch.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        // Reserve 8 MB stack for the main thread. Format: /STACK:reserve[,commit]
        println!("cargo:rustc-link-arg-bin=icelines=/STACK:8388608");
    }
}
