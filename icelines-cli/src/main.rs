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
    Cli, Commands, FantasySubcommand, IceCastScenarioSubcommand, IceCastSubcommand,
    QuerySubcommand, RecordsSubcommand, ReportSubcommand, WatchSubcommand,
};
use config::Config;
use icelines_core::{WorkbenchId, WorkbenchLayoutStore};
use std::io::IsTerminal;

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
    println!("  icelines stathead --markdown");
    println!("  icelines tui player Bedard");
    println!("  icelines --help    All commands and flags");
}

/// Build Clap's large command tree on an explicitly sized stack.
///
/// IceLines has a deliberately broad command surface. On Windows, deriving and
/// traversing that tree can exceed the default main-thread stack, especially
/// while rendering nested help. Keep this boundary in production as well as in
/// the CLI surface tests instead of relying on the caller's platform default.
fn run_on_cli_stack<T: Send + 'static>(
    operation: impl FnOnce() -> T + Send + 'static,
) -> anyhow::Result<T> {
    let handle = std::thread::Builder::new()
        .name("icelines-cli-parser".to_string())
        .stack_size(16 * 1024 * 1024)
        .spawn(operation)
        .context("spawn IceLines CLI parser")?;

    match handle.join() {
        Ok(value) => Ok(value),
        Err(payload) => std::panic::resume_unwind(payload),
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    reset_sigpipe_handler();

    // Bare `icelines` (no args) prints a short banner pointing at
    // the four primary entry points, then exits 0. Pipe-friendly
    // (works under `icelines | less`); script-friendly (no surprise
    // raw-mode TUI launch); discovery-friendly (one keystroke to
    // each surface). For the full clap-rendered command list, users
    // run `icelines --help`.
    if std::env::args().len() == 1 {
        if commands::setup::config_exists()? {
            Config::load()?;
        }
        print_short_banner();
        return Ok(());
    }

    let cli = run_on_cli_stack(Cli::parse)?;
    let is_setup_command = matches!(&cli.command, Commands::Setup { .. });
    if should_auto_setup(
        cli.no_setup,
        is_setup_command,
        commands::setup::config_exists()?,
        std::io::stdin().is_terminal(),
        std::io::stdout().is_terminal(),
    ) {
        commands::setup::run(false, false, false).await?;
    }

    // Load config after optional first-run setup so any invalid existing
    // config still surfaces before command dispatch.
    let cfg = Config::load()?;
    config::init_live_feeds(cli.no_live, &cfg);
    config::init_dashboards(cli.no_dashboards, &cfg);

    let result = dispatch(cli, cfg).await;
    if let Err(e) = result {
        error::handle_error(e);
    }
    Ok(())
}

fn should_auto_setup(
    no_setup: bool,
    is_setup_command: bool,
    config_exists: bool,
    stdin_terminal: bool,
    stdout_terminal: bool,
) -> bool {
    !no_setup && !is_setup_command && !config_exists && stdin_terminal && stdout_terminal
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
            ReportSubcommand::CapForecast {
                season,
                years,
                growth_pct,
                team,
                json,
                out,
            } => {
                commands::report::run_cap_forecast(commands::report::CapForecastArgs {
                    season,
                    years,
                    growth_pct,
                    team,
                    json,
                    out,
                })?;
            }
            ReportSubcommand::TeamCeiling {
                roster_season,
                stats_season,
                team,
                json,
                out,
            } => {
                commands::report::run_team_ceiling(commands::report::TeamCeilingArgs {
                    roster_season,
                    stats_season,
                    team,
                    json,
                    out,
                })?;
            }
            ReportSubcommand::TeamLineup {
                roster_season,
                stats_season,
                team,
                json,
                out,
            } => {
                commands::report::run_team_lineup(commands::report::TeamLineupArgs {
                    roster_season,
                    stats_season,
                    team,
                    json,
                    out,
                })?;
            }
            ReportSubcommand::TeamCard {
                roster_season,
                stats_season,
                team,
                scenario_id,
                scenario_comparison_key,
                trials,
                seed,
                generated_at,
                json,
                out,
            } => {
                commands::report::run_team_card(commands::report::TeamCardArgs {
                    roster_season,
                    stats_season,
                    team,
                    scenario_id,
                    scenario_comparison_key,
                    trials,
                    seed,
                    generated_at,
                    json,
                    out,
                })
                .await?;
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
        Commands::Stathead {
            pack,
            json,
            markdown,
            commands_only,
            read_only,
            writes_only,
            out,
        } => {
            commands::stathead::run(
                pack,
                json,
                markdown,
                commands_only,
                read_only,
                writes_only,
                out,
            )?;
        }
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
        Commands::DataStatus {
            shard,
            stale_only,
            json,
        } => {
            commands::data_status::run(shard, stale_only, json).await?;
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
        } => {
            commands::tonight::run_schedule(team, days, json, csv, date).await?;
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
                        out,
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
            evidence,
            json,
        } => {
            commands::signals::run_signals_roster(
                team,
                season,
                season_type.to_core(),
                evidence,
                json,
            )
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
                out,
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
                if out.is_some() && (explain || playoff || week || month) {
                    anyhow::bail!("--out is only supported for standard `query leaders` output");
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
                    out,
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
            FantasySubcommand::LeagueSchemeSet { scheme, league } => {
                commands::fantasy::run_league_scheme_set(scheme, league).await?
            }
            FantasySubcommand::CompetitionShow { league, json } => {
                commands::fantasy::run_competition_show(league, json).await?
            }
            FantasySubcommand::CompetitionSet {
                mode,
                categories,
                minimum_goalie_appearances,
                tie_policy,
                league,
            } => {
                commands::fantasy::run_competition_set(
                    mode,
                    categories,
                    minimum_goalie_appearances,
                    tie_policy,
                    league,
                )
                .await?
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
            FantasySubcommand::TeamShow {
                name,
                league,
                stats_season,
            } => commands::fantasy::run_team_show(name, league, stats_season).await?,
            FantasySubcommand::TeamAdd {
                team,
                player,
                league,
                stats_season,
            } => commands::fantasy::run_team_add(team, player, league, stats_season).await?,
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
            FantasySubcommand::SeasonSim {
                league,
                team,
                teams,
                playoff_teams,
                trials,
                seed,
                injury_rate,
                trade_probability,
                opponent_pickup_accuracy,
                pickup_reserve,
                exceptional_reserve_min_value,
                exceptional_reserve_min_games,
                strict_pickup_reserve,
                scenario_matrix,
                manager_matrix,
                reserve_matrix,
                season,
                stats_season,
                json,
            } => {
                commands::fantasy::run_season_sim(commands::fantasy::SeasonSimArgs {
                    league,
                    team,
                    teams,
                    playoff_teams,
                    trials,
                    seed,
                    injury_rate,
                    trade_probability,
                    opponent_pickup_accuracy,
                    pickup_reserve,
                    exceptional_reserve_min_value,
                    exceptional_reserve_min_games,
                    strict_pickup_reserve,
                    scenario_matrix,
                    manager_matrix,
                    reserve_matrix,
                    season,
                    stats_season,
                    json,
                })
                .await?
            }
            FantasySubcommand::ScheduleEdge {
                season,
                week,
                teams,
                league,
                off_night_max_games,
                classes,
                refresh,
                json,
                out,
            } => {
                commands::fantasy::run_schedule_edge(commands::fantasy::ScheduleEdgeArgs {
                    season,
                    week,
                    teams,
                    league,
                    off_night_max_games,
                    classes,
                    refresh,
                    json,
                    out,
                })
                .await?
            }
            FantasySubcommand::BenchCoverage {
                week,
                weeks,
                team,
                league,
                season,
                stats_season,
                off_night_max_games,
                refresh,
                json,
            } => {
                commands::fantasy::run_bench_coverage(
                    week,
                    weeks,
                    team,
                    league,
                    season,
                    stats_season,
                    off_night_max_games,
                    refresh,
                    json,
                )
                .await?
            }
            FantasySubcommand::PlayoffPortfolio {
                rounds,
                start,
                team,
                league,
                season,
                stats_season,
                off_night_max_games,
                candidates,
                top,
                json,
            } => {
                commands::fantasy::run_playoff_portfolio(
                    rounds,
                    start,
                    team,
                    league,
                    season,
                    stats_season,
                    off_night_max_games,
                    candidates,
                    top,
                    json,
                )
                .await?
            }
            FantasySubcommand::PlayoffCalendarSet {
                start,
                rounds,
                league,
                json,
            } => commands::fantasy::run_playoff_calendar_set(start, rounds, league, json).await?,
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
            FantasySubcommand::MatchupPlan {
                week,
                team,
                opponent,
                strategy,
                user_higher_seed,
                category_snapshot,
                through,
                user_current,
                opponent_current,
                current_source,
                status_max_age_minutes,
                league,
                stats_season,
                candidates,
                json,
            } => {
                commands::fantasy::run_matchup_plan(
                    week,
                    team,
                    opponent,
                    strategy,
                    user_higher_seed,
                    category_snapshot,
                    through,
                    user_current,
                    opponent_current,
                    current_source,
                    status_max_age_minutes,
                    league,
                    stats_season,
                    candidates,
                    json,
                )
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
                replace,
                allow_provisional,
                json,
            } => {
                commands::fantasy::run_import_yahoo(
                    file,
                    league,
                    my_team,
                    dry_run,
                    replace,
                    allow_provisional,
                    json,
                )
                .await?
            }
            FantasySubcommand::SyncYahoo {
                file,
                league,
                my_team,
                apply,
                allow_provisional,
                json,
            } => {
                commands::fantasy::run_sync_yahoo(
                    file,
                    league,
                    my_team,
                    apply,
                    allow_provisional,
                    json,
                )
                .await?
            }
            FantasySubcommand::SnapshotYahoo {
                file,
                league,
                apply,
                json,
            } => commands::fantasy::run_snapshot_yahoo(file, league, apply, json).await?,
            FantasySubcommand::SnapshotShow { league, json } => {
                commands::fantasy::run_snapshot_show(league, json).await?
            }
            FantasySubcommand::RosterShape { league, json } => {
                commands::fantasy::run_roster_shape_show(league, json).await?
            }
            FantasySubcommand::RosterShapeSet { shape, league } => {
                commands::fantasy::run_roster_shape_set(shape, league).await?
            }
            FantasySubcommand::RosterShapeValidate { league, team, json } => {
                commands::fantasy::run_roster_shape_validate(league, team, json).await?
            }
            FantasySubcommand::TradeReadiness {
                league,
                team,
                stats_season,
                json,
            } => commands::fantasy::run_trade_readiness(league, team, stats_season, json).await?,
            FantasySubcommand::AssistantSetup { league, json } => {
                commands::fantasy::run_assistant_setup(league, json).await?
            }
            FantasySubcommand::AssistantRules { league, json } => {
                commands::fantasy::run_assistant_rules(league, json).await?
            }
            FantasySubcommand::DraftBoard {
                taken_file,
                eligibility_file,
                pick,
                league,
                league_size,
                stats_season,
                top,
                json,
            } => {
                commands::fantasy::run_draft_board(
                    taken_file,
                    eligibility_file,
                    pick,
                    league,
                    league_size,
                    stats_season,
                    top,
                    json,
                    false,
                    None,
                )
                .await?
            }
            FantasySubcommand::DraftSim {
                taken_file,
                eligibility_file,
                market_file,
                league,
                league_size,
                draft_slot,
                rounds,
                max_goalies,
                market_rank_buffer,
                off_night_max_games,
                replacement_scenarios,
                strategies,
                stats_season,
                json,
            } => {
                commands::fantasy::run_draft_board(
                    taken_file,
                    eligibility_file,
                    None,
                    league,
                    Some(league_size),
                    stats_season,
                    15,
                    json,
                    false,
                    Some(commands::fantasy::DraftSimulationArgs {
                        league_size,
                        draft_slot,
                        rounds,
                        max_goalies,
                        market_rank_buffer,
                        off_night_max_games,
                        replacement_scenarios,
                        strategies,
                        market_file,
                    }),
                )
                .await?
            }
            FantasySubcommand::DraftCard {
                taken_file,
                eligibility_file,
                pick,
                league,
                league_size,
                stats_season,
                top,
                json,
            } => {
                commands::fantasy::run_draft_board(
                    taken_file,
                    eligibility_file,
                    pick,
                    league,
                    league_size,
                    stats_season,
                    top,
                    json,
                    true,
                    None,
                )
                .await?
            }
            FantasySubcommand::WeeklyBudget { league, at, json } => {
                commands::fantasy::run_weekly_budget(league, at, json).await?
            }
            FantasySubcommand::WeeklyPickups {
                date,
                league,
                stats_season,
                candidates,
                top,
                json,
            } => {
                commands::fantasy::run_weekly_pickups(
                    date,
                    league,
                    stats_season,
                    candidates,
                    top,
                    json,
                )
                .await?
            }
            FantasySubcommand::Sleepers {
                league,
                stats_season,
                baseline_season,
                positions,
                top,
                json,
            } => {
                commands::fantasy::run_sleepers(
                    league,
                    stats_season,
                    baseline_season,
                    positions,
                    top,
                    json,
                )
                .await?
            }
            FantasySubcommand::AcquisitionRecord {
                add,
                drop,
                kind,
                at,
                league,
                no_count,
                json,
            } => {
                commands::fantasy::run_acquisition_record(
                    add, drop, kind, at, league, no_count, json,
                )
                .await?
            }
            FantasySubcommand::StatusRecord {
                player,
                status,
                source,
                source_url,
                observed_at,
                confidence,
                detail,
                league,
                json,
            } => {
                commands::fantasy::run_status_record(
                    player,
                    status,
                    source,
                    source_url,
                    observed_at,
                    confidence,
                    detail,
                    league,
                    json,
                )
                .await?
            }
            FantasySubcommand::StatusShow {
                player,
                league,
                max_age_minutes,
                json,
            } => commands::fantasy::run_status_show(player, league, max_age_minutes, json).await?,
            FantasySubcommand::GoalieStartRecord {
                player,
                date,
                state,
                source,
                source_url,
                observed_at,
                detail,
                league,
                json,
            } => {
                commands::fantasy::run_goalie_start_record(
                    player,
                    date,
                    state,
                    source,
                    source_url,
                    observed_at,
                    detail,
                    league,
                    json,
                )
                .await?
            }
            FantasySubcommand::GoalieStartShow {
                player,
                week,
                date,
                league,
                max_age_minutes,
                json,
            } => {
                commands::fantasy::run_goalie_start_show(
                    player,
                    week,
                    date,
                    league,
                    max_age_minutes,
                    json,
                )
                .await?
            }
            FantasySubcommand::GoalieStartImport {
                file,
                source,
                observed_at,
                league,
                json,
            } => {
                commands::fantasy::run_goalie_start_import(file, source, observed_at, league, json)
                    .await?
            }
            FantasySubcommand::GoalieStartTemplate {
                date,
                team,
                league,
                stats_season,
                top_streams,
                out,
            } => {
                commands::fantasy::run_goalie_start_template(
                    date,
                    team,
                    league,
                    stats_season,
                    top_streams,
                    out,
                )
                .await?
            }
            FantasySubcommand::GoaliePlan {
                week,
                date,
                team,
                league,
                stats_season,
                strategy,
                current_appearances,
                max_age_minutes,
                json,
            } => {
                commands::fantasy::run_goalie_plan(
                    week,
                    date,
                    team,
                    league,
                    stats_season,
                    strategy,
                    current_appearances,
                    max_age_minutes,
                    json,
                )
                .await?
            }
            FantasySubcommand::InjuryPlan {
                date,
                league,
                stats_season,
                max_age_minutes,
                json,
            } => {
                commands::fantasy::run_injury_plan(
                    date,
                    league,
                    stats_season,
                    max_age_minutes,
                    json,
                )
                .await?
            }
            FantasySubcommand::RosterCard {
                date,
                league,
                season,
                stats_season,
                max_age_minutes,
                off_night_max_games,
                classes,
                json,
            } => {
                commands::fantasy::run_roster_card(
                    date,
                    league,
                    season,
                    stats_season,
                    max_age_minutes,
                    off_night_max_games,
                    classes,
                    json,
                )
                .await?
            }
            FantasySubcommand::Morning {
                date,
                at,
                league,
                stats_season,
                max_age_minutes,
                current_goalie_appearances,
                material_only,
                json,
            } => {
                commands::fantasy::run_morning(
                    date,
                    at,
                    league,
                    stats_season,
                    max_age_minutes,
                    current_goalie_appearances,
                    material_only,
                    json,
                    commands::fantasy::MorningOutput::Briefing,
                )
                .await?
            }
            FantasySubcommand::Today {
                date,
                at,
                league,
                stats_season,
                max_age_minutes,
                current_goalie_appearances,
                json,
            } => {
                commands::fantasy::run_morning(
                    date,
                    at,
                    league,
                    stats_season,
                    max_age_minutes,
                    current_goalie_appearances,
                    false,
                    json,
                    commands::fantasy::MorningOutput::Today,
                )
                .await?
            }
            FantasySubcommand::MorningCard {
                date,
                at,
                league,
                stats_season,
                max_age_minutes,
                current_goalie_appearances,
                json,
            } => {
                commands::fantasy::run_morning(
                    date,
                    at,
                    league,
                    stats_season,
                    max_age_minutes,
                    current_goalie_appearances,
                    false,
                    json,
                    commands::fantasy::MorningOutput::Card,
                )
                .await?
            }
            FantasySubcommand::Trade {
                player1,
                to_team,
                for_player,
                execute,
                save_offer,
                league,
                stats_season,
                json,
            } => {
                commands::fantasy::run_trade(
                    player1,
                    to_team,
                    for_player,
                    execute,
                    save_offer,
                    league,
                    stats_season,
                    json,
                    false,
                )
                .await?
            }
            FantasySubcommand::TradeCard {
                player1,
                to_team,
                for_player,
                league,
                stats_season,
                json,
            } => {
                commands::fantasy::run_trade(
                    player1,
                    to_team,
                    for_player,
                    false,
                    false,
                    league,
                    stats_season,
                    json,
                    true,
                )
                .await?
            }
            FantasySubcommand::TradeHistory {
                league,
                limit,
                json,
            } => commands::fantasy::run_trade_history(league, limit, json)?,
            FantasySubcommand::TradeOffers {
                status,
                actionable_only,
                league,
                limit,
                json,
            } => commands::fantasy::run_trade_offers(status, actionable_only, league, limit, json)?,
            FantasySubcommand::TradeOfferClose {
                id,
                status,
                league,
                json,
            } => commands::fantasy::run_trade_offer_close(id, status, league, json)?,
            FantasySubcommand::TradeFinder {
                team,
                to_team,
                max_package,
                fairness_percent,
                protect,
                include_anchors,
                require_complete,
                top,
                league,
                stats_season,
                json,
            } => {
                commands::fantasy::run_trade_finder(
                    team,
                    to_team,
                    max_package,
                    fairness_percent,
                    protect,
                    include_anchors,
                    require_complete,
                    top,
                    league,
                    stats_season,
                    json,
                )
                .await?
            }
            FantasySubcommand::Serve { port, league } => {
                commands::fantasy::run_serve(port, league).await?
            }
        },
        Commands::Icecast(IceCastSubcommand::DraftPickCurve {
            start_year,
            cutoff_year,
            completed_season_start_year,
            horizon,
            max_pick,
            annual_future_discount,
            generated_at,
            json,
            out,
        }) => {
            commands::icecast::run_draft_pick_curve(
                start_year,
                cutoff_year,
                completed_season_start_year,
                horizon,
                max_pick,
                annual_future_discount,
                generated_at,
                json,
                out,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::TradeMarketAssemble {
            input,
            curve,
            baseline_forecast,
            scenario_forecast,
            buyer_lineup,
            seller_lineup,
            json,
            out,
        }) => commands::icecast::run_trade_market_assemble(
            commands::icecast::IceCastTradeMarketAssembleArgs {
                input,
                curve,
                baseline_forecast,
                scenario_forecast,
                buyer_lineup,
                seller_lineup,
                json,
                out,
            },
        )?,
        Commands::Icecast(IceCastSubcommand::TradeMarket { input, json, out }) => {
            commands::icecast::run_trade_market(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::TradeScout { input, json, out }) => {
            commands::icecast::run_trade_scout(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::TradeScoutLeague { input, json, out }) => {
            commands::icecast::run_trade_scout_league(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::TradeScoutPopulate {
            camp,
            input,
            pick_assets,
            board_out,
            json,
            out,
        }) => commands::icecast::run_trade_scout_populate(
            camp,
            input,
            pick_assets,
            board_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::TradeCalibrate { input, json, out }) => {
            commands::icecast::run_trade_calibrate(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::TradeFeatures { input, json, out }) => {
            commands::icecast::run_trade_features(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::TradePickPopulate {
            ownership,
            curve,
            policy,
            season_forecast,
            json,
            out,
        }) => commands::icecast::run_trade_pick_populate(
            ownership,
            curve,
            policy,
            season_forecast,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::TradePickCoverage {
            ownership,
            draft_year,
            as_of,
            json,
            out,
        }) => commands::icecast::run_trade_pick_coverage(ownership, draft_year, as_of, json, out)?,
        Commands::Icecast(IceCastSubcommand::TradeLineup {
            lineup,
            change,
            json,
            out,
        }) => commands::icecast::run_trade_lineup(lineup, change, json, out)?,
        Commands::Icecast(IceCastSubcommand::TradeLineupBoard {
            lineup,
            input,
            json,
            out,
        }) => commands::icecast::run_trade_lineup_board(lineup, input, json, out)?,
        Commands::Icecast(IceCastSubcommand::Season {
            season,
            stats_season,
            candidate_overlay,
            teams,
            trials,
            seed,
            scenario,
            scenario_id,
            isolated_impacts,
            auto_personnel,
            trade_mode,
            replay_mode,
            ignore_replay_personnel_after,
            through,
            retrospective_opening_lineups,
            all_games,
            refresh,
            json,
            out,
            game_forecast_out,
        }) => {
            commands::icecast::run_season(commands::icecast::IceCastSeasonArgs {
                season,
                stats_season,
                candidate_overlay,
                teams,
                trials,
                seed,
                scenario,
                scenario_id,
                isolated_impacts,
                auto_personnel,
                trade_mode,
                replay_mode,
                ignore_replay_personnel_after,
                through,
                retrospective_opening_lineups,
                all_games,
                refresh,
                json,
                out,
                game_forecast_out,
            })
            .await?
        }
        Commands::Icecast(IceCastSubcommand::Edge {
            forecast,
            evidence,
            model,
            json,
            out,
            enhanced_forecast_out,
        }) => commands::icecast::run_edge(commands::icecast::IceCastEdgeArgs {
            forecast,
            evidence,
            model,
            json,
            out,
            enhanced_forecast_out,
        })?,
        Commands::Icecast(IceCastSubcommand::EdgeCard {
            inputs,
            game_id,
            team,
            team_name,
            generated_at,
            market_benchmark,
            out,
        }) => commands::icecast::run_edge_card(
            inputs,
            game_id,
            team,
            team_name,
            generated_at,
            market_benchmark,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::SeasonSimulate {
            forecast,
            trials,
            seed,
            scenario,
            through,
            out,
        }) => {
            commands::icecast::run_season_simulate(forecast, trials, seed, scenario, through, out)?
        }
        Commands::Icecast(IceCastSubcommand::EdgePreseasonEvidence {
            forecast,
            created_at,
            out,
        }) => commands::icecast::run_edge_preseason_evidence(forecast, created_at, out)?,
        Commands::Icecast(IceCastSubcommand::EdgeEvidence {
            forecast,
            input,
            line_matchups,
            out,
        }) => commands::icecast::run_edge_evidence(forecast, input, line_matchups, out)?,
        Commands::Icecast(IceCastSubcommand::EdgeObserve {
            edges,
            outcomes,
            created_at,
            out,
        }) => commands::icecast::run_edge_observe(edges, outcomes, created_at, out)?,
        Commands::Icecast(IceCastSubcommand::EdgeOutcomes {
            season,
            captured_at,
            refresh,
            allow_partial,
            out,
        }) => {
            commands::icecast::run_edge_outcomes(season, captured_at, refresh, allow_partial, out)
                .await?
        }
        Commands::Icecast(IceCastSubcommand::EdgeTrain {
            observations,
            config,
            registration,
            validate,
            model_out,
            out,
        }) => commands::icecast::run_edge_train(commands::icecast::IceCastEdgeTrainArgs {
            observations,
            config,
            registration,
            validate,
            model_out,
            out,
        })?,
        Commands::Icecast(IceCastSubcommand::EdgeRegisterHoldout {
            season,
            registered_at,
            outcome_not_before,
            config,
            out,
        }) => commands::icecast::run_edge_register_holdout(
            season,
            registered_at,
            outcome_not_before,
            config,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::BehaviorRankings {
            target_season,
            window,
            json,
            out,
        }) => commands::icecast::run_behavior_rankings(target_season, window, json, out).await?,
        Commands::Icecast(IceCastSubcommand::BehaviorResearch {
            rankings,
            research,
            json,
            out,
        }) => commands::icecast::run_behavior_research(rankings, research, json, out)?,
        Commands::Icecast(IceCastSubcommand::Camp {
            input,
            trials,
            seed,
            json,
            out,
            lineup_set_out,
            max_lineup_branches,
            blender_set_out,
            season_scenario_out,
            season_max_roster_branches,
            camp_max_candidates,
        }) => commands::icecast::run_camp(
            input,
            trials,
            seed,
            json,
            out,
            lineup_set_out,
            max_lineup_branches,
            blender_set_out,
            season_scenario_out,
            season_max_roster_branches,
            camp_max_candidates,
        )?,
        Commands::Icecast(IceCastSubcommand::CampLeague {
            rosters,
            bios,
            stats,
            goalie_stats,
            candidate_overlay,
            authored_input,
            season,
            trials,
            seed,
            json,
            out,
        }) => commands::icecast::run_camp_league(
            rosters,
            bios,
            stats,
            goalie_stats,
            candidate_overlay,
            authored_input,
            season,
            trials,
            seed,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::Bubble {
            input,
            transaction_context,
            top,
            json,
            out,
        }) => commands::icecast::run_bubble(input, transaction_context, top, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateMap { json, out }) => {
            commands::icecast::run_affiliate_map(json, out)?
        }
        Commands::Icecast(IceCastSubcommand::Affiliate { input, json, out }) => {
            commands::icecast::run_affiliate(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateIdentities {
            snapshot,
            team,
            candidates,
            discover_official,
            refresh,
            json,
            out,
        }) => {
            commands::icecast::run_affiliate_identities(
                snapshot,
                team,
                candidates,
                discover_official,
                refresh,
                json,
                out,
                &cfg,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::EdgeReplayXg {
            forecast,
            moneypuck_dir,
            retrieved_at,
            trailing_games,
            out,
        }) => commands::icecast::run_edge_replay_xg(
            forecast,
            moneypuck_dir,
            retrieved_at,
            trailing_games,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::EdgeReplayConfirmed {
            forecast,
            morning_evidence,
            boxscore_dir,
            retrieved_at,
            refresh,
            concurrency,
            out,
        }) => {
            commands::icecast::run_edge_replay_confirmed(
                forecast,
                morning_evidence,
                boxscore_dir,
                retrieved_at,
                refresh,
                concurrency,
                out,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::EdgeReplayGoalies {
            forecast,
            confirmed_evidence,
            goalie_dir,
            retrieved_at,
            trailing_appearances,
            refresh,
            concurrency,
            out,
        }) => {
            commands::icecast::run_edge_replay_goalies(
                forecast,
                confirmed_evidence,
                goalie_dir,
                retrieved_at,
                trailing_appearances,
                refresh,
                concurrency,
                out,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateIdentitiesLeague {
            snapshot,
            candidates,
            discover_official,
            refresh,
            as_of,
            max_age,
            json,
            out,
        }) => {
            commands::icecast::run_affiliate_identities_league(
                snapshot,
                candidates,
                discover_official,
                refresh,
                as_of,
                max_age,
                json,
                out,
                &cfg,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateReviewDraft {
            crosswalk,
            include_aliases,
            include_conflicts,
            out,
        }) => commands::icecast::run_affiliate_review_draft(
            crosswalk,
            include_aliases,
            include_conflicts,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewDraftLeague {
            league_crosswalk,
            include_aliases,
            include_conflicts,
            out,
        }) => commands::icecast::run_affiliate_review_draft_league(
            league_crosswalk,
            include_aliases,
            include_conflicts,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewExact {
            crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_exact(
            crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewExactLeague {
            league_crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_exact_league(
            league_crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewAliases {
            crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_aliases(
            crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewAliasesLeague {
            league_crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_aliases_league(
            league_crosswalk,
            reviewer,
            reviewed_at,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewConflictsLeague {
            league_crosswalk,
            nhl_player_id,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_conflicts_league(
            league_crosswalk,
            nhl_player_id,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewBirthDateLeague {
            league_crosswalk,
            nhl_player_id,
            canonical_birth_date,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_birth_date_league(
            league_crosswalk,
            nhl_player_id,
            canonical_birth_date,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewCollisionLeague {
            league_crosswalk,
            proposed_nhl_player_id,
            canonical_nhl_player_id,
            canonical_name,
            canonical_birth_date,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_collision_league(
            league_crosswalk,
            proposed_nhl_player_id,
            canonical_nhl_player_id,
            canonical_name,
            canonical_birth_date,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewReject {
            crosswalk,
            provider_player_id,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_reject(
            crosswalk,
            provider_player_id,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewRejectLeague {
            league_crosswalk,
            provider_player_id,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_reject_league(
            league_crosswalk,
            provider_player_id,
            evidence_urls,
            reviewer,
            reviewed_at,
            note,
            decisions_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewLeague {
            crosswalks,
            league_crosswalks,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_league(
            crosswalks,
            league_crosswalks,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewBoard { review, json, out }) => {
            commands::icecast::run_affiliate_review_board(review, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateReviewShow {
            crosswalk,
            attention_only,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_show(crosswalk, attention_only, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateReviewApply {
            crosswalk,
            decisions,
            json,
            out,
        }) => commands::icecast::run_affiliate_review_apply(crosswalk, decisions, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateStatusDraft {
            prior_snapshot,
            crosswalk,
            camp,
            nhl_team,
            ahl_team,
            out,
        }) => commands::icecast::run_affiliate_status_draft(
            prior_snapshot,
            crosswalk,
            camp,
            nhl_team,
            ahl_team,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateStatusDraftLeague {
            prior_snapshot,
            league_crosswalk,
            camp_forecast,
            config,
            json,
            out,
        }) => commands::icecast::run_affiliate_status_draft_league(
            prior_snapshot,
            league_crosswalk,
            camp_forecast,
            config,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateStatusShow { review, json, out }) => {
            commands::icecast::run_affiliate_status_show(review, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateStatusEvidence {
            review,
            career_history,
            as_of,
            maximum_fact_age_days,
            json,
            out,
        }) => commands::icecast::run_affiliate_status_evidence(
            review,
            career_history,
            as_of,
            maximum_fact_age_days,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateStatusEvidenceApply {
            review,
            ledger,
            json,
            out,
        }) => commands::icecast::run_affiliate_status_evidence_apply(review, ledger, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateTransactionState {
            transactions,
            league_crosswalk,
            affiliations,
            cutoff,
            json,
            out,
        }) => commands::icecast::run_affiliate_transaction_state(
            transactions,
            league_crosswalk,
            affiliations,
            cutoff,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateTransactionStateApply {
            workboard,
            ledger,
            json,
            out,
        }) => {
            commands::icecast::run_affiliate_transaction_state_apply(workboard, ledger, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateWaiversDraft {
            workboard,
            cutoff,
            json,
            out,
        }) => commands::icecast::run_affiliate_waivers_draft(workboard, cutoff, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateWaiversFinalize {
            draft,
            decisions,
            json,
            out,
        }) => commands::icecast::run_affiliate_waivers_finalize(draft, decisions, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateWaiversApply {
            workboard,
            review,
            json,
            out,
        }) => commands::icecast::run_affiliate_waivers_apply(workboard, review, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateStatusApply {
            prior_snapshot,
            crosswalk,
            camp,
            review,
            config,
            out,
        }) => commands::icecast::run_affiliate_status_apply(
            prior_snapshot,
            crosswalk,
            camp,
            review,
            config,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateStatusApplyLeague {
            prior_snapshot,
            league_crosswalk,
            camp_forecast,
            review,
            config,
            out,
        }) => commands::icecast::run_affiliate_status_apply_league(
            prior_snapshot,
            league_crosswalk,
            camp_forecast,
            review,
            config,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateProfessionalGames {
            league_crosswalk,
            career_history,
            policy,
            camp_forecast,
            json,
            out,
        }) => commands::icecast::run_affiliate_professional_games(
            league_crosswalk,
            career_history,
            policy,
            camp_forecast,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateValues {
            snapshot,
            league_crosswalk,
            policy,
            json,
            out,
        }) => {
            commands::icecast::run_affiliate_values(snapshot, league_crosswalk, policy, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateValuesApply {
            workboard,
            ledger,
            json,
            out,
        }) => commands::icecast::run_affiliate_values_apply(workboard, ledger, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateValuesCrossLeague {
            workboard,
            career_history,
            policy,
            json,
            out,
        }) => commands::icecast::run_affiliate_values_cross_league(
            workboard,
            career_history,
            policy,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateValuesCrossLeagueApply {
            workboard,
            ledger,
            json,
            out,
        }) => commands::icecast::run_affiliate_values_cross_league_apply(
            workboard, ledger, json, out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateProspects {
            workboard,
            career_history,
            policy,
            json,
            out,
        }) => commands::icecast::run_affiliate_prospects(
            workboard,
            career_history,
            policy,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateProspectsApply {
            workboard,
            ledger,
            json,
            out,
        }) => commands::icecast::run_affiliate_prospects_apply(workboard, ledger, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateReadiness {
            workboard,
            career_history,
            camp_forecast,
            policy,
            json,
            out,
        }) => commands::icecast::run_affiliate_readiness(
            workboard,
            career_history,
            camp_forecast,
            policy,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateReadinessApply {
            workboard,
            ledger,
            json,
            out,
        }) => commands::icecast::run_affiliate_readiness_apply(workboard, ledger, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateFactsBoard {
            rollover,
            professional_games,
            json,
            out,
        }) => {
            commands::icecast::run_affiliate_facts_board(rollover, professional_games, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateFactsStatus {
            input,
            require_ready,
            json,
            out,
        }) => commands::icecast::run_affiliate_facts_status(input, require_ready, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateFactsDraft { workboard, out }) => {
            commands::icecast::run_affiliate_facts_draft(workboard, out)?
        }
        Commands::Icecast(IceCastSubcommand::AffiliateFactsApply {
            workboard,
            overlay,
            json,
            out,
        }) => commands::icecast::run_affiliate_facts_apply(workboard, overlay, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateInputsLeague {
            application,
            rule,
            json,
            out,
        }) => commands::icecast::run_affiliate_inputs_league(application, rule, json, out)?,
        Commands::Icecast(IceCastSubcommand::AffiliateProfessionalGamesApply {
            crosswalk,
            ledger,
            facts,
            nhl_team,
            ahl_team,
            out,
        }) => commands::icecast::run_affiliate_professional_games_apply(
            crosswalk, ledger, facts, nhl_team, ahl_team, out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateInput {
            snapshot,
            crosswalk,
            facts,
            nhl_team,
            ahl_team,
            out,
        }) => commands::icecast::run_affiliate_input(
            snapshot, crosswalk, facts, nhl_team, ahl_team, out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateRollover {
            prior_snapshot,
            crosswalk,
            camp,
            camp_forecast,
            config,
            json,
            out,
        }) => commands::icecast::run_affiliate_rollover(
            prior_snapshot,
            crosswalk,
            camp,
            camp_forecast,
            config,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateRolloverConfigLeague {
            league_crosswalk,
            camp_forecast,
            prior_affiliations,
            affiliations,
            as_of,
            source_urls,
            out,
        }) => commands::icecast::run_affiliate_rollover_config_league(
            league_crosswalk,
            camp_forecast,
            prior_affiliations,
            affiliations,
            as_of,
            source_urls,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::AffiliateRolloverLeague {
            prior_snapshot,
            league_crosswalk,
            camp_forecast,
            config,
            json,
            out,
        }) => commands::icecast::run_affiliate_rollover_league(
            prior_snapshot,
            league_crosswalk,
            camp_forecast,
            config,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::Organization { input, json, out }) => {
            commands::icecast::run_organization(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::Blender {
            lineup,
            pair_evidence,
            shift_season,
            refresh_shifts,
            shift_report_out,
            max_candidates,
            allow_off_wing,
            review_games,
            minimum_points_percentage,
            max_changes,
            max_choices,
            scenario_out,
            json,
            out,
        }) => {
            commands::icecast::run_blender(commands::icecast::IceCastBlenderArgs {
                lineup,
                pair_evidence,
                shift_season,
                refresh_shifts,
                shift_report_out,
                max_candidates,
                allow_off_wing,
                review_games,
                minimum_points_percentage,
                max_changes,
                max_choices,
                scenario_out,
                json,
                out,
            })
            .await?
        }
        Commands::Icecast(IceCastSubcommand::Bench {
            forecast,
            lineup,
            profile,
            style_evidence,
            stats_season,
            scenario_out,
            json,
            out,
        }) => commands::icecast::run_bench(commands::icecast::IceCastBenchArgs {
            forecast,
            lineup,
            profile,
            style_evidence,
            stats_season,
            scenario_out,
            json,
            out,
        })?,
        Commands::Icecast(IceCastSubcommand::LineMatchup {
            input,
            away_bench_plan,
            home_bench_plan,
            json,
            out,
        }) => commands::icecast::run_line_matchup(commands::icecast::IceCastLineMatchupArgs {
            input,
            away_bench_plan,
            home_bench_plan,
            json,
            out,
        })?,
        Commands::Icecast(IceCastSubcommand::LineMatchupCard {
            input,
            edge,
            team,
            team_name,
            generated_at,
            out,
        }) => commands::icecast::run_line_matchup_card(
            input,
            edge,
            team,
            team_name,
            generated_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::LineMatchupShow { input }) => {
            commands::icecast::run_line_matchup_show(input)?
        }
        Commands::Icecast(IceCastSubcommand::LineMatchupProfiles {
            lineup,
            role_evidence,
            shift_report,
            evidence_cutoff_at,
            recency,
            source_fingerprints,
            json,
            out,
        }) => commands::icecast::run_line_matchup_profiles(
            commands::icecast::IceCastLineMatchupProfilesArgs {
                lineup,
                role_evidence,
                shift_report,
                evidence_cutoff_at,
                recency,
                source_fingerprints,
                json,
                out,
            },
        )?,
        Commands::Icecast(IceCastSubcommand::LineChemistry {
            team,
            forecast_at,
            input,
            out,
        }) => commands::icecast::run_line_chemistry(team, forecast_at, input, out)?,
        Commands::Icecast(IceCastSubcommand::LineChemistryMoneyPuck {
            team,
            forecast_at,
            line_games,
            baselines,
            minimum_shared_minutes,
            out,
        }) => commands::icecast::run_line_chemistry_moneypuck(
            team,
            forecast_at,
            line_games,
            baselines,
            minimum_shared_minutes,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::LineChemistryMoneyPuckAuto {
            team,
            season_start,
            forecast_at,
            trailing_games,
            minimum_player_games,
            minimum_shared_minutes,
            summary,
            line_game_dir,
            skater_game_dir,
            team_game_dir,
            rights_basis,
            out,
        }) => commands::icecast::run_line_chemistry_moneypuck_auto(
            team,
            season_start,
            forecast_at,
            trailing_games,
            minimum_player_games,
            minimum_shared_minutes,
            summary,
            line_game_dir,
            skater_game_dir,
            team_game_dir,
            rights_basis,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::LineMatchupObserve {
            forecasts,
            predictions,
            outcomes,
            created_at,
            out,
        }) => commands::icecast::run_line_matchup_observe(
            forecasts,
            predictions,
            outcomes,
            created_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::LineMatchupPublish {
            card,
            published_at,
            data_root,
            out,
        }) => commands::icecast::run_line_matchup_publish(card, published_at, data_root, out)?,
        Commands::Icecast(IceCastSubcommand::LineMatchupValidate {
            input,
            created_at,
            out,
        }) => commands::icecast::run_line_matchup_validate(input, created_at, out)?,
        Commands::Icecast(IceCastSubcommand::LineMatchupCompare {
            input,
            focus_team,
            baseline,
            out,
        }) => commands::icecast::run_line_matchup_compare(input, focus_team, baseline, out)?,
        Commands::Icecast(IceCastSubcommand::SeasonCard {
            input,
            team,
            team_name,
            generated_at,
            calendar_fingerprint,
            out,
        }) => commands::icecast::run_season_card(
            input,
            team,
            team_name,
            generated_at,
            calendar_fingerprint,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::Movement {
            earlier,
            later,
            earlier_label,
            later_label,
            teams,
            json,
            out,
        }) => commands::icecast::run_movement(
            earlier,
            later,
            earlier_label,
            later_label,
            teams,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::MovementCard {
            input,
            team,
            team_name,
            generated_at,
            out,
        }) => commands::icecast::run_movement_card(input, team, team_name, generated_at, out)?,
        Commands::Icecast(IceCastSubcommand::History {
            inputs,
            teams,
            json,
            out,
        }) => commands::icecast::run_history(inputs, teams, json, out)?,
        Commands::Icecast(IceCastSubcommand::HistoryCard {
            input,
            team,
            team_name,
            generated_at,
            out,
        }) => commands::icecast::run_history_card(input, team, team_name, generated_at, out)?,
        Commands::Icecast(IceCastSubcommand::Window {
            input,
            team,
            json,
            markdown,
            out,
        }) => commands::icecast::run_window(input, team, json, markdown, out)?,
        Commands::Icecast(IceCastSubcommand::WindowProfileHistoryBuild {
            boards,
            history_id,
            created_at,
            out,
        }) => commands::icecast::run_window_profile_history_build(
            boards, history_id, created_at, out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowProfileHistoryBackfill {
            origins,
            history_id,
            created_at,
            out,
        }) => commands::icecast::run_window_profile_history_backfill(
            origins, history_id, created_at, out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowProfileHistoryBaseline {
            source_package,
            ahl_workboard,
            history_id,
            created_at,
            out,
        }) => commands::icecast::run_window_profile_history_baseline(
            source_package,
            ahl_workboard,
            history_id,
            created_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowProfileHistoryAudit {
            input,
            generated_at,
            out,
        }) => commands::icecast::run_window_profile_history_audit(input, generated_at, out)?,
        Commands::Icecast(IceCastSubcommand::WindowProfileHistoryDelta {
            input,
            earlier_season,
            earlier_as_of,
            later_season,
            later_as_of,
            horizon,
            generated_at,
            out,
        }) => commands::icecast::run_window_profile_history_delta(
            input,
            earlier_season,
            earlier_as_of,
            later_season,
            later_as_of,
            horizon,
            generated_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowProfileHistoryCard {
            input,
            team,
            team_name,
            generated_at,
            out,
        }) => commands::icecast::run_window_profile_history_card(
            input,
            team,
            team_name,
            generated_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowBuild {
            season,
            as_of,
            generated_at,
            source_package,
            team_season_forecast,
            team_game_forecast,
            team_lineups,
            ahl_affiliates,
            organization_lineups,
            prospect_program,
            prospect_conversion,
            training_camp,
            schedule_rest,
            profile_history,
            require_ranked,
            out,
        }) => commands::icecast::run_window_build(commands::icecast::WindowBuildArgs {
            season,
            as_of,
            generated_at,
            source_package,
            team_season_forecast,
            team_game_forecast,
            team_lineups,
            ahl_affiliates,
            organization_lineups,
            prospect_program,
            prospect_conversion,
            training_camp,
            schedule_rest,
            profile_history,
            require_ranked,
            out,
        })?,
        Commands::Icecast(IceCastSubcommand::WindowSourcePackage {
            season,
            as_of,
            team_season_forecast,
            team_game_forecast,
            cache_team_lineups,
            stats_season,
            team_lineups,
            ahl_affiliates,
            ahl_projection_inputs,
            organization_lineups,
            prospect_program,
            cache_prospect_program,
            career_history,
            prospect_conversion,
            training_camp,
            schedule_rest,
            profile_history,
            out,
        }) => commands::icecast::run_window_source_package(
            commands::icecast::WindowSourcePackageArgs {
                season,
                as_of,
                team_season_forecast,
                team_game_forecast,
                cache_team_lineups,
                stats_season,
                team_lineups,
                ahl_affiliates,
                ahl_projection_inputs,
                organization_lineups,
                prospect_program,
                cache_prospect_program,
                career_history,
                prospect_conversion,
                training_camp,
                schedule_rest,
                profile_history,
                out,
            },
        )?,
        Commands::Icecast(IceCastSubcommand::WindowSourceRefreshLineups {
            input,
            stats_season,
            training_camp,
            career_history,
            out,
        }) => commands::icecast::run_window_source_refresh_lineups(
            commands::icecast::WindowSourceRefreshLineupsArgs {
                input,
                stats_season,
                training_camp,
                career_history,
                out,
            },
        )?,
        Commands::Icecast(IceCastSubcommand::WindowSourceRefreshAffiliates {
            input,
            ahl_projection_inputs,
            ahl_facts_application,
            ahl_development_rule,
            out,
        }) => commands::icecast::run_window_source_refresh_affiliates(
            commands::icecast::WindowSourceRefreshAffiliatesArgs {
                input,
                ahl_projection_inputs,
                ahl_facts_application,
                ahl_development_rule,
                out,
            },
        )?,
        Commands::Icecast(IceCastSubcommand::WindowSourceRefreshHistory {
            input,
            profile_history,
            out,
        }) => commands::icecast::run_window_source_refresh_history(input, profile_history, out)?,
        Commands::Icecast(IceCastSubcommand::WindowSourceAudit {
            input,
            generated_at,
            out,
        }) => commands::icecast::run_window_source_audit(input, generated_at, out)?,
        Commands::Icecast(IceCastSubcommand::WindowCard {
            input,
            team,
            team_name,
            generated_at,
            out,
        }) => commands::icecast::run_window_card(input, team, team_name, generated_at, out)?,
        Commands::Icecast(IceCastSubcommand::WindowMovement {
            earlier,
            later,
            bridge,
            out,
        }) => commands::icecast::run_window_movement(earlier, later, bridge, out)?,
        Commands::Icecast(IceCastSubcommand::WindowPersonnelAttribution {
            earlier,
            later,
            movement,
            input,
            out,
        }) => commands::icecast::run_window_personnel_attribution(
            earlier, later, movement, input, out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowPersonnelInputBuild {
            actual_forecast,
            counterfactual_board,
            earlier_as_of,
            later_as_of,
            attribution_id,
            scenario_id,
            rationale,
            out,
        }) => commands::icecast::run_window_personnel_input_build(
            actual_forecast,
            counterfactual_board,
            earlier_as_of,
            later_as_of,
            attribution_id,
            scenario_id,
            rationale,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowPersonnelSummary { input, out }) => {
            commands::icecast::run_window_personnel_summary(input, out)?
        }
        Commands::Icecast(IceCastSubcommand::WindowRebase {
            input,
            target_manifest,
            bridge,
            out,
        }) => commands::icecast::run_window_rebase(input, target_manifest, bridge, out)?,
        Commands::Icecast(IceCastSubcommand::WindowHistory { inputs, out }) => {
            commands::icecast::run_window_history(inputs, out)?
        }
        Commands::Icecast(IceCastSubcommand::WindowScenario {
            baseline,
            scenario,
            scenario_id,
            authorities,
            team_season_authorities,
            training_camp_authorities,
            out,
        }) => commands::icecast::run_window_scenario(
            baseline,
            scenario,
            scenario_id,
            authorities,
            team_season_authorities,
            training_camp_authorities,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowScenarioDistribute {
            baseline,
            input,
            out,
        }) => commands::icecast::run_window_scenario_distribution(baseline, input, out)?,
        Commands::Icecast(IceCastSubcommand::WindowCalibrate {
            target,
            origins,
            minimum_origins,
            out,
        }) => commands::icecast::run_window_calibrate(target, origins, minimum_origins, out)?,
        Commands::Icecast(IceCastSubcommand::WindowEvaluate {
            target,
            origins,
            minimum_training_origins,
            out,
        }) => {
            commands::icecast::run_window_evaluate(target, origins, minimum_training_origins, out)?
        }
        Commands::Icecast(IceCastSubcommand::Backtest { inputs, json, out }) => {
            commands::icecast::run_backtest(inputs, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::CalibrateDevelopment {
            start_season,
            end_season,
            breakout_threshold,
            downturn_threshold,
            prior_sample_size,
            json,
            out,
        }) => commands::icecast::run_development_calibration(
            start_season,
            end_season,
            breakout_threshold,
            downturn_threshold,
            prior_sample_size,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::CalibrateScenarioDevelopment {
            input,
            calibration,
            json,
            out,
        }) => {
            commands::icecast::run_scenario_development_calibration(input, calibration, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::ProspectArrivalCalibrate {
            input,
            career_discovery,
            player_id,
            event_id,
            forecast_season,
            conversion_board,
            conversion_archive,
            json,
            out,
            input_out,
        }) => commands::icecast::run_prospect_arrival_calibration(
            input,
            career_discovery,
            player_id,
            event_id,
            forecast_season,
            conversion_board,
            conversion_archive,
            json,
            out,
            input_out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectArrivalLeague {
            camp_forecast,
            career_discoveries,
            career_history,
            source_package,
            require_complete_population,
            conversion_board,
            conversion_archive,
            forecast_season,
            as_of,
            max_age,
            json,
            out,
            discovery_out,
        }) => commands::icecast::run_prospect_arrival_league(
            camp_forecast,
            career_discoveries,
            career_history,
            source_package,
            require_complete_population,
            conversion_board,
            conversion_archive,
            forecast_season,
            as_of,
            max_age,
            json,
            out,
            discovery_out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectArrivalCard {
            input,
            team,
            team_name,
            evidence_at,
            out,
        }) => {
            commands::icecast::run_prospect_arrival_card(input, team, team_name, evidence_at, out)?
        }
        Commands::Icecast(IceCastSubcommand::ProspectArrivalBoard {
            input,
            generated_at,
            json,
            out,
        }) => commands::icecast::run_prospect_arrival_board(input, generated_at, json, out)?,
        Commands::Icecast(IceCastSubcommand::WindowStandings {
            target_season,
            date,
            captured_at,
            out,
        }) => {
            commands::icecast::run_window_standings(target_season, date, captured_at, out).await?
        }
        Commands::Icecast(IceCastSubcommand::WindowOriginBuild {
            source_season,
            target_season,
            as_of,
            generated_at,
            role,
            standings,
            out,
        }) => commands::icecast::run_window_origin_build(
            source_season,
            target_season,
            as_of,
            generated_at,
            role,
            standings,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowHoldoutRegister {
            source_season,
            target_season,
            feature_cutoff,
            outcome_not_before,
            registered_at,
            out,
        }) => commands::icecast::run_window_holdout_register(
            source_season,
            target_season,
            feature_cutoff,
            outcome_not_before,
            registered_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::WindowHoldoutScore {
            registration,
            standings,
            scored_at,
            out,
        }) => commands::icecast::run_window_holdout_score(registration, standings, scored_at, out)?,
        Commands::Icecast(IceCastSubcommand::WindowCompletionStatus {
            source_audit,
            holdout_registration,
            holdout_result,
            evaluated_at,
            require_complete,
            out,
        }) => commands::icecast::run_window_completion_status(
            source_audit,
            holdout_registration,
            holdout_result,
            evaluated_at,
            require_complete,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectStudy { input, json, out }) => {
            commands::icecast::run_prospect_study(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::ProspectContext {
            snapshots,
            league_crosswalks,
            affiliations,
            as_of,
            max_age,
            minimum_ahl_seasons,
            json,
            out,
        }) => commands::icecast::run_prospect_context(
            snapshots,
            league_crosswalks,
            affiliations,
            as_of,
            max_age,
            minimum_ahl_seasons,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectLeague {
            snapshots,
            crosswalks,
            context,
            json,
            out,
        }) => commands::icecast::run_prospect_league(snapshots, crosswalks, context, json, out)?,
        Commands::Icecast(IceCastSubcommand::ProspectPopulationAudit {
            input,
            require_fully_classified,
            json,
            out,
        }) => commands::icecast::run_prospect_population_audit(
            input,
            require_fully_classified,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::IdentityReviewWorkboard {
            source_package,
            json,
            out,
        }) => commands::icecast::run_identity_review_workboard(source_package, json, out)?,
        Commands::Icecast(IceCastSubcommand::OfficialIdentityCandidates {
            workboard,
            refresh,
            offline,
            search_concurrency,
            evidence_cutoff,
            json,
            out,
        }) => {
            commands::icecast::run_official_identity_candidates(
                workboard,
                refresh,
                offline,
                search_concurrency,
                evidence_cutoff,
                json,
                out,
                &cfg,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::OfficialIdentityReviewLedger {
            candidates,
            provider,
            registry_url,
            reviewer,
            reviewed_at,
            out,
        }) => commands::icecast::run_official_identity_review_ledger(
            candidates,
            provider,
            registry_url,
            reviewer,
            reviewed_at,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectCensus {
            source_package,
            pipeline,
            league_discoveries,
            career_discoveries,
            program_board,
            pipeline_out,
            prospects_per_team,
            require_publishable,
            json,
            out,
        }) => commands::icecast::run_prospect_census(
            source_package,
            pipeline,
            league_discoveries,
            career_discoveries,
            program_board,
            pipeline_out,
            usize::from(prospects_per_team),
            require_publishable,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectCensusReadiness { input, json, out }) => {
            commands::icecast::run_prospect_census_readiness(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::ProspectAuthorityClosure { input, json, out }) => {
            commands::icecast::run_prospect_authority_closure(input, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::ProspectAuthorityProgress {
            prior,
            current,
            json,
            out,
        }) => commands::icecast::run_prospect_authority_progress(prior, current, json, out)?,
        Commands::Icecast(IceCastSubcommand::ProspectCareerContext {
            camp_forecast,
            rosters,
            bios,
            candidate_overlay,
            career_history,
            as_of,
            max_age,
            json,
            out,
        }) => commands::icecast::run_prospect_career_context(
            camp_forecast,
            rosters,
            bios,
            candidate_overlay,
            career_history,
            as_of,
            max_age,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectCareer {
            context,
            career_history,
            json,
            out,
        }) => commands::icecast::run_prospect_career(context, career_history, json, out)?,
        Commands::Icecast(IceCastSubcommand::ProspectProgram {
            league_discoveries,
            career_discoveries,
            studies,
            prior_board,
            source_package,
            maximum_nhl_games,
            prospects_per_team,
            require_complete_rankings,
            require_complete_population,
            json,
            out,
        }) => commands::icecast::run_prospect_program(
            league_discoveries,
            career_discoveries,
            studies,
            prior_board,
            source_package,
            icelines_core::ProspectProgramBoardConfig {
                maximum_nhl_games_played: maximum_nhl_games,
                prospects_per_team: usize::from(prospects_per_team),
                ..icelines_core::ProspectProgramBoardConfig::default()
            },
            commands::icecast::ProspectProgramPublicationOptions {
                require_complete_rankings,
                require_complete_population,
                json,
                out,
            },
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectProgramSensitivity {
            league_discoveries,
            career_discoveries,
            studies,
            thresholds,
            json,
            out,
        }) => commands::icecast::run_prospect_program_sensitivity(
            league_discoveries,
            career_discoveries,
            studies,
            thresholds,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectProgramHistory { boards, json, out }) => {
            commands::icecast::run_prospect_program_history(boards, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::ProspectConversion {
            league_discoveries,
            career_discoveries,
            studies,
            career_history,
            baseline_season,
            through_season,
            performance,
            performance_out,
            input_out,
            archive_out,
            json,
            out,
        }) => commands::icecast::run_prospect_conversion(
            league_discoveries,
            career_discoveries,
            studies,
            career_history,
            baseline_season,
            through_season,
            performance,
            performance_out,
            input_out,
            archive_out,
            json,
            out,
        )?,
        Commands::Icecast(IceCastSubcommand::ProspectConversionReplay {
            input,
            archive,
            json,
            out,
        }) => commands::icecast::run_prospect_conversion_replay(input, archive, json, out)?,
        Commands::Icecast(IceCastSubcommand::ProspectBoard { studies, json, out }) => {
            commands::icecast::run_prospect_board(studies, json, out)?
        }
        Commands::Icecast(IceCastSubcommand::ImportOpeningRosters {
            manifest,
            dry_run,
            allow_partial_evaluation,
        }) => {
            commands::icecast::run_import_opening_rosters(
                manifest,
                dry_run,
                allow_partial_evaluation,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::DiscoverOpeningRosters {
            season,
            out,
            manifest_out,
            partial_manifest_out,
            cache_only,
        }) => {
            commands::icecast::run_discover_opening_rosters(
                season,
                out,
                manifest_out,
                partial_manifest_out,
                cache_only,
            )
            .await?
        }
        Commands::Icecast(IceCastSubcommand::Scenario { command }) => match command {
            IceCastScenarioSubcommand::Import {
                id,
                path,
                season,
                evidence,
                json,
            } => commands::icecast::run_scenario_import(id, path, season, evidence, json).await?,
            IceCastScenarioSubcommand::List { json } => commands::icecast::run_scenario_list(json)?,
            IceCastScenarioSubcommand::Show { id, json } => {
                commands::icecast::run_scenario_show(id, json)?
            }
        },
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)] // Dispatch helpers intentionally follow focused main tests.
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn l0_auto_setup_runs_only_for_interactive_missing_config() {
        assert!(should_auto_setup(false, false, false, true, true));
    }

    #[test]
    fn l0_auto_setup_skips_setup_command_and_existing_config() {
        assert!(!should_auto_setup(false, true, false, true, true));
        assert!(!should_auto_setup(false, false, true, true, true));
    }

    #[test]
    fn l0_auto_setup_skips_no_setup_and_non_interactive() {
        assert!(!should_auto_setup(true, false, false, true, true));
        assert!(!should_auto_setup(false, false, false, false, true));
        assert!(!should_auto_setup(false, false, false, true, false));
    }

    #[test]
    fn l0_nested_icecast_help_renders_on_production_parser_stack() {
        let result =
            run_on_cli_stack(|| Cli::try_parse_from(["icelines", "icecast", "backtest", "--help"]))
                .expect("large-stack parser thread should start");
        let help = result.expect_err("--help should stop argument parsing");

        assert_eq!(help.kind(), ErrorKind::DisplayHelp);
        let rendered = help.to_string();
        assert!(rendered.contains("Cross-validate Elo blends"));
        assert!(rendered.contains("--input <INPUTS>"));
    }
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
