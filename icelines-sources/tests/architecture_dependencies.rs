use std::fs;
use std::path::Path;

#[test]
fn l0_sources_manifest_has_only_the_reviewed_direct_dependencies() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read icelines-sources Cargo.toml");
    let dependencies = manifest
        .split("[dependencies]")
        .nth(1)
        .expect("dependencies section")
        .split("[dev-dependencies]")
        .next()
        .expect("end of dependencies section");

    for forbidden in [
        "reqwest",
        "tokio",
        "fletch",
        "rusqlite",
        "axum",
        "ratatui",
        "icelines-fetch",
        "icelines-query",
        "icelines-cli",
        "icelines-web",
        "icelines-site",
    ] {
        assert!(
            !dependencies.contains(forbidden),
            "icelines-sources has forbidden direct dependency {forbidden}"
        );
    }
}

#[test]
fn l0_sources_code_has_no_transport_or_persistence_calls() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rust_files(&src, &mut files);

    for path in files {
        let source = fs::read_to_string(&path).expect("read icelines-sources Rust source");
        for forbidden in ["std::fs", "tokio::", "reqwest::", "rusqlite::", "TcpStream"] {
            assert!(
                !source.contains(forbidden),
                "{} contains forbidden transport/persistence token {forbidden}",
                path.display()
            );
        }
    }
}

fn collect_rust_files(directory: &Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("read icelines-sources source directory") {
        let path = entry.expect("read source directory entry").path();
        if path.is_dir() {
            collect_rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}
