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
    pub const CAN: CountryCode = CountryCode([b'C', b'A', b'N']);
    pub const USA: CountryCode = CountryCode([b'U', b'S', b'A']);
    pub const SWE: CountryCode = CountryCode([b'S', b'W', b'E']);
    pub const FIN: CountryCode = CountryCode([b'F', b'I', b'N']);
    pub const RUS: CountryCode = CountryCode([b'R', b'U', b'S']);
    pub const CZE: CountryCode = CountryCode([b'C', b'Z', b'E']);
    pub const SVK: CountryCode = CountryCode([b'S', b'V', b'K']);

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
}
