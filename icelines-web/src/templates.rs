//! Askama template structs.
//!
//! Phase King Clancy King.1.4 — `base.html` + `home.html` ship today.
//! Each future sub-phase adds its own template alongside the matching
//! handler:
//!
//! - King.2 — `leaders.html`, `leaders_partial.html` (HTMX fragment)
//! - King.3 — `player.html`, `player_partial.html`, `compare.html`, `comps.html`
//! - King.4 — `team.html`, `depth.html`, `class.html`, `trade.html`
//! - King.5 — `goalies.html`, `goalie.html`
//! - King.6 — `reports.html`, `seasons.html`
//! - King.7 — `scores.html`, `schedule.html`, `playoffs.html`, `series.html`, `game.html`
//! - King.8 — `transactions.html`, `search.html`, `docs.html`, `groups.html`, `games.html`
//! - King.9 — `fantasy/*.html`
//!
//! ## Inheritance
//!
//! Every page extends `base.html`. The base owns:
//! - `<head>`: charset, viewport, title block, stylesheet link, favicon
//! - skip-to-content link
//! - active-(season, season_type) sticky header
//! - `<main id="main">` landmark
//!
//! Pages override `{% block title %}` and `{% block content %}`. Every
//! page must therefore receive `active_label: String` so the base
//! header renders. The `l1_html_each_route_has_active_season_header`
//! fence (King.1.4 closeout) walks the router and asserts the label
//! appears in every HTML response.
//!
//! ## Compile-time templates
//!
//! `askama` parses templates at *build* time. A typo in a template
//! (`{% if foo` instead of `{% if foo %}`) fails `cargo build`, not
//! a runtime request. This is one of the spec's "vendored
//! interactivity" wins — no runtime template engine, no runtime
//! template fetch.

use askama::Template;

/// `home.html` — the `/` page.
///
/// Carries `active_label` (e.g. `"25-26 · Regular"`) into the base
/// template so the season header renders. Also displayed inline in
/// the page footer for visual confirmation.
#[derive(Template)]
#[template(path = "home.html")]
pub struct HomeTemplate {
    pub active_label: String,
}

/// `coming_soon.html` — placeholder for routes that have a slot in
/// the spec but no handler shipped yet. Each section nav link on the
/// home page mounts one of these so a click lands on a real page
/// (with the active-season header + back-to-dashboard link) rather
/// than axum's bare 404. As real handlers ship in King.2+, the
/// coming-soon mounts get replaced.
#[derive(Template)]
#[template(path = "coming_soon.html")]
pub struct ComingSoonTemplate {
    pub title: String,
    pub king_phase: String,
    pub description: String,
    pub active_label: String,
}

/// `leaders.html` — King.2.x leaderboards. King.2.1 shipped the
/// minimum viable real-data path; King.2.2 adds sort + position +
/// top-N query params; King.2.3 adds the boolean filter form;
/// King.2.4 adds the JSON twin at `/api/v1/leaders`.
#[derive(Template)]
#[template(path = "leaders.html")]
pub struct LeadersTemplate {
    pub active_label: String,
    pub rows: Vec<LeaderRow>,
    pub total: usize,
    /// Human label for the active sort, e.g. "Points/Game".
    pub active_sort_label: String,
    /// Active sort URL token (`points`/`goals`/`assists`/`gp`/`ppg`).
    pub active_sort: String,
    /// Active position filter URL token (`C`/`LW`/`F`/... or `""`).
    pub active_pos: String,
    /// Top-N rows requested (default 20).
    pub active_top: usize,
    /// Position-filter strip — pre-computed (label, url-value, is_active)
    /// triples so the template doesn't need to compare String against
    /// &str (askama doesn't auto-coerce).
    pub pos_chips: Vec<PosChip>,
    /// Column headers — same pre-computation pattern.
    pub col_headers: Vec<ColHeader>,
    /// Active `?filter=` strings, in URL order. Each renders as one
    /// row in the filter form. Empty Vec → form starts blank.
    pub active_filters: Vec<String>,
}

/// One chip in the position-filter strip.
#[derive(Debug, Clone)]
pub struct PosChip {
    /// What the user sees ("All", "C", "LW", ...).
    pub label: String,
    /// `?pos=` value (empty for "All").
    pub value: String,
    /// True when this chip matches the active filter.
    pub is_active: bool,
}

/// One sortable column header.
#[derive(Debug, Clone)]
pub struct ColHeader {
    /// `?sort=` URL token.
    pub url_token: String,
    /// Visible header label ("GP", "G", "A", "P", "P/GP").
    pub label: String,
    /// True when this column is the active sort.
    pub is_active: bool,
}

