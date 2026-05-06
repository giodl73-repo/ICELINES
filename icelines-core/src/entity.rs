//! Phase Foster — `EntityRef`, the canonical "what kind of thing
//! and which one" handle.
//!
//! Stringly-typed everywhere: JSON envelopes, SQLite TEXT columns,
//! URL query params, CLI args. The wire form is the `Display`
//! output (`player:8478402` / `team:EDM` / `game:2025020001`); serde
//! delegates to `Display`/`FromStr` so reading and writing always
//! round-trip through one canonical representation.
//!
//! Why stringly-typed: SQLite `TEXT` round-trips without a per-row
//! JSON parse cost, URLs `%3A`-encode cleanly, and there is no
//! split between JSON-shape and SQL-shape that drifts as schemas
//! evolve. The `LeagueAbbrev` newtype in `career_history` is the
//! same pattern — proven through Calder.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use thiserror::Error;

use crate::identity::{GameId, PlayerId};
use crate::model::TeamAbbr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntityRef {
    Player(PlayerId),
    Team(TeamAbbr),
    Game(GameId),
}

impl EntityRef {
    /// Tag string (`"player"` / `"team"` / `"game"`) — used by
    /// EventStream's `entity_kind` column and the migration 006
    /// backfill.
    pub fn kind_str(&self) -> &'static str {
        match self {
            Self::Player(_) => "player",
            Self::Team(_) => "team",
            Self::Game(_) => "game",
        }
    }

    /// Key string — the part after the colon. Stable across
    /// `Display`/`FromStr` round-trips.
    pub fn key_str(&self) -> String {
        match self {
            Self::Player(id) => id.0.to_string(),
            Self::Team(abbr) => abbr.0.clone(),
            Self::Game(id) => id.0.to_string(),
        }
    }
}

impl fmt::Display for EntityRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Player(id) => write!(f, "player:{}", id.0),
            Self::Team(abbr) => write!(f, "team:{}", abbr.0),
            Self::Game(id) => write!(f, "game:{}", id.0),
        }
    }
}

impl FromStr for EntityRef {
    type Err = EntityRefError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (kind, key) = s
            .split_once(':')
            .ok_or_else(|| EntityRefError::Malformed(s.to_string()))?;
        if key.is_empty() {
            return Err(EntityRefError::Malformed(s.to_string()));
        }
        if !key.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(EntityRefError::Malformed(s.to_string()));
        }
        match kind {
            "player" => {
                let id: u32 = key.parse().map_err(|_| EntityRefError::BadKey {
                    kind: "player",
                    key: key.to_string(),
                    reason: "expected unsigned integer".into(),
                })?;
                Ok(Self::Player(PlayerId(id)))
            }
            "team" => {
                if !(2..=4).contains(&key.len()) || !key.chars().all(|c| c.is_ascii_uppercase()) {
                    return Err(EntityRefError::BadKey {
                        kind: "team",
                        key: key.to_string(),
                        reason: "expected 2-4 uppercase ASCII letters".into(),
                    });
                }
                Ok(Self::Team(TeamAbbr(key.to_string())))
            }
            "game" => {
                let id: u64 = key.parse().map_err(|_| EntityRefError::BadKey {
                    kind: "game",
                    key: key.to_string(),
                    reason: "expected unsigned integer".into(),
                })?;
                Ok(Self::Game(GameId(id)))
            }
            other => Err(EntityRefError::UnknownKind(other.to_string())),
        }
    }
}

impl Serialize for EntityRef {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for EntityRef {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = String::deserialize(de)?;
        EntityRef::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum EntityRefError {
    #[error("malformed entity ref '{0}' — expected 'player:ID' / 'team:ABBR' / 'game:ID'")]
    Malformed(String),
    #[error("unknown entity kind '{0}' — expected one of: player, team, game")]
    UnknownKind(String),
    #[error("invalid {kind} key '{key}': {reason}")]
    BadKey {
        kind: &'static str,
        key: String,
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_foster01_player_round_trip() {
        let r = EntityRef::Player(PlayerId(8478402));
        assert_eq!(r.to_string(), "player:8478402");
        assert_eq!(EntityRef::from_str("player:8478402").unwrap(), r);
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"player:8478402\"");
        assert_eq!(serde_json::from_str::<EntityRef>(&json).unwrap(), r);
    }

    #[test]
    fn l0_foster01_team_round_trip() {
        let r = EntityRef::Team(TeamAbbr("EDM".into()));
        assert_eq!(r.to_string(), "team:EDM");
        assert_eq!(EntityRef::from_str("team:EDM").unwrap(), r);
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"team:EDM\"");
        assert_eq!(serde_json::from_str::<EntityRef>(&json).unwrap(), r);
    }

