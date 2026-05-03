use crate::identity::PlayerId;
use crate::model::{DepthChart, DepthChartSlot, Position, Season, TeamAbbr, MIN_GP};
use crate::stats_repository::PlayerView;
use std::collections::HashMap;

pub struct DepthChartBuilder;

/// Extract the renderable fields a depth chart cares about from a
/// `PlayerView`. The `team` parameter is provided explicitly so callers
/// (notably `build_views_with_swap`) can override it for a hypothetical
/// trade slot — see that function's rustdoc.
fn slot_from_view(view: &PlayerView<'_>, team: TeamAbbr) -> DepthChartSlot {
    DepthChartSlot {
        player_id: view.identity.id,
        full_name: view.identity.full_name.clone(),
        name_normalized: view.identity.name_normalized.clone(),
        team,
        position: view.position(),
        pace_82: view.pace_82(),
        goals_per_82: view.goals_per_82(),
        gp: Some(view.gp()),
        headshot_canonical_url: view.identity.headshot_canonical_url.clone(),
    }
}

/// Pace_82 sort key matching `crate::scoring::sort_by_pace`: descending
/// pace_82, with `None` sorted last. Used to order players in the
/// greedy slot assignment.
fn pace_sort_key(view: &PlayerView<'_>) -> std::cmp::Reverse<i64> {
    // Multiply by a constant to preserve sub-decimal ordering as i64 for stable Reverse.
    let raw = view.pace_82().unwrap_or(f64::NEG_INFINITY);
    std::cmp::Reverse((raw * 1_000.0) as i64)
}

