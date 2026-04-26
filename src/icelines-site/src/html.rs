//! HTML fragment generation for lineup cards.
//! Produces markdown-embedded HTML matching the existing fantasy.css contract.

use icelines_core::{
    cross_team::{CrossTeamMetrics, WebFitClass},
    model::Player,
};

pub fn player_cell(player: Option<&Player>, metrics: Option<&CrossTeamMetrics>) -> String {
    let Some(p) = player else {
        return r#"<td class="player-cell empty"><span class="player-name">—</span></td>"#
            .to_owned();
    };

    let (css, label) = match metrics {
        Some(m) => {
            let cls = m.web_fit_class().css_class();
            let avg = m.avg_other_line;
            let lbl = match m.web_fit_class() {
                WebFitClass::Elite => format!("★ avg L{avg:.1}"),
                WebFitClass::Solid => format!("~ avg L{avg:.1}"),
                WebFitClass::Buried => format!("↑ avg L{avg:.1} — underused"),
                WebFitClass::Stretch => format!("↓ avg L{avg:.1} — overextended"),
            };
            (cls, lbl)
        }
        None => ("fit", "no metrics".to_owned()),
    };

    let photo = p.headshot_url.as_deref().unwrap_or("");
    let photo_tag = if photo.is_empty() {
        String::new()
    } else {
        format!(
            r#"<img class="player-photo" src="{photo}" alt="{name}" onerror="this.style.display='none'">"#,
            name = p.full_name
        )
    };

    let stat_str = match p.pace_score {
        Some(s) => {
            let ppg = s.pace_82 / 82.0;
            let gpg = s.goals_per_82 / 82.0;
            let proj = s.pace_82;
            format!(
                r#"<span class="player-gp">{gp}gp &nbsp;·&nbsp; <b>{ppg:.2}</b> pts/gp &nbsp;·&nbsp; {gpg:.2} g/gp &nbsp;·&nbsp; {proj:.0} proj</span>"#,
                gp = s.gp,
                ppg = ppg,
                gpg = gpg,
                proj = proj,
            )
        }
        None => format!(
            r#"<span class="player-gp">{} pts raw</span>"#,
            p.season_points
        ),
    };

    format!(
        r#"<td class="player-cell {css}">{photo}<span class="player-name">{name}</span>{stat}<span class="player-fit-label">{label}</span></td>"#,
        css = css,
        photo = photo_tag,
        name = p.full_name,
        stat = stat_str,
        label = label,
    )
}

pub fn team_logo_url(team: &str) -> String {
    let nhl_abbrev = match team {
        "LA" => "LAK",
        "NJ" => "NJD",
        "TB" => "TBL",
        "SJ" => "SJS",
        other => other,
    };
    format!("https://assets.nhle.com/logos/nhl/svg/{nhl_abbrev}_light.svg")
}

pub fn bar_fill(pts: f32, max_pts: f32, width: u32) -> u32 {
    if max_pts <= 0.0 {
        return 0;
    }
    ((pts / max_pts * width as f32) as u32).min(width)
}
