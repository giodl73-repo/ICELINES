use crate::{IcelinesError, Position};

pub struct PositionResolver;

impl PositionResolver {
    /// Parse a Yahoo-format eligible positions string.
    /// Returns (primary, all_eligible).
    /// "C,LW,Util" → (Center, [Center, LeftWing])
    /// "D,Util"    → (Defense, [Defense])
    pub fn parse(eligible_str: &str) -> Result<(Position, Vec<Position>), IcelinesError> {
        const NON_SLOT: &[&str] = &["Util", "IR", "IR+", "BN", "NA", "G"];

        let positions: Vec<Position> = eligible_str
            .split(',')
            .map(str::trim)
            .filter(|s| !NON_SLOT.contains(s) && !s.is_empty())
            .filter_map(Self::parse_single)
            .collect();

        if positions.is_empty() {
            return Err(IcelinesError::InvalidPosition(eligible_str.to_owned()));
        }

        let primary = positions[0];
        Ok((primary, positions))
    }

    /// Parse a single position token. Returns None for unrecognised tokens
    /// (silently skips rather than failing — Yahoo can add new slot types).
    fn parse_single(s: &str) -> Option<Position> {
        match s {
            "C" => Some(Position::Center),
            "LW" => Some(Position::LeftWing),
            "RW" => Some(Position::RightWing),
            "D" => Some(Position::Defense),
            _ => None,
        }
    }

    /// Return only the forward positions from an eligible string.
    pub fn fwd_positions(eligible_str: &str) -> Vec<Position> {
        match Self::parse(eligible_str) {
            Ok((_, all)) => all.into_iter().filter(|p| p.is_forward()).collect(),
            Err(_) => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_position_center_only() {
        let (primary, all) = PositionResolver::parse("C,Util").unwrap();
        assert_eq!(primary, Position::Center);
        assert_eq!(all, vec![Position::Center]);
    }

    #[test]
    fn l0_position_multi_c_lw() {
        // Draisaitl-style: eligible at C and LW
        let (primary, all) = PositionResolver::parse("C,LW,Util").unwrap();
        assert_eq!(primary, Position::Center);
        assert_eq!(all, vec![Position::Center, Position::LeftWing]);
    }

    #[test]
    fn l0_position_defense() {
        let (primary, all) = PositionResolver::parse("D,Util").unwrap();
        assert_eq!(primary, Position::Defense);
        assert_eq!(all, vec![Position::Defense]);
    }

    #[test]
    fn l0_position_fwd_positions_filters_defense() {
        let fwds = PositionResolver::fwd_positions("C,LW,Util");
        assert!(fwds.iter().all(|p| p.is_forward()));
    }

    #[test]
    fn l0_position_empty_string_is_error() {
        assert!(PositionResolver::parse("").is_err());
    }

    #[test]
    fn l0_position_util_only_is_error() {
        assert!(PositionResolver::parse("Util").is_err());
    }

    #[test]
    fn l0_position_goalie_slot_skipped() {
        // G in the eligible string is a goalie slot type, not a position we use
        assert!(PositionResolver::parse("G").is_err());
    }
}
