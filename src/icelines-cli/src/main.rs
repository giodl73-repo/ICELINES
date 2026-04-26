mod cli;
mod commands;
mod config;
mod error;
mod render;

use clap::Parser;
use cli::{Cli, Commands};
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load config early so any error surfaces before command dispatch.
    let _cfg = Config::load()?;

    let cli = Cli::parse();

    let result = dispatch(cli).await;
    if let Err(e) = result {
        error::handle_error(e);
    }
    Ok(())
}

async fn dispatch(cli: Cli) -> anyhow::Result<()> {
    match cli.command {
        // ── Phase 1 implemented commands ──────────────────────────────────────
        Commands::Fetch(sub) => {
            commands::fetch::run(sub).await?;
        }
        Commands::Team {
            team,
            scheme,
            no_color,
        } => {
            commands::team::run(team, scheme, no_color).await?;
        }
        Commands::Rank { top, pos, scheme } => {
            commands::rank::run(top, pos, scheme).await?;
        }
        Commands::Snapshot(sub) => {
            commands::snapshot::run(sub).await?;
        }

        // ── Phase 2 implemented commands ─────────────────────────────────────
        Commands::Build { no_site } => {
            commands::build::run(no_site).await?;
        }

        // ── Phase 3 stubs ────────────────────────────────────────────────────
        Commands::Serve => stub("serve"),
        Commands::Deploy => stub("deploy"),
        Commands::Compare => stub("compare"),
        Commands::Tonight => stub("tonight"),
        Commands::Schedule => stub("schedule"),
        Commands::Trade => stub("trade"),
        Commands::Project => stub("project"),
        Commands::Tui => stub("tui"),
        Commands::Players => stub("players"),
        Commands::Class => stub("class"),
        Commands::Peers => stub("peers"),
        Commands::History => stub("history"),
        Commands::Mates => stub("mates"),
        Commands::Group => stub("group"),
        Commands::Scouting => stub("scouting"),
        Commands::Scheme(sub) => {
            commands::scheme::run(sub).await?;
        }
        Commands::Dashboard => stub("dashboard"),
    }
    Ok(())
}

fn stub(name: &str) {
    println!("icelines {name}: not yet implemented in Phase 1 scaffolding");
}
