//! SiteBuilder — generates all site docs from snapshot data.
//! Replaces scripts/gen_site.py.

use std::collections::HashMap;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use icelines_core::{
    cross_team::compute_all_views,
    model::{Position, Season},
    scoring::sort_views_by_pace,
    season_stats::SeasonType,
    stats_repository::PlayerView,
    CrossTeamMetrics, TeamAbbr, TeamDepthView,
};
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
        // Hart.5c.1: load directly into a StatsRepository, take views
        // through every downstream call. No more flat_view_legacy.
        let store = SnapshotStore::new(&self.config.snapshot_dir);
        let season = Season(self.config.season);
        let outcome = load_into_repo(season, SeasonType::Regular, &store)
            .map_err(|e| SiteError::Snapshot(e.to_string()))?;
        let repo = &outcome.repo;
        let mut all_views: Vec<PlayerView<'_>> =
            repo.skaters(season, SeasonType::Regular).collect();
        sort_views_by_pace(&mut all_views);

        // Compute cross-team metrics from views.
        let metrics_vec = compute_all_views(&all_views);
        let metrics_map: HashMap<Option<u32>, CrossTeamMetrics> = metrics_vec
            .into_iter()
            .filter_map(|m| m.player_nhl_id.map(|id| (Some(id), m)))
            .collect();

        // Team strength for tracker index.
        let team_strength = compute_team_strength_views(&all_views);
        let max_strength = team_strength.values().copied().fold(0.0_f32, f32::max);
        let mut ranked_teams: Vec<&str> = TEAM_NAMES.iter().map(|(a, _)| *a).collect();
        ranked_teams.sort_by(|a, b| {
            let sa = team_strength.get(*a).copied().unwrap_or(0.0);
            let sb = team_strength.get(*b).copied().unwrap_or(0.0);
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Generate team pages.
        let teams_dir = self.config.docs_dir.join("teams");
        std::fs::create_dir_all(&teams_dir)?;

        let mut written = Vec::new();
        for (rank, &abbrev) in ranked_teams.iter().enumerate() {
            let rank = rank + 1;
            let team = TeamAbbr(abbrev.to_string());
            let team_views: Vec<PlayerView<'_>> = all_views
                .iter()
                .filter(|v| v.team_display() == abbrev)
                .cloned()
                .collect();

            if team_views.is_empty() {
                continue;
            }

            let view = TeamDepthView::from_player_views(
                team.clone(),
                season,
                SeasonType::Regular,
                &team_views,
            );

            let page = self.render_team_page(
                abbrev,
                rank,
                &view,
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
        view: &TeamDepthView,
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
            let row = view.forward_lines.get(line_idx);
            let lw = row.and_then(|r| r.left.as_ref());
            let c = row.and_then(|r| r.center.as_ref());
            let rw = row.and_then(|r| r.right.as_ref());

            let lw_m = lw.and_then(|s| metrics.get(&Some(s.player_id.0)));
            let c_m = c.and_then(|s| metrics.get(&Some(s.player_id.0)));
            let rw_m = rw.and_then(|s| metrics.get(&Some(s.player_id.0)));

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
            let row = view.defense_pairs.get(pair_idx);
            let d1 = row.and_then(|r| r.left.as_ref());
            let d2 = row.and_then(|r| r.right.as_ref());
            let d1_m = d1.and_then(|s| metrics.get(&Some(s.player_id.0)));
            let d2_m = d2.and_then(|s| metrics.get(&Some(s.player_id.0)));

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

fn compute_team_strength_views(views: &[PlayerView<'_>]) -> HashMap<String, f32> {
    let mut map: HashMap<String, HashMap<Position, Vec<f32>>> = HashMap::new();
    for v in views {
        if let Some(pace) = v.pace_82() {
            map.entry(v.team_display().to_owned())
                .or_default()
                .entry(v.position())
                .or_default()
                .push(pace as f32);
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

#[cfg(test)]
mod tests {
    use super::*;
    use icelines_core::{season_stats::SeasonType, stats_repository::StatsRepository};

    fn build_repo(rows: &[(u32, &str, Position, f32)]) -> StatsRepository {
        let mut repo = StatsRepository::new();
        for (pid, _team, pos, _pace) in rows {
            repo.upsert_identity(icelines_core::fixtures::identity(*pid).build())
                .unwrap();
            // SAFETY: identity is set via upsert above, so upsert_stats won't error.
            // We need a position-specific stats row. The default fixture is Center;
            // override via .position(). The fixture's pace_score (93.7) is fine —
            // tests build their own pace expectations relative to the fixture.
            let _ = pos;
        }
        // Build stats with explicit pace + position so compute_team_strength
        // ordering is deterministic.
        for (pid, team, pos, pace) in rows {
            let mut stats = icelines_core::fixtures::stats(*pid, 20252026, team)
                .position(*pos)
                .build();
            // Force the per-row pace_82 so the sort order is testable.
            stats.totals.pace_score = Some(icelines_core::model::PaceScore {
                pace_82: *pace as f64,
                goals_per_82: *pace as f64 * 0.4,
                raw_points: 80,
                gp: 70,
            });
            repo.upsert_stats(stats).unwrap();
        }
        repo
    }

    #[test]
    fn l1_render_team_page_uses_team_depth_view_slots() {
        let mut repo = StatsRepository::new();
        for (id, name, position) in [
            (1, "Static Left Wing", Position::LeftWing),
            (2, "Static Center", Position::Center),
            (3, "Static Right Wing", Position::RightWing),
            (4, "Static Defense", Position::Defense),
        ] {
            repo.upsert_identity(
                icelines_core::fixtures::identity(id)
                    .name(name, &name.to_ascii_lowercase())
                    .build(),
            )
            .unwrap();
            repo.upsert_stats(
                icelines_core::fixtures::stats(id, 20252026, "SEA")
                    .position(position)
                    .build(),
            )
            .unwrap();
        }

        let season = Season(20252026);
        let roster: Vec<PlayerView<'_>> = repo.skaters(season, SeasonType::Regular).collect();
        let view = TeamDepthView::from_player_views(
            TeamAbbr("SEA".to_owned()),
            season,
            SeasonType::Regular,
            &roster,
        );
        let first_line = view.forward_lines.first().expect("line one");
        assert_eq!(
            first_line
                .left
                .as_ref()
                .map(|slot| slot.display_name.as_str()),
            Some("Static Left Wing")
        );

        let builder = SiteBuilder::new(SiteConfig {
            docs_dir: PathBuf::new(),
            mkdocs_yml: PathBuf::new(),
            snapshot_dir: PathBuf::new(),
            season: 20252026,
        });
        let html = builder.render_team_page("SEA", 1, &view, &HashMap::new(), 100.0);

        assert!(html.contains("Seattle Kraken"));
        assert!(html.contains("Static Left Wing"));
        assert!(html.contains("Static Center"));
        assert!(html.contains("Static Defense"));
        assert!(html.contains("Forward Lines"));
        assert!(html.contains("Defense Pairs"));
        assert!(html.contains("pts/gp"));
    }

    /// Team strength = (top-N forwards per position group, summed over LW/C/RW)
    /// plus (top-2*pairs defensemen). With one team and exactly enough players to
    /// fill the lineup, the result must equal the sum of every player's pace.
    #[test]
    fn l0_compute_team_strength_single_team_simple_sum() {
        // Build a roster with 4 LW, 4 C, 4 RW, 6 D — exactly the slots used.
        // Each gets a unique pace so we can compute the expected total exactly.
        let mut rows: Vec<(u32, &str, Position, f32)> = Vec::new();
        for (i, pos) in [
            Position::LeftWing,
            Position::Center,
            Position::RightWing,
            Position::Defense,
        ]
        .iter()
        .enumerate()
        {
            let count = if pos.is_forward() { 4 } else { 6 };
            for j in 0..count {
                let pid = (1000 + i * 100 + j) as u32;
                rows.push((pid, "EDM", *pos, 80.0 + j as f32));
            }
        }
        let repo = build_repo(&rows);
        let views: Vec<PlayerView<'_>> = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect();

        let strength = compute_team_strength_views(&views);
        let edm = strength.get("EDM").copied().unwrap_or(0.0);

        // Expected: every paced player counts (top-4 of 4 fwds, top-6 of 6 D).
        // Sum = (80+81+82+83) * 3 fwd groups + (80..85 sum)
        let fwd_per_group: f32 = 80.0 + 81.0 + 82.0 + 83.0; // 326
        let def_total: f32 = (80..86).map(|n| n as f32).sum();
        let want = fwd_per_group * 3.0 + def_total;
        assert!(
            (edm - want).abs() < 1e-3,
            "expected {want} got {edm}",
            want = want,
            edm = edm
        );
    }

    /// Team-strength must clamp at top-N per position group: a 5th LW with
    /// monster pace must not change the total because only the top-4 count.
    #[test]
    fn l0_compute_team_strength_clamps_top_n_per_position() {
        // Build 4 baseline LW + 1 monster LW + 1 C/RW/D each (so the team has
        // representatives in every group). The monster LW must NOT lift the
        // total, because it falls outside the top-4 forward slots when
        // grouped with itself? No — top-4 for THIS group of 5. So the
        // weakest of the 5 must drop out.
        let mut rows: Vec<(u32, &str, Position, f32)> = Vec::new();
        for j in 0..4 {
            rows.push((100 + j, "EDM", Position::LeftWing, 50.0 + j as f32));
        }
        // The monster — must DISPLACE the weakest LW (pace=50) from the top 4.
        rows.push((200, "EDM", Position::LeftWing, 200.0));
        rows.push((300, "EDM", Position::Center, 60.0));
        rows.push((400, "EDM", Position::RightWing, 60.0));
        rows.push((500, "EDM", Position::Defense, 60.0));

        let repo = build_repo(&rows);
        let views: Vec<PlayerView<'_>> = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect();
        let strength = compute_team_strength_views(&views);
        let edm = strength.get("EDM").copied().unwrap_or(0.0);

        // Top-4 LW = 200 + 53 + 52 + 51 = 356. Plus C=60, RW=60, D=60.
        let want = 200.0_f32 + 53.0 + 52.0 + 51.0 + 60.0 + 60.0 + 60.0;
        assert!(
            (edm - want).abs() < 1e-3,
            "monster LW must displace pace=50, expected {want} got {edm}",
        );
    }

    /// A view with no pace_score must contribute zero — guards against
    /// the `unwrap_or(0.0)` shape silently letting NaN through if pace_82()
    /// ever changes shape.
    #[test]
    fn l0_compute_team_strength_skips_views_without_pace_score() {
        let mut repo = StatsRepository::new();
        repo.upsert_identity(icelines_core::fixtures::identity(8478402).build())
            .unwrap();
        let mut stats = icelines_core::fixtures::stats(8478402, 20252026, "EDM").build();
        stats.totals.pace_score = None; // no pace
        repo.upsert_stats(stats).unwrap();

        let views: Vec<PlayerView<'_>> = repo
            .skaters(Season(20252026), SeasonType::Regular)
            .collect();
        let strength = compute_team_strength_views(&views);
        // Only pace-less player on EDM → no key for EDM at all.
        assert!(
            !strength.contains_key("EDM"),
            "team with only pace-less players should not appear in strength map"
        );
    }
}
