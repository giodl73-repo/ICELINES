//! Exact arithmetic probe for fictional game design, not NHL event probabilities.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScoreRule {
    Direct,
    HalfOffset,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Contest {
    pub actor_score: u8,
    pub opposition_score: u8,
    pub score_rule: ScoreRule,
    pub actor_adjustment: i8,
    pub opposition_adjustment: i8,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    ActorWins,
    Stalemate,
    OppositionWins,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Resolution {
    pub actor_die: u8,
    pub opposition_die: u8,
    pub actor_total: i16,
    pub opposition_total: i16,
    pub margin: i16,
    pub outcome: Outcome,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct ExactCounts {
    pub combinations: u16,
    pub actor_wins: u16,
    pub stalemates: u16,
    pub opposition_wins: u16,
}
impl Contest {
    fn validate(self) -> Result<(), String> {
        if !(1..=20).contains(&self.actor_score)
            || !(1..=20).contains(&self.opposition_score)
            || !(-20..=20).contains(&self.actor_adjustment)
            || !(-20..=20).contains(&self.opposition_adjustment)
        {
            return Err("Fictional scores must be 1–20 and adjustments -20–20".into());
        }
        Ok(())
    }
    pub fn terms(self) -> Result<[i16; 2], String> {
        self.validate()?;
        let score = |x: u8| match self.score_rule {
            ScoreRule::Direct => i16::from(x),
            ScoreRule::HalfOffset => (i16::from(x) - 10).div_euclid(2),
        };
        Ok([
            score(self.actor_score) + i16::from(self.actor_adjustment),
            score(self.opposition_score) + i16::from(self.opposition_adjustment),
        ])
    }
    pub fn resolve(self, actor_die: u8, opposition_die: u8) -> Result<Resolution, String> {
        if !(1..=20).contains(&actor_die) || !(1..=20).contains(&opposition_die) {
            return Err("Supply two d20 results in 1–20".into());
        }
        let [a, b] = self.terms()?;
        let actor_total = a + i16::from(actor_die);
        let opposition_total = b + i16::from(opposition_die);
        let margin = actor_total - opposition_total;
        let outcome = match margin.cmp(&0) {
            std::cmp::Ordering::Greater => Outcome::ActorWins,
            std::cmp::Ordering::Equal => Outcome::Stalemate,
            std::cmp::Ordering::Less => Outcome::OppositionWins,
        };
        Ok(Resolution {
            actor_die,
            opposition_die,
            actor_total,
            opposition_total,
            margin,
            outcome,
        })
    }
    /// Exhaust all 400 equally likely pairs. No RNG, seed or sampling interval.
    pub fn exact_counts(self) -> Result<ExactCounts, String> {
        self.validate()?;
        let mut result = ExactCounts {
            combinations: 400,
            actor_wins: 0,
            stalemates: 0,
            opposition_wins: 0,
        };
        for a in 1..=20 {
            for b in 1..=20 {
                match self.resolve(a, b)?.outcome {
                    Outcome::ActorWins => result.actor_wins += 1,
                    Outcome::Stalemate => result.stalemates += 1,
                    Outcome::OppositionWins => result.opposition_wins += 1,
                }
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contest(a: u8, b: u8, rule: ScoreRule) -> Contest {
        Contest {
            actor_score: a,
            opposition_score: b,
            score_rule: rule,
            actor_adjustment: 0,
            opposition_adjustment: 0,
        }
    }
    #[test]
    fn equal_scores_have_a_hand_counted_diagonal() {
        for rule in [ScoreRule::Direct, ScoreRule::HalfOffset] {
            assert_eq!(
                contest(12, 12, rule).exact_counts().unwrap(),
                ExactCounts {
                    combinations: 400,
                    actor_wins: 190,
                    stalemates: 20,
                    opposition_wins: 190
                }
            );
        }
    }
    #[test]
    fn floor_is_correct_below_ten_and_twenty_is_not_an_auto_win() {
        assert_eq!(
            contest(1, 9, ScoreRule::HalfOffset).terms().unwrap(),
            [-5, -1]
        );
        let result = contest(1, 20, ScoreRule::Direct).resolve(20, 1).unwrap();
        assert_eq!(result.outcome, Outcome::Stalemate);
    }
    #[test]
    fn support_changes_odds_but_does_not_validate_a_chemistry_bonus() {
        let mut c = contest(12, 12, ScoreRule::HalfOffset);
        c.actor_adjustment = 2;
        assert_eq!(
            c.exact_counts().unwrap(),
            ExactCounts {
                combinations: 400,
                actor_wins: 229,
                stalemates: 18,
                opposition_wins: 153
            }
        );
    }
    #[test]
    fn extremes_reveal_the_difference_between_candidate_rules() {
        assert_eq!(
            contest(20, 1, ScoreRule::Direct)
                .exact_counts()
                .unwrap()
                .actor_wins,
            399
        );
        assert_eq!(
            contest(20, 1, ScoreRule::HalfOffset)
                .exact_counts()
                .unwrap()
                .actor_wins,
            345
        );
    }
    #[test]
    fn all_scores_conserve_mass_reverse_symmetry_and_are_monotone() {
        for rule in [ScoreRule::Direct, ScoreRule::HalfOffset] {
            for b in 1..=20 {
                let mut previous = 0;
                for a in 1..=20 {
                    let c = contest(a, b, rule).exact_counts().unwrap();
                    let reverse = contest(b, a, rule).exact_counts().unwrap();
                    assert_eq!(c.actor_wins + c.stalemates + c.opposition_wins, 400);
                    assert_eq!(c.actor_wins, reverse.opposition_wins);
                    assert!(c.actor_wins >= previous);
                    previous = c.actor_wins;
                }
            }
        }
    }
    #[test]
    fn invalid_scores_adjustments_dice_and_empirical_fields_are_rejected() {
        assert!(contest(0, 10, ScoreRule::Direct).exact_counts().is_err());
        assert!(contest(10, 21, ScoreRule::Direct).exact_counts().is_err());
        let mut c = contest(10, 10, ScoreRule::Direct);
        c.actor_adjustment = 21;
        assert!(c.exact_counts().is_err());
        assert!(contest(10, 10, ScoreRule::Direct).resolve(0, 20).is_err());
        assert!(serde_json::from_str::<Contest>(r#"{"actor_score":10,"opposition_score":10,"score_rule":"direct","actor_adjustment":0,"opposition_adjustment":0,"nhl_probability":0.5}"#).is_err());
    }
}
