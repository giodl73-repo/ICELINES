//! Phase Hart — normalized player identity.
//!
//! `PlayerIdentity` holds the once-per-player-ever facts: name, bio,
//! canonical headshot. Per-season facts (position, sweater, team) live
//! on `SeasonStats` instead. This module is added in Hart.1 and stays
//! parallel to the legacy flat `model::Player` until Hart.5 deletes
//! the old shape.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Stable NHL player ID. The natural primary key — unique across
/// trades, retirements, name changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PlayerId(pub u32);

impl std::fmt::Display for PlayerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerBio {
    #[serde(default)]
    pub birth_date: Option<String>,
    #[serde(default)]
    pub birth_country: Option<String>,
    #[serde(default)]
    pub nationality_code: Option<String>,
    #[serde(default)]
    pub height_in_inches: Option<u32>,
    #[serde(default)]
    pub weight_lbs: Option<u32>,
    #[serde(default)]
    pub draft_year: Option<u16>,
    #[serde(default)]
    pub draft_round: Option<u8>,
    #[serde(default)]
    pub draft_overall: Option<u16>,
    #[serde(default)]
    pub shoots_catches: Option<String>,
    /// First NHL season ever, in `YYYYZZZZ` form (e.g. "20212022").
    /// Immutable once set — used for reissue detection.
    #[serde(default)]
    pub rookie_season: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerIdentity {
    pub id: PlayerId,
    pub full_name: String,
    pub name_normalized: String,
    /// Stable canonical CDN headshot
    /// (`assets.nhle.com/mugs/nhl/default/{nhl_id}.png`). Season-agnostic.
    /// Per-season+team URLs are computed at render time from
    /// `SeasonStats.team_stints.last()`.
    #[serde(default)]
    pub headshot_canonical_url: Option<String>,
    pub bio: PlayerBio,
}

const WEIGHT_FLOOR_LBS: u32 = 100;
const WEIGHT_CEIL_LBS: u32 = 350;
const HEIGHT_FLOOR_IN: u32 = 60;
const HEIGHT_CEIL_IN: u32 = 84;

#[derive(Debug, Error, PartialEq)]
pub enum IdentityMergeError {
    #[error(
        "likely PlayerId reissue: {id} prior_rookie={prior_rookie_season:?} \
         incoming={incoming_rookie_season:?}"
    )]
    LikelyIdReissue {
        id: PlayerId,
        prior_rookie_season: Option<String>,
        incoming_rookie_season: Option<String>,
    },
}

