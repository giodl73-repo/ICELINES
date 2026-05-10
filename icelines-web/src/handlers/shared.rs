use crate::templates::LeaderRow;

/// Build the NHL CDN headshot URL for a player.
pub(crate) fn build_headshot_url(season: u32, team: &str, nhl_id: u32) -> String {
    let team = team.trim();
    let valid_shape = (2..=3).contains(&team.len()) && team.chars().all(|c| c.is_ascii_uppercase());
    let valid_team = valid_shape && team != "RET";
    if !valid_team {
        return format!("https://assets.nhle.com/mugs/nhl/default/{nhl_id}.png");
    }
    format!("https://assets.nhle.com/mugs/nhl/{season}/{team}/{nhl_id}.png")
}

/// Same as `build_headshot_url` but uses the primary team from a display string.
pub(crate) fn build_headshot_url_for_display(
    season: u32,
    team_display: &str,
    nhl_id: u32,
) -> String {
    let primary = team_display.split('/').next().unwrap_or("").trim();
    build_headshot_url(season, primary, nhl_id)
}

/// Project a `PlayerView` into the `LeaderRow` shape shared by multiple routes.
pub(crate) fn project_leader_row(v: &icelines_core::stats_repository::PlayerView) -> LeaderRow {
    project_leader_row_with_prior(v, None)
}

/// Like `project_leader_row`, but carries an optional prior-season points total.
pub(crate) fn project_leader_row_with_prior(
    v: &icelines_core::stats_repository::PlayerView,
    prior_points: Option<u32>,
) -> LeaderRow {
    let gp = v.gp();
    let points = v.points();
    let ppg_str = if gp > 0 {
        format!("{:.2}", points as f64 / gp as f64)
    } else {
        String::new()
    };
    let totals = &v.stats.totals;
    let plus_minus = v.plus_minus();
    let hits = v.hits();
    let blocks = v.blocked_shots();
    let shooting_pct = totals.shooting_pct;
    let faceoff_pct = totals.faceoff_win_pct;
    let opt_u = |o: Option<u32>| -> String {
        match o {
            Some(n) => n.to_string(),
            None => "—".to_owned(),
        }
    };
    let opt_pct = |o: Option<f32>| -> String {
        match o {
            Some(p) => {
                if p.abs() <= 1.5 {
                    format!("{:.1}%", p * 100.0)
                } else {
                    format!("{:.1}%", p)
                }
            }
            None => "—".to_owned(),
        }
    };
    let team_display = v.team_display().to_owned();
    let primary_team = team_display
        .split('/')
        .next()
        .unwrap_or("")
        .trim()
        .to_owned();
    let headshot_url = build_headshot_url(v.season().0, &primary_team, v.id().0);
    let headshot_fallback_url =
        format!("https://assets.nhle.com/mugs/nhl/default/{}.png", v.id().0);

    let total_toi_secs: Option<u64> = totals
        .toi_per_game_sec
        .map(|tpg| u64::from(tpg) * u64::from(gp))
        .filter(|s| *s > 0);
    let per_60 =
        |stat: f64| -> Option<f64> { total_toi_secs.map(|toi| stat * 3600.0 / toi as f64) };
    let opt_p60 = |o: Option<f64>| -> String {
        match o {
            Some(v) => format!("{:.2}", v),
            None => "—".to_owned(),
        }
    };
    let goals_per_60 = per_60(v.goals() as f64);
    let assists_per_60 = per_60(v.assists() as f64);
    let points_per_60 = per_60(v.points() as f64);
    let hits_per_60 = hits.and_then(|h| per_60(h as f64));
    let blocks_per_60 = blocks.and_then(|b| per_60(b as f64));

    let points_delta = prior_points.map(|prev| v.points() as i32 - prev as i32);
    let (points_delta_str, points_delta_class) = match points_delta {
        Some(d) if d > 0 => (format!("{:+}", d), "delta-up".to_owned()),
        Some(d) if d < 0 => (format!("{:+}", d), "delta-down".to_owned()),
        Some(_) => ("0".to_owned(), "delta-flat".to_owned()),
        None => (String::new(), String::new()),
    };

    LeaderRow {
        nhl_id: v.id().0,
        name: v.full_name().to_owned(),
        position: v.position().abbreviation().to_owned(),
        team: team_display,
        gp,
        goals: v.goals(),
        assists: v.assists(),
        points,
        ppg_str,
        plus_minus_str: format!("{:+}", plus_minus),
        pim: totals.pim,
        shots: totals.shots,
        shooting_pct_str: opt_pct(shooting_pct),
        hits_str: opt_u(hits),
        blocks_str: opt_u(blocks),
        faceoff_pct_str: opt_pct(faceoff_pct),
        pp_points: totals.pp_points,
        plus_minus,
        shooting_pct,
        hits,
        blocks,
        faceoff_pct,
        headshot_url,
        headshot_fallback_url,
        points_per_60_str: opt_p60(points_per_60),
        goals_per_60_str: opt_p60(goals_per_60),
        assists_per_60_str: opt_p60(assists_per_60),
        hits_per_60_str: opt_p60(hits_per_60),
        blocks_per_60_str: opt_p60(blocks_per_60),
        points_per_60,
        goals_per_60,
        assists_per_60,
        hits_per_60,
        blocks_per_60,
        points_delta,
        points_delta_str,
        points_delta_class,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_headshot_seasonal_team_path() {
        let url = build_headshot_url(20252026, "EDM", 8478402);
        assert_eq!(
            url,
            "https://assets.nhle.com/mugs/nhl/20252026/EDM/8478402.png"
        );
    }

    #[test]
    fn l0_headshot_falls_back_to_default_for_sentinel_team() {
        for sentinel in ["", "—", "RET", "EDM/CGY", "edm", "abc123"] {
            let url = build_headshot_url(20252026, sentinel, 1);
            assert_eq!(
                url, "https://assets.nhle.com/mugs/nhl/default/1.png",
                "sentinel team {sentinel:?} should fall back to default"
            );
        }
    }

    #[test]
    fn l0_headshot_for_display_picks_primary_team_in_trade() {
        let url = build_headshot_url_for_display(20252026, "SEA/NYR", 8481789);
        assert_eq!(
            url,
            "https://assets.nhle.com/mugs/nhl/20252026/SEA/8481789.png"
        );
    }

    #[test]
    fn l0_headshot_for_display_passthrough_for_single_team() {
        assert_eq!(
            build_headshot_url_for_display(20252026, "EDM", 8478402),
            build_headshot_url(20252026, "EDM", 8478402),
        );
    }
}
