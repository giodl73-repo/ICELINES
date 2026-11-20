//! Paired-seed experiments under the existing scene's sole-legal-action policy.
//! Fictional endpoint distributions, never NHL probabilities or an overall utility ranking.
use super::rinkcraft_game::{self as game, GameCharacter, GameRules};
use super::rinkcraft_possession::{replay, Action, Phase, Request, Rules, Session};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const MAX_INPUT_BYTES: usize = 128 * 1024;
pub const MAX_TRIALS: u16 = 4096;
pub const ENDPOINTS: [Phase; 5] = [
    Phase::Complete,
    Phase::Stalemate,
    Phase::CounterAdvanced,
    Phase::CounterStopped,
    Phase::WindowEnded,
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExperimentRequest {
    pub schema: String,
    pub character_rules: GameRules,
    pub rules: Rules,
    pub hero: GameCharacter,
    pub master_seed: u32,
    pub trials: u16,
}
#[derive(Clone, Debug, Serialize)]
pub struct Trial {
    pub seed: u32,
    pub hold_phase: Phase,
    pub commit_phase: Phase,
    pub hold_actions: Vec<Action>,
    pub commit_actions: Vec<Action>,
    pub first_action_difference: Option<usize>,
}
#[derive(Clone, Debug, Serialize)]
pub struct EndpointDelta {
    pub phase: Phase,
    pub hold_count: u32,
    pub commit_count: u32,
    pub commit_minus_hold_count: i32,
    pub difference_percentage_points: f64,
    pub paired_mc_se_percentage_points: f64,
}
#[derive(Clone, Debug, Serialize)]
pub struct Exposure {
    pub label: String,
    pub hold_uses: u32,
    pub commit_uses: u32,
}
#[derive(Clone, Debug, Serialize)]
pub struct Experiment {
    pub schema: String,
    pub request_sha256: String,
    pub request: ExperimentRequest,
    pub policy: String,
    pub coupling: String,
    pub sampling: String,
    pub endpoint_order: [Phase; 5],
    /// Rows hold, columns commit, in endpoint_order. No ordinal utility implied.
    pub paired_endpoint_counts: [[u32; 5]; 5],
    pub endpoints: Vec<EndpointDelta>,
    pub different_terminal_phase_count: u32,
    pub different_action_path_count: u32,
    pub action_exposure: Vec<Exposure>,
    pub hero_capability_exposure: Vec<Exposure>,
    pub trials: Vec<Trial>,
    pub interpretation: String,
}

fn play(q: &ExperimentRequest, seed: u32, commit_support: bool) -> Result<Session, String> {
    let mut request = Request {
        hero: q.hero.clone(),
        seed,
        commit_support,
        actions: vec![],
    };
    let mut session = replay(&q.character_rules, &q.rules, request.clone())?;
    while !session.legal_actions.is_empty() {
        if session.legal_actions.len() != 1 || request.actions.len() >= 16 {
            return Err(
                "Sole-legal-action policy requires exactly one legal action and at most 16 rolls"
                    .into(),
            );
        }
        request.actions.push(session.legal_actions[0]);
        session = replay(&q.character_rules, &q.rules, request.clone())?;
    }
    Ok(session)
}

fn paired_se(n: u32, positive: u32, negative: u32) -> f64 {
    // Sample variance of paired indicator differences in {-1,0,1}, not two independent rates.
    let n = f64::from(n);
    let sum = f64::from(positive) - f64::from(negative);
    let variance = ((f64::from(positive + negative) - sum * sum / n) / (n - 1.)).max(0.);
    let finite_population = (4294967296. - n) / 4294967295.;
    (variance / n * finite_population).sqrt() * 100.
}

pub fn run(q: ExperimentRequest) -> Result<Experiment, String> {
    if q.schema != "rinkcraft.possession-experiment-request.v1"
        || !(2..=MAX_TRIALS).contains(&q.trials)
    {
        return Err("Use experiment request v1 and 2–4096 paired trials".into());
    }
    // Validate before generating the seed schedule, including the exported hero's provenance.
    play(&q, 0, false)?;
    let request_sha256 = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&q).map_err(|e| e.to_string())?)
    );
    let mut rng = u64::from(q.master_seed);
    let mut seeds = BTreeSet::new();
    let mut matrix = [[0u32; 5]; 5];
    let actions = [
        Action::Outlet,
        Action::Carry,
        Action::Pass,
        Action::Recover,
        Action::Counter,
    ];
    let mut action_exposure: Vec<_> = actions
        .iter()
        .map(|x| Exposure {
            label: format!("{x:?}").to_lowercase(),
            hold_uses: 0,
            commit_uses: 0,
        })
        .collect();
    let mut hero_capability_exposure: Vec<_> = q
        .hero
        .capabilities
        .iter()
        .map(|x| Exposure {
            label: x.label.clone(),
            hold_uses: 0,
            commit_uses: 0,
        })
        .collect();
    let mut trials = Vec::new();
    let mut different_action_path_count = 0;
    for _ in 0..q.trials {
        let seed = loop {
            let next = game::random_word(&mut rng);
            if seeds.insert(next) {
                break next;
            }
        };
        let hold = play(&q, seed, false)?;
        let commit = play(&q, seed, true)?;
        let a = ENDPOINTS
            .iter()
            .position(|x| *x == hold.state.phase)
            .ok_or("Nonterminal hold endpoint")?;
        let b = ENDPOINTS
            .iter()
            .position(|x| *x == commit.state.phase)
            .ok_or("Nonterminal commit endpoint")?;
        matrix[a][b] += 1;
        for (session, is_commit) in [(&hold, false), (&commit, true)] {
            for event in &session.events {
                let index = actions
                    .iter()
                    .position(|x| *x == event.action)
                    .ok_or("Unknown action")?;
                let exposure = &mut action_exposure[index];
                if is_commit {
                    exposure.commit_uses += 1
                } else {
                    exposure.hold_uses += 1
                }
                for (slot, capability) in [
                    (&event.actor, &event.actor_capability),
                    (&event.opposition, &event.opposition_capability),
                ] {
                    if slot == &session.hero_slot {
                        let exposure = hero_capability_exposure
                            .iter_mut()
                            .find(|x| &x.label == capability)
                            .ok_or("Unknown hero capability")?;
                        if is_commit {
                            exposure.commit_uses += 1
                        } else {
                            exposure.hold_uses += 1
                        }
                    }
                }
            }
        }
        let hold_actions: Vec<_> = hold.events.iter().map(|e| e.action).collect();
        let commit_actions: Vec<_> = commit.events.iter().map(|e| e.action).collect();
        let first_action_difference = (0..hold_actions.len().max(commit_actions.len()))
            .find(|i| hold_actions.get(*i) != commit_actions.get(*i))
            .map(|i| i + 1);
        different_action_path_count += u32::from(first_action_difference.is_some());
        trials.push(Trial {
            seed,
            hold_phase: hold.state.phase,
            commit_phase: commit.state.phase,
            hold_actions,
            commit_actions,
            first_action_difference,
        });
    }
    let endpoints = ENDPOINTS
        .iter()
        .enumerate()
        .map(|(index, phase)| {
            let hold_count = matrix[index].iter().sum();
            let commit_count = matrix.iter().map(|row| row[index]).sum();
            let difference = commit_count as i32 - hold_count as i32;
            EndpointDelta {
                phase: *phase,
                hold_count,
                commit_count,
                commit_minus_hold_count: difference,
                difference_percentage_points: 100. * f64::from(difference) / f64::from(q.trials),
                paired_mc_se_percentage_points: paired_se(
                    u32::from(q.trials),
                    commit_count - matrix[index][index],
                    hold_count - matrix[index][index],
                ),
            }
        })
        .collect();
    let same: u32 = (0..5).map(|i| matrix[i][i]).sum();
    Ok(Experiment {
        schema:"icelines.possession-plan-experiment.v1".into(),request_sha256,policy:"sole_legal_action_v1: at each state take its unique legal action; fail if multiple choices appear; stop at the existing terminal state or 16-roll cap".into(),
        coupling:"The same seed resets each plan's PRNG. Dice are coupled by roll ordinal, not semantic event: after paths diverge, the same ordinal may resolve different actions. Each plan follows its own legal path.".into(),
        sampling:"SplitMix64 high-u32 seed schedule from master_seed, rejecting duplicates. Paired MC SE approximates uniform sampling without replacement from 2^32 seeds; it describes simulation precision under that PRNG assumption, not model or NHL uncertainty. Zero observed discordance does not prove equal population rates.".into(),
        endpoint_order:ENDPOINTS,paired_endpoint_counts:matrix,endpoints,different_terminal_phase_count:u32::from(q.trials)-same,different_action_path_count,action_exposure,hero_capability_exposure,trials,request:q,
        interpretation:"Counts describe this fictional character, fixed generated units, authored rules and declared response policy. Completion is a received pass, not a shot or goal. Counter advance/stoppage, ties and the roll cap remain separate endpoints; no overall utility or strategy winner is assigned. No full-match, balance or empirical hockey inference.".into(),
    })
}

