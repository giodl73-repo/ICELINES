use std::collections::HashMap;
use icelines_core::name::normalize_name;
use crate::error::FetchError;
use crate::schema::SkaterBio;

/// Candidate player for disambiguation output.
#[derive(Debug, Clone)]
pub struct Candidate {
    pub player_id: u32,
    pub full_name: String,
    pub team:      String,
}

pub struct PlayerResolver {
    /// normalized_name → list of candidates (usually 1, occasionally 2 for name collisions)
    index: HashMap<String, Vec<Candidate>>,
}

impl PlayerResolver {
    /// Build resolver from a list of SkaterBio records.
    pub fn from_bios(bios: &[SkaterBio]) -> Self {
        let mut index: HashMap<String, Vec<Candidate>> = HashMap::new();
        for bio in bios {
            let key = normalize_name(&bio.skater_full_name);
            index.entry(key).or_default().push(Candidate {
                player_id: bio.player_id,
                full_name: bio.skater_full_name.clone(),
                team:      bio.current_team_abbrev.clone(),
            });
        }
        Self { index }
    }

    /// Resolve a player name to an NHL player ID.
    ///
    /// Resolution chain:
    ///   1. Exact normalized match → unique candidate → Ok
    ///   2. Exact normalized match → multiple candidates → try team disambiguation
    ///   3. No match → Err(PlayerNotFound)
    ///   4. Multiple candidates, team ambiguous → Err(NameAmbiguous)
    pub fn resolve(&self, name: &str, team_hint: Option<&str>) -> Result<u32, FetchError> {
        let key = normalize_name(name);
        let candidates = match self.index.get(&key) {
            Some(v) => v,
            None => return Err(FetchError::PlayerNotFound { name: name.to_owned() }),
        };

        if candidates.len() == 1 {
            return Ok(candidates[0].player_id);
        }

        // Multiple candidates — try team-based disambiguation (Sebastian Aho case)
        if let Some(team) = team_hint {
            let team_upper = team.trim().to_uppercase();
            let matching: Vec<&Candidate> = candidates.iter()
                .filter(|c| c.team.to_uppercase() == team_upper)
                .collect();
            if matching.len() == 1 {
                return Ok(matching[0].player_id);
            }
        }

        Err(FetchError::NameAmbiguous {
            name: name.to_owned(),
            candidates: candidates.iter()
                .map(|c| (c.player_id, c.full_name.clone(), c.team.clone()))
                .collect(),
        })
    }

    /// Resolve all names in a batch. Returns (resolved, errors).
    pub fn resolve_batch(&self, names: &[(String, Option<String>)]) -> (Vec<(String, u32)>, Vec<FetchError>) {
        let mut resolved = Vec::new();
        let mut errors   = Vec::new();
        for (name, team) in names {
            match self.resolve(name, team.as_deref()) {
                Ok(id)  => resolved.push((name.clone(), id)),
                Err(e)  => errors.push(e),
            }
        }
        (resolved, errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bio(id: u32, name: &str, team: &str) -> SkaterBio {
        SkaterBio {
            player_id:            id,
            skater_full_name:     name.to_owned(),
            last_name:            name.split_whitespace().last().unwrap_or("").to_owned(),
            games_played:         50,
            goals:                10,
            assists:              20,
            points:               30,
            current_team_abbrev:  team.to_owned(),
            position_code:        "C".to_owned(),
            birth_date:           None,
            birth_country:        None,
            nationality_code:     None,
            shoots_catches:       None,
            draft_year:           None,
            draft_round:          None,
            draft_overall:        None,
            birth_city:           None,
            birth_state_province_code: None,
            height:               None,
            weight:               None,
            first_season_for_game_type: None,
            is_in_hall_of_fame_yn: None,
        }
    }

    #[test]
    fn l1_resolver_slafkovsky_normalizes() {
        // Juraj Slafkovský must resolve via diacritic-stripped name
        let bios = vec![bio(8482078, "Juraj Slafkovský", "MTL")];
        let r = PlayerResolver::from_bios(&bios);
        assert_eq!(r.resolve("Juraj Slafkovský", None).unwrap(), 8482078);
        assert_eq!(r.resolve("Juraj Slafkovsky", None).unwrap(), 8482078);
    }

    #[test]
    fn l1_resolver_sebastian_aho_disambiguates_by_team() {
        // Two players named Sebastian Aho — CAR and SEA
        let bios = vec![
            bio(8478427, "Sebastian Aho", "CAR"),
            bio(8480222, "Sebastian Aho", "SEA"),
        ];
        let r = PlayerResolver::from_bios(&bios);
        // Without team hint → ambiguous
        assert!(r.resolve("Sebastian Aho", None).is_err());
        // With team hint → unique
        assert_eq!(r.resolve("Sebastian Aho", Some("CAR")).unwrap(), 8478427);
        assert_eq!(r.resolve("Sebastian Aho", Some("SEA")).unwrap(), 8480222);
    }

    #[test]
    fn l1_resolver_not_found_returns_error() {
        let bios = vec![bio(1, "Connor McDavid", "EDM")];
        let r = PlayerResolver::from_bios(&bios);
        assert!(r.resolve("Unknown Player", None).is_err());
    }

    #[test]
    fn l1_resolver_exact_match_returns_id() {
        let bios = vec![bio(8480001, "Connor McPlayer", "EDM")];
        let r = PlayerResolver::from_bios(&bios);
        assert_eq!(r.resolve("Connor McPlayer", Some("EDM")).unwrap(), 8480001);
    }
}
