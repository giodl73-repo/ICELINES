//! Fictional Rinkcraft characters. Never an NHL capability assessment.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const RULES_SCHEMA: &str = "rinkcraft.game-rules.v1";
pub const CHARACTER_SCHEMA: &str = "icelines.game-character.v1";
pub const SKATER: [&str; 6] = [
    "Skating",
    "Handling",
    "Passing",
    "Shooting",
    "Reading",
    "Defending",
];
pub const GOALIE: [&str; 6] = [
    "Tracking",
    "Positioning",
    "Movement",
    "Rebounds",
    "Recovery",
    "Puck Play",
];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Branch {
    Skater,
    Goalie,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameRules {
    pub schema: String,
    pub version: String,
    pub title: String,
    pub standard_array: [u8; 6],
    pub templates: Vec<GameTemplate>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameTemplate {
    pub id: String,
    pub branch: Branch,
    pub class_id: String,
    pub class_name: String,
    pub specialty: String,
    pub description: String,
    pub priority: [usize; 6],
    pub task_skills: Vec<String>,
    pub methods: Vec<String>,
    pub tendencies: Vec<String>,
    pub playbook_options: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ScoreMethod {
    StandardArray,
    Roll { seed: u32 },
    Manual { scores: [u8; 6] },
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inspiration {
    pub player_id: u32,
    pub player_name: String,
    pub evidence_season: u32,
    pub documentary_catalog_sha256: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildRequest {
    pub template_id: String,
    pub name: String,
    pub position: String,
    pub scores: ScoreMethod,
    pub inspiration: Option<Inspiration>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub label: String,
    pub score: u8,
    pub source: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameCharacter {
    pub schema: String,
    pub profile_kind: String,
    pub rules_version: String,
    pub rules_sha256: String,
    pub template_id: String,
    pub name: String,
    pub position: String,
    pub branch: Branch,
    pub class_id: String,
    pub class_name: String,
    pub specialty: String,
    pub description: String,
    pub capabilities: Vec<Capability>,
    pub task_skills: Vec<String>,
    pub methods: Vec<String>,
    pub tendencies: Vec<String>,
    pub playbook_options: Vec<String>,
    pub score_method: ScoreMethod,
    pub dice: Vec<[u8; 4]>,
    pub rolled_totals: Vec<u8>,
    pub inspiration: Option<Inspiration>,
    pub assessment_status: String,
}
fn text(value: &str, limit: usize) -> bool {
    !value.trim().is_empty()
        && value.chars().count() <= limit
        && !value.chars().any(char::is_control)
}
fn scores_valid(values: &[u8; 6]) -> bool {
    values.iter().all(|x| (1..=20).contains(x))
}
impl GameRules {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != RULES_SCHEMA
            || !text(&self.version, 80)
            || !text(&self.title, 120)
            || !scores_valid(&self.standard_array)
            || self.templates.is_empty()
            || self.templates.len() > 64
        {
            return Err("Invalid game rules header or standard array".into());
        }
        let mut ids = BTreeSet::new();
        for t in &self.templates {
            if !text(&t.id, 64)
                || !ids.insert(&t.id)
                || !text(&t.class_id, 64)
                || !text(&t.class_name, 40)
                || !text(&t.specialty, 60)
                || !text(&t.description, 300)
                || t.priority.iter().copied().collect::<BTreeSet<_>>() != (0..6).collect()
            {
                return Err("Invalid/duplicate game template or capability priority".into());
            }
            for values in [
                &t.task_skills,
                &t.methods,
                &t.tendencies,
                &t.playbook_options,
            ] {
                if values.len() > 12 || values.iter().any(|x| !text(x, 80)) {
                    return Err("Invalid game repertoire".into());
                }
            }
        }
        Ok(())
    }
}
pub(crate) fn random_word(state: &mut u64) -> u32 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    ((z ^ (z >> 31)) >> 32) as u32
}
fn die(state: &mut u64) -> u8 {
    let limit = u32::MAX - u32::MAX % 6;
    loop {
        let n = random_word(state);
        if n < limit {
            return (n % 6 + 1) as u8;
        }
    }
}
pub fn build(rules: &GameRules, request: BuildRequest) -> Result<GameCharacter, String> {
    rules.validate()?;
    let template = rules
        .templates
        .iter()
        .find(|t| t.id == request.template_id)
        .ok_or("Unknown game template")?;
    let position = request.position.as_str();
    if !text(&request.name, 80)
        || !["F", "C", "LW", "RW", "L", "R", "D", "LD", "RD", "G"].contains(&position)
        || (position == "G") != (template.branch == Branch::Goalie)
    {
        return Err("Name, position or template branch is invalid".into());
    }
    if let Some(i) = &request.inspiration {
        if i.player_id == 0
            || !text(&i.player_name, 100)
            || i.evidence_season / 10000 + 1 != i.evidence_season % 10000
            || i.evidence_season / 10000 < 1900
            || i.evidence_season / 10000 > 2200
            || i.documentary_catalog_sha256.as_ref().is_some_and(|s| {
                s.len() != 64
                    || !s
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
            })
        {
            return Err("Invalid inspiration provenance".into());
        }
    }
    let mut dice = Vec::new();
    let mut totals = Vec::new();
    let values = match &request.scores {
        ScoreMethod::Manual { scores } => {
            if !scores_valid(scores) {
                return Err("Game scores must be 1–20".into());
            }
            *scores
        }
        method => {
            let mut sorted = match method {
                ScoreMethod::StandardArray => rules.standard_array,
                ScoreMethod::Roll { seed } => {
                    let mut state = u64::from(*seed);
                    let mut rolled = [0; 6];
                    for score in &mut rolled {
                        let d = [
                            die(&mut state),
                            die(&mut state),
                            die(&mut state),
                            die(&mut state),
                        ];
                        *score = d.iter().sum::<u8>() - d.iter().min().unwrap();
                        dice.push(d);
                        totals.push(*score);
                    }
                    rolled
                }
                _ => unreachable!(),
            };
            sorted.sort_by(|a, b| b.cmp(a));
            let mut assigned = [0; 6];
            for (i, target) in template.priority.iter().enumerate() {
                assigned[*target] = sorted[i];
            }
            assigned
        }
    };
    let labels = if template.branch == Branch::Goalie {
        GOALIE
    } else {
        SKATER
    };
    let capabilities = labels
        .iter()
        .enumerate()
        .map(|(i, label)| Capability {
            id: label.to_lowercase().replace(' ', "_"),
            label: (*label).into(),
            score: values[i],
            source: "game_rules_not_nhl_assessment".into(),
        })
        .collect();
    let canonical = serde_json::to_vec(rules).map_err(|e| e.to_string())?;
    Ok(GameCharacter {
        schema: CHARACTER_SCHEMA.into(),
        profile_kind: "fictional_game_character".into(),
        rules_version: rules.version.clone(),
        rules_sha256: format!("{:x}", Sha256::digest(canonical)),
        template_id: template.id.clone(),
        name: request.name,
        position: request.position,
        branch: template.branch.clone(),
        class_id: template.class_id.clone(),
        class_name: template.class_name.clone(),
        specialty: template.specialty.clone(),
        description: template.description.clone(),
        capabilities,
        task_skills: template.task_skills.clone(),
        methods: template.methods.clone(),
        tendencies: template.tendencies.clone(),
        playbook_options: template.playbook_options.clone(),
        score_method: request.scores,
        dice,
        rolled_totals: totals,
        inspiration: request.inspiration,
        assessment_status:
            "Authored game build; not measured NHL abilities or a validated simulation model."
                .into(),
    })
}
pub fn build_from_json(rules: &str, request: &str) -> Result<String, String> {
    if rules.len() > 64 * 1024 || request.len() > 8 * 1024 {
        return Err("Game input too large".into());
    }
    let rules: GameRules = serde_json::from_str(rules).map_err(|e| e.to_string())?;
    let request: BuildRequest = serde_json::from_str(request).map_err(|e| e.to_string())?;
    serde_json::to_string(&build(&rules, request)?).map_err(|e| e.to_string())
}
pub fn import_from_json(rules_json: &str, character_json: &str) -> Result<String, String> {
    if rules_json.len() > 64 * 1024 || character_json.len() > 32 * 1024 {
        return Err("Game import too large".into());
    }
    let rules: GameRules = serde_json::from_str(rules_json).map_err(|e| e.to_string())?;
    let imported: GameCharacter =
        serde_json::from_str(character_json).map_err(|e| e.to_string())?;
    let rebuilt = build(
        &rules,
        BuildRequest {
            template_id: imported.template_id.clone(),
            name: imported.name.clone(),
            position: imported.position.clone(),
            scores: imported.score_method.clone(),
            inspiration: imported.inspiration.clone(),
        },
    )?;
    if imported != rebuilt {
        return Err(
            "Game profile does not match its rules, method or provenance; edit using manual mode"
                .into(),
        );
    }
    serde_json::to_string(&rebuilt).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn rules() -> GameRules {
        GameRules {
            schema: RULES_SCHEMA.into(),
            version: "test".into(),
            title: "Neutral game rules".into(),
            standard_array: [17, 16, 15, 13, 11, 9],
            templates: vec![GameTemplate {
                id: "test".into(),
                branch: Branch::Skater,
                class_id: "test".into(),
                class_name: "Test".into(),
                specialty: "Route reader".into(),
                description: "A fictional test build.".into(),
                priority: [2, 4, 1, 0, 5, 3],
                task_skills: vec!["Outlet".into()],
                methods: vec!["Quick exchange".into()],
                tendencies: vec![],
                playbook_options: vec!["High support".into()],
            }],
        }
    }
    fn request(scores: ScoreMethod) -> BuildRequest {
        BuildRequest {
            template_id: "test".into(),
            name: "Fictional skater".into(),
            position: "F".into(),
            scores,
            inspiration: None,
        }
    }
    #[test]
    fn priorities_differentiate_without_measuring_a_player() {
        let p = build(&rules(), request(ScoreMethod::StandardArray)).unwrap();
        assert_eq!(
            p.capabilities.iter().map(|a| a.score).collect::<Vec<_>>(),
            vec![13, 15, 17, 9, 16, 11]
        );
        assert!(p
            .capabilities
            .iter()
            .all(|a| a.source == "game_rules_not_nhl_assessment"));
        assert_eq!(p.profile_kind, "fictional_game_character");
    }
    #[test]
    fn seeded_rolls_replay_and_drop_one_minimum() {
        let a = build(&rules(), request(ScoreMethod::Roll { seed: 42 })).unwrap();
        let b = build(&rules(), request(ScoreMethod::Roll { seed: 42 })).unwrap();
        assert_eq!(a.dice, b.dice);
        assert_eq!(a.capabilities, b.capabilities);
        assert_eq!(a.dice.len(), 6);
        for (d, total) in a.dice.iter().zip(&a.rolled_totals) {
            assert!(d.iter().all(|x| (1..=6).contains(x)));
            assert_eq!(*total, d.iter().sum::<u8>() - d.iter().min().unwrap());
        }
        let c = build(&rules(), request(ScoreMethod::Roll { seed: 43 })).unwrap();
        assert_ne!(a.dice, c.dice);
    }
    #[test]
    fn manual_values_are_not_reordered_and_bounds_are_real() {
        let p = build(
            &rules(),
            request(ScoreMethod::Manual {
                scores: [1, 2, 3, 18, 19, 20],
            }),
        )
        .unwrap();
        assert_eq!(p.capabilities[0].score, 1);
        assert!(build(&rules(), request(ScoreMethod::Manual { scores: [0; 6] })).is_err());
        assert!(build(&rules(), request(ScoreMethod::Manual { scores: [21; 6] })).is_err());
    }
    #[test]
    fn branches_and_labels_never_mix() {
        let mut r = rules();
        r.templates[0].branch = Branch::Goalie;
        assert!(build(&r, request(ScoreMethod::StandardArray)).is_err());
        let mut req = request(ScoreMethod::StandardArray);
        req.position = "G".into();
        let p = build(&r, req).unwrap();
        assert_eq!(p.capabilities[0].label, "Tracking");
        assert_eq!(p.capabilities[5].label, "Puck Play");
    }
    #[test]
    fn invalid_priorities_and_schema_are_rejected() {
        let mut r = rules();
        r.templates[0].priority = [0; 6];
        assert!(r.validate().is_err());
        r = rules();
        r.templates.push(r.templates[0].clone());
        assert!(r.validate().is_err());
        r = rules();
        r.schema = "rinkcraft.character-catalog.v1".into();
        assert!(r.validate().is_err());
    }
    #[test]
    fn inspiration_is_provenance_not_rating_input() {
        let mut r = request(ScoreMethod::Roll { seed: 42 });
        let a = build(&rules(), r.clone()).unwrap();
        r.inspiration = Some(Inspiration {
            player_id: 1,
            player_name: "Inspiration only".into(),
            evidence_season: 20252026,
            documentary_catalog_sha256: None,
        });
        let b = build(&rules(), r).unwrap();
        assert_eq!(a.capabilities, b.capabilities);
        assert!(b.inspiration.is_some());
    }
    #[test]
    fn exported_profiles_roundtrip_and_reject_rating_tampering() {
        let rules_json = serde_json::to_string(&rules()).unwrap();
        let p = build(&rules(), request(ScoreMethod::Roll { seed: 42 })).unwrap();
        let data = serde_json::to_string(&p).unwrap();
        assert_eq!(import_from_json(&rules_json, &data).unwrap(), data);
        let mut tampered = serde_json::to_value(&p).unwrap();
        tampered["capabilities"][0]["score"] = serde_json::json!(20);
        assert!(import_from_json(&rules_json, &tampered.to_string()).is_err());
        let mut changed = rules();
        changed.version = "different".into();
        assert!(import_from_json(&serde_json::to_string(&changed).unwrap(), &data).is_err());
    }
    #[test]
    fn json_boundary_rejects_unknown_and_oversized_input() {
        let rules_json = serde_json::to_string(&rules()).unwrap();
        let req = serde_json::to_value(request(ScoreMethod::StandardArray)).unwrap();
        let mut invalid = req.clone();
        invalid["verified_nhl_rating"] = serde_json::json!(true);
        assert!(build_from_json(&rules_json, &invalid.to_string()).is_err());
        assert!(build_from_json(&"x".repeat(65537), &req.to_string()).is_err());
        let game = build_from_json(&rules_json, &req.to_string()).unwrap();
        assert!(
            serde_json::from_str::<crate::view_model::rinkcraft_character::CharacterCatalog>(&game)
                .is_err()
        );
    }
}
