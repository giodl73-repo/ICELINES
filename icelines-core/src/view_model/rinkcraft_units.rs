//! Explicit fictional units and scene-local duties, separately versioned from legacy saves.
use super::rinkcraft_game::{self as game, Branch, BuildRequest, GameCharacter, GameRules};
use super::rinkcraft_possession::{self as legacy, Action, Participant, SceneBindings, Session};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_INPUT_BYTES: usize = 256 * 1024;
pub const MAX_REPLAY_BYTES: usize = 512 * 1024;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Home,
    Away,
}
impl Side {
    fn label(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Away => "away",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    F1,
    F2,
    F3,
    D1,
    D2,
    G,
}
impl Slot {
    pub const ALL: [Self; 6] = [Self::F1, Self::F2, Self::F3, Self::D1, Self::D2, Self::G];
    fn label(self) -> &'static str {
        match self {
            Self::F1 => "F1",
            Self::F2 => "F2",
            Self::F3 => "F3",
            Self::D1 => "D1",
            Self::D2 => "D2",
            Self::G => "G",
        }
    }
    fn address(self, side: Side) -> String {
        format!("{}:{}", side.label(), self.label())
    }
    pub(crate) fn accepts(self, c: &GameCharacter) -> bool {
        match self {
            Self::G => c.branch == Branch::Goalie && c.position == "G",
            Self::D1 | Self::D2 => {
                c.branch == Branch::Skater && ["D", "LD", "RD"].contains(&c.position.as_str())
            }
            _ => {
                c.branch == Branch::Skater
                    && ["F", "C", "LW", "RW", "L", "R"].contains(&c.position.as_str())
            }
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Member {
    pub id: String,
    pub side: Side,
    pub slot: Slot,
    pub character: GameCharacter,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MemberRecipe {
    pub id: String,
    pub side: Side,
    pub slot: Slot,
    pub build: BuildRequest,
}
/// Transport-only file references are resolved by the caller, never by the model.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Blueprint {
    pub schema: String,
    pub character_rules_path: String,
    pub unit_rules_path: String,
    pub members: Vec<MemberRecipe>,
    pub assignments: Assignments,
    pub hero_id: String,
    pub seed: u32,
    pub commit_support: bool,
    pub actions: Vec<Action>,
}
impl Blueprint {
    pub fn validate_shape(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.unit-blueprint.v1"
            || self.members.len() != 12
            || self.character_rules_path.len() > 1024
            || self.unit_rules_path.len() > 1024
        {
            return Err(
                "Use unit blueprint v1 with twelve recipes and bounded rule references".into(),
            );
        }
        Ok(())
    }
}
pub fn build_input(
    character_rules: GameRules,
    rules: Rules,
    b: Blueprint,
) -> Result<Input, String> {
    b.validate_shape()?;
    rules.validate()?;
    character_rules.validate()?;
    if serde_json::to_vec(&character_rules)
        .map_err(|e| e.to_string())?
        .len()
        > 65536
    {
        return Err("Character rules exceed 64 KiB".into());
    }
    let members = b
        .members
        .into_iter()
        .map(|m| {
            Ok(Member {
                id: m.id,
                side: m.side,
                slot: m.slot,
                character: game::build(&character_rules, m.build)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let input = Input {
        schema: "rinkcraft.unit-possession-input.v1".into(),
        character_rules,
        rules,
        request: Request {
            members,
            assignments: b.assignments,
            hero_id: b.hero_id,
            seed: b.seed,
            commit_support: b.commit_support,
            actions: b.actions,
        },
    };
    replay(&input)?;
    Ok(input)
}
/// Slot addresses belong to this scene only; duties do not follow class or roster position.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Assignments {
    pub carrier: Slot,
    pub receiver: Slot,
    pub high_cover: Slot,
    pub counter_defender: Slot,
    pub away_pressure: Slot,
    pub away_gap: Slot,
    pub away_interceptor: Slot,
}
impl Default for Assignments {
    fn default() -> Self {
        Self {
            carrier: Slot::F1,
            receiver: Slot::F2,
            high_cover: Slot::F3,
            counter_defender: Slot::D2,
            away_pressure: Slot::F1,
            away_gap: Slot::D1,
            away_interceptor: Slot::F2,
        }
    }
}
impl Assignments {
    fn validate(&self) -> Result<(), String> {
        if [
            self.carrier,
            self.receiver,
            self.high_cover,
            self.counter_defender,
            self.away_pressure,
            self.away_gap,
            self.away_interceptor,
        ]
        .contains(&Slot::G)
        {
            return Err("Skater duties cannot be assigned to a goalie".into());
        }
        if self.carrier == self.receiver
            || [self.carrier, self.receiver, self.counter_defender].contains(&self.high_cover)
        {
            return Err(
                "Carrier and receiver must differ; high cover must be a distinct supporting player"
                    .into(),
            );
        }
        Ok(())
    }
    fn bindings(&self) -> SceneBindings {
        SceneBindings {
            carrier: self.carrier.address(Side::Home),
            receiver: self.receiver.address(Side::Home),
            cover: self.high_cover.address(Side::Home),
            counter_defender: self.counter_defender.address(Side::Home),
            pressure: self.away_pressure.address(Side::Away),
            gap: self.away_gap.address(Side::Away),
            interceptor: self.away_interceptor.address(Side::Away),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub schema: String,
    pub version: String,
    pub support_bonus: u8,
}
impl Rules {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema != "rinkcraft.unit-possession-rules.v1"
            || self.version != "0.2.0-playtest"
            || self.support_bonus > 2
        {
            return Err("Use unit possession rules v0.2.0-playtest and support 0–2".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub members: Vec<Member>,
    pub assignments: Assignments,
    pub hero_id: String,
    pub seed: u32,
    pub commit_support: bool,
    pub actions: Vec<Action>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub schema: String,
    pub character_rules: GameRules,
    pub rules: Rules,
    pub request: Request,
}
#[derive(Clone, Debug, Serialize)]
pub struct UnitSession {
    pub schema: String,
    pub units_sha256: String,
    pub hero_id: String,
    pub assignments: Assignments,
    pub scene: Session,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Replay {
    pub schema: String,
    pub input: Input,
    pub session: serde_json::Value,
}

fn hash<T: Serialize>(value: &T) -> Result<String, String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(value).map_err(|e| e.to_string())?)
    ))
}
fn canonical_members(input: &Input) -> Result<Vec<Member>, String> {
    if input.schema != "rinkcraft.unit-possession-input.v1"
        || input.request.members.len() != 12
        || input.request.actions.len() > 16
    {
        return Err("Use unit input v1, exactly twelve members and at most sixteen actions".into());
    }
    input.rules.validate()?;
    input.character_rules.validate()?;
    input.request.assignments.validate()?;
    let character_rules =
        serde_json::to_string(&input.character_rules).map_err(|e| e.to_string())?;
    if character_rules.len() > 65536 {
        return Err("Character rules exceed 64 KiB".into());
    }
    let mut ids = BTreeSet::new();
    let mut addresses = BTreeMap::new();
    for member in &input.request.members {
        if member.id.is_empty()
            || member.id.len() > 64
            || !member
                .id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            || !ids.insert(member.id.clone())
        {
            return Err("Member IDs must be unique ASCII identifiers of 1–64 characters".into());
        }
        if !member.slot.accepts(&member.character) {
            return Err(format!(
                "Member {} has an incompatible roster position/branch for {}",
                member.id,
                member.slot.label()
            ));
        }
        game::import_from_json(
            &character_rules,
            &serde_json::to_string(&member.character).map_err(|e| e.to_string())?,
        )?;
        if addresses
            .insert((member.side, member.slot), member.clone())
            .is_some()
        {
            return Err("A scene slot cannot contain two members".into());
        }
    }
    for side in [Side::Home, Side::Away] {
        for slot in Slot::ALL {
            if !addresses.contains_key(&(side, slot)) {
                return Err("Each side requires F1/F2/F3/D1/D2/G exactly once".into());
            }
        }
    }
    if !input
        .request
        .members
        .iter()
        .any(|m| m.id == input.request.hero_id && m.side == Side::Home)
    {
        return Err("Selected hero must identify a home-unit member".into());
    }
    Ok(addresses.into_values().collect())
}
fn station(m: &Member, a: &Assignments, commit: bool) -> String {
    if m.slot == Slot::G {
        return "crease exchange".into();
    }
    let text = match m.side {
        Side::Home => {
            if m.slot == a.carrier {
                "wall carrier"
            } else if m.slot == a.receiver {
                "middle receiver"
            } else if m.slot == a.high_cover {
                if commit {
                    "outlet support; high cover vacated"
                } else {
                    "high cover"
                }
            } else if m.slot == a.counter_defender {
                "last line"
            } else {
                "unit support"
            }
        }
        Side::Away => {
            if m.slot == a.away_pressure {
                "pressure lane"
            } else if m.slot == a.away_gap {
                "entry defense"
            } else if m.slot == a.away_interceptor {
                "passing lane"
            } else {
                "unit support"
            }
        }
    };
    text.into()
}
pub fn replay(input: &Input) -> Result<UnitSession, String> {
    let members = canonical_members(input)?;
    let q = &input.request;
    let hero = members
        .iter()
        .find(|m| m.id == q.hero_id)
        .ok_or("Missing hero")?;
    let hero_slot = hero.slot.address(hero.side);
    let participants = members
        .iter()
        .map(|m| Participant {
            slot: m.slot.address(m.side),
            station: station(m, &q.assignments, q.commit_support),
            character: m.character.clone(),
        })
        .collect();
    // Arithmetic/state kernel is shared. Rules metadata is replaced with the explicit-unit version below.
    let arithmetic = legacy::Rules {
        schema: "rinkcraft.possession-rules.v1".into(),
        version: "0.1.0-playtest".into(),
        support_bonus: input.rules.support_bonus,
    };
    let request = legacy::Request {
        hero: hero.character.clone(),
        seed: q.seed,
        commit_support: q.commit_support,
        actions: q.actions.clone(),
    };
    let mut scene = legacy::resolve_scene(
        &arithmetic,
        &request,
        hero_slot,
        participants,
        &q.assignments.bindings(),
    )?;
    scene.schema = "icelines.possession-session.v2".into();
    scene.rules_version = input.rules.version.clone();
    scene.rules_sha256 = hash(&input.rules)?;
    scene.note="Fictional explicit-unit outlet/carry/pass scene; named assignments and seeded contests. A received pass is not a shot or goal. No clock, fatigue, continuous movement or NHL probabilities. F1/F2/F3/D1/D2 are local scene addresses; roster positions and phase duties are separate.".into();
    Ok(UnitSession {
        schema: "icelines.unit-possession-session.v1".into(),
        units_sha256: hash(&(&members, &q.assignments))?,
        hero_id: q.hero_id.clone(),
        assignments: q.assignments.clone(),
        scene,
    })
}
pub fn export(input: Input) -> Result<Replay, String> {
    let session = serde_json::to_value(replay(&input)?).map_err(|e| e.to_string())?;
    let file = Replay {
        schema: "rinkcraft.unit-possession-replay.v1".into(),
        input,
        session,
    };
    if serde_json::to_vec(&file).map_err(|e| e.to_string())?.len() > MAX_REPLAY_BYTES {
        return Err("Unit replay exceeds 512 KiB".into());
    }
    Ok(file)
}
pub fn import(raw: &str) -> Result<Replay, String> {
    if raw.len() > MAX_REPLAY_BYTES {
        return Err("Unit replay exceeds 512 KiB".into());
    }
    let file: Replay = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    if file.schema != "rinkcraft.unit-possession-replay.v1" {
        return Err("Unsupported unit replay schema".into());
    }
    if file.session != serde_json::to_value(replay(&file.input)?).map_err(|e| e.to_string())? {
        return Err(
            "Unit replay does not match its roster, assignments, rules, dice or state".into(),
        );
    }
    Ok(file)
}
pub fn from_json(raw: &str) -> Result<String, String> {
    if raw.len() > MAX_INPUT_BYTES {
        return Err("Unit input exceeds 256 KiB".into());
    }
    let input = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    serde_json::to_string(&export(input)?).map_err(|e| e.to_string())
}
/// Existing file remains unchanged; conversion creates an explicitly versioned copy.
pub fn freeze_legacy(raw: &str) -> Result<Input, String> {
    let old = legacy::import_replay(raw)?;
    let session = legacy::replay(&old.character_rules, &old.rules, old.request.clone())?;
    let mut members = Vec::new();
    for side in [Side::Home, Side::Away] {
        for slot in Slot::ALL {
            let p = session
                .participants
                .iter()
                .find(|p| p.slot == slot.address(side))
                .ok_or("Missing legacy slot")?;
            members.push(Member {
                id: p.slot.replace(':', "-"),
                side,
                slot,
                character: p.character.clone(),
            });
        }
    }
    let assignments = Assignments {
        carrier: if session.hero_slot == "home:D1" {
            Slot::D1
        } else {
            Slot::F1
        },
        ..Default::default()
    };
    let input = Input {
        schema: "rinkcraft.unit-possession-input.v1".into(),
        character_rules: old.character_rules,
        rules: Rules {
            schema: "rinkcraft.unit-possession-rules.v1".into(),
            version: "0.2.0-playtest".into(),
            support_bonus: old.rules.support_bonus,
        },
        request: Request {
            members,
            assignments,
            hero_id: session.hero_slot.replace(':', "-"),
            seed: old.request.seed,
            commit_support: old.request.commit_support,
            actions: old.request.actions,
        },
    };
    replay(&input)?;
    Ok(input)
}
pub fn complete(input: &mut Input) -> Result<UnitSession, String> {
    let mut session = replay(input)?;
    while !session.scene.legal_actions.is_empty() {
        if session.scene.legal_actions.len() != 1 {
            return Err("Sole-legal-action policy cannot choose among multiple actions".into());
        }
        input.request.actions.push(session.scene.legal_actions[0]);
        session = replay(input)?;
    }
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn golden() -> Vec<serde_json::Value> {
        serde_json::from_str::<serde_json::Value>(include_str!(
            "../../tests/fixtures/rinkcraft-legacy-possession.json"
        ))
        .unwrap()["cases"]
            .as_array()
            .unwrap()
            .clone()
    }
    fn fixture() -> Input {
        freeze_legacy(&golden()[1].to_string()).unwrap()
    }
    fn set_score(input: &mut Input, slot: Slot, index: usize, score: u8) {
        let m = input
            .request
            .members
            .iter_mut()
            .find(|m| m.side == Side::Home && m.slot == slot)
            .unwrap();
        let mut scores = [12; 6];
        scores[index] = score;
        m.character = game::build(
            &input.character_rules,
            BuildRequest {
                template_id: "s".into(),
                name: m.character.name.clone(),
                position: m.character.position.clone(),
                scores: game::ScoreMethod::Manual { scores },
                inspiration: None,
            },
        )
        .unwrap();
    }
    #[test]
    fn frozen_legacy_files_remain_exact_and_conversions_preserve_events_and_control() {
        for file in golden() {
            let raw = file.to_string();
            let imported = legacy::import_replay(&raw).unwrap();
            assert_eq!(
                serde_json::to_value(
                    legacy::replay(&imported.character_rules, &imported.rules, imported.request)
                        .unwrap()
                )
                .unwrap(),
                file["session"]
            );
            let input = freeze_legacy(&raw).unwrap();
            let new = replay(&input).unwrap();
            assert_eq!(
                serde_json::to_value(new.scene.events).unwrap(),
                file["session"]["events"]
            );
            assert_eq!(
                serde_json::to_value(new.scene.state).unwrap(),
                file["session"]["state"]
            );
            assert!(legacy::import_replay(
                &serde_json::to_string(&export(input).unwrap()).unwrap()
            )
            .is_err());
            assert!(import(&raw).is_err());
        }
    }
    #[test]
    fn canonical_units_are_order_independent_but_bind_duties_and_character_content() {
        let input = fixture();
        let a = replay(&input).unwrap();
        let mut reversed = input.clone();
        reversed.request.members.reverse();
        assert_eq!(
            serde_json::to_value(&a).unwrap(),
            serde_json::to_value(replay(&reversed).unwrap()).unwrap()
        );
        reversed.request.assignments.receiver = Slot::D1;
        let b = replay(&reversed).unwrap();
        assert_ne!(a.units_sha256, b.units_sha256);
        let mut changed = input;
        set_score(&mut changed, Slot::D2, 5, 20);
        assert_ne!(a.units_sha256, replay(&changed).unwrap().units_sha256);
    }
    #[test]
    fn selected_d2_defends_without_becoming_carrier_and_score_changes_counter_only() {
        let mut low = fixture();
        low.request.seed = 26;
        low.request.hero_id = "home-D2".into();
        set_score(&mut low, Slot::D2, 5, 1);
        let mut high = low.clone();
        set_score(&mut high, Slot::D2, 5, 20);
        let a = replay(&low).unwrap();
        let b = replay(&high).unwrap();
        assert_eq!(a.scene.hero_slot, "home:D2");
        assert_eq!(a.scene.events[0].after.control, None);
        assert_eq!(
            serde_json::to_value(&a.scene.events[..2]).unwrap(),
            serde_json::to_value(&b.scene.events[..2]).unwrap()
        );
        assert_eq!(a.scene.events[1].actor, "home:F1");
        assert_eq!(a.scene.state.phase, legacy::Phase::CounterAdvanced);
        assert_eq!(b.scene.state.phase, legacy::Phase::CounterStopped);
        let x = &a.scene.events[2];
        let y = &b.scene.events[2];
        assert_eq!(x.opposition, "home:D2");
        assert_eq!(x.opposition_capability, "Defending");
        assert_eq!(
            (x.resolution.actor_die, x.resolution.opposition_die),
            (y.resolution.actor_die, y.resolution.opposition_die)
        );
        assert_ne!(x.counts, y.counts);
        assert_eq!(y.terms[1] - x.terms[1], 10);
        let mut selected = high.clone();
        selected.request.hero_id = "home-F1".into();
        let c = replay(&selected).unwrap();
        assert_eq!(
            serde_json::to_value(b.scene.events).unwrap(),
            serde_json::to_value(c.scene.events).unwrap()
        );
    }
    #[test]
    fn duties_change_actor_and_receiver_while_forward_can_cover_a_defender() {
        let mut input = freeze_legacy(&golden()[0].to_string()).unwrap();
        input.request.assignments.carrier = Slot::D1;
        input.request.assignments.receiver = Slot::F3;
        input.request.assignments.high_cover = Slot::F2;
        let s = replay(&input).unwrap();
        assert_eq!(s.scene.events[1].actor, "home:D1");
        assert_eq!(s.scene.events[2].after.control.as_deref(), Some("home:F3"));
        assert_eq!(s.scene.state.cover_owner.as_deref(), Some("home:F2"));
        input.request.assignments.away_pressure = Slot::D2;
        input.request.assignments.away_gap = Slot::F1;
        input.request.assignments.away_interceptor = Slot::D1;
        let s = replay(&input).unwrap();
        assert_eq!(s.scene.events[0].opposition, "away:D2");
        assert_eq!(s.scene.events[1].opposition, "away:F1");
        assert_eq!(s.scene.events[2].opposition, "away:D1");
        input.request.assignments.counter_defender = Slot::F1;
        input.request.actions.clear();
        assert!(replay(&input).is_ok());
    }
    #[test]
    fn invalid_rosters_heroes_duties_tampering_and_bounds_are_rejected() {
        let input = fixture();
        let mut q = input.clone();
        q.request.members.pop();
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.members[1].id = q.request.members[0].id.clone();
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.members[1].slot = q.request.members[0].slot;
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.members[0].slot = Slot::G;
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.hero_id = "away-D2".into();
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.assignments.high_cover = q.request.assignments.counter_defender;
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.assignments.away_gap = Slot::G;
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.members[0].character.capabilities[0].score = 20;
        assert!(replay(&q).is_err());
        q = input.clone();
        q.rules.version = "0.1.0-playtest".into();
        assert!(replay(&q).is_err());
        q = input.clone();
        q.request.actions = vec![Action::Outlet; 17];
        assert!(replay(&q).is_err());
        let file = export(input).unwrap();
        let raw = serde_json::to_string(&file).unwrap();
        assert!(import(&raw).is_ok());
        let mut bad = serde_json::to_value(file).unwrap();
        bad["session"]["scene"]["state"]["control"] = serde_json::json!("home:G");
        assert!(import(&bad.to_string()).is_err());
        assert!(from_json(&"x".repeat(MAX_INPUT_BYTES + 1)).is_err());
        assert!(import(&"x".repeat(MAX_REPLAY_BYTES + 1)).is_err());
        assert!(from_json("{}").is_err());
        let mut unknown = serde_json::to_value(fixture()).unwrap();
        unknown["request"]["automatic_class_bonus"] = serde_json::json!(2);
        assert!(from_json(&unknown.to_string()).is_err());
    }
    #[test]
    fn complete_follows_only_legal_actions_and_selected_goalie_remains_a_goalie() {
        let mut input = fixture();
        input.request.actions.clear();
        input.request.hero_id = "home-G".into();
        let s = complete(&mut input).unwrap();
        assert!(s.scene.legal_actions.is_empty());
        assert_eq!(s.scene.hero_slot, "home:G");
        assert_eq!(s.scene.events[0].actor_capability, "Puck Play");
        assert!(input.request.actions.len() <= 16);
        let file = export(input).unwrap();
        assert_eq!(
            import(&serde_json::to_string(&file).unwrap())
                .unwrap()
                .session,
            file.session
        );
    }
}
