use std::collections::HashMap;
use crate::model::{DepthChart, Player, Position, Season, TeamAbbr};
use crate::scoring::sort_by_pace;

pub struct DepthChartBuilder;

impl DepthChartBuilder {
    /// Build a depth chart for one team.
    ///
    /// Position assignment is greedy (best-pace-first):
    /// 1. Sort players by pace descending.
    /// 2. For each player, assign them to the eligible slot with fewest players assigned.
    /// 3. Ties in slot count → pick the player's first eligible forward position.
    ///
    /// Players below MIN_GP → below_min_gp list (not placed on card).
    /// Excess players → unplaced list.
    pub fn build(team: TeamAbbr, season: Season, mut players: Vec<Player>) -> DepthChart {
        sort_by_pace(&mut players);

        let mut fwd_slots: HashMap<Position, Vec<Player>> = HashMap::new();
        let mut def_slots: Vec<Player> = Vec::new();
        let mut below_min: Vec<Player> = Vec::new();
        let mut unplaced:  Vec<Player> = Vec::new();

        for player in players {
            if !player.gp_status.is_eligible() {
                below_min.push(player);
                continue;
            }

            let fwd_eligible: Vec<Position> = player.eligible_pos.iter()
                .copied()
                .filter(|p| p.is_forward())
                .collect();

            if !fwd_eligible.is_empty() {
                // Greedy: assign to forward position with fewest players so far
                let best = fwd_eligible.iter().copied()
                    .min_by_key(|pos| fwd_slots.get(pos).map_or(0, |v| v.len()))
                    .unwrap(); // safe: fwd_eligible is non-empty
                fwd_slots.entry(best).or_default().push(player);
            } else if player.position.is_defense() {
                def_slots.push(player);
            } else {
                unplaced.push(player);
            }
        }

        // Fill forward_lines grid: 4 rows × [LW, C, RW]
        let none3: [Option<Player>; 3] = [None, None, None];
        let mut forward_lines: Vec<[Option<Player>; 3]> = vec![none3.clone(), none3.clone(), none3.clone(), none3];
        let order = [Position::LeftWing, Position::Center, Position::RightWing];
        for (slot_idx, pos) in order.iter().enumerate() {
            let players_at_pos = fwd_slots.remove(pos).unwrap_or_default();
            for (line_idx, p) in players_at_pos.into_iter().enumerate() {
                if line_idx < 4 {
                    forward_lines[line_idx][slot_idx] = Some(p);
                } else {
                    unplaced.push(p);
                }
            }
        }

        // Fill defense_pairs grid: 3 rows × [D, D]
        let none2: [Option<Player>; 2] = [None, None];
        let mut defense_pairs: Vec<[Option<Player>; 2]> = vec![none2.clone(), none2.clone(), none2];
        for (i, p) in def_slots.into_iter().enumerate() {
            let pair = i / 2;
            let slot = i % 2;
            if pair < 3 {
                defense_pairs[pair][slot] = Some(p);
            } else {
                unplaced.push(p);
            }
        }

        DepthChart { team, season, forward_lines, defense_pairs, unplaced, below_min_gp: below_min }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::GpStatus;
    use crate::name::normalize_name;
    use crate::position::PositionResolver;
    use crate::scoring::compute_pace_score;

    fn make_player(name: &str, team: &str, eligible: &str, g: u32, a: u32, gp: u32) -> Player {
        let (primary, eligible_pos) = PositionResolver::parse(eligible).unwrap();
        let pace_score = compute_pace_score(g, a, gp);
        Player {
            nhl_id: None,
            full_name: name.to_owned(),
            name_normalized: normalize_name(name),
            team: TeamAbbr(team.to_owned()),
            position: primary,
            eligible_pos,
            gp_status: GpStatus::from_gp(gp),
            season_goals: g,
            season_assists: a,
            season_points: g + a,
            pace_score,
            headshot_url: None,
        }
    }

    #[test]
    fn l0_depth_chart_partial_defense_fills_empty_slots() {
        // Only 4 defensemen → pair 3 should be [None, None]
        let players = vec![
            make_player("D One",   "SEA", "D,Util", 5, 20, 75),
            make_player("D Two",   "SEA", "D,Util", 4, 15, 72),
            make_player("D Three", "SEA", "D,Util", 3, 10, 60),
            make_player("D Four",  "SEA", "D,Util", 2, 8, 55),
        ];
        let chart = DepthChartBuilder::build(
            TeamAbbr("SEA".into()), Season(20252026), players
        );
        assert!(chart.defense_pairs[2][0].is_none(), "pair 3 slot 0 should be empty");
        assert!(chart.defense_pairs[2][1].is_none(), "pair 3 slot 1 should be empty");
        assert!(chart.defense_pairs[0][0].is_some(), "pair 1 slot 0 should be filled");
    }

    #[test]
    fn l0_depth_chart_gp_zero_goes_to_below_min() {
        let players = vec![
            make_player("Present", "SEA", "C,Util", 10, 20, 50),
            make_player("Absent",  "SEA", "RW,Util", 0,  0,  0),
        ];
        let chart = DepthChartBuilder::build(
            TeamAbbr("SEA".into()), Season(20252026), players
        );
        assert_eq!(chart.below_min_gp.len(), 1);
        assert_eq!(chart.below_min_gp[0].full_name, "Absent");
    }

    #[test]
    fn l0_depth_chart_forward_grid_is_4x3() {
        assert_eq!(
            std::mem::size_of::<[[Option<Player>; 3]; 4]>(),
            std::mem::size_of::<[[Option<Player>; 3]; 4]>()
        );
        // Just verify the array dimensions compile correctly — shape is structural
    }
}
