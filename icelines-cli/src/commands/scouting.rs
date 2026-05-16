use crate::config::Config;
use anyhow::{bail, Context};
use icelines_core::{
    compute_projection,
    cross_team::{compute_all_views, CrossTeamMetrics},
    history::CareerSummary,
    model::{Season, MIN_GP},
    name::normalize_name,
    scouting_report_sections,
    season_stats::SeasonType,
    stats_repository::PlayerView,
    ProjectionMode, ReportFormat, ReportKind, ReportView, ViewContext, ViewWindow,
};
use icelines_fetch::{career::load_career, snapshot::SnapshotStore, stats_loader::load_into_repo};
use std::fmt::Write as _;

/// Normalize the user-supplied format string. Returns the canonical name
/// on success, or a user-facing error listing the valid options.
pub(crate) fn validate_format(format: &str) -> anyhow::Result<&'static str> {
    match format.to_lowercase().as_str() {
        "terminal" => Ok("terminal"),
        "markdown" => Ok("markdown"),
        "json" => Ok("json"),
        "csv" => Ok("csv"),
        _ => bail!("unknown format '{format}' — valid: terminal, markdown, json, csv"),
    }
}

pub async fn run(player_name: String, format: String) -> anyhow::Result<()> {
    let fmt = validate_format(&format)?;

    let cfg = Config::load()?;
    let season_u32: u32 = cfg
        .season_str()
        .parse()
        .unwrap_or(icelines_core::CURRENT_SEASON);
    let season = Season(season_u32);
    let stype = SeasonType::Regular;

    let store = SnapshotStore::new(cfg.snapshot_dir());
    let outcome =
        load_into_repo(season, stype, &store).map_err(|e| anyhow::anyhow!("loading repo: {e}"))?;
    let repo = &outcome.repo;
    let all_views: Vec<PlayerView<'_>> = repo.skaters(season, stype).collect();

    let norm = normalize_name(&player_name);
    let view = all_views
        .iter()
        .find(|v| v.identity.name_normalized.contains(&norm))
        .copied()
        .with_context(|| format!("player '{player_name}' not found"))?;

    let career = load_career(&view.identity.full_name, 5, &store);

    // Phase Calder.5 — pre-NHL stints for the new "Development arc"
    // line in section 3. Empty when the user hasn't run
    // `icelines fetch career` yet; renderer prints nothing in that
    // case so untouched installs see no change.
    let career_history_store = icelines_fetch::career_landing::load_local_store();
    let pre_nhl_stints: Vec<icelines_core::career_history::CareerStint> = career_history_store
        .get(view.identity.id.0)
        .map(|h| {
            use icelines_core::career_history::{CareerGameType, LeagueTier};
            h.stints
                .iter()
                .filter(|s| {
                    s.league.0 != "NHL"
                        && matches!(
                            s.league.tier(),
                            LeagueTier::Pro | LeagueTier::Junior | LeagueTier::College
                        )
                        && matches!(s.game_type, CareerGameType::Regular)
                })
                .cloned()
                .collect()
        })
        .unwrap_or_default();

    let metrics = compute_all_views(&all_views);
    let output = render_report(
        &view,
        &all_views,
        career.as_ref(),
        &metrics,
        &pre_nhl_stints,
        fmt,
    );
    let report = ReportView::rendered(
        ViewContext::new(ViewWindow::new(view.season(), view.season_type())),
        ReportKind::Scouting,
        format!("scouting-{}", view.id().0),
        format!("Scouting Report - {}", view.identity.full_name),
        report_format(fmt),
        scouting_report_sections(),
        output,
    );
    print!("{}", report.rendered_body);
    Ok(())
}

fn report_format(format: &str) -> ReportFormat {
    match format {
        "markdown" => ReportFormat::Markdown,
        "json" => ReportFormat::Json,
        "csv" => ReportFormat::Csv,
        _ => ReportFormat::Terminal,
    }
}

