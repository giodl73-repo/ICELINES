use crate::commands::output::Format;
use crate::config::Config;
use anyhow::Context;
use icelines_core::model::{Season, MIN_GP};
use icelines_core::name::normalize_name;
use icelines_core::season_stats::SeasonType;
use icelines_core::{
    compute_projection, PlayerScoringPaceMetric, PlayerScoringPaceRow, PlayerScoringPaceView,
    ProjectionMode, ViewContext, ViewWindow,
};
use icelines_fetch::career::load_career;
use icelines_fetch::snapshot::SnapshotStore;

const DEFAULT_REMAINING: u32 = 20; // fallback when schedule not available

/// Compute age from `bio.birth_date` ("YYYY-MM-DD") relative to 2026.
/// Defaults to 27 when birth_date is missing or malformed.
fn age_from_view(v: &icelines_core::stats_repository::PlayerView<'_>) -> u8 {
    v.identity
        .bio
        .birth_date
        .as_deref()
        .and_then(|d| d.get(..4))
        .and_then(|y| y.parse::<u16>().ok())
        .map(|y| (2026u16.saturating_sub(y)).min(99) as u8)
        .unwrap_or(27)
}

pub async fn run(
    target: Option<String>,
    team: Option<String>,
    mode: String,
    games: Option<u32>,
    json: bool,
    csv: bool,
    out: Option<std::path::PathBuf>,
) -> anyhow::Result<()> {
    let mode: ProjectionMode = mode.parse().map_err(|e: String| anyhow::anyhow!(e))?;

    // Hart.5b2: load via load_into_repo + .skaters() instead of
    // load_all_players (Vec<Player>).
    let cfg = Config::load()?;
    let store = SnapshotStore::new(cfg.snapshot_dir());
    let (outcome, season, _) =
        crate::commands::players::load_repo_for_season(None, Some(SeasonType::Regular))?;
    let season_u32 = season.0;

    let remaining = games.unwrap_or(DEFAULT_REMAINING);
    let format = Format::resolve(csv, json)?;

    if let Some(name) = target {
        // Single player
        let norm = normalize_name(&name);
        let view = outcome
            .repo
            .skaters(Season(season_u32), SeasonType::Regular)
            .find(|v| v.name_normalized().contains(&norm))
            .with_context(|| format!("player '{name}' not found — try a partial name"))?;

        let Some(score) = view.pace_score().copied() else {
            anyhow::bail!(
                "'{}' has fewer than {MIN_GP} games — not enough data to project",
                view.full_name()
            );
        };

        let current_ppg = score.pace_82 / 82.0;
        let age = age_from_view(&view);

        // Use real career PPG from bundled historical data for regressed mode
        let career_ppg = load_career(view.full_name(), 5, &store).map(|c| c.career_ppg as f64);

        let result = compute_projection(current_ppg, career_ppg, score.gp, age, remaining, mode);
        let pace_view = PlayerScoringPaceView::from_player(
            ViewContext::new(ViewWindow::new(Season(season_u32), SeasonType::Regular)),
            &view,
            Some(remaining),
        );

        if format == Format::Table && out.is_none() {
            println!(
                "PROJECTION — {} ({} · {:?} · {} remaining games)",
                view.full_name(),
                view.team_display(),
                mode,
                remaining
            );
            println!("{}", "─".repeat(56usize));
            println!("  Current PPG:        {:.3}", result.current_ppg);
            if let Some(cp) = career_ppg {
                println!("  Career PPG:         {cp:.3}  (5-season avg)");
            }
            println!("  α (blend weight):   {:.2}", result.alpha);
            println!("  Age factor:         {:.2}", result.age_factor);
            println!("  Projected pts:      {:.1}", result.projected_points);
            println!(
                "  Confidence band:    {:.1} – {:.1}  (±{:.1})",
                result.low_band,
                result.high_band,
                result.confidence_band_width() / 2.0
            );
            println!(
                "  Pace outlook ranges: {}",
                pace_outlook_ranges_label(&pace_view)
            );
            return Ok(());
        }

        // Single-player CSV/JSON: long-form rows (one stat per row).
        let headers = &["stat", "value"];
        let career_str = career_ppg
            .map(|v| format!("{v:.3}"))
            .unwrap_or_else(|| "—".to_owned());
        let rows: Vec<Vec<String>> = vec![
            vec!["player".to_owned(), view.full_name().to_owned()],
            vec!["team".to_owned(), view.team_display().to_owned()],
            vec!["mode".to_owned(), format!("{mode:?}")],
            vec!["remaining_games".to_owned(), remaining.to_string()],
            vec![
                "current_ppg".to_owned(),
                format!("{:.3}", result.current_ppg),
            ],
            vec!["career_ppg".to_owned(), career_str],
            vec!["alpha".to_owned(), format!("{:.2}", result.alpha)],
            vec!["age_factor".to_owned(), format!("{:.2}", result.age_factor)],
            vec![
                "projected_points".to_owned(),
                format!("{:.1}", result.projected_points),
            ],
            vec!["band_low".to_owned(), format!("{:.1}", result.low_band)],
            vec!["band_high".to_owned(), format!("{:.1}", result.high_band)],
        ];
        let mut rows = rows;
        append_pace_outlook_rows(&mut rows, &pace_view);
        format.emit_to(headers, &rows, out.as_deref())?;
    } else if let Some(team_abbr) = team {
        // Team-wide projection
        let team_upper = team_abbr.to_uppercase();
        let team_views: Vec<_> = outcome
            .repo
            .skaters(Season(season_u32), SeasonType::Regular)
            .filter(|v| v.team_display() == team_upper && v.pace_score().is_some())
            .collect();

        if team_views.is_empty() {
            anyhow::bail!(
                "no rankable players found for {} — run `icelines fetch`",
                team_upper
            );
        }

        let headers = &[
            "player",
            "pos",
            "current_ppg",
            "projected_points",
            "band_low",
            "band_high",
        ];
        let rows: Vec<Vec<String>> = team_views
            .iter()
            .map(|v| {
                let score = v.pace_score().unwrap();
                let current_ppg = score.pace_82 / 82.0;
                let age = age_from_view(v);
                let r = compute_projection(current_ppg, None, score.gp, age, remaining, mode);
                vec![
                    v.full_name().to_owned(),
                    v.position().abbreviation().to_owned(),
                    format!("{:.2}", r.current_ppg),
                    format!("{:.1}", r.projected_points),
                    format!("{:.1}", r.low_band),
                    format!("{:.1}", r.high_band),
                ]
            })
            .collect();

        if format == Format::Table && out.is_none() {
            println!(
                "PROJECTIONS — {} ({:?} mode · {} remaining)",
                team_upper, mode, remaining
            );
        }
        format.emit_to(headers, &rows, out.as_deref())?;
    } else {
        anyhow::bail!("specify a player name or --team ABBREV");
    }

    Ok(())
}

