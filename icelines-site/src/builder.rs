//! SiteBuilder — generates all site docs from snapshot data.
//! Replaces scripts/gen_site.py.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use icelines_core::cross_team::compute_all_views as compute_cross_team_metrics_views;
use icelines_core::model::{Position, Season};
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::PlayerView;
use icelines_core::{CrossTeamMetrics, DepthChartBuilder, TeamAbbr};
use icelines_fetch::{snapshot::SnapshotStore, stats_loader::load_into_repo};

use crate::{
    error::SiteError,
    html::{bar_fill, player_cell, team_logo_url},
    nav::update_nav,
};

const FULL_SEASON: u32 = 82;
const FWD_LINES: usize = 4;
const DEF_PAIRS: usize = 3;

const TEAM_NAMES: &[(&str, &str)] = &[
    ("ANA", "Anaheim Ducks"),
    ("BOS", "Boston Bruins"),
    ("BUF", "Buffalo Sabres"),
    ("CAR", "Carolina Hurricanes"),
    ("CBJ", "Columbus Blue Jackets"),
    ("CGY", "Calgary Flames"),
    ("CHI", "Chicago Blackhawks"),
    ("COL", "Colorado Avalanche"),
    ("DAL", "Dallas Stars"),
    ("DET", "Detroit Red Wings"),
    ("EDM", "Edmonton Oilers"),
    ("FLA", "Florida Panthers"),
    ("LAK", "Los Angeles Kings"),
    ("MIN", "Minnesota Wild"),
    ("MTL", "Montréal Canadiens"),
    ("NJD", "New Jersey Devils"),
    ("NSH", "Nashville Predators"),
    ("NYI", "New York Islanders"),
    ("NYR", "New York Rangers"),
    ("OTT", "Ottawa Senators"),
    ("PHI", "Philadelphia Flyers"),
    ("PIT", "Pittsburgh Penguins"),
    ("SEA", "Seattle Kraken"),
    ("SJS", "San Jose Sharks"),
    ("STL", "St. Louis Blues"),
    ("TBL", "Tampa Bay Lightning"),
    ("TOR", "Toronto Maple Leafs"),
    ("UTA", "Utah Hockey Club"),
    ("VAN", "Vancouver Canucks"),
    ("VGK", "Vegas Golden Knights"),
    ("WPG", "Winnipeg Jets"),
    ("WSH", "Washington Capitals"),
];

fn team_full_name(abbrev: &str) -> &'static str {
    TEAM_NAMES
        .iter()
        .find(|(a, _)| *a == abbrev)
        .map(|(_, n)| *n)
        .unwrap_or("Unknown")
}

pub struct SiteConfig {
    pub docs_dir: PathBuf,
    pub mkdocs_yml: PathBuf,
    pub snapshot_dir: PathBuf,
    pub season: u32,
}

pub struct SiteBuilder {
    config: SiteConfig,
}

impl SiteBuilder {
    pub fn new(config: SiteConfig) -> Self {
        Self { config }
    }