/// One row in the leaderboard. Pre-projected from the
/// `PlayerView` so the template doesn't reach into core types
/// (askama doesn't allow `as f64` casts inline; ppg is precomputed).
#[derive(Debug, Clone)]
pub struct LeaderRow {
    /// NHL player id (e.g. 8478402 = Connor McDavid). Used to build
    /// the link to `/player/:id`.
    pub nhl_id: u32,
    pub name: String,
    pub position: String,
    pub team: String,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    /// Points per game, formatted to 2 decimals. Empty string if gp=0.
    pub ppg_str: String,
}

/// `team.html` — King.4.1. Roster page for one team.
#[derive(Template)]
#[template(path = "team.html")]
pub struct TeamTemplate {
    pub active_label: String,
    pub team_abbrev: String,
    pub skaters: Vec<LeaderRow>,
    pub goalies: Vec<GoalieRow>,
}

/// `goalies.html` — King.5.1 minimum viable goalie leaderboard.
#[derive(Template)]
#[template(path = "goalies.html")]
pub struct GoaliesTemplate {
    pub active_label: String,
    pub rows: Vec<GoalieRow>,
    pub total: usize,
    pub qualified_threshold: u32,
}

/// One row in the goalie leaderboard.
#[derive(Debug, Clone)]
pub struct GoalieRow {
    pub nhl_id: u32,
    pub name: String,
    pub team: String,
    pub gp: u32,
    pub wins: u32,
    pub losses: u32,
    pub shutouts: u32,
    pub save_pct_str: String,
    pub gaa_str: String,
}

/// `player.html` — King.3.1 + King.3.2. Player card with career table.
#[derive(Template)]
#[template(path = "player.html")]
pub struct PlayerTemplate {
    pub active_label: String,
    pub nhl_id: u32,
    pub full_name: String,
    pub position: String,
    pub team: String,
    pub headshot_url: Option<String>,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ppg_str: String,
    /// Full career — one row per (season, type) the player has stats
    /// for. King.3.2 lands this; the row count for a 10-year veteran
    /// is ~10-20 (regular only) or ~15-30 (regular + playoff).
    pub career_rows: Vec<CareerRow>,
}

/// One row in the player-card career table.
#[derive(Debug, Clone)]
pub struct CareerRow {
    /// Pretty season label e.g. "2024-25".
    pub season: String,
    /// "Regular" or "Playoff".
    pub season_type: String,
    /// Last team in that (season, type).
    pub team: String,
    pub gp: u32,
    pub goals: u32,
    pub assists: u32,
    pub points: u32,
    pub ppg_str: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// l0_home_template_renders
    /// — askama parses templates at build time, so a render failure
    ///   here means a runtime substitution issue (e.g. malformed
    ///   variable). Lock the rendered output contains both the
    ///   page-specific copy AND the base-inherited active-season
    ///   header label.
    #[test]
    fn l0_home_template_renders() {
        let tmpl = HomeTemplate {
            active_label: "25-26 · Regular".to_owned(),
        };
        let html = tmpl.render().expect("home template renders");
        // Page-specific content
        assert!(html.contains("Welcome"));
        // Base-inherited active-season header
        assert!(
            html.contains("25-26 · Regular"),
            "base template must render active_label, got: {html}"
        );
        // Sticky header structure
        assert!(html.contains("season-header"));
    }

    /// l0_home_template_includes_a11y_baseline
    /// — broadcast finding: every page MUST carry viewport meta,
    ///   skip-link, and `<main>` landmark. The base template owns
    ///   these; this test asserts the inheritance still emits them.
    #[test]
    fn l0_home_template_includes_a11y_baseline() {
        let tmpl = HomeTemplate {
            active_label: "x".to_owned(),
        };
        let html = tmpl.render().unwrap();
        assert!(
            html.contains("name=\"viewport\""),
            "every page must include viewport meta"
        );
        assert!(
            html.contains("skip-link"),
            "every page must include skip-to-content link"
        );
        assert!(
            html.contains("id=\"main\"") || html.contains("id='main'"),
            "every page must include <main id='main'> landmark"
        );
        assert!(
            html.contains("lang=\"en\""),
            "every page must declare a lang"
        );
    }

    /// l0_home_template_links_static_assets
    /// — base must reference /static/style.css and /static/icelines.svg
    ///   from the King.1.3 vendored asset pipeline.
    #[test]
    fn l0_home_template_links_static_assets() {
        let tmpl = HomeTemplate {
            active_label: "x".to_owned(),
        };
        let html = tmpl.render().unwrap();
        assert!(html.contains("/static/style.css"));
        assert!(html.contains("/static/icelines.svg"));
    }
}
