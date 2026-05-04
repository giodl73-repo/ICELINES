mod cli;
mod commands;
mod config;
mod db;
mod error;
pub mod fantasy_db;
mod render;
#[cfg(test)]
mod test_utils;
mod tui;

use anyhow::Context;
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
        Commands::Rank {
            top,
            pos,
            scheme,
            json,
            csv,
            out,
        } => {
            commands::rank::run(top, pos, scheme, json, csv, out).await?;
        }
        Commands::Snapshot(sub) => {
            commands::snapshot::run(sub).await?;
        }

        // ── Site (mkdocs) — Phase King Clancy King.1.0 rename ────────────────
        Commands::Site(sub) => {
            commands::site::run(sub).await?;
        }
        // Deprecated top-level aliases. Removed in v0.14.
        Commands::Build { no_site } => {
            eprintln!(
                "WARNING: 'icelines build' moved to 'icelines site build' in v0.13.\n\
                 The old alias is removed in v0.14. Run 'icelines site build' instead."
            );
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
            csv,
            out,
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
                csv,
                out,
            })
            .await?;
        }
        Commands::Class {
            year,
            pos,
            top,
            json,
            csv,
            out,
        } => {
            commands::analysis::run_class(year, pos, top, json, csv, out).await?;
        }
        Commands::Peers {
            player,
            size,
            json,
            csv,
            out,
        } => {
            commands::analysis::run_peers(player, size, json, csv, out).await?;
        }
        Commands::Compare {
            player1,
            player2,
            json,
            csv,
            out,
        } => {
            commands::analysis::run_compare(player1, player2, json, csv, out).await?;
        }
        Commands::History {
            player,
            seasons,
            json,
            csv,
            out,
        } => {
            commands::analysis::run_history(player, seasons, json, csv, out).await?;
        }
        Commands::Group(sub) => {
            commands::analysis::run_group(sub).await?;
        }
        Commands::Games(sub) => {
            commands::analysis::run_games(sub).await?;
        }

        Commands::Serve { port } => {
            eprintln!(
                "WARNING: 'icelines serve' is being reclaimed for the web dashboard\n\
                 in Phase King Clancy. Use 'icelines site serve' for the mkdocs preview.\n\
                 The old top-level alias is removed in v0.14."
            );
            commands::serve_deploy::run_serve(port).await?;
        }
        Commands::Deploy { remote } => {
            eprintln!(
                "WARNING: 'icelines deploy' moved to 'icelines site deploy' in v0.13.\n\
                 The old alias is removed in v0.14. Run 'icelines site deploy' instead."
            );
            commands::serve_deploy::run_deploy(&remote).await?;
        }
        Commands::Tonight { team } => {
            commands::tonight::run(team).await?;
        }
        Commands::Schedule { team, days } => {
            commands::tonight::run_schedule(team, days).await?;
        }
        Commands::Trade {
            player_out,
            _for: _,
            player_in,
            team,
        } => {
            commands::tonight::run_trade(player_out, player_in, team).await?;
        }
        Commands::Project {
            player,
            team,
            mode,
            games,
            json,
            csv,
            out,
        } => {
            commands::project::run(player, team, mode, games, json, csv, out).await?;
        }
        Commands::Tui => {
            tui::run_tui(false).await?;
        }
        Commands::Docs => {
            // Embed COMMANDS.md at compile time. Always in lockstep
            // with the shipped binary — no internet, no fetching, no
            // chance of drift between docs and behavior.
            print!("{}", include_str!("../../COMMANDS.md"));
        }
        Commands::Export(sub) => {
            commands::export::run(sub).await?;
        }
        Commands::X {
            shape,
            player,
            team,
            pos,
            year,
            top,
            seasons,
            json,
            out,
        } => {
            // Default to CSV (the whole point of `x` is "give me Excel-ready output").
            let csv = !json;
            use cli::ExportShape;
            match shape {
                ExportShape::Rank => {
                    commands::rank::run(top, pos, None, json, csv, out).await?;
                }
                ExportShape::Leaders => {
                    commands::query::run_leaders(commands::query::LeadersArgs {
                        pos,
                        team,
                        age_min: None,
                        age_max: None,
                        nationality: None,
                        draft_year: None,
                        round: None,
                        draft_pick_max: None,
                        undrafted: false,
                        rookie: false,
                        handedness: None,
                        ppg_min: None,
                        gp_min: None,
                        gp_max: None,
                        birth_province: None,
                        toi_min: None,
                        plus_minus_min: None,
                        shots_pg_min: None,
                        ufa: false,
                        rfa: false,
                        elc: false,
                        expiry_year: None,
                        seasons: 1,
                        season: None,
                        season_type: icelines_core::season_stats::SeasonType::Regular,
                        sort: "ppg".to_owned(),
                        top,
                        rate: false,
                        percentiles: false,
                        json,
                        csv,
                        filters: Vec::new(),
                    })
                    .await?;
                }
                ExportShape::Goalies => {
                    commands::query::run_goalies(commands::query::GoaliesArgs {
                        top,
                        sort: "sv-pct".to_owned(),
                        team,
                        min_gp: 5,
                        season: None,
                        season_type: icelines_core::season_stats::SeasonType::Regular,
                        json,
                        csv,
                        filters: Vec::new(),
                    })
                    .await?;
                }
                ExportShape::Players => {
                    commands::players::run(commands::players::PlayersArgs {
                        pos,
                        team,
                        age_max: None,
                        age_min: None,
                        nationality: None,
                        draft_year: None,
                        draft_round: None,
                        ppg_min: None,
                        gp_min: None,
                        top,
                        json,
                        csv,
                        out,
                    })
                    .await?;
                }
                ExportShape::Class => {
                    let yr = year.context("--year is required for `x class`")?;
                    commands::analysis::run_class(yr, pos, Some(top), json, csv, out).await?;
                }
                ExportShape::History => {
                    let p = player.context("--player is required for `x history`")?;
                    commands::analysis::run_history(p, seasons, json, csv, out).await?;
                }
                ExportShape::Peers => {
                    let p = player.context("--player is required for `x peers`")?;
                    commands::analysis::run_peers(p, top, json, csv, out).await?;
                }
                ExportShape::Compare => {
                    // `--player A --player B` is two args; clap can't bind that to one
                    // Option, so we accept comma-separated for the unified path:
                    // `icelines x compare --player "McDavid,Draisaitl"`
                    let p = player.context("--player \"a,b\" is required for `x compare`")?;
                    let (a, b) = p.split_once(',').context(
                        "for `x compare`, pass two names comma-separated, e.g. \
                         --player \"McDavid,Draisaitl\"",
                    )?;
                    commands::analysis::run_compare(
                        a.trim().to_owned(),
                        b.trim().to_owned(),
                        json,
                        csv,
                        out,
                    )
                    .await?;
                }
                ExportShape::Transactions => {
                    commands::transactions::run(
                        team,
                        None,
                        None,
                        None,
                        None,
                        player,
                        None,
                        Some(top),
                        json,
                        csv,
                        out,
                    )
                    .await?;
                }
            }
        }
        Commands::Transactions {
            team,
            since,
            until,
            kind,
            search,
            player,
            season,
            top,
            json,
            csv,
            out,
        } => {
            commands::transactions::run(
                team, since, until, kind, search, player, season, top, json, csv, out,
            )
            .await?;
        }
        Commands::Mates {
            player,
            top,
            json,
            csv,
            out,
        } => {
            commands::mates::run(player, top, json, csv, out).await?;
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
                pos,
                team,
                age_min,
                age_max,
                nationality,
                draft_year,
                round,
                draft_pick_max,
                undrafted,
                rookie,
                handedness,
                ppg_min,
                gp_min,
                gp_max,
                toi_min,
                plus_minus_min,
                shots_pg_min,
                birth_province,
                seasons,
                season,
                season_type,
                sort,
                top,
                rate,
                percentiles,
                json,
                csv,
                ufa,
                rfa,
                elc,
                expiry_year,
                filters,
            } => {
                commands::query::run_leaders(commands::query::LeadersArgs {
                    pos,
                    team,
                    age_min,
                    age_max,
                    nationality,
                    draft_year,
                    round,
                    draft_pick_max,
                    undrafted,
                    rookie,
                    handedness,
                    ppg_min,
                    gp_min,
                    gp_max,
                    birth_province,
                    toi_min,
                    plus_minus_min,
                    shots_pg_min,
                    ufa,
                    rfa,
                    elc,
                    expiry_year,
                    seasons,
                    season,
                    season_type: season_type.to_core(),
                    sort,
                    top,
                    rate,
                    percentiles,
                    json,
                    csv,
                    filters,
                })
                .await?;
            }
            QuerySubcommand::Player {
                name,
                breakdown,
                percentiles,
                last_n,
                season,
                season_type,
                rank_by,
                filters,
                seasons,
            } => {
                commands::query::run_player(
                    name,
                    breakdown,
                    percentiles,
                    last_n,
                    season,
                    season_type.to_core(),
                    rank_by,
                    filters,
                    seasons,
                )
                .await?;
            }
            QuerySubcommand::Compare {
                player1,
                player2,
                similar,
                by,
                season,
                season_type,
                filters,
                seasons,
            } => {
                commands::query::run_compare(
                    player1,
                    player2,
                    similar,
                    by,
                    season,
                    season_type.to_core(),
                    filters,
                    seasons,
                )
                .await?;
            }
            QuerySubcommand::Goalies {
                top,
                sort,
                team,
                min_gp,
                season,
                season_type,
                json,
                csv,
                filters,
            } => {
                commands::query::run_goalies(commands::query::GoaliesArgs {
                    filters,
                    top,
                    sort,
                    team,
                    min_gp,
                    season,
                    season_type: season_type.to_core(),
                    json,
                    csv,
                })
                .await?;
            }
        },
        Commands::Fantasy(sub) => match sub {
            FantasySubcommand::LeagueCreate { name, scheme } => {
                commands::fantasy::run_league_create(name, scheme).await?
            }
            FantasySubcommand::LeagueList => commands::fantasy::run_league_list().await?,
            FantasySubcommand::LeagueUse { name } | FantasySubcommand::LeagueSwitch { name } => {
                commands::fantasy::run_league_use(name).await?
            }
            FantasySubcommand::LeagueDelete { name } => {
                commands::fantasy::run_league_delete(name).await?
            }
            FantasySubcommand::TeamCreate {
                name,
                owner,
                league,
            } => commands::fantasy::run_team_create(name, owner, league).await?,
            FantasySubcommand::TeamList { league } => {
                commands::fantasy::run_team_list(league).await?
            }
            FantasySubcommand::TeamShow { name, league } => {
                commands::fantasy::run_team_show(name, league).await?
            }
            FantasySubcommand::TeamAdd {
                team,
                player,
                league,
            } => commands::fantasy::run_team_add(team, player, league).await?,
            FantasySubcommand::TeamDrop {
                team,
                player,
                league,
            } => commands::fantasy::run_team_drop(team, player, league).await?,
            FantasySubcommand::Standings { league, scheme } => {
                commands::fantasy::run_standings(league, scheme).await?
            }
            FantasySubcommand::Trade {
                player1,
                to_team,
                for_player,
                execute,
                league,
            } => {
                commands::fantasy::run_trade(player1, to_team, for_player, execute, league).await?
            }
            FantasySubcommand::Serve { port, league } => {
                commands::fantasy::run_serve(port, league).await?
            }
        },
    }
    Ok(())
}
