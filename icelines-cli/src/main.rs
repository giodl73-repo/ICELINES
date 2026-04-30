mod cli;
mod commands;
mod config;
mod db;
mod error;
pub mod fantasy_db;
mod render;
#[cfg(test)] mod test_utils;
mod tui;

use clap::Parser;
use cli::{Cli, Commands, FantasySubcommand, QuerySubcommand};
use config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load config early so any error surfaces before command dispatch.
    let cfg = Config::load()?;

    // icelines with no args → launch TUI. Resolve the live-feeds and
    // dashboards toggles from env + config (no CLI flag yet because
    // we haven't parsed).
    if std::env::args().len() == 1 {
        config::init_live_feeds(false, &cfg);
        config::init_dashboards(false, &cfg);
        return tui::run_tui(false).await;
    }

    let cli = Cli::parse();
    config::init_live_feeds(cli.no_live, &cfg);
    config::init_dashboards(cli.no_dashboards, &cfg);

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
        Commands::History { player, seasons, json } => {
            commands::analysis::run_history(player, seasons, json).await?;
        }
        Commands::Group(sub) => {
            commands::analysis::run_group(sub).await?;
        }
        Commands::Games(sub) => {
            commands::analysis::run_games(sub).await?;
        }

        Commands::Serve { port } => {
            commands::serve_deploy::run_serve(port).await?;
        }
        Commands::Deploy { remote } => {
            commands::serve_deploy::run_deploy(&remote).await?;
        }
        Commands::Tonight { team } => {
            commands::tonight::run(team).await?;
        }
        Commands::Schedule { team, days } => {
            commands::tonight::run_schedule(team, days).await?;
        }
        Commands::Trade { player_out, _for: _, player_in, team } => {
            commands::tonight::run_trade(player_out, player_in, team).await?;
        }
        Commands::Project { player, team, mode, games } => {
            commands::project::run(player, team, mode, games).await?;
        }
        Commands::Tui => {
            tui::run_tui(false).await?;
        }
        Commands::Export(sub) => {
            commands::export::run(sub).await?;
        }
        Commands::Mates { player, top } => {
            commands::mates::run(player, top).await?;
        }
        Commands::Scouting { player, format } => {
            commands::scouting::run(player, format).await?;
        }
        Commands::Scheme(sub) => {
            commands::scheme::run(sub).await?;
        }
        Commands::Dashboard => tui::run_tui(false).await?,
        Commands::Data(sub) => commands::data::run(sub).await?,
        Commands::Query(sub) => match sub {
            QuerySubcommand::Leaders {
                pos, team, age_min, age_max, nationality,
                draft_year, round, draft_pick_max, undrafted, rookie,
                handedness, ppg_min, gp_min, gp_max,
                toi_min, plus_minus_min, shots_pg_min, birth_province,
                seasons, season, sort, top, rate, percentiles, json, csv,
                ufa, rfa, elc, expiry_year,
            } => {
                commands::query::run_leaders(commands::query::LeadersArgs {
                    pos, team, age_min, age_max, nationality,
                    draft_year, round, draft_pick_max, undrafted, rookie,
                    handedness, ppg_min, gp_min, gp_max,
                    birth_province, toi_min, plus_minus_min, shots_pg_min,
                    ufa, rfa, elc, expiry_year,
                    seasons, season, sort, top, rate, percentiles, json, csv,
                }).await?;
            }
            QuerySubcommand::Player { name, breakdown, percentiles, last_n, season } => {
                commands::query::run_player(name, breakdown, percentiles, last_n, season).await?;
            }
            QuerySubcommand::Compare { player1, player2, similar, by, season } => {
                commands::query::run_compare(player1, player2, similar, by, season).await?;
            }
            QuerySubcommand::Goalies { top, sort, team, min_gp, season, json, csv } => {
                commands::query::run_goalies(commands::query::GoaliesArgs {
                    top, sort, team, min_gp, season, json, csv,
                }).await?;
            }
        },
        Commands::Fantasy(sub) => match sub {
            FantasySubcommand::LeagueCreate { name, scheme } =>
                commands::fantasy::run_league_create(name, scheme).await?,
            FantasySubcommand::LeagueList =>
                commands::fantasy::run_league_list().await?,
            FantasySubcommand::LeagueUse { name } | FantasySubcommand::LeagueSwitch { name } =>
                commands::fantasy::run_league_use(name).await?,
            FantasySubcommand::LeagueDelete { name } =>
                commands::fantasy::run_league_delete(name).await?,
            FantasySubcommand::TeamCreate { name, owner, league } =>
                commands::fantasy::run_team_create(name, owner, league).await?,
            FantasySubcommand::TeamList { league } =>
                commands::fantasy::run_team_list(league).await?,
            FantasySubcommand::TeamShow { name, league } =>
                commands::fantasy::run_team_show(name, league).await?,
            FantasySubcommand::TeamAdd { team, player, league } =>
                commands::fantasy::run_team_add(team, player, league).await?,
            FantasySubcommand::TeamDrop { team, player, league } =>
                commands::fantasy::run_team_drop(team, player, league).await?,
            FantasySubcommand::Standings { league, scheme } =>
                commands::fantasy::run_standings(league, scheme).await?,
            FantasySubcommand::Trade { player1, to_team, for_player, execute, league } =>
                commands::fantasy::run_trade(player1, to_team, for_player, execute, league).await?,
            FantasySubcommand::Serve { port, league } =>
                commands::fantasy::run_serve(port, league).await?,
        },
    }
    Ok(())
}