fn append_pace_outlook_rows(rows: &mut Vec<Vec<String>>, view: &PlayerScoringPaceView) {
    rows.push(vec![
        "pace_outlook_schema".to_owned(),
        "player_scoring_pace.v1".to_owned(),
    ]);
    rows.push(vec![
        "pace_outlook_sample_status".to_owned(),
        format!("{:?}", view.sample_status),
    ]);
    rows.push(vec![
        "pace_outlook_remaining_games".to_owned(),
        view.remaining_games
            .map(|value| value.to_string())
            .unwrap_or_else(|| "—".to_owned()),
    ]);

    for row in &view.rows {
        let key = pace_metric_key(row.metric);
        rows.push(vec![
            format!("pace_outlook_{key}_projected_finish"),
            format_optional_one_decimal(row.projected_finish),
        ]);
        rows.push(vec![
            format!("pace_outlook_{key}_band_low"),
            format_optional_one_decimal(row.confidence_low),
        ]);
        rows.push(vec![
            format!("pace_outlook_{key}_band_high"),
            format_optional_one_decimal(row.confidence_high),
        ]);
    }
}

fn pace_outlook_ranges_label(view: &PlayerScoringPaceView) -> String {
    view.rows
        .iter()
        .map(|row| format!("{} {}", row.label, pace_range_label(row)))
        .collect::<Vec<_>>()
        .join(" · ")
}

fn pace_range_label(row: &PlayerScoringPaceRow) -> String {
    match (row.confidence_low, row.confidence_high) {
        (Some(low), Some(high)) => format!("{low:.1}-{high:.1}"),
        _ => "—".to_owned(),
    }
}

fn pace_metric_key(metric: PlayerScoringPaceMetric) -> &'static str {
    match metric {
        PlayerScoringPaceMetric::Goals => "goals",
        PlayerScoringPaceMetric::Points => "points",
        PlayerScoringPaceMetric::Shots => "shots",
    }
}

fn format_optional_one_decimal(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "—".to_owned())
}