impl PlayerIdentity {
    /// Merge a freshly-parsed identity over the persisted one.
    ///
    /// Policy: most-recent-non-null-wins, with sanity floors. Birth dates,
    /// draft details, and rookie season are immutable after first capture
    /// (later API rows that contradict them are rejected, not applied).
    /// Out-of-range height/weight readings are dropped — keep prior.
    /// A `rookie_season` mismatch with a prior non-null value triggers
    /// `LikelyIdReissue`: the loader treats this as a hard error rather
    /// than silently overwriting an established player's identity.
    pub fn merge_with(&mut self, incoming: PlayerIdentity) -> Result<(), IdentityMergeError> {
        // Reissue check first — runs before any field mutation so a
        // rejected merge leaves `self` unchanged.
        if let (Some(prior), Some(new)) = (
            self.bio.rookie_season.as_deref(),
            incoming.bio.rookie_season.as_deref(),
        ) {
            if prior != new {
                return Err(IdentityMergeError::LikelyIdReissue {
                    id: self.id,
                    prior_rookie_season: Some(prior.to_string()),
                    incoming_rookie_season: Some(new.to_string()),
                });
            }
        }

        let PlayerIdentity {
            id: _,
            full_name,
            name_normalized,
            headshot_canonical_url,
            bio,
        } = incoming;

        // Names + headshot: most-recent-wins (these are intentionally
        // mutable — name changes, CDN moves).
        if !full_name.is_empty() {
            self.full_name = full_name;
        }
        if !name_normalized.is_empty() {
            self.name_normalized = name_normalized;
        }
        if headshot_canonical_url.is_some() {
            self.headshot_canonical_url = headshot_canonical_url;
        }

        // Bio: immutable fields keep prior; mutable-with-floor fields
        // accept new only when within range.
        let PlayerBio {
            birth_date,
            birth_country,
            nationality_code,
            height_in_inches,
            weight_lbs,
            draft_year,
            draft_round,
            draft_overall,
            shoots_catches,
            rookie_season,
        } = bio;

        // Birth date is immutable. Only set if prior is None.
        if self.bio.birth_date.is_none() {
            self.bio.birth_date = birth_date;
        }

        // Birth country / nationality / shoots_catches: most-recent-non-null-wins
        // for country and nationality (rare but legit corrections); shoots
        // is naturally immutable but we just keep prior on mismatch
        // rather than emit a runtime warning (Hart.3 wraps with tracing).
        if birth_country.is_some() {
            self.bio.birth_country = birth_country;
        }
        if nationality_code.is_some() {
            self.bio.nationality_code = nationality_code;
        }
        if self.bio.shoots_catches.is_none() {
            self.bio.shoots_catches = shoots_catches;
        }

        // Height / weight: accept only if within plausible NHL range.
        if let Some(h) = height_in_inches {
            if (HEIGHT_FLOOR_IN..=HEIGHT_CEIL_IN).contains(&h) {
                self.bio.height_in_inches = Some(h);
            }
        }
        if let Some(w) = weight_lbs {
            if (WEIGHT_FLOOR_LBS..=WEIGHT_CEIL_LBS).contains(&w) {
                self.bio.weight_lbs = Some(w);
            }
        }

        // Draft details: immutable. Only set if prior is None.
        if self.bio.draft_year.is_none() {
            self.bio.draft_year = draft_year;
        }
        if self.bio.draft_round.is_none() {
            self.bio.draft_round = draft_round;
        }
        if self.bio.draft_overall.is_none() {
            self.bio.draft_overall = draft_overall;
        }

        // Rookie season: immutable; same-value re-set is a no-op,
        // mismatched rookie season was caught by the reissue guard.
        if self.bio.rookie_season.is_none() {
            self.bio.rookie_season = rookie_season;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_identity() -> PlayerIdentity {
        PlayerIdentity {
            id: PlayerId(8478402),
            full_name: "Connor McDavid".to_string(),
            name_normalized: "connor mcdavid".to_string(),
            headshot_canonical_url: Some(
                "https://assets.nhle.com/mugs/nhl/default/8478402.png".into(),
            ),
            bio: PlayerBio {
                birth_date: Some("1997-01-13".into()),
                birth_country: Some("CAN".into()),
                nationality_code: Some("CAN".into()),
                height_in_inches: Some(73),
                weight_lbs: Some(193),
                draft_year: Some(2015),
                draft_round: Some(1),
                draft_overall: Some(1),
                shoots_catches: Some("L".into()),
                rookie_season: Some("20152016".into()),
            },
        }
    }

    #[test]
    fn l0_hart1_serde_round_trip_player_id() {
        let id = PlayerId(8478402);
        let s = serde_json::to_string(&id).unwrap();
        assert_eq!(s, "8478402", "PlayerId emits bare integer");
        let back: PlayerId = serde_json::from_str(&s).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn l0_hart1_serde_round_trip_identity() {
        let ident = base_identity();
        let s = serde_json::to_string(&ident).unwrap();
        let back: PlayerIdentity = serde_json::from_str(&s).unwrap();
        assert_eq!(back.id, ident.id);
        assert_eq!(back.full_name, ident.full_name);
        assert_eq!(back.bio.birth_date, ident.bio.birth_date);
        assert_eq!(back.bio.rookie_season, ident.bio.rookie_season);
    }

    #[test]
    fn l0_hart1_serde_default_on_missing_optionals() {
        // A pre-Hart bundle wouldn't have headshot_canonical_url or any
        // bio field. #[serde(default)] should let it parse cleanly.
        let json = r#"{
            "id": 8478402,
            "full_name": "Connor McDavid",
            "name_normalized": "connor mcdavid",
            "bio": {}
        }"#;
        let ident: PlayerIdentity = serde_json::from_str(json).unwrap();
        assert_eq!(ident.headshot_canonical_url, None);
        assert_eq!(ident.bio.birth_date, None);
        assert_eq!(ident.bio.rookie_season, None);
    }

    #[test]
    fn l0_hart1_merge_keeps_prior_on_bogus_weight() {
        let mut prior = base_identity();
        let mut incoming = base_identity();
        incoming.bio.weight_lbs = Some(40); // bogus
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.bio.weight_lbs, Some(193));
    }

    #[test]
    fn l0_hart1_merge_keeps_prior_on_bogus_height() {
        let mut prior = base_identity();
        let mut incoming = base_identity();
        incoming.bio.height_in_inches = Some(48); // bogus
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.bio.height_in_inches, Some(73));
    }

