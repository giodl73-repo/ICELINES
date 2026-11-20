//! Authored readiness challenges attached to verified scenes; never a save bonus or NHL rating.
use super::{
    rinkcraft_contest::{Contest, ExactCounts, Outcome, Resolution, ScoreRule},
    rinkcraft_possession::{self as possession, Phase, State},
    rinkcraft_units as units,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MAX_REPORT_BYTES: usize = 640 * 1024;
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
}
impl Default for Rules {
    fn default() -> Self {
        Self {
            schema: "rinkcraft.goalie-readiness-rules.v1".into(),
            version: "0.1.0-playtest".into(),
        }
    }
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Situation {
    SetAngle,
    MoveAcross,
    Reset,
}
impl Situation {
    fn definition(self) -> (Phase, usize, usize, &'static str, &'static str) {
        match self {
            Self::SetAngle => (Phase::ReadyToShoot,1,4,"Set angle","Controlled receiver before release; Positioning against receiver Reading."),
            Self::MoveAcross => (Phase::ShotInFlight,2,2,"Move across","On-target flight and an explicitly authored lateral-feed context; Movement against receiver Passing."),
            Self::Reset => (Phase::ReboundLoose,4,0,"Reset after save","Failed containment left a loose rebound; Recovery against receiver Skating."),
        }
    }
}
#[derive(Serialize)]
pub struct Opportunity {
    pub situation: Situation,
    pub label: String,
    pub available: bool,
    pub requirement: String,
}
pub fn opportunities(input: &units::Input, phase: Phase) -> Vec<Opportunity> {
    [Situation::SetAngle, Situation::MoveAcross, Situation::Reset]
        .into_iter()
        .map(|s| {
            let (required, _, _, label, requirement) = s.definition();
            Opportunity {
                situation: s,
                label: label.into(),
                available: input.rules.version == "0.4.0-playtest" && phase == required,
                requirement: requirement.into(),
            }
        })
        .collect()
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub rules: Rules,
    pub replay: units::Replay,
    pub situation: Situation,
    pub seed: u32,
    pub lateral_feed_declared: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Readiness {
    AngleSet,
    AngleExposed,
    Across,
    Late,
    Reset,
    Unreset,
    Unresolved,
}
#[derive(Serialize)]
pub struct Assessment {
    pub schema: String,
    pub rules_version: String,
    pub rules_sha256: String,
    pub source_scene_sha256: String,
    pub source_units_sha256: String,
    pub situation: Situation,
    pub seed: u32,
    pub actor: String,
    pub threat: String,
    pub actor_capability: String,
    pub opposition_capability: String,
    pub contest: Contest,
    pub resolution: Resolution,
    pub counts: ExactCounts,
    pub terms: [i16; 2],
    pub readiness: Readiness,
    pub puck_state: State,
    pub note: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub input: Input,
    pub assessment: serde_json::Value,
}
fn hash<T: Serialize>(v: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(v).map_err(|e| e.to_string())?)
    ))
}
pub fn assess(input: &Input) -> Result<Assessment, String> {
    if input.schema != "rinkcraft.goalie-readiness-input.v1"
        || input.rules.schema != Rules::default().schema
        || input.rules.version != Rules::default().version
    {
        return Err(
            "Unsupported goalie readiness input or rules; use readiness 0.1.0-playtest".into(),
        );
    }
    let verified =
        units::import(&serde_json::to_string(&input.replay).map_err(|e| e.to_string())?)?;
    if verified.input.rules.version != "0.4.0-playtest" {
        return Err("Readiness challenges require an explicit 0.4 unit scene".into());
    }
    let scene = units::replay(&verified.input)?;
    let (phase, ai, oi, _, _) = input.situation.definition();
    if scene.scene.state.phase != phase {
        return Err("This scene has no opportunity for the selected readiness task".into());
    }
    if input.lateral_feed_declared != (input.situation == Situation::MoveAcross) {
        return Err("Declare the hypothetical lateral feed only for Move across; scene geometry does not establish it".into());
    }
    let actor = scene
        .scene
        .participants
        .iter()
        .find(|p| p.slot == "away:G")
        .ok_or("Missing away goalie")?;
    let target = verified
        .input
        .request
        .assignments
        .receiver
        .address(units::Side::Home);
    let threat = scene
        .scene
        .participants
        .iter()
        .find(|p| p.slot == target)
        .ok_or("Missing assigned receiver")?;
    let a = &actor.character.capabilities[ai];
    let b = &threat.character.capabilities[oi];
    let contest = Contest {
        actor_score: a.score,
        opposition_score: b.score,
        score_rule: ScoreRule::HalfOffset,
        actor_adjustment: 0,
        opposition_adjustment: 0,
    };
    // A separately declared challenge stream. These are not the next unit-scene dice.
    let mut rng = u64::from(input.seed);
    let resolution = contest.resolve(possession::d20(&mut rng), possession::d20(&mut rng))?;
    let readiness = match (input.situation, resolution.outcome) {
        (_, Outcome::Stalemate) => Readiness::Unresolved,
        (Situation::SetAngle, Outcome::ActorWins) => Readiness::AngleSet,
        (Situation::SetAngle, Outcome::OppositionWins) => Readiness::AngleExposed,
        (Situation::MoveAcross, Outcome::ActorWins) => Readiness::Across,
        (Situation::MoveAcross, Outcome::OppositionWins) => Readiness::Late,
        (Situation::Reset, Outcome::ActorWins) => Readiness::Reset,
        (Situation::Reset, Outcome::OppositionWins) => Readiness::Unreset,
    };
    Ok(Assessment {
        schema:"icelines.goalie-readiness-assessment.v1".into(),rules_version:input.rules.version.clone(),rules_sha256:hash(&input.rules)?,
        source_scene_sha256:hash(&verified)?,source_units_sha256:scene.units_sha256,situation:input.situation,seed:input.seed,
        actor:actor.slot.clone(),threat:threat.slot.clone(),actor_capability:a.label.clone(),opposition_capability:b.label.clone(),
        counts:contest.exact_counts()?,terms:contest.terms()?,contest,resolution,readiness,puck_state:scene.scene.state,
        note:"Authored readiness challenge attached to an immutable 0.4 snapshot. Set angle is pre-release; Move across declares hypothetical lateral context, not observed geometry; Reset follows an uncontrolled saved rebound. This independent seed is not a continuation of unit dice. Readiness awards no save, possession or goal and supplies no bonus or legal-action change to subsequent play. No nearest-player timing, continuous geometry, NHL capability estimate, empirical probability or balance claim.".into(),
    })
}
pub fn export(input: Input) -> Result<Report, String> {
    let assessment = serde_json::to_value(assess(&input)?).map_err(|e| e.to_string())?;
    Ok(Report {
        schema: "rinkcraft.goalie-readiness-report.v1".into(),
        input,
        assessment,
    })
}
pub fn import(raw: &str) -> Result<Report, String> {
    if raw.len() > MAX_REPORT_BYTES {
        return Err("Readiness report exceeds 640 KiB".into());
    }
    let file: Report = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if file.schema != "rinkcraft.goalie-readiness-report.v1" {
        return Err("Unsupported readiness report schema".into());
    }
    let recomputed = export(file.input.clone())?;
    if recomputed.assessment != file.assessment {
        return Err("Readiness report differs from recomputed assessment".into());
    }
    Ok(file)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn input(situation: Situation) -> Input {
        let raw = include_str!("../../tests/fixtures/rinkcraft-unit-v04.json");
        let mut replay = units::import(raw).unwrap();
        replay.input.request.actions.truncate(match situation {
            Situation::SetAngle => 4,
            Situation::MoveAcross => 5,
            Situation::Reset => 7,
        });
        Input {
            schema: "rinkcraft.goalie-readiness-input.v1".into(),
            rules: Rules::default(),
            replay: units::export(replay.input).unwrap(),
            situation,
            seed: 0,
            lateral_feed_declared: situation == Situation::MoveAcross,
        }
    }
    fn score(input: &mut Input, index: usize, score: u8) {
        use super::super::rinkcraft_game as game;
        let m = input
            .replay
            .input
            .request
            .members
            .iter_mut()
            .find(|m| m.side == units::Side::Away && m.slot == units::Slot::G)
            .unwrap();
        let mut scores = std::array::from_fn(|i| m.character.capabilities[i].score);
        scores[index] = score;
        m.character = game::build(
            &input.replay.input.character_rules,
            game::BuildRequest {
                template_id: m.character.template_id.clone(),
                name: m.character.name.clone(),
                position: m.character.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: None,
            },
        )
        .unwrap();
        input.replay = units::export(input.replay.input.clone()).unwrap();
    }
    #[test]
    fn frozen_v04_and_all_three_opportunities_keep_puck_and_replay_exact() {
        let raw = include_str!("../../tests/fixtures/rinkcraft-unit-v04.json");
        assert_eq!(
            serde_json::to_value(units::import(raw).unwrap()).unwrap(),
            serde_json::from_str::<serde_json::Value>(raw).unwrap()
        );
        for situation in [Situation::SetAngle, Situation::MoveAcross, Situation::Reset] {
            let input = input(situation);
            let before = serde_json::to_value(&input.replay).unwrap();
            let a = assess(&input).unwrap();
            assert_eq!(
                serde_json::to_value(&a.puck_state).unwrap(),
                before["session"]["scene"]["state"]
            );
            assert_eq!(serde_json::to_value(&input.replay).unwrap(), before);
            let options = opportunities(&input.replay.input, a.puck_state.phase);
            assert_eq!(options.iter().filter(|o| o.available).count(), 1);
            assert!(
                options
                    .iter()
                    .find(|o| o.situation == situation)
                    .unwrap()
                    .available
            );
            assert_eq!(a.contest.actor_adjustment, 0);
            assert_eq!(a.contest.opposition_adjustment, 0);
            assert_eq!(
                a.counts.actor_wins + a.counts.stalemates + a.counts.opposition_wins,
                400
            );
        }
    }
    #[test]
    fn each_goalie_capability_changes_its_own_readiness_with_identical_dice() {
        for (situation, index) in [
            (Situation::SetAngle, 1),
            (Situation::MoveAcross, 2),
            (Situation::Reset, 4),
        ] {
            let mut low = input(situation);
            let mut high = low.clone();
            score(&mut low, index, 1);
            score(&mut high, index, 20);
            assert_eq!(
                low.replay.session["scene"]["events"],
                high.replay.session["scene"]["events"]
            );
            let mut found = false;
            for seed in 0..64 {
                low.seed = seed;
                high.seed = seed;
                let a = assess(&low).unwrap();
                let b = assess(&high).unwrap();
                assert_eq!(
                    (a.resolution.actor_die, a.resolution.opposition_die),
                    (b.resolution.actor_die, b.resolution.opposition_die)
                );
                if a.resolution.outcome == Outcome::OppositionWins
                    && b.resolution.outcome == Outcome::ActorWins
                {
                    assert_ne!(a.readiness, b.readiness);
                    assert_eq!(b.terms[0] - a.terms[0], 10);
                    assert_eq!(a.puck_state, b.puck_state);
                    assert_ne!(a.source_scene_sha256, b.source_scene_sha256);
                    println!("READINESS_PROBE_{situation:?}_SEED={seed}");
                    found = true;
                    break;
                }
            }
            assert!(found);
        }
    }
    #[test]
    fn unexercised_capabilities_do_not_change_readiness_and_ties_stay_unresolved() {
        for (situation, unused) in [
            (Situation::SetAngle, vec![0, 2, 3, 4, 5]),
            (Situation::MoveAcross, vec![0, 1, 3, 4, 5]),
            (Situation::Reset, vec![1, 2, 5]),
        ] {
            let original = input(situation);
            let a = assess(&original).unwrap();
            let mut changed = original.clone();
            for i in unused {
                score(&mut changed, i, 20);
            }
            let b = assess(&changed).unwrap();
            assert_eq!(a.resolution, b.resolution);
            assert_eq!(a.readiness, b.readiness);
            assert_eq!(a.puck_state, b.puck_state);
            let mut tied = false;
            for seed in 0..256 {
                changed.seed = seed;
                let c = assess(&changed).unwrap();
                if c.resolution.outcome == Outcome::Stalemate {
                    assert_eq!(c.readiness, Readiness::Unresolved);
                    assert_eq!(c.puck_state, b.puck_state);
                    tied = true;
                    break;
                }
            }
            assert!(tied);
        }
    }
    #[test]
    fn wrong_phase_version_and_undeclared_lateral_context_are_rejected() {
        let mut i = input(Situation::SetAngle);
        i.situation = Situation::Reset;
        assert!(assess(&i).is_err());
        let mut i = input(Situation::MoveAcross);
        i.lateral_feed_declared = false;
        assert!(assess(&i).is_err());
        let mut i = input(Situation::SetAngle);
        i.lateral_feed_declared = true;
        assert!(assess(&i).is_err());
        let mut i = input(Situation::SetAngle);
        i.replay.input.rules.version = "0.3.0-playtest".into();
        i.replay = units::export(i.replay.input).unwrap();
        assert!(assess(&i).is_err());
        let mut i = input(Situation::Reset);
        i.rules.version = "future".into();
        assert!(assess(&i).is_err());
    }
    #[test]
    fn reports_recompute_results_source_and_context_and_enforce_bounds() {
        let f = export(input(Situation::Reset)).unwrap();
        let raw = serde_json::to_string(&f).unwrap();
        assert_eq!(
            serde_json::to_value(import(&raw).unwrap()).unwrap(),
            serde_json::to_value(&f).unwrap()
        );
        for field in ["readiness", "source_scene_sha256", "puck_state"] {
            let mut bad = serde_json::to_value(&f).unwrap();
            bad["assessment"][field] = "forged".into();
            assert!(import(&bad.to_string()).is_err());
        }
        let mut bad = serde_json::to_value(&f).unwrap();
        bad["input"]["replay"]["session"]["scene"]["state"]["control"] = "away:G".into();
        assert!(import(&bad.to_string()).is_err());
        let mut bad = serde_json::to_value(&f).unwrap();
        bad["extra"] = "unknown".into();
        assert!(import(&bad.to_string()).is_err());
        assert!(import(&" ".repeat(MAX_REPORT_BYTES + 1)).is_err());
    }
}
