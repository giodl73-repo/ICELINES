mod ai;
mod cli;
mod commands;
mod config;
mod db;
mod error;
mod event_stream;
pub mod fantasy_db;
mod favorites_view;
mod render;
mod start_slug;
#[cfg(test)]
mod test_utils;
mod tui;
mod visual;

use anyhow::Context;
use clap::Parser;
use cli::{
    Cli, Commands, FantasySubcommand, QuerySubcommand, RecordsSubcommand, ReportSubcommand,
    WatchSubcommand,
};
use config::Config;
use icelines_core::{WorkbenchId, WorkbenchLayoutStore};

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
    println!("  icelines tui       Jack Adams terminal dashboard + command bar");
    println!("  icelines serve     Web dashboard at http://localhost:8000");
    println!("  icelines docs      Full command reference");
    println!();
    println!("Quick:");
    println!("  icelines query leaders --filter \"age<=24 AND p>=80\"");
    println!("  icelines report list");
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
        Commands::TeamSeason { team, json } => {
            commands::team::run_team_season(team, json).await?;
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
        Commands::Poach {
            season,
            season_type,
            scheme,
            categories,
            team,
            pos,
            availability,
            top,
            json,
        } => {
            commands::poach::run(commands::poach::PoachArgs {
                season,
                season_type,
                scheme,
                categories,
                teams: team,
                positions: pos,
                availability,
                top,
                json,
            })
            .await?;
        }
        Commands::Report(sub) => match sub {
            ReportSubcommand::List { json } => {
                commands::report::run_list(json)?;
            }
            ReportSubcommand::Poach {
                season,
                season_type,
                scheme,
                categories,
                team,
                pos,
                availability,
                top,
                json,
                out,
            } => {
                commands::poach::run_report_poach(commands::poach::PoachReportArgs {
                    season,
                    season_type,
                    scheme,
                    categories,
                    teams: team,
                    positions: pos,
                    availability,
                    top,
                    json,
                    out,
                })
                .await?;
            }
            ReportSubcommand::Weekly {
                season,
                season_type,
                scheme,
                league,
                categories,
                team,
                pos,
                availability,
                top,
                json,
                out,
            } => {
                commands::poach::run_report_weekly(commands::poach::WeeklyReportArgs {
                    season,
                    season_type,
                    scheme,
                    league,
                    categories,
                    teams: team,
                    positions: pos,
                    availability,
                    top,
                    json,
                    out,
                })
                .await?;
            }
        },
        Commands::Records(sub) => match sub {
            RecordsSubcommand::Player {
                player,
                metric,
                json,
                csv,
                out,
            } => {
                commands::records::run_player(player, metric, json, csv, out).await?;
            }
            RecordsSubcommand::Team {
                team,
                metric,
                json,
                csv,
                out,
            } => {
                commands::records::run_team(team, metric, json, csv, out).await?;
            }
        },
        Commands::Awards {
            player,
            json,
            csv,
            out,
        } => {
            commands::awards::run(player, json, csv, out).await?;
        }
        Commands::Streaks {
            player,
            json,
            csv,
            out,
        } => {
            commands::streaks::run(player, json, csv, out).await?;
        }
        Commands::Watch(sub) => match sub {
            WatchSubcommand::List { json } => {
                commands::poach::run_watch_list(commands::poach::WatchListArgs { json }).await?;
            }
            WatchSubcommand::Note {
                player,
                reason,
                json,
            } => {
                commands::poach::run_watch_note(commands::poach::WatchNoteArgs {
                    player,
                    reason: reason.join(" "),
                    json,
                })
                .await?;
            }
            WatchSubcommand::Rules {
                season,
                season_type,
                json,
            } => {
                commands::poach::run_watch_rules(commands::poach::WatchRulesArgs {
                    season,
                    season_type,
                    json,
                })
                .await?;
            }
            WatchSubcommand::Enable { id, json } => {
                commands::poach::run_watch_set_enabled(commands::poach::WatchSetEnabledArgs {
                    id,
                    enabled: true,
                    json,
                })
                .await?;
            }
            WatchSubcommand::Disable { id, json } => {
                commands::poach::run_watch_set_enabled(commands::poach::WatchSetEnabledArgs {
                    id,
                    enabled: false,
                    json,
                })
                .await?;
            }
            WatchSubcommand::Fire {
                id,
                player,
                message,
                json,
            } => {
                commands::poach::run_watch_fire(commands::poach::WatchFireArgs {
                    id,
                    player,
                    message: message.join(" "),
                    json,
                })
                .await?;
            }
            WatchSubcommand::History { limit, json } => {
                commands::poach::run_watch_history(commands::poach::WatchHistoryArgs {
                    limit,
                    json,
                })
                .await?;
            }
            WatchSubcommand::Alerts {
                season,
                season_type,
                top,
                save,
                json,
            } => {
                commands::poach::run_watch_alerts(commands::poach::WatchAlertsArgs {
                    season,
                    season_type,
                    top,
                    save,
                    json,
                })
                .await?;
            }
            WatchSubcommand::Player {
                player,
                when,
                season,
                season_type,
                json,
                save,
            } => {
                commands::poach::run_watch_player(commands::poach::WatchPlayerArgs {
                    player,
                    when,
                    season,
                    season_type,
                    json,
                    save,
                })
                .await?;
            }
            WatchSubcommand::Deployment {
                team,
                line_change,
                season,
                season_type,
                json,
                save,
            } => {
                commands::poach::run_watch_deployment(commands::poach::WatchDeploymentArgs {
                    team,
                    line_change,
                    season,
                    season_type,
                    json,
                    save,
                })
                .await?;
            }
        },
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
            series,
        } => {
            if let Some(letter) = series {
                commands::playoffs::run_series_momentum(season, letter, json).await?;
            } else {
                commands::playoffs::run(season, round, json, csv).await?;
            }
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
        Commands::Tui {
            surface,
            start,
            layout,
            standalone,
            mdi,
            classic,
            render_leaders_active_filter_snapshot,
        } => {
            if render_leaders_active_filter_snapshot {
                print!("{}", tui::render_leaders_active_filter_snapshot()?);
                return Ok(());
            }

            // LB.1+LB.2+LB.3 — resolve start screen BEFORE entering raw
            // mode, so resolution failures (unknown slug / unknown player
            // name / ambiguous match / bad team abbrev) print to normal
            // stderr (not the alt-screen) and exit non-zero cleanly.
            //
            // Precedence: sugar subcommand > --start flag > default (League).
            // If the user passes both, sugar wins (we could error, but
            // letting sugar win keeps `icelines tui goalies --start scores`
            // intuitive — the explicit subcommand reads first).
            let layout_record = if let Some(name) = layout.as_deref() {
                let store = WorkbenchLayoutStore::load_from_path(cfg.layout_store_path())
                    .with_context(|| format!("load layout {name:?}"))?;
                Some(store.get(name)?.clone())
            } else {
                None
            };
            let spec: start_slug::ScreenSpec = if let Some(layout_record) = layout_record.as_ref() {
                screen_spec_for_workbench(layout_record.center_id()?)?
            } else {
                match (surface, start.as_deref()) {
                    (Some(s), _) => s.into_screen_spec(),
                    (None, Some(slug)) => start_slug::parse_start_slug(slug)
                        .with_context(|| format!("invalid --start {slug:?}"))?,
                    (None, None) => start_slug::ScreenSpec::nav(start_slug::NavSpec::Home),
                }
            };
            // Resolution can fail for parameterized variants (player /
            // team / goalie / comps). Errors carry the candidate listing
            // for the Sebastian Aho ambiguity case.
            let start_screen = spec.into_screen()?;
            let dashboard_mode = mdi || (!classic && !standalone);
            tui::run_tui(tui::RunTuiOpts {
                no_color: false,
                start_screen,
                standalone,
                mdi: dashboard_mode,
                layout: layout_record,
            })
            .await?;
        }
        Commands::Layout(sub) => {
            commands::layout::run(sub, &cfg)?;
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
                        json_envelope: false,
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
        Commands::Signals {
            player,
            season,
            season_type,
            json,
        } => {
            commands::signals::run_signals(player, season, season_type.to_core(), json).await?;
        }
        Commands::SignalsRoster {
            team,
            season,
            season_type,
            json,
        } => {
            commands::signals::run_signals_roster(team, season, season_type.to_core(), json)
                .await?;
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
                json_envelope,
                csv,
                ufa,
                rfa,
                elc,
                expiry_year,
                filters,
                week,
                month,
                playoff,
                explain,
            } => {
                if json_envelope && (explain || playoff || week || month) {
                    anyhow::bail!(
                        "--json-envelope is only supported for standard `query leaders` output"
                    );
                }
                if explain {
                    // Phase Art Ross A.5 — print the parsed plan
                    // and exit. Doesn't load player data.
                    commands::query::run_explain(&filters, json)?;
                    return Ok(());
                }
                if playoff {
                    commands::query_window::run_playoff_leaders(top, sort, json).await?;
                    return Ok(());
                }
                if week || month {
                    // Wave 11 — `run_windowed_leaders` doesn't yet
                    // wire `--filter` through. Silently dropping
                    // user input is the worst UX; reject loudly
                    // until F.5b ships filter application on the
                    // boxscore-aggregate path.
                    if !filters.is_empty() {
                        anyhow::bail!(
                            "--filter is not yet supported on `query leaders --week`/`--month` \
                             (filters drop on the boxscore-aggregate path). For windowed \
                             filtering, run a fresh `icelines fetch boxscore` first and use \
                             the per-game lines via `icelines favorites --range week`."
                        );
                    }
                    let timeframe = if week {
                        icelines_core::timeframe::Timeframe::Week
                    } else {
                        icelines_core::timeframe::Timeframe::Month
                    };
                    commands::query_window::run_windowed_leaders(timeframe, top, sort, json)
                        .await?;
                    return Ok(());
                }
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
                    json_envelope,
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
                filters,
            } => {
                if week || month {
                    eprintln!(
                        "error: --week / --month not supported on `query career` (junior seasons\n       aren't aligned with NHL week boundaries). Use --season instead."
                    );
                    std::process::exit(2);
                }
                commands::query_career::run(league, season, top, sort, json, csv, filters).await?;
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
            FantasySubcommand::TeamUse { name, league } => {
                commands::fantasy::run_team_use(name, league).await?
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
            FantasySubcommand::Gaps {
                league,
                scheme,
                categories,
                top,
                json,
            } => commands::fantasy::run_gaps(league, scheme, categories, top, json).await?,
            FantasySubcommand::Simulate {
                league,
                scheme,
                weeks,
                add_player,
                drop_player,
                json,
            } => {
                commands::fantasy::run_simulate(
                    league,
                    scheme,
                    weeks,
                    add_player,
                    drop_player,
                    json,
                )
                .await?
            }
            FantasySubcommand::Daily {
                date,
                league,
                season,
                season_type,
                json,
            } => {
                commands::fantasy::run_daily(date, league, season, season_type.to_core(), json)
                    .await?
            }
            FantasySubcommand::Matchup {
                date,
                league,
                season,
                season_type,
                json,
            } => {
                commands::fantasy::run_matchup(date, league, season, season_type.to_core(), json)
                    .await?
            }
            FantasySubcommand::MatchupSet {
                week,
                home,
                away,
                league,
            } => commands::fantasy::run_matchup_set(week, home, away, league).await?,
            FantasySubcommand::ImportYahoo {
                file,
                league,
                my_team,
                dry_run,
                json,
            } => commands::fantasy::run_import_yahoo(file, league, my_team, dry_run, json).await?,
            FantasySubcommand::RosterShape { league, json } => {
                commands::fantasy::run_roster_shape_show(league, json).await?
            }
            FantasySubcommand::RosterShapeSet { shape, league } => {
                commands::fantasy::run_roster_shape_set(shape, league).await?
            }
            FantasySubcommand::RosterShapeValidate { league, team, json } => {
                commands::fantasy::run_roster_shape_validate(league, team, json).await?
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

fn screen_spec_for_workbench(id: WorkbenchId) -> anyhow::Result<start_slug::ScreenSpec> {
    match id {
        WorkbenchId::League => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Home)),
        WorkbenchId::Depth => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Depth)),
        WorkbenchId::Stats => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Queries)),
        WorkbenchId::Goalies => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Goalies)),
        WorkbenchId::Scores => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Tonight)),
        WorkbenchId::Schedule => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Schedule)),
        WorkbenchId::Transactions => Ok(start_slug::ScreenSpec::nav(
            start_slug::NavSpec::Transactions,
        )),
        WorkbenchId::Playoffs => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Playoffs)),
        WorkbenchId::Poach => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Poach)),
        WorkbenchId::Watchlist => Ok(start_slug::ScreenSpec::nav(start_slug::NavSpec::Watchlist)),
        other => anyhow::bail!(
            "layout center '{}' cannot be restored in the TUI",
            other.slug()
        ),
    }
}
