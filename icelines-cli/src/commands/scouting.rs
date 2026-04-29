use anyhow::{bail, Context};
use icelines_core::{
    compute_cross_team_metrics, compute_projection,
    cross_team::CrossTeamMetrics,
    history::CareerSummary,
    model::{Player, MIN_GP},
    name::normalize_name,
    ProjectionMode,
};
use icelines_fetch::{career::load_career, snapshot::SnapshotStore};
use std::fmt::Write as _;
use crate::commands::players::load_all_players;
use crate::config::Config;

/// Normalize the user-supplied format string. Returns the canonical name
/// on success, or a user-facing error listing the valid options.
pub(crate) fn validate_format(format: &str) -> anyhow::Result<&'static str> {
    match format.to_lowercase().as_str() {
        "terminal" => Ok("terminal"),
        "markdown" => Ok("markdown"),
        "json"     => Ok("json"),
        _ => bail!("unknown format '{format}' — valid: terminal, markdown, json"),
    }
}

pub async fn run(player_name: String, format: String) -> anyhow::Result<()> {
    let fmt = validate_format(&format)?;

    let players = load_all_players()?;
    let norm    = normalize_name(&player_name);
    let player  = players.iter()
        .find(|p| p.name_normalized.contains(&norm))
        .with_context(|| format!("player '{player_name}' not found"))?;

    let career = {
        let store = if let Ok(cfg) = Config::load() {
            SnapshotStore::new(cfg.snapshot_dir())
        } else {
            SnapshotStore::new(SnapshotStore::default_root())
        };
        load_career(&player.full_name, 5, &store)
    };

    let metrics = compute_cross_team_metrics(&players);
    let output  = render_report(player, &players, career.as_ref(), &metrics, fmt);
    print!("{output}");
    Ok(())
}

