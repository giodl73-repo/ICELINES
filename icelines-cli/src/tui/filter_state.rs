use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use icelines_core::identity::PlayerId;
use icelines_core::model::Season;
use icelines_core::season_stats::SeasonType;
use icelines_core::stats_repository::PlayerView;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CountryCode([u8; 3]);

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CountryCodeError {
    #[error("country code must be 3 ASCII letters; got {0:?}")]
    NotThreeAscii(String),
}

impl CountryCode {
    pub const CAN: CountryCode = CountryCode(*b"CAN");
    pub const USA: CountryCode = CountryCode(*b"USA");
    pub const SWE: CountryCode = CountryCode(*b"SWE");
    pub const FIN: CountryCode = CountryCode(*b"FIN");
    pub const RUS: CountryCode = CountryCode(*b"RUS");
    pub const CZE: CountryCode = CountryCode(*b"CZE");
    pub const SVK: CountryCode = CountryCode(*b"SVK");

    pub fn parse(s: &str) -> Result<Self, CountryCodeError> {
        let trimmed = s.trim();
        if trimmed.len() != 3 || !trimmed.is_ascii() {
            return Err(CountryCodeError::NotThreeAscii(s.to_owned()));
        }

        let mut bytes = [0u8; 3];
        for (idx, b) in trimmed.as_bytes().iter().enumerate() {
            if !b.is_ascii_alphabetic() {
                return Err(CountryCodeError::NotThreeAscii(s.to_owned()));
            }
            bytes[idx] = b.to_ascii_uppercase();
        }
        Ok(Self(bytes))
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::CAN => "CAN",
            Self::USA => "USA",
            Self::SWE => "SWE",
            Self::FIN => "FIN",
            Self::RUS => "RUS",
            Self::CZE => "CZE",
            Self::SVK => "SVK",
            _ => "???",
        }
    }
}

