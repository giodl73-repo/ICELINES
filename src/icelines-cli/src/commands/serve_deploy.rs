use anyhow::Context;
use std::process::Command;

fn project_root() -> anyhow::Result<std::path::PathBuf> {
    let mut dir = std::env::current_dir()?;
    loop {
        if dir.join("mkdocs.yml").exists() { return Ok(dir); }
        match dir.parent() {
            Some(p) => dir = p.to_owned(),
            None => anyhow::bail!("mkdocs.yml not found — run from the project root"),
        }
    }
}

/// icelines serve — build site then launch mkdocs serve locally.
pub async fn run_serve(port: u16) -> anyhow::Result<()> {
    let root = project_root()?;

    // Build first
    println!("Building site...");
    super::build::run(true).await?;

    println!("Serving at http://127.0.0.1:{port}  (Ctrl+C to stop)");
    let status = Command::new("mkdocs")
        .args(["serve", "--dev-addr", &format!("127.0.0.1:{port}")])
        .env("PYTHONUTF8", "1")
        .current_dir(&root)
        .status()
        .context("mkdocs not found — install with `pip install mkdocs-material`")?;

    if !status.success() {
        anyhow::bail!("mkdocs serve exited with {}", status);
    }
    Ok(())
}

/// icelines deploy — build site and push to GitHub Pages.
pub async fn run_deploy(remote: &str) -> anyhow::Result<()> {
    let root = project_root()?;

    println!("Building site...");
    super::build::run(false).await?;

    println!("Deploying to GitHub Pages (remote: {remote})...");
    let status = Command::new("mkdocs")
        .args(["gh-deploy", "--remote-name", remote, "--force"])
        .env("PYTHONUTF8", "1")
        .current_dir(&root)
        .status()
        .context("mkdocs not found — install with `pip install mkdocs-material`")?;

    if !status.success() {
        anyhow::bail!("mkdocs gh-deploy exited with {}", status);
    }
    println!("Deployed. Live at https://giodl73-repo.github.io/ICELINES/");
    Ok(())
}