    /// Build all docs/ markdown files from snapshot data.
    pub fn build(&self) -> Result<Vec<String>, SiteError> {
        // Hart.5b2k: load via load_into_repo + iterate as PlayerView.
        // Cross-team metrics + team-strength compute on views; depth
        // charts go through DepthChartBuilder::build_views (which
        // converts to legacy Player internally so the renderers see
        // DepthChart<Player> as before).
        let store = SnapshotStore::new(&self.config.snapshot_dir);
        let outcome = load_into_repo(Season(self.config.season), SeasonType::Regular, &store)
            .map_err(|e| SiteError::Snapshot(e.to_string()))?;
        let views: Vec<PlayerView<'_>> = outcome
            .repo
            .skaters(Season(self.config.season), SeasonType::Regular)
            .collect();

        // Cross-team metrics
        let metrics_vec = compute_cross_team_metrics_views(&views);
        let metrics_map: HashMap<Option<u32>, CrossTeamMetrics> = metrics_vec
            .into_iter()
            .filter_map(|m| m.player_nhl_id.map(|id| (Some(id), m)))
            .collect();

        // Team strength for tracker index
        let team_strength = compute_team_strength_views(&views);
        let max_strength = team_strength.values().copied().fold(0.0_f32, f32::max);
        let mut ranked_teams: Vec<&str> = TEAM_NAMES.iter().map(|(a, _)| *a).collect();
        ranked_teams.sort_by(|a, b| {
            let sa = team_strength.get(*a).copied().unwrap_or(0.0);
            let sb = team_strength.get(*b).copied().unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Generate team pages
        let teams_dir = self.config.docs_dir.join("teams");
        std::fs::create_dir_all(&teams_dir)?;

        let mut written = Vec::new();
        for (rank, &abbrev) in ranked_teams.iter().enumerate() {
            let rank = rank + 1;
            let team = TeamAbbr(abbrev.to_string());
            // team_roster gives last-stint roster (current home);
            // filter to skaters since DepthChartBuilder expects them.
            let team_views: Vec<_> = outcome
                .repo
                .team_roster(&team, Season(self.config.season), SeasonType::Regular)
                .into_iter()
                .filter(|v| !v.is_goalie())
                .collect();

            if team_views.is_empty() {
                continue;
            }

            let chart = DepthChartBuilder::build_views(
                team.clone(),
                Season(self.config.season),
                &team_views,
            );

            let page = self.render_team_page(
                abbrev,
                rank,
                &chart,
                &metrics_map,
                team_strength.get(abbrev).copied().unwrap_or(0.0),
            );
            let path = teams_dir.join(format!("{abbrev}.md"));
            write_file(&path, &page)?;
            written.push(format!("docs/teams/{abbrev}.md"));
        }

        // Generate index
        let index = self.render_index(&ranked_teams, &team_strength, max_strength);
        write_file(&self.config.docs_dir.join("index.md"), &index)?;
        written.push("docs/index.md".to_owned());

        // Update mkdocs nav
        update_nav(&self.config.mkdocs_yml, &ranked_teams)?;

        Ok(written)
    }

    // ── Team page ─────────────────────────────────────────────────────────────

    fn render_team_page(
        &self,
        abbrev: &str,
        rank: usize,
        chart: &icelines_core::model::DepthChart,
        metrics: &HashMap<Option<u32>, CrossTeamMetrics>,
        total_pts: f32,
    ) -> String {
        let name = team_full_name(abbrev);
        let logo = team_logo_url(abbrev);
        let mut out = String::new();

        // Header
        out.push_str(&format!(
            r#"<div class="team-page-header"><img class="team-page-logo" src="{logo}" alt="{name} logo"><div><h1>{abbrev} — {name}</h1><p class="team-page-meta"><strong>Overall Rank: #{rank} of 32</strong> &nbsp;|&nbsp; <strong>{total_pts:.0} proj pts</strong> (pts/gp pace × {FULL_SEASON} · top-4 F + top-6 D)</p></div></div>"#
        ));
        out.push('\n');

        // Forward lines
        out.push_str("\n## Forward Lines\n\n<div class=\"lineup-section\">\n");
        out.push_str("<table class=\"lineup-table\">\n");
        out.push_str("<thead><tr><th class=\"line-col\"></th><th class=\"pos-lw\">LW</th><th class=\"pos-c\">C</th><th class=\"pos-rw\">RW</th></tr></thead>\n<tbody>\n");

        for line_idx in 0..FWD_LINES {
            let row = chart.forward_lines.get(line_idx);
            let lw = row.and_then(|r| r[0].as_ref());
            let c = row.and_then(|r| r[1].as_ref());
            let rw = row.and_then(|r| r[2].as_ref());

            let lw_m = lw
                .and_then(|p| p.nhl_id)
                .and_then(|id| metrics.get(&Some(id)));
            let c_m = c
                .and_then(|p| p.nhl_id)
                .and_then(|id| metrics.get(&Some(id)));
            let rw_m = rw
                .and_then(|p| p.nhl_id)
                .and_then(|id| metrics.get(&Some(id)));

            out.push_str("<tr>\n");
            out.push_str(&format!(
                "<td class=\"line-num-cell\">Line {}</td>\n",
                line_idx + 1
            ));
            out.push_str(&player_cell(lw, lw_m));
            out.push('\n');
            out.push_str(&player_cell(c, c_m));
            out.push('\n');
            out.push_str(&player_cell(rw, rw_m));
            out.push('\n');
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody></table></div>\n");

        // Defense pairs
        out.push_str("\n## Defense Pairs\n\n<div class=\"lineup-section\">\n");
        out.push_str("<table class=\"lineup-table\">\n");
        out.push_str("<thead><tr><th class=\"line-col\"></th><th class=\"pos-d\">D</th><th class=\"pos-d\">D</th></tr></thead>\n<tbody>\n");

        for pair_idx in 0..DEF_PAIRS {
            let row = chart.defense_pairs.get(pair_idx);
            let d1 = row.and_then(|r| r[0].as_ref());
            let d2 = row.and_then(|r| r[1].as_ref());
            let d1_m = d1
                .and_then(|p| p.nhl_id)
                .and_then(|id| metrics.get(&Some(id)));
            let d2_m = d2
                .and_then(|p| p.nhl_id)
                .and_then(|id| metrics.get(&Some(id)));

            out.push_str("<tr>\n");
            out.push_str(&format!(
                "<td class=\"pair-num-cell\">Pair {}</td>\n",
                pair_idx + 1
            ));
            out.push_str(&player_cell(d1, d1_m));
            out.push('\n');
            out.push_str(&player_cell(d2, d2_m));
            out.push('\n');
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody></table></div>\n");

        out.push_str("\n---\n\n*Back to [League Tracker](../index.md)*\n");
        out
    }

    // ── Index page ────────────────────────────────────────────────────────────

    fn render_index(
        &self,
        ranked_teams: &[&str],
        strength: &HashMap<String, f32>,
        max_pts: f32,
    ) -> String {
        let mut out = String::new();
        out.push_str("# NHL Fantasy Tracker — 2025–26\n\n");
        out.push_str(&format!(
            "Real lineup depth charts for all 32 teams. \
             **Ranking:** pts/gp pace (G+A÷GP×{FULL_SEASON}), goals/gp tiebreaker. \
             Team score = top-4 F × 3 positions + top-6 D.\n\n"
        ));
        out.push_str("**Fit:** ★ elite fit · ~ solid · ↑ buried · ↓ overextended\n\n");

        // 2-column grid
        out.push_str("<div class=\"tracker-grid\">\n");
        let half = 16;
        let cols: [&[&str]; 2] = [&ranked_teams[..half], &ranked_teams[half..]];
        for (col_idx, col) in cols.iter().enumerate() {
            out.push_str("<div>\n");
            for (i, abbrev) in col.iter().enumerate() {
                let global_rank = col_idx * half + i + 1;
                let pts = strength.get(*abbrev).copied().unwrap_or(0.0);
                let fill = bar_fill(pts, max_pts, 120);
                let name = team_full_name(abbrev);
                let logo = team_logo_url(abbrev);
                let rank_cls = if global_rank <= 5 {
                    "top5"
                } else if global_rank <= 10 {
                    "top10"
                } else if global_rank >= 28 {
                    "bot5"
                } else {
                    ""
                };

                out.push_str(&format!(
                    r#"<a class="tracker-card" href="teams/{abbrev}.md"><span class="tracker-rank {rank_cls}">#{global_rank}</span><img class="tracker-logo" src="{logo}" alt="{abbrev}"><div class="tracker-bar-wrap"><div class="tracker-bar"><div class="tracker-bar-fill" style="width:{fill}px"></div></div><div style="font-size:0.7rem;color:#6b7280">{name}</div></div><span class="tracker-score">{pts:.0}</span></a>"#
                ));
                out.push('\n');
            }
            out.push_str("</div>\n");
        }
        out.push_str("</div>\n");
        out
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Hart.5b2k: PlayerView analog of the legacy `compute_team_strength`.
/// Same formula (top-4 forwards per slot + top-6 D, summed as f32).
fn compute_team_strength_views(views: &[PlayerView<'_>]) -> HashMap<String, f32> {
    let mut map: HashMap<String, HashMap<Position, Vec<f32>>> = HashMap::new();
    for v in views {
        if let Some(s) = v.pace_score() {
            map.entry(v.team_display().to_owned())
                .or_default()
                .entry(v.position())
                .or_default()
                .push(s.pace_82 as f32);
        }
    }
    map.iter()
        .map(|(team, pos_map)| {
            let team = team.clone();
            let fwd_pts: f32 = [Position::LeftWing, Position::Center, Position::RightWing]
                .iter()
                .map(|pos| {
                    let mut v = pos_map.get(pos).cloned().unwrap_or_default();
                    v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                    v.iter().take(FWD_LINES).sum::<f32>()
                })
                .sum();
            let def_pts: f32 = {
                let mut v = pos_map.get(&Position::Defense).cloned().unwrap_or_default();
                v.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                v.iter().take(DEF_PAIRS * 2).sum::<f32>()
            };
            (team.clone(), fwd_pts + def_pts)
        })
        .collect()
}

fn write_file(path: &Path, content: &str) -> Result<(), SiteError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut f = std::fs::File::create(path)?;
    f.write_all(content.as_bytes())?;
    Ok(())
}