/// Pure renderer: produces the full scouting report as a String. No I/O.
/// Tests call this directly with a fixture `PlayerView` + empty/synthetic
/// auxiliary data to assert section structure and format-specific output.
pub(crate) fn render_report(
    view: &PlayerView<'_>,
    all_views: &[PlayerView<'_>],
    career: Option<&CareerSummary>,
    metrics: &[CrossTeamMetrics],
    pre_nhl_stints: &[icelines_core::career_history::CareerStint],
    format: &str,
) -> String {
    let mut out = String::new();
    let md = format == "markdown";

    let bio = &view.identity.bio;
    let totals = &view.stats.totals;

    let age = bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string())
        .unwrap_or_else(|| "—".to_owned());
    let draft = match (bio.draft_year, bio.draft_round, bio.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} · Round {r} · Pick #{o}"),
        (Some(y), _, _) => y.to_string(),
        _ => "Undrafted".to_owned(),
    };

    let team_str = view.team_display();
    let pace = view.pace_score().copied();

    // ── CSV output path ──────────────────────────────────────────────────────
    if format == "csv" {
        let rows: &[(&str, String)] = &[
            ("player", view.identity.full_name.clone()),
            ("team", team_str.to_owned()),
            ("position", view.position().abbreviation().to_owned()),
            ("age", age.clone()),
            ("draft", draft.clone()),
            (
                "nationality",
                bio.nationality_code.clone().unwrap_or_default(),
            ),
            ("handedness", bio.shoots_catches.clone().unwrap_or_default()),
            (
                "height_in",
                bio.height_in_inches
                    .map(|h| h.to_string())
                    .unwrap_or_default(),
            ),
            (
                "weight_lbs",
                bio.weight_lbs.map(|w| w.to_string()).unwrap_or_default(),
            ),
            ("gp", view.gp().to_string()),
            ("goals", totals.goals.to_string()),
            ("assists", totals.assists.to_string()),
            ("points", totals.points.to_string()),
            (
                "ppg",
                pace.map(|s| format!("{:.3}", s.pace_82 / 82.0))
                    .unwrap_or_default(),
            ),
            (
                "pts_82",
                pace.map(|s| format!("{:.1}", s.pace_82))
                    .unwrap_or_default(),
            ),
            (
                "goals_82",
                pace.map(|s| format!("{:.1}", s.goals_per_82))
                    .unwrap_or_default(),
            ),
            ("pp_goals", totals.pp_goals.to_string()),
            ("pp_points", totals.pp_points.to_string()),
            ("shots", view.shots().to_string()),
            (
                "shooting_pct",
                totals
                    .shooting_pct
                    .map(|p| format!("{p:.3}"))
                    .unwrap_or_default(),
            ),
            ("plus_minus", view.plus_minus().to_string()),
            ("toi_mmss", view.toi_mmss().unwrap_or_default()),
            (
                "contract_expiry_year",
                view.contract_expiry_year()
                    .map(|y| y.to_string())
                    .unwrap_or_default(),
            ),
        ];
        let _ = writeln!(out, "stat,value");
        for (k, v) in rows {
            let _ = writeln!(
                out,
                "{},{}",
                crate::commands::output::escape_csv(k),
                crate::commands::output::escape_csv(v)
            );
        }
        return out;
    }

    // ── JSON output path ─────────────────────────────────────────────────────
    if format == "json" {
        let report = serde_json::json!({
            "player":      view.identity.full_name,
            "team":        team_str,
            "position":    view.position().abbreviation(),
            "age":         age,
            "draft":       draft,
            "nationality": bio.nationality_code,
            "handedness":  bio.shoots_catches,
            "height_in":   bio.height_in_inches,
            "weight_lbs":  bio.weight_lbs,
            "current_season": {
                "gp":        view.gp(),
                "goals":     totals.goals,
                "assists":   totals.assists,
                "pts":       totals.points,
                "ppg":       pace.map(|s| s.pace_82 / 82.0),
                "pts_82":    pace.map(|s| s.pace_82),
                "goals_82":  pace.map(|s| s.goals_per_82),
                "pp_goals":  totals.pp_goals,
                "pp_points": totals.pp_points,
                "gwg":       totals.gwg,
                "shots":     view.shots(),
                "shooting_pct": totals.shooting_pct,
                "plus_minus": view.plus_minus(),
                "toi_mmss":  view.toi_mmss(),
            },
            "contract": {
                "expiry_year": view.contract_expiry_year(),
                "expiry_type": view.contract_expiry_type(),
            }
        });
        let pretty = serde_json::to_string_pretty(&report).unwrap_or_else(|_| String::from("{}"));
        let _ = writeln!(out, "{pretty}");
        return out;
    }

    let sep: String = if md {
        "---".to_owned()
    } else {
        "─".repeat(60usize)
    };

    // ── Section 1: Bio ────────────────────────────────────────────────────────
    let _ = writeln!(
        out,
        "{}",
        if md {
            format!("# Scouting Report — {}", view.identity.full_name)
        } else {
            format!("SCOUTING REPORT — {}", view.identity.full_name)
        }
    );
    let _ = writeln!(out, "{sep}");
    let _ = writeln!(out);
    let _ = writeln!(out, "## 1. Bio");
    let _ = writeln!(
        out,
        "  Team:         {} ({})",
        team_str,
        if md {
            format!("*{team_str}*")
        } else {
            team_str.to_owned()
        }
    );
    let _ = writeln!(out, "  Position:     {:?}", view.position());
    let _ = writeln!(out, "  Age:          {age}");
    let _ = writeln!(
        out,
        "  Nationality:  {}",
        bio.nationality_code.as_deref().unwrap_or("—")
    );
    let _ = writeln!(out, "  Draft:        {draft}");
    let _ = writeln!(
        out,
        "  Handedness:   {}",
        bio.shoots_catches.as_deref().unwrap_or("—")
    );

    // ── Section 2: Current season stats ──────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 2. Current Season");
    if let Some(s) = pace {
        let ppg = s.pace_82 / 82.0;
        let gpg = s.goals_per_82 / 82.0;
        let _ = writeln!(out, "  GP:           {}", s.gp);
        let _ = writeln!(
            out,
            "  G:            {}  →  {:.0}/82",
            totals.goals, s.goals_per_82
        );
        let _ = writeln!(
            out,
            "  A:            {}  →  {:.0}/82",
            totals.assists,
            s.pace_82 - s.goals_per_82
        );
        let _ = writeln!(out, "  PPG:          {ppg:.3} pts/gp");
        let _ = writeln!(out, "  G/gp:         {gpg:.3}");
        let _ = writeln!(out, "  Proj/82g:     {:.1}", s.pace_82);
    } else {
        let _ = writeln!(out, "  < {MIN_GP} games played — not enough data");
    }

    // ── Section 3: Career trajectory ─────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 3. Career Trajectory");

    // Phase Calder.5 — Development arc, prepended to section 3 when
    // the local career-history store has data for this player. Tells
    // the "road to the NHL" story in one or two lines:
    //   GTHL minor → OHL Erie 2012-15 (3 yrs, 285 pts) → NHL EDM
    // Players without a populated store render no extra lines.
    if !pre_nhl_stints.is_empty() {
        let _ = writeln!(out, "  Development arc:");
        // Group consecutive stints by league for the summary line.
        // (A player's WHL years collapse into one entry.)
        let mut groups: Vec<(String, Vec<&icelines_core::career_history::CareerStint>)> =
            Vec::new();
        for s in pre_nhl_stints {
            match groups.last_mut() {
                Some((lg, ss)) if *lg == s.league.0 => ss.push(s),
                _ => groups.push((s.league.0.clone(), vec![s])),
            }
        }
        for (league, ss) in &groups {
            let years = ss.len();
            let teams: std::collections::BTreeSet<&str> =
                ss.iter().map(|s| s.team.as_str()).collect();
            let total_p: u32 = ss.iter().filter_map(|s| s.points).sum();
            let total_gp: u32 = ss.iter().map(|s| s.gp).sum();
            let teams_str = teams.into_iter().collect::<Vec<_>>().join(", ");
            let span = if years == 1 {
                format!("{} season", years)
            } else {
                format!("{} seasons", years)
            };
            let _ = writeln!(
                out,
                "    {league:<12} {teams_str:<28} {span:<12} GP {total_gp:>4}  P {total_p:>4}"
            );
        }
        let _ = writeln!(out);
    }

    match career {
        Some(c) if !c.seasons.is_empty() => {
            let _ = writeln!(
                out,
                "  {:<10} {:<4} {:>4} {:>4} {:>4}  {:>7}",
                "Season", "Team", "GP", "G", "A", "PPG"
            );
            for line in &c.seasons {
                let lbl = if line.season.len() == 8 {
                    format!("{}-{}", &line.season[2..4], &line.season[6..8])
                } else {
                    line.season.clone()
                };
                let _ = writeln!(
                    out,
                    "  {:<10} {:<4} {:>4} {:>4} {:>4}  {:>7.3}",
                    lbl, line.team, line.gp, line.goals, line.assists, line.ppg
                );
            }
            let _ = writeln!(
                out,
                "  Career PPG: {:.3}  Peak: {} ({:.3})",
                c.career_ppg,
                if c.peak_season.len() == 8 {
                    format!("{}-{}", &c.peak_season[2..4], &c.peak_season[6..8])
                } else {
                    c.peak_season.clone()
                },
                c.peak_ppg
            );
        }
        _ => {
            let _ = writeln!(out, "  Career history not available in bundled data.");
        }
    }

    // ── Section 4: Peer group rank ────────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 4. Peer Group Rank");
    let draft_year = bio.draft_year.unwrap_or(0);
    if draft_year > 0 {
        let peers: Vec<_> = all_views
            .iter()
            .filter(|v| {
                v.position() == view.position()
                    && v.identity
                        .bio
                        .draft_year
                        .map(|y| (y as i32 - draft_year as i32).abs() <= 1)
                        .unwrap_or(false)
                    && v.pace_score().is_some()
            })
            .collect();
        let my_pace = pace.map(|s| s.pace_82).unwrap_or(0.0);
        let rank = peers
            .iter()
            .filter(|v| v.pace_82().unwrap_or(0.0) > my_pace)
            .count()
            + 1;
        let _ = writeln!(
            out,
            "  Draft class:  {} ± 1 year, {:?}",
            draft_year,
            view.position()
        );
        let _ = writeln!(out, "  Peer count:   {}", peers.len());
        let _ = writeln!(out, "  Peer rank:    #{rank} of {}", peers.len());
        let pct = if peers.len() > 1 {
            100 - (rank * 100 / peers.len())
        } else {
            100
        };
        let ord = match pct % 100 {
            11..=13 => "th",
            _ => match pct % 10 {
                1 => "st",
                2 => "nd",
                3 => "rd",
                _ => "th",
            },
        };
        let _ = writeln!(out, "  Percentile:   {pct}{ord}");
    } else {
        let _ = writeln!(out, "  Draft data not available");
    }

    // ── Section 5: Linemates ──────────────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 5. Linemates");
    let _ = writeln!(
        out,
        "  Shift-profile fetch/bundling is parked; run `icelines mates {}` for the roster fallback.",
        view.identity
            .full_name
            .split_whitespace()
            .last()
            .unwrap_or(&view.identity.full_name)
    );
    let teammates: Vec<_> = all_views
        .iter()
        .filter(|v| {
            v.team_display() == team_str
                && v.position() == view.position()
                && v.identity.name_normalized != view.identity.name_normalized
                && v.pace_score().is_some()
        })
        .take(3)
        .collect();
    let _ = writeln!(out, "  Same-team same-position players:");
    for t in &teammates {
        let ppg = t
            .pace_82()
            .map(|p| format!("{:.2}", p / 82.0))
            .unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(out, "    {} ({} pts/gp)", t.identity.full_name, ppg);
    }

    // ── Section 6: Depth chart position ──────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 6. Depth Chart Position");
    let same_pos: Vec<_> = all_views
        .iter()
        .filter(|v| {
            v.team_display() == team_str
                && v.position() == view.position()
                && v.pace_score().is_some()
        })
        .collect();
    let my_pace = pace.map(|s| s.pace_82).unwrap_or(0.0);
    let rank_on_team = same_pos
        .iter()
        .filter(|v| v.pace_82().unwrap_or(0.0) > my_pace)
        .count()
        + 1;
    let _ = writeln!(
        out,
        "  Line {} {:?} on {} (#{rank_on_team} of {} {:?}s)",
        rank_on_team,
        view.position(),
        team_str,
        same_pos.len(),
        view.position()
    );

    // ── Section 7: Cross-team value ───────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 7. Cross-Team Value");
    let pid = view.identity.id.0;
    if let Some(m) = metrics.iter().find(|m| m.player_nhl_id == Some(pid)) {
        let _ = writeln!(out, "  Own line:      #{}", m.own_line);
        let _ = writeln!(out, "  Avg elsewhere: L{:.2}", m.avg_other_line);
        let _ = writeln!(out, "  Delta:         {:+.2}", m.delta);
        let cls = m.web_fit_class();
        let _ = writeln!(
            out,
            "  Fit class:     {} {}",
            crate::visual::web_fit_ascii_label(cls),
            crate::visual::web_fit_report_description(cls)
        );
    } else {
        let _ = writeln!(out, "  Cross-team metrics unavailable (GP < {MIN_GP})");
    }

    // ── Section 8: Fit interpretation ────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 8. Fit Interpretation");
    if let Some(s) = pace {
        let age_n: u8 = age.parse().unwrap_or(27);
        let proj = compute_projection(
            s.pace_82 / 82.0,
            None,
            s.gp,
            age_n,
            20,
            ProjectionMode::Regressed,
        );
        let _ = writeln!(
            out,
            "  Regressed projection (next 20 games): {:.1} pts",
            proj.projected_points
        );
        let _ = writeln!(
            out,
            "  Confidence band: {:.1} – {:.1}",
            proj.low_band, proj.high_band
        );
        let assess = if s.pace_82 > 80.0 {
            "elite-tier producer — franchise-caliber"
        } else if s.pace_82 > 50.0 {
            "top-6 contributor — strong fantasy asset"
        } else if s.pace_82 > 30.0 {
            "depth/third-line player"
        } else {
            "fourth-line / below average production"
        };
        let _ = writeln!(out, "  Assessment: {assess}");
    }
    let _ = writeln!(out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{fixtures, identity::PlayerId, stats_repository::StatsRepository};

    /// Build a one-skater repo at McDavid's 2022-23 fixture defaults.
    /// Pace_82 = 93.7 → > 80 elite-tier branch in section 8.
    fn fixture_repo() -> (StatsRepository, PlayerId, Season, SeasonType) {
        let id = fixtures::identity(8478402).build();
        let stats = fixtures::stats(8478402, 20242025, "EDM").build();
        let repo = fixtures::test_repo_with(id, stats);
        (
            repo,
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
        )
    }

    /// A second fixture: same identity but stats with GP=3 (below MIN_GP)
    /// so pace_score is None — exercises the "not enough data" branch.
    fn fixture_repo_low_gp() -> (StatsRepository, PlayerId, Season, SeasonType) {
        let id = fixtures::identity(8478402).build();
        // Build a custom stats with low GP and no pace_score.
        let totals = icelines_core::season_stats::StatTotals {
            gp: 3,
            goals: 1,
            assists: 2,
            points: 3,
            ..Default::default()
        };
        let stint = icelines_core::season_stats::TeamStint {
            team: icelines_core::TeamAbbr("EDM".into()),
            started: Some("2024-10-15".into()),
            ended: Some("2024-10-30".into()),
            gp: 3,
            goals: 1,
            assists: 2,
            points: 3,
            goalie: None,
        };
        let stats = icelines_core::season_stats::SeasonStatsBuilder::new(
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
            icelines_core::Position::Center,
        )
        .with_totals(totals)
        .add_team_stint(stint)
        .build();
        let repo = fixtures::test_repo_with(id, stats);
        (
            repo,
            PlayerId(8478402),
            Season(20242025),
            SeasonType::Regular,
        )
    }

    // ── validate_format ──────────────────────────────────────────────────────

    #[test]
    fn l0_unknown_format_errors_with_valid_options_listed() {
        let err = validate_format("xml").unwrap_err().to_string();
        assert!(err.contains("unknown format"), "missing prefix, got: {err}");
        assert!(err.contains("terminal"), "missing 'terminal', got: {err}");
        assert!(err.contains("markdown"), "missing 'markdown', got: {err}");
        assert!(err.contains("json"), "missing 'json', got: {err}");
    }

    #[test]
    fn l0_validate_format_accepts_canonical_names() {
        assert_eq!(validate_format("terminal").unwrap(), "terminal");
        assert_eq!(validate_format("markdown").unwrap(), "markdown");
        assert_eq!(validate_format("json").unwrap(), "json");
        assert_eq!(validate_format("JSON").unwrap(), "json");
        assert_eq!(validate_format("Terminal").unwrap(), "terminal");
    }

    // ── render_report — section presence ─────────────────────────────────────

    /// Phase Calder.5 / l0_dev_arc_renders_when_stints_present
    /// — When the renderer receives non-empty pre-NHL stints, section
    ///   3 carries a "Development arc:" line plus one row per
    ///   league grouping. McDavid's 3 OHL Erie seasons collapse
    ///   into one row showing GP=166 P=285.
    #[test]
    fn l0_dev_arc_renders_when_stints_present() {
        use icelines_core::career_history::{CareerGameType, CareerStint, LeagueAbbrev};
        let make = |season: u32, points: u32, gp: u32| CareerStint {
            season: icelines_core::Season(season),
            league: LeagueAbbrev::new("OHL"),
            team: "Erie".into(),
            game_type: CareerGameType::Regular,
            sequence: 1,
            gp,
            goals: Some(points / 2),
            assists: Some(points - points / 2),
            points: Some(points),
            pim: None,
            plus_minus: None,
            power_play_goals: None,
            power_play_points: None,
            shorthanded_goals: None,
            shorthanded_points: None,
            game_winning_goals: None,
            ot_goals: None,
            shots: None,
            shooting_pct: None,
            avg_toi_sec: None,
            faceoff_win_pct: None,
            games_started: None,
            wins: None,
            losses: None,
            ot_losses: None,
            goals_against: None,
            goals_against_avg: None,
            save_pct: None,
            shots_against: None,
            shutouts: None,
            time_on_ice_sec: None,
        };
        let stints = vec![
            make(20122013, 66, 63),
            make(20132014, 99, 56),
            make(20142015, 120, 47),
        ];
        let (repo, pid, s, t) = fixture_repo();
        let view = repo.view(pid, s, t).unwrap();
        let out = render_report(
            &view,
            std::slice::from_ref(&view),
            None,
            &[],
            &stints,
            "terminal",
        );
        assert!(
            out.contains("Development arc:"),
            "section 3 should carry 'Development arc:' header when stints present, got:\n{out}"
        );
        // The 3 OHL Erie seasons collapse into one row with totals.
        assert!(
            out.contains("OHL") && out.contains("Erie") && out.contains("3 seasons"),
            "row missing OHL/Erie/3 seasons in:\n{out}"
        );
        // GP 63+56+47 = 166, P 66+99+120 = 285.
        assert!(out.contains("166"), "GP total wrong, got:\n{out}");
        assert!(out.contains("285"), "P total wrong, got:\n{out}");
    }

    /// Phase Calder.5 / l0_dev_arc_omitted_when_stints_empty
    /// — Empty slice → no Development arc line. Mirrors the existing
    ///   "no career history" path so untouched installs see no change.
    #[test]
    fn l0_dev_arc_omitted_when_stints_empty() {
        let (repo, pid, s, t) = fixture_repo();
        let view = repo.view(pid, s, t).unwrap();
        let out = render_report(
            &view,
            std::slice::from_ref(&view),
            None,
            &[],
            &[],
            "terminal",
        );
        assert!(
            !out.contains("Development arc:"),
            "no stints → no Development arc line; got:\n{out}"
        );
    }

    #[test]
    fn l0_format_terminal_includes_all_eight_sections() {
        let (repo, pid, s, t) = fixture_repo();
        let view = repo.view(pid, s, t).unwrap();
        let out = render_report(
            &view,
            std::slice::from_ref(&view),
            None,
            &[],
            &[],
            "terminal",
        );
        for n in 1..=8 {
            let header = format!("## {n}.");
            assert!(
                out.contains(&header),
                "section header '{header}' missing in:\n{out}"
            );
        }
        assert!(out.contains("─"), "terminal separator missing");
        assert!(
            !out.starts_with("# "),
            "terminal must not start with markdown H1"
        );
    }

    #[test]
    fn l0_format_markdown_uses_h2_headings() {
        let (repo, pid, s, t) = fixture_repo();
        let view = repo.view(pid, s, t).unwrap();
        let out = render_report(
            &view,
            std::slice::from_ref(&view),
            None,
            &[],
            &[],
            "markdown",
        );
        assert!(
            out.starts_with("# Scouting Report"),
            "markdown H1 missing, got start: {}",
            &out[..40.min(out.len())]
        );
        assert!(out.contains("---"), "markdown horizontal rule missing");
        for n in 1..=8 {
            assert!(
                out.contains(&format!("## {n}.")),
                "section {n} heading missing"
            );
        }
        assert!(out.contains("*EDM*"), "markdown italic team marker missing");
    }

    #[test]
    fn l0_format_json_has_section_keys() {
        let (repo, pid, s, t) = fixture_repo();
        let view = repo.view(pid, s, t).unwrap();
        let out = render_report(&view, std::slice::from_ref(&view), None, &[], &[], "json");
        let v: serde_json::Value =
            serde_json::from_str(out.trim()).expect("render_report json output must parse as JSON");
        for k in &[
            "player",
            "team",
            "position",
            "age",
            "draft",
            "current_season",
            "contract",
        ] {
            assert!(v.get(*k).is_some(), "JSON missing key '{k}'");
        }
        let cs = v.get("current_season").unwrap();
        for k in &["gp", "goals", "assists", "pts", "ppg", "pts_82", "shots"] {
            assert!(cs.get(*k).is_some(), "current_season missing '{k}'");
        }
        assert_eq!(v["player"], "Connor McDavid");
        assert_eq!(v["team"], "EDM");
    }

    // ── low-GP path ──────────────────────────────────────────────────────────

    #[test]
    fn l0_low_gp_skips_current_season_numerics() {
        let (repo, pid, s, t) = fixture_repo_low_gp();
        let view = repo.view(pid, s, t).unwrap();
        let out = render_report(
            &view,
            std::slice::from_ref(&view),
            None,
            &[],
            &[],
            "terminal",
        );
        assert!(out.contains("## 2. Current Season"));
        assert!(
            out.contains("not enough data"),
            "low-GP message missing, got:\n{out}"
        );
        assert!(
            !out.contains("PPG:"),
            "PPG label must be skipped for low-GP player"
        );
        assert!(
            !out.contains("Proj/82g:"),
            "projection label must be skipped for low-GP player"
        );
    }

    #[test]
    fn l0_render_report_returns_non_empty() {
        let (repo, pid, s, t) = fixture_repo();
        let view = repo.view(pid, s, t).unwrap();
        for fmt in &["terminal", "markdown", "json"] {
            let out = render_report(&view, std::slice::from_ref(&view), None, &[], &[], fmt);
            assert!(
                !out.trim().is_empty(),
                "format '{fmt}' produced empty output"
            );
        }
    }
}
