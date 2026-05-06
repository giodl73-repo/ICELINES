mod cli;
mod commands;
mod config;
mod db;
mod error;
mod event_stream;
pub mod fantasy_db;
mod render;
mod start_slug;
#[cfg(test)]
mod test_utils;
mod tui;

use anyhow::Context;
use clap::Parser;
use cli::{Cli, Commands, FantasySubcommand, QuerySubcommand};
use config::Config;

/// Post-LP review fix #4 — Reset SIGPIPE to SIG_DFL on Unix so a
/// downstream `| head` (or any consumer that closes its end) ends our
/// process with EPIPE → exit 141, instead of letting Rust's default
/// SIGPIPE-ignored behavior turn the next `println!` into a panic.
/// Windows has no SIGPIPE; this is a no-op there.
#[cfg(unix)]
fn reset_sigpipe_handler() {
    // SAFETY: signal(2) on a valid signal number with SIG_DFL is the
    // canonical way to opt out of Rust's libstd default. No
    // multi-threaded races at startup — main is single-threaded here.
    unsafe {
        let _ = libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe_handler() {
    // No SIGPIPE on Windows.
}

/// Bare-invocation banner. ≤12 lines, pipe-friendly. The four bold
/// doors first (menu / tui / serve / docs), then two example
/// invocations that hint at the filter grammar (the killer feature
/// most discoverers don't know about) and the drill-down launcher.
/// `icelines --help` still routes through clap for the full surface.
fn print_short_banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!("icelines v{version} — NHL analytics + fantasy CLI (38 seasons bundled)");
    println!();
    println!("Pick a surface:");
    println!("  icelines menu      Numbered launcher (P/T/G/C drill-downs, W web, Q quit)");
    println!("  icelines tui       Full terminal dashboard, 8 tabs");
    println!("  icelines serve     Web dashboard at http://localhost:8000");
    println!("  icelines docs      Full command reference");
    println!();
    println!("Quick:");
    println!("  icelines query leaders --filter \"age<=24 AND p>=80\"");
    println!("  icelines tui player Bedard");
    println!("  icelines --help    All commands and flags");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    reset_sigpipe_handler();

    // Load config early so any error surfaces before command dispatch.
    let cfg = Config::load()?;

    // Bare `icelines` (no args) prints a short banner pointing at
    // the four primary entry points, then exits 0. Pipe-friendly
    // (works under `icelines | less`); script-friendly (no surprise
    // raw-mode TUI launch); discovery-friendly (one keystroke to
    // each surface). For the full clap-rendered command list, users
    // run `icelines --help`.
    if std::env::args().len() == 1 {
        print_short_banner();
        return Ok(());
    }

    let cli = Cli::parse();
    config::init_live_feeds(cli.no_live, &cfg);
    config::init_dashboards(cli.no_dashboards, &cfg);

    let result = dispatch(cli, cfg).await;
    if let Err(e) = result {
        error::handle_error(e);
    }
    Ok(())
}

async fn dispatch(cli: Cli, cfg: Config) -> anyhow::Result<()> {
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
        Commands::Config(sub) => {
            commands::config::run(sub).await?;
        }
        Commands::Setup {
            accept_defaults,
            dry_run,
            reset,
        } => {
            commands::setup::run(accept_defaults, dry_run, reset).await?;
        }
        Commands::DataStatus { shard, stale_only } => {
            commands::data_status::run(shard, stale_only).await?;
        }
        Commands::Favorites {
            date,
            range,
            group,
            json,
        } => {
            commands::favorites::run(date, range, group, json).await?;
        }

        Commands::Serve {
            port,
            bind,
            no_open,
            no_cache,
            cors_origin,
        } => {
            commands::serve::run(port, bind, no_open, no_cache, cors_origin, &cfg).await?;
        }
        Commands::Tonight {
            team,
            date,
            week,
            month,
        } => {
            commands::tonight::run(team, date, week || month).await?;
        }
        Commands::Schedule {
            team,
            days,
            json,
            csv,
            date,
            start,
        } => {
            // Phase Foster.1 — `--start` is a deprecated alias for `--date`.
            // CLI flag explicit > deprecated alias > today.
            let resolved_date = date.or(start);
            commands::tonight::run_schedule(team, days, json, csv, resolved_date).await?;
        }
        Commands::Playoffs {
            season,
            round,
            json,
            csv,
        } => {
            commands::playoffs::run(season, round, json, csv).await?;
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
        Commands::Tui { surface, start } => {
            // LB.1+LB.2+LB.3 — resolve start screen BEFORE entering raw
            // mode, so resolution failures (unknown slug / unknown player
            // name / ambiguous match / bad team abbrev) print to normal
            // stderr (not the alt-screen) and exit non-zero cleanly.
            //
            // Precedence: sugar subcommand > --start flag > default (League).
            // If the user passes both, sugar wins (we could error, but
            // letting sugar win keeps `icelines tui goalies --start scores`
            // intuitive — the explicit subcommand reads first).
            let spec: start_slug::ScreenSpec = match (surface, start.as_deref()) {
                (Some(s), _) => s.into_screen_spec(),
                (None, Some(slug)) => start_slug::parse_start_slug(slug)
                    .with_context(|| format!("invalid --start {slug:?}"))?,
                (None, None) => start_slug::ScreenSpec::nav(start_slug::NavSpec::Home),
            };
            // Resolution can fail for parameterized variants (player /
            // team / goalie / comps). Errors carry the candidate listing
            // for the Sebastian Aho ambiguity case.
            let start_screen = spec.into_screen()?;
            tui::run_tui(tui::RunTuiOpts {
                no_color: false,
                start_screen,
            })
            .await?;
        }
        Commands::Docs => {
            // Embed COMMANDS.md at compile time. Always in lockstep
            // with the shipped binary — no internet, no fetching, no
            // chance of drift between docs and behavior.
            print!("{}", include_str!("../../COMMANDS.md"));
        }
        Commands::Menu => {
            commands::menu::run(&cfg).await?;
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
        Commands::Dashboard => tui::run_tui(tui::RunTuiOpts::home()).await?,
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
            QuerySubcommand::Career {
                league,
                season,
                top,
                sort,
                json,
                csv,
                week,
                month,
            } => {
                if week || month {
                    eprintln!(
                        "error: --week / --month not supported on `query career` (junior seasons\n       aren't aligned with NHL week boundaries). Use --season instead."
                    );
                    std::process::exit(2);
                }
                commands::query_career::run(league, season, top, sort, json, csv).await?;
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