pub const COUNTRY_CYCLE: &[CountryCode] = &[
    CountryCode::CAN,
    CountryCode::USA,
    CountryCode::SWE,
    CountryCode::FIN,
    CountryCode::RUS,
    CountryCode::CZE,
    CountryCode::SVK,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PosFilter {
    #[default]
    All,
    Forwards,
    Defense,
    C,
    LW,
    RW,
    LD,
    RD,
}

impl PosFilter {
    pub const ALL: &'static [PosFilter] = &[
        PosFilter::All,
        PosFilter::Forwards,
        PosFilter::Defense,
        PosFilter::C,
        PosFilter::LW,
        PosFilter::RW,
        PosFilter::LD,
        PosFilter::RD,
    ];

    pub fn label(self) -> &'static str {
        match self {
            PosFilter::All => "All",
            PosFilter::Forwards => "F",
            PosFilter::Defense => "D",
            PosFilter::C => "C",
            PosFilter::LW => "LW",
            PosFilter::RW => "RW",
            PosFilter::LD => "LD",
            PosFilter::RD => "RD",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn matches(self, pos_abbrev: &str) -> bool {
        match self {
            PosFilter::All => true,
            PosFilter::Forwards => matches!(pos_abbrev, "C" | "LW" | "RW" | "F"),
            PosFilter::Defense => matches!(pos_abbrev, "LD" | "RD" | "D"),
            PosFilter::C => pos_abbrev == "C",
            PosFilter::LW => pos_abbrev == "LW",
            PosFilter::RW => pos_abbrev == "RW",
            PosFilter::LD => pos_abbrev == "LD",
            PosFilter::RD => pos_abbrev == "RD",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GoalieRoleFilter {
    #[default]
    All,
    Starters,
    Backups,
}

impl GoalieRoleFilter {
    pub const ALL: &'static [GoalieRoleFilter] = &[
        GoalieRoleFilter::All,
        GoalieRoleFilter::Starters,
        GoalieRoleFilter::Backups,
    ];

    pub fn label(self) -> &'static str {
        match self {
            GoalieRoleFilter::All => "All",
            GoalieRoleFilter::Starters => "Starters",
            GoalieRoleFilter::Backups => "Backups",
        }
    }

    pub fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|f| *f == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    pub fn matches_gp(self, gp: u32, threshold: Option<u32>) -> bool {
        match self {
            GoalieRoleFilter::All => true,
            GoalieRoleFilter::Starters => threshold.map(|t| gp >= t).unwrap_or(true),
            GoalieRoleFilter::Backups => threshold.map(|t| gp < t).unwrap_or(true),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ForcedColumns(u8);

impl ForcedColumns {
    pub const HITS: Self = Self(0b0000_0001);
    #[allow(dead_code)]
    pub const BLOCKS: Self = Self(0b0000_0010);
    #[allow(dead_code)]
    pub const TOI: Self = Self(0b0000_0100);
    #[allow(dead_code)]
    pub const SAVES: Self = Self(0b0000_1000);

    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self(0)
    }

    #[allow(dead_code)]
    pub fn bits(self) -> u8 {
        self.0
    }

    pub fn contains(self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }

    pub fn insert(&mut self, flag: Self) {
        self.0 |= flag.0;
    }

    pub fn toggle(&mut self, flag: Self) {
        self.0 ^= flag.0;
    }
}

#[derive(Debug, Clone, Default)]
pub struct RosterFilterState {
    pub pos_filter: PosFilter,
    pub country_filter: Option<CountryCode>,
    pub min_gp: u32,
    pub forced_columns: ForcedColumns,
    #[allow(dead_code)]
    pub free_filter_signature: Option<u64>,
    cached: Option<FilterCache>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RosterKvArgs {
    pub sort: Option<String>,
    pub pos: Option<PosFilter>,
    pub country: Option<CountryCode>,
    pub min_gp: Option<u32>,
    pub forced_columns: ForcedColumns,
    pub forced_column_keys: ForcedColumns,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum KvParseError {
    #[error("unknown key {key:?}")]
    UnknownKey { key: String },
    #[error("duplicate key {0:?}")]
    DuplicateKey(String),
    #[error("invalid {key} value {raw:?}: {reason}")]
    InvalidValue {
        key: &'static str,
        raw: String,
        reason: String,
    },
}

pub fn parse_roster_kv(input: &str) -> Result<RosterKvArgs, KvParseError> {
    let mut out = RosterKvArgs::default();
    let mut seen = std::collections::HashSet::new();
    for token in input.split_whitespace() {
        let (raw_key, value) = token
            .split_once('=')
            .ok_or_else(|| KvParseError::UnknownKey {
                key: token.to_string(),
            })?;
        let key = canonical_key(raw_key);
        if !seen.insert(key.to_string()) {
            return Err(KvParseError::DuplicateKey(raw_key.to_string()));
        }
        match key {
            "sort" => out.sort = Some(value.to_ascii_lowercase()),
            "pos" => out.pos = Some(parse_pos(value)?),
            "country" => {
                out.country =
                    Some(
                        CountryCode::parse(value).map_err(|err| KvParseError::InvalidValue {
                            key: "country",
                            raw: value.to_string(),
                            reason: err.to_string(),
                        })?,
                    )
            }
            "min-gp" => {
                out.min_gp = Some(
                    value
                        .parse::<u32>()
                        .map_err(|_| KvParseError::InvalidValue {
                            key: "min-gp",
                            raw: value.to_string(),
                            reason: "expected non-negative integer".to_string(),
                        })?,
                )
            }
            "hits" => {
                apply_on_off(&mut out.forced_columns, ForcedColumns::HITS, "hits", value)?;
                out.forced_column_keys.insert(ForcedColumns::HITS);
            }
            "saves" => {
                apply_on_off(
                    &mut out.forced_columns,
                    ForcedColumns::SAVES,
                    "saves",
                    value,
                )?;
                out.forced_column_keys.insert(ForcedColumns::SAVES);
            }
            _ => {
                return Err(KvParseError::UnknownKey {
                    key: raw_key.to_string(),
                })
            }
        }
    }
    Ok(out)
}

fn canonical_key(key: &str) -> &str {
    match key {
        "country" | "nationality" => "country",
        "min_gp" | "mingp" => "min-gp",
        other => other,
    }
}

fn parse_pos(value: &str) -> Result<PosFilter, KvParseError> {
    match value.to_ascii_uppercase().as_str() {
        "ALL" => Ok(PosFilter::All),
        "F" | "FORWARD" | "FORWARDS" => Ok(PosFilter::Forwards),
        "D" | "DEFENSE" | "DEFENCE" => Ok(PosFilter::Defense),
        "C" => Ok(PosFilter::C),
        "LW" => Ok(PosFilter::LW),
        "RW" => Ok(PosFilter::RW),
        "LD" => Ok(PosFilter::LD),
        "RD" => Ok(PosFilter::RD),
        _ => Err(KvParseError::InvalidValue {
            key: "pos",
            raw: value.to_string(),
            reason: "expected All, F, D, C, LW, RW, LD, or RD".to_string(),
        }),
    }
}

fn apply_on_off(
    columns: &mut ForcedColumns,
    flag: ForcedColumns,
    key: &'static str,
    value: &str,
) -> Result<(), KvParseError> {
    match value.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => {
            if !columns.contains(flag) {
                columns.toggle(flag);
            }
            Ok(())
        }
        "off" | "false" | "0" => {
            if columns.contains(flag) {
                columns.toggle(flag);
            }
            Ok(())
        }
        _ => Err(KvParseError::InvalidValue {
            key,
            raw: value.to_string(),
            reason: "expected on or off".to_string(),
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FilterCache {
    season: Season,
    season_type: SeasonType,
    repo_generation: u64,
    filter_signature: u64,
    filtered_pids: Vec<PlayerId>,
}

impl RosterFilterState {
    pub fn invalidate(&mut self) {
        self.cached = None;
    }

    pub fn cycle_country(&mut self) {
        self.country_filter = match self.country_filter {
            None => Some(COUNTRY_CYCLE[0]),
            Some(cur) => {
                let idx = COUNTRY_CYCLE.iter().position(|c| *c == cur).unwrap_or(0);
                if idx + 1 >= COUNTRY_CYCLE.len() {
                    None
                } else {
                    Some(COUNTRY_CYCLE[idx + 1])
                }
            }
        };
        self.invalidate();
    }

    pub fn country_label(&self) -> &'static str {
        self.country_filter
            .map(CountryCode::as_str)
            .unwrap_or("All")
    }

    pub fn matches_view(&self, view: &PlayerView<'_>) -> bool {
        self.pos_filter.matches(view.position().abbreviation())
            && self.matches_country(view)
            && view.gp() >= self.min_gp
    }

    #[allow(dead_code)]
    pub fn filter_player_ids(
        &mut self,
        views: &[PlayerView<'_>],
        season: Season,
        season_type: SeasonType,
        repo_generation: u64,
    ) -> &[PlayerId] {
        let filter_signature = self.compute_filter_signature();
        let cache_hit = matches!(
            &self.cached,
            Some(cache)
                if cache.season == season
                    && cache.season_type == season_type
                    && cache.repo_generation == repo_generation
                    && cache.filter_signature == filter_signature
        );

        if !cache_hit {
            let filtered_pids = views
                .iter()
                .filter(|view| self.matches_view(view))
                .map(|view| view.id())
                .collect();
            self.cached = Some(FilterCache {
                season,
                season_type,
                repo_generation,
                filter_signature,
                filtered_pids,
            });
        }

        &self
            .cached
            .as_ref()
            .expect("cache populated before return")
            .filtered_pids
    }

    #[allow(dead_code)]
    pub fn compute_filter_signature(&self) -> u64 {
        let mut h = DefaultHasher::new();
        self.pos_filter.hash(&mut h);
        self.country_filter.hash(&mut h);
        self.min_gp.hash(&mut h);
        self.forced_columns.bits().hash(&mut h);
        self.free_filter_signature.hash(&mut h);
        h.finish()
    }

    fn matches_country(&self, view: &PlayerView<'_>) -> bool {
        match self.country_filter {
            None => true,
            Some(code) => view
                .identity
                .bio
                .nationality_code
                .as_deref()
                .and_then(|raw| CountryCode::parse(raw).ok())
                .map(|got| got == code)
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l0_messier_country_code_parses_uppercase_ascii() {
        assert_eq!(CountryCode::parse("can").unwrap(), CountryCode::CAN);
        assert_eq!(CountryCode::parse(" usa ").unwrap().as_str(), "USA");
        assert!(CountryCode::parse("CA").is_err());
        assert!(CountryCode::parse("C4N").is_err());
    }

    #[test]
    fn l0_messier_pos_filter_cycles_through_all() {
        let mut p = PosFilter::default();
        let mut seen = vec![p];
        for _ in 0..PosFilter::ALL.len() {
            p = p.next();
            seen.push(p);
        }
        assert_eq!(seen.first(), seen.last());
    }

    #[test]
    fn l0_messier_goalie_role_filter_cycles_and_matches_threshold() {
        let mut role = GoalieRoleFilter::default();
        assert_eq!(role, GoalieRoleFilter::All);
        role = role.next();
        assert_eq!(role, GoalieRoleFilter::Starters);
        assert!(role.matches_gp(36, Some(30)));
        assert!(!role.matches_gp(20, Some(30)));
        role = role.next();
        assert_eq!(role, GoalieRoleFilter::Backups);
        assert!(role.matches_gp(20, Some(30)));
        assert!(!role.matches_gp(36, Some(30)));
    }

    #[test]
    fn l0_messier_forced_columns_toggle_is_idempotent_pair() {
        let mut cols = ForcedColumns::empty();
        assert!(!cols.contains(ForcedColumns::HITS));
        cols.toggle(ForcedColumns::HITS);
        assert!(cols.contains(ForcedColumns::HITS));
        cols.toggle(ForcedColumns::HITS);
        assert!(!cols.contains(ForcedColumns::HITS));
    }

    #[test]
    fn l0_messier_filter_signature_changes_on_semantic_inputs() {
        let mut state = RosterFilterState::default();
        let base = state.compute_filter_signature();
        state.pos_filter = PosFilter::Forwards;
        assert_ne!(base, state.compute_filter_signature());
        state.pos_filter = PosFilter::All;
        state.country_filter = Some(CountryCode::CAN);
        assert_ne!(base, state.compute_filter_signature());
    }

    #[test]
    fn l0_messier_parse_roster_kv_success_and_aliases() {
        let args = parse_roster_kv("sort=gaa min-gp=20 nationality=CAN pos=LW saves=on").unwrap();
        assert_eq!(args.sort.as_deref(), Some("gaa"));
        assert_eq!(args.min_gp, Some(20));
        assert_eq!(args.country, Some(CountryCode::CAN));
        assert_eq!(args.pos, Some(PosFilter::LW));
        assert!(args.forced_columns.contains(ForcedColumns::SAVES));
    }

    #[test]
    fn l0_messier_parse_roster_kv_rejects_duplicate_keys() {
        assert!(matches!(
            parse_roster_kv("country=CAN nationality=USA"),
            Err(KvParseError::DuplicateKey(_))
        ));
    }
}
