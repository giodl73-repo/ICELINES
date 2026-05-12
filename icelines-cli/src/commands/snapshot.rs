use crate::cli::SnapshotSubcommand;
use crate::config::Config;
use anyhow::Context;
use icelines_core::{SnapshotEntryInput, SnapshotView, ViewContext, ViewWindow, CURRENT_SEASON};
use icelines_fetch::snapshot::{SnapshotEntry, SnapshotStore};

pub async fn run(cmd: SnapshotSubcommand) -> anyhow::Result<()> {
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());

    match cmd {
        SnapshotSubcommand::List => {
            let manifest = store.load_manifest().context("loading snapshot manifest")?;
            let view = snapshot_view(manifest.active, manifest.snapshots, None);
            if view.rows.is_empty() {
                println!("No snapshots - run `icelines fetch all` to create one.");
                return Ok(());
            }
            print_snapshot_list(&view);
        }

        SnapshotSubcommand::Show { name } => {
            let manifest = store.load_manifest()?;
            let view = snapshot_view(manifest.active, manifest.snapshots, Some(&name));
            match view.selected {
                None => println!("Snapshot '{name}' not found."),
                Some(e) => print_snapshot_detail(&e),
            }
        }

        SnapshotSubcommand::Use { name } => {
            store
                .set_active(&name)
                .with_context(|| format!("setting active snapshot to '{name}'"))?;
            println!("Active snapshot set to '{name}'.");
        }

        SnapshotSubcommand::Verify { name } => {
            let target = match name {
                Some(n) => n,
                None => {
                    let manifest = store.load_manifest()?;
                    manifest
                        .active
                        .context("no active snapshot - specify a name")?
                }
            };
            print!("Verifying '{target}'... ");
            let failures = store
                .verify(&target)
                .with_context(|| format!("verifying '{target}'"))?;
            if failures.is_empty() {
                println!("OK - all integrity checks passed.");
            } else {
                println!("FAILED - {} file(s) corrupt or missing:", failures.len());
                for f in &failures {
                    println!("  {f}");
                }
                anyhow::bail!("snapshot integrity check failed");
            }
        }

        SnapshotSubcommand::Delete { name } => {
            store
                .delete(&name)
                .with_context(|| format!("deleting snapshot '{name}'"))?;
            println!("Deleted snapshot '{name}'.");
        }

        SnapshotSubcommand::Rebuild { name, chunked } => {
            if !chunked {
                anyhow::bail!(
                    "snapshot rebuild requires --chunked (the only supported rebuild mode in v1)"
                );
            }
            if store.is_chunked(&name) {
                println!("Snapshot '{name}' is already chunked - nothing to do.");
                return Ok(());
            }
            let cm = store
                .rebuild_chunked(&name)
                .with_context(|| format!("rebuilding snapshot '{name}' as chunked"))?;
            println!(
                "Migrated '{name}' to chunked layout ({} bios + {} stats chunks).",
                cm.bios().len(),
                cm.stats().len(),
            );
            println!("Run `icelines snapshot gc` to sweep any chunks now unreferenced.");
        }

        SnapshotSubcommand::Gc { dry_run } => {
            let report = store.gc_chunks(dry_run).context("gc_chunks failed")?;
            let kb = report.bytes_freed / 1024;
            if report.dry_run {
                println!(
                    "Dry run - would remove {} chunk(s), freeing ~{} KB.",
                    report.removed, kb,
                );
            } else if report.removed == 0 {
                println!("Nothing to sweep - all chunks are still referenced.");
            } else {
                println!("Swept {} chunk(s), freed ~{} KB.", report.removed, kb);
            }
        }

        SnapshotSubcommand::Prune { keep, dry_run } => {
            let report = store.prune(keep, dry_run).context("prune failed")?;
            if report.dry_run {
                if report.planned == 0 {
                    println!("Dry run - nothing to prune (kept {keep} per tier).");
                } else {
                    println!(
                        "Dry run - would delete {} snapshot(s) (keeping newest {keep} per tier):",
                        report.planned,
                    );
                    for n in &report.names {
                        println!("  {n}");
                    }
                    println!("Run without --dry-run to commit. Then `snapshot gc` to reclaim chunk space.");
                }
            } else if report.deleted == 0 {
                println!("Nothing to prune (kept {keep} per tier).");
            } else {
                println!("Pruned {} snapshot(s):", report.deleted);
                for n in &report.names {
                    println!("  {n}");
                }
                println!("Run `icelines snapshot gc` to reclaim freed chunk space.");
            }
        }

        SnapshotSubcommand::Diff { a, b } => {
            let report = store
                .diff(&a, &b)
                .with_context(|| format!("diff '{a}' vs '{b}'"))?;
            if report.is_empty() {
                println!("No differences between '{a}' and '{b}'.");
            } else {
                println!("Diff: '{a}' -> '{b}'");
                if !report.removed.is_empty() {
                    println!(
                        "  Removed players ({}): {}",
                        report.removed.len(),
                        report
                            .removed
                            .iter()
                            .take(10)
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if !report.added.is_empty() {
                    println!(
                        "  Added players ({}): {}",
                        report.added.len(),
                        report
                            .added
                            .iter()
                            .take(10)
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if !report.changed_bios.is_empty() {
                    println!(
                        "  Changed bios ({}): {}",
                        report.changed_bios.len(),
                        report
                            .changed_bios
                            .iter()
                            .take(10)
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
                if !report.changed_stats.is_empty() {
                    println!(
                        "  Changed stats ({}): {}",
                        report.changed_stats.len(),
                        report
                            .changed_stats
                            .iter()
                            .take(10)
                            .map(u32::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    );
                }
            }
        }
    }
    Ok(())
}

fn snapshot_view(
    active: Option<String>,
    entries: Vec<SnapshotEntry>,
    selected_name: Option<&str>,
) -> SnapshotView {
    SnapshotView::from_entries(
        ViewContext::new(ViewWindow::new(
            icelines_core::model::Season(CURRENT_SEASON),
            icelines_core::season_stats::SeasonType::Regular,
        )),
        active,
        entries
            .into_iter()
            .map(|entry| SnapshotEntryInput {
                name: entry.name,
                season: entry.season,
                tier: format!("{:?}", entry.tier),
                date: entry.date,
                created_at: entry.created_at,
                parent_key: entry.parent_key,
                file_count: entry.file_count,
                sealed: entry.sealed,
            })
            .collect(),
        selected_name,
    )
}

fn print_snapshot_list(view: &SnapshotView) {
    println!(
        "{:<40} {:<10} {:<12} {:<8} {:<6}",
        "Name", "Season", "Date", "Sealed", "Files"
    );
    println!("{}", "-".repeat(80usize));
    for e in &view.rows {
        let active = if e.is_active { " <-" } else { "" };
        println!(
            "{:<40} {:<10} {:<12} {:<8} {}{}",
            e.name, e.season, e.date, e.sealed_label, e.file_count, active
        );
    }
}

fn print_snapshot_detail(e: &icelines_core::SnapshotRow) {
    println!(
        "Snapshot:  {}{}",
        e.name,
        if e.is_active { " (ACTIVE)" } else { "" }
    );
    println!("Season:    {}", e.season);
    println!("Tier:      {}", e.tier);
    println!("Date:      {}", e.date);
    println!("Created:   {}", e.created_at);
    println!("Sealed:    {}", e.sealed);
    println!("Files:     {}", e.file_count);
    if let Some(ref p) = e.parent_key {
        println!("Parent:    {p}");
    }
}
