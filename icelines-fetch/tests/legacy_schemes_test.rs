//! Phase Lindsay L.5.5 — DI-25 frozen-golden assertion for fantasy schemes.
//!
//! Per DI-25: every pre-Lindsay scheme TOML loads byte-identical to its
//! frozen golden via the legacy-key alias map. This test covers the
//! 3 built-in schemes (yahoo-standard, espn-standard, simple-pts) as
//! the L.5.5 minimum corpus. The full 5-named legacy-fixture corpus
//! (`yahoo-standard`, `espn-standard`, `custom-points-only`,
//! `head-to-head-9cat`, `rotisserie-with-goalie` per FORGE-R3 / BENCH-R2
//! L2-B24) is a carry-forward.
//!
//! ## Bootstrap mode
//!
//! When the fixture file doesn't exist, this test serializes the
//! built-in scheme via `toml::to_string_pretty` and writes it as the
//! frozen golden. Subsequent runs assert byte-identity. To regenerate
//! after an INTENDED scheme change:
//!
//! ```bash
//! LINDSAY_L55_REGEN=1 cargo test -p icelines-fetch \
//!     --test legacy_schemes_test l1_legacy_schemes_load_byte_identical
//! ```
//!
//! Commit the regenerated fixtures alongside the change that
//! necessitated them.

use icelines_core::scheme::Scheme;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("legacy_schemes")
}

/// Each built-in scheme serializes byte-identically to its frozen golden,
/// AND deserializes back to a scheme that re-serializes to the same bytes.
/// This is DI-25's load-and-compare invariant.
#[test]
fn l1_legacy_schemes_load_byte_identical() {
    let regen = std::env::var("LINDSAY_L55_REGEN").is_ok();
    let dir = fixtures_dir();
    std::fs::create_dir_all(&dir).expect("create fixtures dir");

    let schemes = Scheme::all_builtins();
    assert!(
        !schemes.is_empty(),
        "Scheme::all_builtins() must yield at least one entry"
    );

    for scheme in schemes {
        let fname = format!("{}.toml", scheme.name);
        let path = dir.join(&fname);

        // Canonical bytes: serialize the in-memory builtin via toml.
        let canonical = toml::to_string_pretty(&scheme)
            .expect("serialize scheme");

        if regen || !path.exists() {
            std::fs::write(&path, &canonical).expect("write golden");
            if regen {
                println!("regenerated {}", path.display());
            }
            continue;
        }

        let golden = std::fs::read_to_string(&path)
            .expect("read golden");

        assert_eq!(
            canonical, golden,
            "DI-25: scheme {} serializes differently than its frozen golden \
             at {}.\n\nIf this drift is intentional, re-run with \
             LINDSAY_L55_REGEN=1 to regenerate the golden, then commit it.",
            scheme.name,
            path.display(),
        );

        // Also verify load → re-serialize round-trips byte-identically:
        // catches loader-side drift (e.g. an alias map that loses fidelity
        // on a known field).
        let loaded: Scheme = toml::from_str(&golden)
            .expect("load golden as Scheme");
        let re_serialized = toml::to_string_pretty(&loaded)
            .expect("re-serialize loaded scheme");
        assert_eq!(
            canonical, re_serialized,
            "DI-25 round-trip drift: load + re-serialize ≠ canonical for {}",
            scheme.name,
        );
    }
}
