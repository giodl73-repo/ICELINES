//! Curated, source-linked class identity. Never infer a class from statistical bands.
use crate::{identity::PlayerId, model::Season};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const CHARACTER_CATALOG_SCHEMA: &str = "rinkcraft.character-catalog.v1";
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterCatalog {
    pub schema: String,
    pub protocol: String,
    pub classes: Vec<CharacterClass>,
    pub assessments: Vec<CharacterAssessment>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterClass {
    pub id: String,
    pub name: String,
    pub analogy: String,
    pub purpose: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterAssessment {
    pub player_id: PlayerId,
    pub player_name: String,
    pub assessed: String,
    pub primary: ClassAssignment,
    pub secondary: Vec<ClassAssignment>,
    pub specialty: String,
    pub summary: String,
    pub traits: Vec<CharacterFeature>,
    pub features: Vec<CharacterFeature>,
    pub sources: Vec<CharacterSource>,
    pub unknowns: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClassAssignment {
    pub class_id: String,
    pub from_season: Season,
    pub source_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterFeature {
    pub name: String,
    pub detail: String,
    pub evidence_type: String,
    pub from_season: Season,
    pub source_ids: Vec<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSource {
    pub id: String,
    pub title: String,
    pub url: String,
    pub published: String,
    pub season: Season,
    pub locator: String,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CharacterSheet {
    pub schema: String,
    pub protocol: String,
    pub catalog_sha256: String,
    pub player_id: PlayerId,
    pub player_name: String,
    pub assessed: String,
    pub as_of_season: Season,
    pub evidence_through: String,
    pub status: String,
    pub primary: CharacterClass,
    pub secondary: Vec<CharacterClass>,
    pub specialty: String,
    pub summary: String,
    pub traits: Vec<CharacterFeature>,
    pub features: Vec<CharacterFeature>,
    pub sources: Vec<CharacterSource>,
    pub unknowns: Vec<String>,
}
impl CharacterCatalog {
    pub fn validate(&self) -> Result<(), String> {
        use std::collections::HashSet;
        if self.schema != CHARACTER_CATALOG_SCHEMA || self.protocol.is_empty() {
            return Err("unsupported character catalog".into());
        }
        let mut class_ids = HashSet::new();
        for c in &self.classes {
            if !matches!(
                c.id.as_str(),
                "conductor" | "hunter" | "warden" | "vanguard" | "marksman" | "connector"
            ) || c.name.is_empty()
                || !class_ids.insert(&c.id)
            {
                return Err("invalid or duplicate class definition".into());
            }
        }
        let mut players = HashSet::new();
        for a in &self.assessments {
            if !players.insert(a.player_id) || a.player_name.is_empty() {
                return Err("duplicate or unnamed character assessment".into());
            }
            let mut ids = HashSet::new();
            for s in &a.sources {
                if !ids.insert(&s.id)
                    || !s.url.starts_with("https://")
                    || chrono::NaiveDate::parse_from_str(&s.published, "%Y-%m-%d").is_err()
                {
                    return Err("invalid character source".into());
                }
            }
            let supported = |from: Season, refs: &[String]| {
                !refs.is_empty()
                    && refs
                        .iter()
                        .all(|id| a.sources.iter().any(|s| s.id == *id && s.season <= from))
            };
            for c in std::iter::once(&a.primary).chain(&a.secondary) {
                if !class_ids.contains(&c.class_id) || !supported(c.from_season, &c.source_ids) {
                    return Err("unsupported character class assignment".into());
                }
            }
            for f in a.traits.iter().chain(&a.features) {
                if !supported(f.from_season, &f.source_ids) {
                    return Err("unsupported character feature".into());
                }
            }
        }
        Ok(())
    }
    /// Returns no identity before its documentary support. Later features never leak backward.
    pub fn sheet(&self, pid: PlayerId, season: Season) -> Result<Option<CharacterSheet>, String> {
        self.validate()?;
        let Some(a) = self
            .assessments
            .iter()
            .find(|a| a.player_id == pid && a.primary.from_season <= season)
        else {
            return Ok(None);
        };
        let class = |id: &str| {
            self.classes
                .iter()
                .find(|c| c.id == id)
                .cloned()
                .expect("validated class")
        };
        let traits: Vec<_> = a
            .traits
            .iter()
            .filter(|f| f.from_season <= season)
            .cloned()
            .collect();
        let features: Vec<_> = a
            .features
            .iter()
            .filter(|f| f.from_season <= season)
            .cloned()
            .collect();
        let secondaries: Vec<_> = a
            .secondary
            .iter()
            .filter(|c| c.from_season <= season)
            .collect();
        let used: std::collections::HashSet<_> = a
            .primary
            .source_ids
            .iter()
            .chain(secondaries.iter().flat_map(|c| c.source_ids.iter()))
            .chain(
                traits
                    .iter()
                    .chain(&features)
                    .flat_map(|f| f.source_ids.iter()),
            )
            .collect();
        let sources: Vec<_> = a
            .sources
            .iter()
            .filter(|s| s.season <= season && used.contains(&s.id))
            .cloned()
            .collect();
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        Ok(Some(CharacterSheet {
            schema: "rinkcraft.character-sheet.v1".into(),
            protocol: self.protocol.clone(),
            catalog_sha256: format!("{:x}", Sha256::digest(bytes)),
            player_id: pid,
            player_name: a.player_name.clone(),
            assessed: a.assessed.clone(),
            as_of_season: season,
            evidence_through: sources
                .iter()
                .map(|s| s.published.clone())
                .max()
                .unwrap_or_default(),
            status: "provisional_documentary".into(),
            primary: class(&a.primary.class_id),
            secondary: secondaries.iter().map(|c| class(&c.class_id)).collect(),
            specialty: a.specialty.clone(),
            summary: a.summary.clone(),
            traits,
            features,
            sources,
            unknowns: a.unknowns.clone(),
        }))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn catalog() -> CharacterCatalog {
        serde_json::from_value(serde_json::json!({"schema":CHARACTER_CATALOG_SCHEMA,"protocol":"test","classes":[{"id":"conductor","name":"Conductor","analogy":"Wizard","purpose":"Distribution"}],"assessments":[{"player_id":1,"player_name":"Synthetic Player","assessed":"2026-11-08","primary":{"class_id":"conductor","from_season":20232024,"source_ids":["S1"]},"secondary":[],"specialty":"Test","summary":"Authored illustration","traits":[],"features":[{"name":"Later feature","detail":"Test","evidence_type":"attributed","from_season":20252026,"source_ids":["S2"]}],"sources":[{"id":"S1","title":"Synthetic","url":"https://example.org/one","published":"2024-04-03","season":20232024,"locator":"test"},{"id":"S2","title":"Synthetic","url":"https://example.org/two","published":"2026-03-13","season":20252026,"locator":"test"}],"unknowns":[]}]})).unwrap()
    }
    #[test]
    fn documentary_classes_are_scoped_and_future_features_do_not_leak() {
        let c = catalog();
        assert!(c.sheet(PlayerId(1), Season(20222023)).unwrap().is_none());
        let s = c.sheet(PlayerId(1), Season(20232024)).unwrap().unwrap();
        assert!(s.features.is_empty());
        assert_eq!(s.sources.len(), 1);
        assert_eq!(
            c.sheet(PlayerId(1), Season(20252026))
                .unwrap()
                .unwrap()
                .features
                .len(),
            1
        );
    }
    #[test]
    fn missing_support_and_unsafe_source_links_reject() {
        let mut c = catalog();
        c.assessments[0].primary.source_ids = vec!["missing".into()];
        assert!(c.validate().is_err());
        let mut c = catalog();
        c.assessments[0].sources[0].url = "javascript:alert(1)".into();
        assert!(c.validate().is_err());
    }
}