    #[test]
    fn l0_hart1_merge_immovable_birth_date() {
        let mut prior = base_identity();
        let mut incoming = base_identity();
        incoming.bio.birth_date = Some("2001-01-01".into());
        prior.merge_with(incoming).unwrap();
        // Prior had a non-null birth_date — keep it.
        assert_eq!(prior.bio.birth_date, Some("1997-01-13".into()));
    }

    #[test]
    fn l0_hart1_merge_fills_null_birth_date() {
        let mut prior = base_identity();
        prior.bio.birth_date = None;
        let incoming = base_identity();
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.bio.birth_date, Some("1997-01-13".into()));
    }

    #[test]
    fn l0_hart1_merge_immovable_draft_details() {
        let mut prior = base_identity();
        let mut incoming = base_identity();
        incoming.bio.draft_year = Some(1999);
        incoming.bio.draft_round = Some(7);
        incoming.bio.draft_overall = Some(202);
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.bio.draft_year, Some(2015));
        assert_eq!(prior.bio.draft_round, Some(1));
        assert_eq!(prior.bio.draft_overall, Some(1));
    }

    #[test]
    fn l0_hart1_merge_rookie_season_immovable_silent_when_same() {
        let mut prior = base_identity();
        let incoming = base_identity();
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.bio.rookie_season, Some("20152016".into()));
    }

    #[test]
    fn l0_hart1_merge_rookie_season_mismatch_is_reissue_error() {
        let mut prior = base_identity();
        let mut incoming = base_identity();
        incoming.bio.rookie_season = Some("20212022".into());
        let err = prior.merge_with(incoming).unwrap_err();
        assert_eq!(
            err,
            IdentityMergeError::LikelyIdReissue {
                id: PlayerId(8478402),
                prior_rookie_season: Some("20152016".into()),
                incoming_rookie_season: Some("20212022".into()),
            }
        );
        // Critically: prior must be unchanged after a rejected merge.
        assert_eq!(prior.bio.rookie_season, Some("20152016".into()));
        assert_eq!(prior.bio.draft_year, Some(2015));
    }

    #[test]
    fn l0_hart1_merge_rookie_season_fills_null() {
        let mut prior = base_identity();
        prior.bio.rookie_season = None;
        let incoming = base_identity();
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.bio.rookie_season, Some("20152016".into()));
    }

    #[test]
    fn l0_hart1_merge_shoots_catches_immovable_on_mismatch() {
        let mut prior = base_identity();
        let mut incoming = base_identity();
        incoming.bio.shoots_catches = Some("R".into());
        prior.merge_with(incoming).unwrap();
        // Prior had "L"; mismatch keeps prior.
        assert_eq!(prior.bio.shoots_catches, Some("L".into()));
    }

    #[test]
    fn l0_hart1_merge_fills_name_when_incoming_better() {
        let mut prior = base_identity();
        prior.full_name = "C. McDavid".into();
        let incoming = base_identity();
        prior.merge_with(incoming).unwrap();
        assert_eq!(prior.full_name, "Connor McDavid");
    }

    proptest::proptest! {
        /// Merge policy: bad-incoming values never poison good prior values.
        /// proptest spans the bogus ranges and asserts prior is preserved.
        #[test]
        fn merge_policy_proptest(
            prior_w in 100u32..=350,
            prior_h in 60u32..=84,
            bad_w_low  in 0u32..100,
            bad_w_high in 351u32..1000,
            bad_h_low  in 0u32..60,
            bad_h_high in 85u32..120,
        ) {
            for &bad_w in &[bad_w_low, bad_w_high] {
                for &bad_h in &[bad_h_low, bad_h_high] {
                    let mut prior = base_identity();
                    prior.bio.weight_lbs = Some(prior_w);
                    prior.bio.height_in_inches = Some(prior_h);

                    let mut incoming = base_identity();
                    incoming.bio.weight_lbs = Some(bad_w);
                    incoming.bio.height_in_inches = Some(bad_h);

                    prior.merge_with(incoming).unwrap();
                    proptest::prop_assert_eq!(prior.bio.weight_lbs, Some(prior_w));
                    proptest::prop_assert_eq!(prior.bio.height_in_inches, Some(prior_h));
                }
            }
        }
    }
}
