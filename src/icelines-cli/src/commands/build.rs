use crate::config::Config;
use anyhow::Context;
use icelines_site::builder::{SiteBuilder, SiteConfig};

pub async fn run(no_site: bool) -> anyhow::Result<()> {
    let cfg = Config::load()?;

    // Resolve paths relative to the project root (where mkdocs.yml lives)
    let project_root = find_project_root()?;
    let docs_dir = project_root.join("docs");
    let mkdocs_yml = project_root.join("mkdocs.yml");

    let builder = SiteBuilder::new(SiteConfig {
        docs_dir: docs_dir.clone(),
        mkdocs_yml: mkdocs_yml.clone(),
        snapshot_dir: cfg.snapshot_dir(),
        season: cfg.season_str().parse().unwrap_or(20252026),
    });

    println!("Building site from snapshot data...");
    let written = builder.build().context("site generation failed")?;

    println!(
        "  {} files written to {}",
        written.len(),
        docs_dir.display()
    );
    for f in &written {
        println!("  {f}");
    }

    if no_site {
        println!("Done (--no-site: skipping mkdocs).");
        return Ok(());
    }

    // Run mkdocs build
    println!("Running mkdocs build...");
    let status = std::process::Command::new("mkdocs")
        .arg("build")
        .env("PYTHONUTF8", "1")
        .current_dir(&project_root)
        .status()
        .context("mkdocs not found — install with `pip install mkdocs-material`")?;

    if !status.success() {
        anyhow::bail!("mkdocs build failed (exit {})", status);
    }
    println!("Site built successfully.");
    Ok(())
}

/// Walk up from the binary's working directory to find the project root
/// (the directory containing mkdocs.yml).
fn find_project_root() -> anyhow::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("mkdocs.yml").exists() {
            return Ok(dir);
        }
        match dir.parent() {
            Some(p) => dir = p.to_owned(),
            None => anyhow::bail!("mkdocs.yml not found — run icelines from the project root"),
        }
    }
}