    #[test]
    fn l0_foster01_game_round_trip() {
        let r = EntityRef::Game(GameId(2025020001));
        assert_eq!(r.to_string(), "game:2025020001");
        assert_eq!(EntityRef::from_str("game:2025020001").unwrap(), r);
        let json = serde_json::to_string(&r).unwrap();
        assert_eq!(json, "\"game:2025020001\"");
        assert_eq!(serde_json::from_str::<EntityRef>(&json).unwrap(), r);
    }

    #[test]
    fn l0_foster01_hash_equality_across_kinds() {
        use std::collections::HashSet;
        let a = EntityRef::Player(PlayerId(1));
        let b = EntityRef::Team(TeamAbbr("EDM".into()));
        let c = EntityRef::Game(GameId(1));
        let mut set = HashSet::new();
        set.insert(a.clone());
        set.insert(b.clone());
        set.insert(c.clone());
        // 3 distinct entries; same-key but-different-kind don't collide.
        assert_eq!(set.len(), 3);
        // Re-inserting clones is idempotent (Hash + Eq align with PartialEq).
        set.insert(a);
        set.insert(b);
        set.insert(c);
        assert_eq!(set.len(), 3);
    }

    #[test]
    fn l0_foster01_malformed_no_colon() {
        let err = EntityRef::from_str("player8478402").unwrap_err();
        assert!(matches!(err, EntityRefError::Malformed(_)));
    }

    #[test]
    fn l0_foster01_malformed_empty_key() {
        let err = EntityRef::from_str("player:").unwrap_err();
        assert!(matches!(err, EntityRefError::Malformed(_)));
    }

    #[test]
    fn l0_foster01_unknown_kind() {
        let err = EntityRef::from_str("coach:8478402").unwrap_err();
        assert_eq!(err, EntityRefError::UnknownKind("coach".into()));
    }

    #[test]
    fn l0_foster01_bad_player_key_non_numeric() {
        let err = EntityRef::from_str("player:McDavid").unwrap_err();
        match err {
            EntityRefError::BadKey { kind, .. } => assert_eq!(kind, "player"),
            other => panic!("expected BadKey, got {other:?}"),
        }
    }

    #[test]
    fn l0_foster01_bad_team_key_lowercase() {
        let err = EntityRef::from_str("team:edm").unwrap_err();
        match err {
            EntityRefError::BadKey { kind, .. } => assert_eq!(kind, "team"),
            other => panic!("expected BadKey, got {other:?}"),
        }
    }

    #[test]
    fn l0_foster01_kind_str_matches_display() {
        assert_eq!(EntityRef::Player(PlayerId(1)).kind_str(), "player");
        assert_eq!(EntityRef::Team(TeamAbbr("EDM".into())).kind_str(), "team");
        assert_eq!(EntityRef::Game(GameId(1)).kind_str(), "game");
    }

    #[test]
    fn l0_foster01_url_safe_no_percent_encoding_needed() {
        // `:` is reserved in URLs but allowed in path/query unencoded
        // per RFC 3986. The key portion is alphanumeric — confirms we
        // can hand the Display form straight to URL-builder code
        // without escaping.
        for r in [
            EntityRef::Player(PlayerId(8478402)),
            EntityRef::Team(TeamAbbr("EDM".into())),
            EntityRef::Game(GameId(2025020001)),
        ] {
            let s = r.to_string();
            for c in s.chars() {
                assert!(
                    c.is_ascii_alphanumeric() || c == ':',
                    "EntityRef {s:?} has non-URL-safe char {c:?}"
                );
            }
        }
    }
}
