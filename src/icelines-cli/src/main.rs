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

        // ── Phase 2 player analysis ───────────────────────────────────────────
        Commands::Players {
            pos,
            team,
            age_max,
            age_min,
            nationality,
            draft_year,
            draft_round,
            ppg_min,
            gp_min,
            top,
            json,
        } => {
            commands::players::run(commands::players::PlayersArgs {
                pos,
                team,
                age_max,
                age_min,
                nationality,
                draft_year,
                draft_round,
                ppg_min,
                gp_min,
                top,
                json,
            })
            .await?;
        }
        Commands::Class {
            year,
            pos,
            top,
            json,
        } => {
            commands::analysis::run_class(year, pos, top, json).await?;
        }
        Commands::Peers { player, size, json } => {
            commands::analysis::run_peers(player, size, json).await?;
        }
        Commands::Compare {
            player1,
            player2,
            json,
        } => {
            commands::analysis::run_compare(player1, player2, json).await?;
        }
        Commands::History { player, json } => {
            commands::analysis::run_history(player, json).await?;
        }
        Commands::Group(sub) => {
            commands::analysis::run_group(sub).await?;
        }

        // ── Phase 3 stubs ────────────────────────────────────────────────────
        Commands::Serve => stub("serve"),
        Commands::Deploy => stub("deploy"),
        Commands::Tonight => stub("tonight"),
        Commands::Schedule => stub("schedule"),
        Commands::Trade => stub("trade"),
        Commands::Project => stub("project"),
        Commands::Tui => stub("tui"),
        Commands::Mates => stub("mates"),
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