pub fn run_from_json(raw: &str) -> Result<String, String> {
    if raw.len() > MAX_INPUT_BYTES {
        return Err("Experiment request exceeds 128 KiB".into());
    }
    let q = serde_json::from_str(raw).map_err(|e| format!("Invalid experiment request: {e}"))?;
    serde_json::to_string(&run(q)?).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::view_model::rinkcraft_game::{build, BuildRequest, ScoreMethod};
    fn fixture() -> ExperimentRequest {
        let templates:Vec<_>=[("s","skater"),("g","goalie")].iter().map(|(id,branch)|serde_json::json!({"id":id,"branch":branch,"class_id":"test","class_name":"Test","specialty":"Test","description":"Fictional fixture","priority":[0,1,2,3,4,5],"task_skills":[],"methods":[],"tendencies":[],"playbook_options":[]})).collect();
        let character_rules:GameRules=serde_json::from_value(serde_json::json!({"schema":"rinkcraft.game-rules.v1","version":"test","title":"Test","standard_array":[12,12,12,12,12,12],"templates":templates})).unwrap();
        let hero = build(
            &character_rules,
            BuildRequest {
                template_id: "s".into(),
                name: "Hero".into(),
                position: "F".into(),
                scores: ScoreMethod::StandardArray,
                inspiration: None,
            },
        )
        .unwrap();
        ExperimentRequest {
            schema: "rinkcraft.possession-experiment-request.v1".into(),
            character_rules,
            rules: Rules {
                schema: "rinkcraft.possession-rules.v1".into(),
                version: "0.1.0-playtest".into(),
                support_bonus: 2,
            },
            hero,
            master_seed: 123,
            trials: 64,
        }
    }
    #[test]
    fn paired_schedule_reproduces_and_marginals_conserve_all_trials() {
        let q = fixture();
        let result = run(q.clone()).unwrap();
        let repeated = run(q).unwrap();
        assert_eq!(
            serde_json::to_string(&result).unwrap(),
            serde_json::to_string(&repeated).unwrap()
        );
        assert_eq!(
            result
                .trials
                .iter()
                .map(|x| x.seed)
                .collect::<BTreeSet<_>>()
                .len(),
            64
        );
        assert_eq!(
            result.paired_endpoint_counts.iter().flatten().sum::<u32>(),
            64
        );
        assert_eq!(
            result.endpoints.iter().map(|x| x.hold_count).sum::<u32>(),
            64
        );
        assert_eq!(
            result.endpoints.iter().map(|x| x.commit_count).sum::<u32>(),
            64
        );
        assert_eq!(
            result
                .endpoints
                .iter()
                .map(|x| x.commit_minus_hold_count)
                .sum::<i32>(),
            0
        );
        assert!(result.different_action_path_count > 0);
        for trial in &result.trials {
            assert!(trial.hold_actions.len() <= 16 && trial.commit_actions.len() <= 16)
        }
    }
    #[test]
    fn zero_bonus_is_a_paired_negative_control_and_uses_only_active_hero_attributes() {
        let mut q = fixture();
        q.rules.support_bonus = 0;
        let result = run(q.clone()).unwrap();
        let mut changed = q;
        changed.hero = build(
            &changed.character_rules,
            BuildRequest {
                template_id: "s".into(),
                name: "Hero".into(),
                position: "F".into(),
                scores: ScoreMethod::Manual {
                    scores: [12, 1, 12, 20, 12, 1],
                },
                inspiration: None,
            },
        )
        .unwrap();
        let changed = run(changed).unwrap();
        assert_eq!(
            serde_json::to_string(&result.trials).unwrap(),
            serde_json::to_string(&changed.trials).unwrap()
        );
        assert_eq!(result.different_terminal_phase_count, 0);
        assert_eq!(result.different_action_path_count, 0);
        assert!(result.endpoints.iter().all(
            |x| x.difference_percentage_points == 0. && x.paired_mc_se_percentage_points == 0.
        ));
        for label in ["Handling", "Shooting", "Defending"] {
            let x = result
                .hero_capability_exposure
                .iter()
                .find(|x| x.label == label)
                .unwrap();
            assert_eq!((x.hold_uses, x.commit_uses), (0, 0))
        }
        assert!(result
            .hero_capability_exposure
            .iter()
            .any(|x| x.label == "Reading" && x.hold_uses > 0));
    }
    #[test]
    fn stored_paths_replay_to_their_reported_endpoints_and_dice_pair_only_by_ordinal() {
        let q = fixture();
        let result = run(q.clone()).unwrap();
        for trial in &result.trials {
            let mut paired = Vec::new();
            for (commit_support, actions, phase) in [
                (false, &trial.hold_actions, trial.hold_phase),
                (true, &trial.commit_actions, trial.commit_phase),
            ] {
                let s = replay(
                    &q.character_rules,
                    &q.rules,
                    Request {
                        hero: q.hero.clone(),
                        seed: trial.seed,
                        commit_support,
                        actions: actions.clone(),
                    },
                )
                .unwrap();
                assert!(s.legal_actions.is_empty());
                assert_eq!(s.state.phase, phase);
                paired.push(s);
            }
            for (a, b) in paired[0].events.iter().zip(&paired[1].events) {
                assert_eq!(
                    (a.resolution.actor_die, a.resolution.opposition_die),
                    (b.resolution.actor_die, b.resolution.opposition_die)
                );
            }
        }
    }
    #[test]
    fn paired_se_is_hand_checked_and_not_an_independent_rate_error() {
        assert_eq!(paired_se(4, 0, 0), 0.);
        let expected = (1.0f64 / 3.).sqrt() * 100. * ((4294967296.0f64 - 4.) / 4294967295.).sqrt();
        assert!((paired_se(4, 2, 2) - expected).abs() < 1e-10);
        assert!((paired_se(4, 1, 0) - 25.).abs() < 1e-6);
    }
    #[test]
    fn bounds_unknowns_and_tampered_characters_fail_before_sampling() {
        let mut q = fixture();
        q.trials = 1;
        assert!(run(q.clone()).is_err());
        q.trials = 4097;
        assert!(run(q.clone()).is_err());
        q.trials = 2;
        q.hero.capabilities[0].score = 20;
        assert!(run(q).is_err());
        let mut value = serde_json::to_value(fixture()).unwrap();
        value["utility"] = serde_json::json!(10);
        assert!(run_from_json(&value.to_string()).is_err());
        assert!(run_from_json(&"x".repeat(MAX_INPUT_BYTES + 1)).is_err());
        assert!(run_from_json("{}").is_err());
    }
}