/// Pure renderer: produces the full scouting report as a String. No I/O.
/// Tests call this directly with a fixture `Player` + empty/synthetic
/// auxiliary data to assert section structure and format-specific output.
pub(crate) fn render_report(
    player:      &Player,
    all_players: &[Player],
    career:      Option<&CareerSummary>,
    metrics:     &[CrossTeamMetrics],
    format:      &str,
) -> String {
    let mut out = String::new();
    let md = format == "markdown";

    let age = player.birth_date.as_deref()
        .and_then(|d| d.get(..4)).and_then(|y| y.parse::<u16>().ok())
        .map(|y| 2026u16.saturating_sub(y).to_string()).unwrap_or_else(|| "—".to_owned());
    let draft = match (player.draft_year, player.draft_round, player.draft_overall) {
        (Some(y), Some(r), Some(o)) => format!("{y} · Round {r} · Pick #{o}"),
        (Some(y), _, _)             => y.to_string(),
        _                           => "Undrafted".to_owned(),
    };

    // ── JSON output path ─────────────────────────────────────────────────────
    if format == "json" {
        let report = serde_json::json!({
            "player":      player.full_name,
            "team":        player.team.as_str(),
            "position":    player.position.abbreviation(),
            "age":         age,
            "draft":       draft,
            "nationality": player.nationality_code,
            "handedness":  player.shoots_catches,
            "height_in":   player.height_in_inches,
            "weight_lbs":  player.weight_lbs,
            "current_season": {
                "gp":        player.gp(),
                "goals":     player.season_goals,
                "assists":   player.season_assists,
                "pts":       player.season_points,
                "ppg":       player.pace_score.map(|s| s.pace_82 / 82.0),
                "pts_82":    player.pace_score.map(|s| s.pace_82),
                "goals_82":  player.pace_score.map(|s| s.goals_per_82),
                "pp_goals":  player.pp_goals,
                "pp_points": player.pp_points,
                "gwg":       player.gwg,
                "shots":     player.shots,
                "shooting_pct": player.shooting_pct,
                "plus_minus": player.plus_minus,
                "toi_mmss":  player.toi_mmss(),
            },
            "contract": {
                "expiry_year": player.contract_expiry_year,
                "expiry_type": player.expiry_type,
            }
        });
        let pretty = serde_json::to_string_pretty(&report)
            .unwrap_or_else(|_| String::from("{}"));
        let _ = writeln!(out, "{pretty}");
        return out;
    }

    let sep: String = if md { "---".to_owned() } else { "─".repeat(60usize) };

    // ── Section 1: Bio ────────────────────────────────────────────────────────
    let _ = writeln!(out, "{}", if md {
        format!("# Scouting Report — {}", player.full_name)
    } else {
        format!("SCOUTING REPORT — {}", player.full_name)
    });
    let _ = writeln!(out, "{sep}");
    let _ = writeln!(out);
    let _ = writeln!(out, "## 1. Bio");
    let _ = writeln!(out, "  Team:         {} ({})", player.team.as_str(),
        if md { format!("*{}*", player.team.as_str()) } else { player.team.as_str().to_owned() });
    let _ = writeln!(out, "  Position:     {:?}", player.position);
    let _ = writeln!(out, "  Age:          {}", age);
    let _ = writeln!(out, "  Nationality:  {}", player.nationality_code.as_deref().unwrap_or("—"));
    let _ = writeln!(out, "  Draft:        {}", draft);
    let _ = writeln!(out, "  Handedness:   {}", player.shoots_catches.as_deref().unwrap_or("—"));

    // ── Section 2: Current season stats ──────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 2. Current Season");
    if let Some(s) = player.pace_score {
        let ppg  = s.pace_82 / 82.0;
        let gpg  = s.goals_per_82 / 82.0;
        let _ = writeln!(out, "  GP:           {}", s.gp);
        let _ = writeln!(out, "  G:            {}  →  {:.0}/82", player.season_goals, s.goals_per_82);
        let _ = writeln!(out, "  A:            {}  →  {:.0}/82", player.season_assists, s.pace_82 - s.goals_per_82);
        let _ = writeln!(out, "  PPG:          {ppg:.3} pts/gp");
        let _ = writeln!(out, "  G/gp:         {gpg:.3}");
        let _ = writeln!(out, "  Proj/82g:     {:.1}", s.pace_82);
    } else {
        let _ = writeln!(out, "  < {MIN_GP} games played — not enough data");
    }

    // ── Section 3: Career trajectory ─────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 3. Career Trajectory");
    match career {
        Some(c) if !c.seasons.is_empty() => {
            let _ = writeln!(out, "  {:<10} {:<4} {:>4} {:>4} {:>4}  {:>7}",
                "Season", "Team", "GP", "G", "A", "PPG");
            for line in &c.seasons {
                let lbl = if line.season.len() == 8 {
                    format!("{}-{}", &line.season[2..4], &line.season[6..8])
                } else { line.season.clone() };
                let _ = writeln!(out, "  {:<10} {:<4} {:>4} {:>4} {:>4}  {:>7.3}",
                    lbl, line.team, line.gp, line.goals, line.assists, line.ppg);
            }
            let _ = writeln!(out, "  Career PPG: {:.3}  Peak: {} ({:.3})",
                c.career_ppg,
                if c.peak_season.len() == 8 {
                    format!("{}-{}", &c.peak_season[2..4], &c.peak_season[6..8])
                } else { c.peak_season.clone() },
                c.peak_ppg);
        }
        _ => {
            let _ = writeln!(out, "  Career history not available in bundled data.");
        }
    }

    // ── Section 4: Peer group rank ────────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 4. Peer Group Rank");
    let draft_year = player.draft_year.unwrap_or(0);
    if draft_year > 0 {
        let peers: Vec<_> = all_players.iter()
            .filter(|p| {
                p.position == player.position &&
                p.draft_year.map(|y| (y as i32 - draft_year as i32).abs() <= 1).unwrap_or(false) &&
                p.pace_score.is_some()
            })
            .collect();
        let rank = peers.iter()
            .filter(|p| p.pace_score.map(|s| s.pace_82).unwrap_or(0.0) >
                player.pace_score.map(|s| s.pace_82).unwrap_or(0.0))
            .count() + 1;
        let _ = writeln!(out, "  Draft class:  {} ± 1 year, {:?}", draft_year, player.position);
        let _ = writeln!(out, "  Peer count:   {}", peers.len());
        let _ = writeln!(out, "  Peer rank:    #{rank} of {}", peers.len());
        let pct = if peers.len() > 1 { 100 - (rank * 100 / peers.len()) } else { 100 };
        let ord = match pct % 100 {
            11..=13 => "th",
            _ => match pct % 10 { 1=>"st", 2=>"nd", 3=>"rd", _=>"th" },
        };
        let _ = writeln!(out, "  Percentile:   {pct}{ord}");
    } else {
        let _ = writeln!(out, "  Draft data not available");
    }

    // ── Section 5: Linemates ──────────────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 5. Linemates");
    let _ = writeln!(out,
        "  Run `icelines fetch shifts` then `icelines mates {}` for shift-based linemate data.",
        player.full_name.split_whitespace().last().unwrap_or(&player.full_name));
    let teammates: Vec<_> = all_players.iter()
        .filter(|p| p.team == player.team && p.position == player.position
            && p.name_normalized != player.name_normalized && p.pace_score.is_some())
        .take(3).collect();
    let _ = writeln!(out, "  Same-team same-position players:");
    for t in &teammates {
        let ppg = t.pace_score.map(|s| format!("{:.2}", s.pace_82/82.0)).unwrap_or_else(|| "—".to_owned());
        let _ = writeln!(out, "    {} ({} pts/gp)", t.full_name, ppg);
    }

    // ── Section 6: Depth chart position ──────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 6. Depth Chart Position");
    let same_pos: Vec<_> = all_players.iter()
        .filter(|p| p.team == player.team && p.position == player.position && p.pace_score.is_some())
        .collect();
    let rank_on_team = same_pos.iter()
        .filter(|p| p.pace_score.map(|s| s.pace_82).unwrap_or(0.0) >
            player.pace_score.map(|s| s.pace_82).unwrap_or(0.0))
        .count() + 1;
    let _ = writeln!(out, "  Line {} {:?} on {} (#{rank_on_team} of {} {:?}s)",
        rank_on_team, player.position, player.team.as_str(),
        same_pos.len(), player.position);

    // ── Section 7: Cross-team value ───────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 7. Cross-Team Value");
    if let Some(m) = metrics.iter().find(|m| m.player_nhl_id == player.nhl_id) {
        let _ = writeln!(out, "  Own line:      #{}", m.own_line);
        let _ = writeln!(out, "  Avg elsewhere: L{:.2}", m.avg_other_line);
        let _ = writeln!(out, "  Delta:         {:+.2}", m.delta);
        let cls = m.web_fit_class();
        let _ = writeln!(out, "  Fit class:     {} {}", cls.label(), match cls {
            icelines_core::WebFitClass::Elite   => "elite — plays above their line on most teams",
            icelines_core::WebFitClass::Solid   => "solid — fits their role well",
            icelines_core::WebFitClass::Buried  => "buried — underused, worth more elsewhere",
            icelines_core::WebFitClass::Stretch => "stretch — overextended in current role",
        });
    } else {
        let _ = writeln!(out, "  Cross-team metrics unavailable (GP < {MIN_GP})");
    }

    // ── Section 8: Fit interpretation ────────────────────────────────────────
    let _ = writeln!(out);
    let _ = writeln!(out, "## 8. Fit Interpretation");
    if let Some(s) = player.pace_score {
        let age_n: u8 = age.parse().unwrap_or(27);
        let proj = compute_projection(s.pace_82/82.0, None, s.gp, age_n, 20, ProjectionMode::Regressed);
        let _ = writeln!(out, "  Regressed projection (next 20 games): {:.1} pts", proj.projected_points);
        let _ = writeln!(out, "  Confidence band: {:.1} – {:.1}", proj.low_band, proj.high_band);
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
    use icelines_core::model::{GpStatus, PaceScore, Player, Position, TeamAbbr};

    /// Build a Player fixture with all fields populated (modeled on
    /// `make_test_player` in icelines-core::model::tests).
    fn fixture_player() -> Player {
        Player {
            nhl_id: Some(8478402),
            full_name: "Test Skater".to_owned(),
            name_normalized: "test_skater".to_owned(),
            team: TeamAbbr("EDM".to_owned()),
            position: Position::Center,
            eligible_pos: vec![Position::Center],
            gp_status: GpStatus::Eligible(60),
            season_goals: 30,
            season_assists: 50,
            season_points: 80,
            pace_score: Some(PaceScore {
                pace_82: 109.3,    // > 80 → elite-tier branch
                goals_per_82: 41.0,
                raw_points: 80,
                gp: 60,
            }),
            pp_goals: 12,
            pp_points: 22,
            sh_goals: 1,
            sh_points: 2,
            gwg: 5,
            ot_goals: 1,
            shots: 200,
            shooting_pct: Some(0.15),
            plus_minus: 14,
            toi_per_game_sec: Some(1240.0),
            faceoff_win_pct: Some(0.51),
            hits: 80,
            blocked_shots: 35,
            missed_shots: 30,
            giveaways: 22,
            takeaways: 38,
            pim: 18,
            xg: None, xg_per_60: None, cf_pct_5v5: None,
            ff_pct_5v5: None, xgf_pct_5v5: None,
            headshot_url: None,
            sweater_number: Some(97),
            birth_date: Some("1997-01-13".to_owned()),
            birth_country: Some("CAN".to_owned()),
            nationality_code: Some("CAN".to_owned()),
            birth_city: Some("Edmonton".to_owned()),
            birth_state_province: Some("AB".to_owned()),
            shoots_catches: Some("L".to_owned()),
            height_in_inches: Some(73),
            weight_lbs: Some(193),
            draft_year: Some(2015),
            draft_round: Some(1),
            draft_overall: Some(1),
            rookie_season: None,
            contract_expiry_year: None,
            expiry_type: None,
            salary: None,
        }
    }

    fn fixture_low_gp_player() -> Player {
        let mut p = fixture_player();
        p.gp_status = GpStatus::BelowThreshold(3);
        p.pace_score = None;          // triggers the "not enough data" branch
        p.season_goals = 1;
        p.season_assists = 2;
        p.season_points = 3;
        p
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
        // Case-insensitive
        assert_eq!(validate_format("JSON").unwrap(), "json");
        assert_eq!(validate_format("Terminal").unwrap(), "terminal");
    }

    // ── render_report — section presence ─────────────────────────────────────

    #[test]
    fn l0_format_terminal_includes_all_eight_sections() {
        let p = fixture_player();
        let out = render_report(&p, std::slice::from_ref(&p), None, &[], "terminal");
        for n in 1..=8 {
            let header = format!("## {n}.");
            assert!(out.contains(&header), "section header '{header}' missing in:\n{out}");
        }
        // Terminal mode uses the unicode separator, not '---'
        assert!(out.contains("─"), "terminal separator missing");
        assert!(!out.starts_with("# "), "terminal must not start with markdown H1");
    }

    #[test]
    fn l0_format_markdown_uses_h2_headings() {
        let p = fixture_player();
        let out = render_report(&p, std::slice::from_ref(&p), None, &[], "markdown");
        assert!(out.starts_with("# Scouting Report"), "markdown H1 missing, got start: {}", &out[..40.min(out.len())]);
        assert!(out.contains("---"), "markdown horizontal rule missing");
        for n in 1..=8 {
            assert!(out.contains(&format!("## {n}.")), "section {n} heading missing");
        }
        // Italic team annotation only in markdown
        assert!(out.contains("*EDM*"), "markdown italic team marker missing");
    }

    #[test]
    fn l0_format_json_has_section_keys() {
        let p = fixture_player();
        let out = render_report(&p, std::slice::from_ref(&p), None, &[], "json");
        // Must be valid JSON
        let v: serde_json::Value = serde_json::from_str(out.trim())
            .expect("render_report json output must parse as JSON");
        // Top-level keys
        for k in &["player", "team", "position", "age", "draft", "current_season", "contract"] {
            assert!(v.get(*k).is_some(), "JSON missing key '{k}'");
        }
        // Nested current_season keys
        let cs = v.get("current_season").unwrap();
        for k in &["gp", "goals", "assists", "pts", "ppg", "pts_82", "shots"] {
            assert!(cs.get(*k).is_some(), "current_season missing '{k}'");
        }
        assert_eq!(v["player"], "Test Skater");
        assert_eq!(v["team"], "EDM");
    }

    // ── low-GP path ──────────────────────────────────────────────────────────

    #[test]
    fn l0_low_gp_skips_current_season_numerics() {
        let p = fixture_low_gp_player();
        let out = render_report(&p, std::slice::from_ref(&p), None, &[], "terminal");
        // Section 2 header still present
        assert!(out.contains("## 2. Current Season"));
        // But the content is the "not enough data" message, not numeric stats
        assert!(out.contains("not enough data"),
            "low-GP message missing, got:\n{out}");
        // Numeric labels must be absent (no "PPG:", "G/gp:" etc.)
        assert!(!out.contains("PPG:"), "PPG label must be skipped for low-GP player");
        assert!(!out.contains("Proj/82g:"), "projection label must be skipped for low-GP player");
    }

    #[test]
    fn l0_render_report_returns_non_empty() {
        let p = fixture_player();
        for fmt in &["terminal", "markdown", "json"] {
            let out = render_report(&p, std::slice::from_ref(&p), None, &[], fmt);
            assert!(!out.trim().is_empty(), "format '{fmt}' produced empty output");
        }
    }
}
