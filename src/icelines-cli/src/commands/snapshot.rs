use anyhow::Context;
use crate::cli::SnapshotSubcommand;
use crate::config::Config;
use icelines_fetch::snapshot::SnapshotStore;

pub async fn run(cmd: SnapshotSubcommand) -> anyhow::Result<()> {
    let cfg   = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    match cmd {
        SnapshotSubcommand::List => {
            let manifest = store.load_manifest().context("loading snapshot manifest")?;
            let entries  = manifest.snapshots;
            if entries.is_empty() {
                println!("No snapshots — run `icelines fetch all` to create one.");
                return Ok(());
            }
            println!("{:<40} {:<10} {:<12} {:<8} {:<6}",
                "Name", "Season", "Date", "Sealed", "Files");
            println!("{}", "─".repeat(80usize));
            for e in &entries {
                let active = if manifest.active.as_deref() == Some(&e.name) { " ←" } else { "" };
                println!("{:<40} {:<10} {:<12} {:<8} {}{}",
                    e.name, e.season, e.date,
                    if e.sealed { "✓" } else { "…" },
                    e.file_count, active);
            }
        }

        SnapshotSubcommand::Show { name } => {
            let manifest = store.load_manifest()?;
            let meta = store.load_manifest()
                .ok()
                .and_then(|_| store.list().ok())
                .and_then(|entries| entries.into_iter().find(|e| e.name == name));

            match meta {
                None => println!("Snapshot '{name}' not found."),
                Some(e) => {
                    let is_active = manifest.active.as_deref() == Some(&name);
                    println!("Snapshot:  {}{}", e.name, if is_active { " (ACTIVE)" } else { "" });
                    println!("Season:    {}", e.season);
                    println!("Tier:      {:?}", e.tier);
                    println!("Date:      {}", e.date);
                    println!("Created:   {}", e.created_at);
                    println!("Sealed:    {}", e.sealed);
                    println!("Files:     {}", e.file_count);
                    if let Some(ref p) = e.parent_key {
                        println!("Parent:    {p}");
                    }
                }
            }
        }

        SnapshotSubcommand::Use { name } => {
            store.set_active(&name).with_context(|| format!("setting active snapshot to '{name}'"))?;
            println!("Active snapshot set to '{name}'.");
        }

        SnapshotSubcommand::Verify { name } => {
            let target = match name {
                Some(n) => n,
                None => {
                    let manifest = store.load_manifest()?;
                    manifest.active.context("no active snapshot — specify a name")?
                }
            };
            print!("Verifying '{target}'... ");
            let failures = store.verify(&target).with_context(|| format!("verifying '{target}'"))?;
            if failures.is_empty() {
                println!("OK — all integrity checks passed.");
            } else {
                println!("FAILED — {} file(s) corrupt or missing:", failures.len());
                for f in &failures {
                    println!("  {f}");
                }
                anyhow::bail!("snapshot integrity check failed");
            }
        }

        SnapshotSubcommand::Delete { name } => {
            store.delete(&name).with_context(|| format!("deleting snapshot '{name}'"))?;
            println!("Deleted snapshot '{name}'.");
        }
    }
    Ok(())
}