impl DepthChartBuilder {
    /// Build a depth chart for one team from `PlayerView`s.
    ///
    /// Position assignment is greedy (best-pace-first):
    /// 1. Sort by pace_82 descending (None sorts last).
    /// 2. For each view, assign to the position slot with fewest
    ///    players so far (ties → primary forward position).
    /// 3. Players below MIN_GP → `below_min_gp` list.
    /// 4. Excess players → `unplaced` list.
    ///
    /// Hart.5c.1: replaces the legacy `build(Vec<Player>)` signature.
    /// Multi-position eligibility was never populated on the live path
    /// (see `design/specs/depth-chart.md:197`); the new path uses primary
    /// position only.
    pub fn build_views(team: TeamAbbr, season: Season, views: &[PlayerView<'_>]) -> DepthChart {
        let mut sorted: Vec<&PlayerView<'_>> = views.iter().collect();
        sorted.sort_by_key(|v| pace_sort_key(v));

        let mut fwd_slots: HashMap<Position, Vec<DepthChartSlot>> = HashMap::new();
        let mut def_slots: Vec<DepthChartSlot> = Vec::new();
        let mut below_min: Vec<DepthChartSlot> = Vec::new();
        let mut unplaced: Vec<DepthChartSlot> = Vec::new();

        for view in sorted {
            let slot = slot_from_view(view, team.clone());
            if view.gp() < MIN_GP {
                below_min.push(slot);
                continue;
            }
            let pos = view.position();
            if pos.is_forward() {
                fwd_slots.entry(pos).or_default().push(slot);
            } else if pos.is_defense() {
                def_slots.push(slot);
            } else {
                unplaced.push(slot);
            }
        }

        let none3: [Option<DepthChartSlot>; 3] = [None, None, None];
        let mut forward_lines: Vec<[Option<DepthChartSlot>; 3]> =
            vec![none3.clone(), none3.clone(), none3.clone(), none3];
        let order = [Position::LeftWing, Position::Center, Position::RightWing];
        for (slot_idx, pos) in order.iter().enumerate() {
            let players_at_pos = fwd_slots.remove(pos).unwrap_or_default();
            for (line_idx, slot) in players_at_pos.into_iter().enumerate() {
                if line_idx < 4 {
                    forward_lines[line_idx][slot_idx] = Some(slot);
                } else {
                    unplaced.push(slot);
                }
            }
        }

        let none2: [Option<DepthChartSlot>; 2] = [None, None];
        let mut defense_pairs: Vec<[Option<DepthChartSlot>; 2]> =
            vec![none2.clone(), none2.clone(), none2];
        for (i, slot) in def_slots.into_iter().enumerate() {
            let pair = i / 2;
            let pair_slot = i % 2;
            if pair < 3 {
                defense_pairs[pair][pair_slot] = Some(slot);
            } else {
                unplaced.push(slot);
            }
        }

        DepthChart {
            team,
            season,
            forward_lines,
            defense_pairs,
            unplaced,
            below_min_gp: below_min,
        }
    }

    /// Build a hypothetical depth chart for `team` after swapping
    /// `swap_out_id` out and `swap_in` in.
    ///
    /// **IMPORTANT — hypothetical contract**: the returned chart's slot
    /// for `swap_in` will have `team == <destination team>`, NOT
    /// `swap_in.team()` (which reports the player's actual current team).
    /// Any downstream consumer that joins back to the repo via
    /// `(player_id, team)` will mismatch on the swap-in slot. Renderers
    /// that need ground-truth team membership must read from
    /// `repo.view(slot.player_id, season, season_type).team()`, not from
    /// `slot.team`.
    pub fn build_views_with_swap<'a>(
        team: TeamAbbr,
        season: Season,
        base_views: &[PlayerView<'a>],
        swap_in: PlayerView<'a>,
        swap_out_id: PlayerId,
    ) -> DepthChart {
        let mut filtered: Vec<PlayerView<'a>> = base_views
            .iter()
            .filter(|v| v.identity.id != swap_out_id)
            .cloned()
            .collect();
        filtered.push(swap_in);
        Self::build_views(team, season, &filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;
    use crate::model::{PaceScore, Position};
    use crate::season_stats::{SeasonType, StatTotals, TeamStint};
    use crate::stats_repository::StatsRepository;

    /// Build a small repo with the given (id, position, gp, points) tuples.
    /// Pace = points/gp * 82 → orders default greedy assignment.
    fn build_repo(rows: &[(u32, Position, u32, u32, &str)], season: u32) -> StatsRepository {
        let mut repo = StatsRepository::new();
        for (pid, pos, gp, points, team) in rows {
            repo.upsert_identity(fixtures::identity(*pid).build())
                .unwrap();
            // Custom stats: vary gp/points so pace_82 ordering is meaningful.
            let totals = StatTotals {
                gp: *gp,
                goals: points / 2,
                assists: points - points / 2,
                points: *points,
                plus_minus: 0,
                pim: 0,
                shots: 0,
                shooting_pct: None,
                toi_per_game_sec: None,
                pp_goals: 0,
                pp_points: 0,
                sh_goals: 0,
                sh_points: 0,
                gwg: 0,
                ot_goals: 0,
                faceoff_win_pct: None,
                pace_score: if *gp >= 10 {
                    let pace = (*points as f64) / (*gp as f64) * 82.0;
                    Some(PaceScore {
                        pace_82: pace,
                        goals_per_82: ((*points / 2) as f64) / (*gp as f64) * 82.0,
                        raw_points: *points,
                        gp: *gp,
                    })
                } else {
                    None
                },
            };
            let stint = TeamStint {
                team: TeamAbbr((*team).to_string()),
                started: Some("2024-10-15".into()),
                ended: Some("2025-04-13".into()),
                gp: *gp,
                goals: points / 2,
                assists: points - points / 2,
                points: *points,
                goalie: None,
            };
            let stats = fixtures::stats(*pid, season, team).position(*pos).build();
            // Replace fixture totals + stint with our own.
            let stats = crate::season_stats::SeasonStatsBuilder::new(
                stats.player_id,
                stats.season,
                stats.season_type,
                stats.position,
            )
            .with_totals(totals)
            .add_team_stint(stint)
            .build();
            repo.upsert_stats(stats).unwrap();
        }
        repo
    }

    #[test]
    fn l0_depth_chart_partial_defense_fills_empty_slots() {
        let repo = build_repo(
            &[
                (1, Position::Defense, 75, 25, "SEA"),
                (2, Position::Defense, 72, 19, "SEA"),
                (3, Position::Defense, 60, 13, "SEA"),
                (4, Position::Defense, 55, 10, "SEA"),
            ],
            20242025,
        );
        let views: Vec<PlayerView<'_>> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let chart =
            DepthChartBuilder::build_views(TeamAbbr("SEA".into()), Season(20242025), &views);
        assert!(
            chart.defense_pairs[2][0].is_none(),
            "pair 3 slot 0 should be empty"
        );
        assert!(
            chart.defense_pairs[2][1].is_none(),
            "pair 3 slot 1 should be empty"
        );
        assert!(
            chart.defense_pairs[0][0].is_some(),
            "pair 1 slot 0 should be filled"
        );
    }

    #[test]
    fn l0_depth_chart_gp_zero_goes_to_below_min() {
        let repo = build_repo(
            &[
                (8478402, Position::Center, 50, 30, "SEA"),
                (8479318, Position::RightWing, 0, 0, "SEA"),
            ],
            20242025,
        );
        let views: Vec<PlayerView<'_>> = repo
            .skaters(Season(20242025), SeasonType::Regular)
            .collect();
        let chart =
            DepthChartBuilder::build_views(TeamAbbr("SEA".into()), Season(20242025), &views);
        assert_eq!(chart.below_min_gp.len(), 1);
        assert_eq!(chart.below_min_gp[0].player_id, PlayerId(8479318));
    }

    /// Hart.5c.7: was an adapter-parity test against
    /// `flat_view_legacy → player_from_view`. With the legacy adapter
    /// deleted, this asserts `slot_from_view` directly against the
    /// view's accessors — same field set, same expected values for
    /// the McDavid 2024-25 fixture (default fixtures::stats: pace_82
    /// 93.7, gp 70 from the totals).
    #[test]
    fn l0_slot_from_view_pins_field_mapping_for_known_fixture() {
        let mut repo = StatsRepository::new();
        repo.upsert_identity(fixtures::identity(8478402).build())
            .unwrap();
        repo.upsert_stats(fixtures::stats(8478402, 20242025, "EDM").build())
            .unwrap();

        let view = repo
            .view(PlayerId(8478402), Season(20242025), SeasonType::Regular)
            .unwrap();
        let slot = slot_from_view(&view, view.team().cloned().unwrap());

        assert_eq!(slot.player_id, view.identity.id);
        assert_eq!(slot.full_name, view.full_name());
        assert_eq!(slot.name_normalized, view.identity.name_normalized);
        assert_eq!(slot.team.as_str(), view.team_display());
        assert_eq!(slot.position, view.position());
        assert_eq!(slot.pace_82, view.pace_82());
        assert_eq!(slot.gp, Some(view.gp()));
        assert_eq!(
            slot.headshot_canonical_url,
            view.identity.headshot_canonical_url
        );
    }

    #[test]
    fn l0_hart5c1_build_views_with_swap_destination_team() {
        let repo = build_repo(
            &[
                (1, Position::Center, 70, 70, "EDM"),
                (2, Position::RightWing, 70, 60, "EDM"),
                (3, Position::Center, 70, 80, "TOR"),
            ],
            20242025,
        );
        let edm_views: Vec<PlayerView<'_>> = repo.team_roster(
            &TeamAbbr("EDM".into()),
            Season(20242025),
            SeasonType::Regular,
        );
        let tor_view = repo
            .view(PlayerId(3), Season(20242025), SeasonType::Regular)
            .unwrap();

        let chart = DepthChartBuilder::build_views_with_swap(
            TeamAbbr("EDM".into()),
            Season(20242025),
            &edm_views,
            tor_view,
            PlayerId(2),
        );
        // Player 3's slot must report destination team EDM, not TOR.
        let placed_3 = chart
            .forward_lines
            .iter()
            .flatten()
            .filter_map(|s| s.as_ref())
            .find(|s| s.player_id == PlayerId(3))
            .expect("swap-in player 3 must be on the EDM chart");
        assert_eq!(
            placed_3.team.as_str(),
            "EDM",
            "swap-in slot's team is the destination, not the player's actual team"
        );
        // Player 2 must NOT appear (was swapped out).
        let placed_2 = chart
            .forward_lines
            .iter()
            .flatten()
            .filter_map(|s| s.as_ref())
            .find(|s| s.player_id == PlayerId(2));
        assert!(placed_2.is_none(), "swap-out player 2 must be removed");
    }
}
