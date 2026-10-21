//! Compact mutable normalized season rows for prepared-package consumers.
//! This is a separate store, not a replacement for StatsRepository's roster/report/LRU API.
//! Dense owner tables and repaired swap slots follow MISER EXP006/009 (MIT).
use crate::{
    contract::PlayerContract,
    identity::{PlayerId, PlayerIdentity},
    model::{Position, Season},
    season_stats::*,
    stats_repository::{ReadPlayerView, RepoError},
    SeasonStatsRead,
};
use std::{collections::HashMap, marker::PhantomData, num::NonZeroU32};

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum SparseSeasonError {
    #[error(transparent)]
    Repository(#[from] RepoError),
    #[error("sparse payload slot width exceeded")]
    SlotOverflow,
    #[error("invalid sparse owner/slot index")]
    InvalidIndex,
}
type Key = (PlayerId, Season, SeasonType);
fn hot_key(row: &Hot) -> Key {
    (row.player_id, row.season, row.season_type)
}
fn next_slot(len: usize) -> Result<NonZeroU32, SparseSeasonError> {
    len.checked_add(1)
        .and_then(|n| u32::try_from(n).ok())
        .and_then(NonZeroU32::new)
        .ok_or(SparseSeasonError::SlotOverflow)
}
macro_rules! hot_row {
    ($($field:ident:$ty:ty),+ $(,)?)=>{
        #[derive(Default)] struct Slots{$($field:Option<NonZeroU32>,)+}
        struct Hot{player_id:PlayerId,season:Season,season_type:SeasonType,position:Position,sweater_number:Option<u32>,totals:StatTotals,team_stints:Vec<TeamStint>,slots:Slots}
    }
}
hot_row! { realtime:RealtimeStats,advanced:AdvancedStats,goalie:GoalieSeasonStats,time_on_ice:TimeOnIceStats,goals_for_against:GoalsForAgainstStats,goalie_advanced:GoalieAdvancedStats,goalie_saves_by_strength:GoalieSavesByStrengthStats,goalie_bios:GoalieBios }
struct Owned<T> {
    owner: usize,
    value: T,
}
fn owned_put<T>(table: &mut Vec<Owned<T>>, owner: usize, value: Option<T>) -> Option<NonZeroU32> {
    value.map(|value| {
        let slot = next_slot(table.len()).expect("all slot lengths prechecked");
        table.push(Owned { owner, value });
        slot
    })
}
fn owned_get<T>(table: &[Owned<T>], slot: Option<NonZeroU32>) -> Option<&T> {
    slot.map(|s| &table[s.get() as usize - 1].value)
}
fn checked_after_remove(len: usize, remove: bool, present: bool) -> Result<(), SparseSeasonError> {
    let len = len
        .checked_sub(usize::from(remove))
        .ok_or(SparseSeasonError::SlotOverflow)?;
    if present {
        next_slot(len)?;
    }
    Ok(())
}
macro_rules! mutable_tables {
    ($($field:ident:$ty:ty),+ $(,)?) => {
        #[derive(Default)]
        struct OwnedTables { $($field:Vec<Owned<$ty>>,)+ }
        impl OwnedTables {
            fn precheck(&self,row:&SeasonStats,old:Option<&Slots>)->Result<(),SparseSeasonError> {
                $(checked_after_remove(self.$field.len(),old.is_some_and(|s|s.$field.is_some()),row.$field.is_some())?;)+
                Ok(())
            }
            fn pack(&mut self,owner:usize,row:SeasonStats)->Hot {
                let SeasonStats {player_id,season,season_type,position,sweater_number,team_stints,totals,$($field,)+}=row;
                Hot {player_id,season,season_type,position,sweater_number,team_stints,totals,
                    slots:Slots {$($field:owned_put(&mut self.$field,owner,$field),)+}}
            }
            fn release(&mut self,rows:&mut [Hot],owner:usize) {
                $(if let Some(slot)=rows[owner].slots.$field.take() {
                    let index=slot.get() as usize-1;
                    self.$field.swap_remove(index); // drops the removed owned value, including strings
                    if let Some(moved)=self.$field.get(index) {
                        rows[moved.owner].slots.$field=Some(slot);
                    }
                })+
            }
            fn moved_row(&mut self,row:&Hot,new_owner:usize) {
                $(if let Some(slot)=row.slots.$field {
                    self.$field[slot.get() as usize-1].owner=new_owner;
                })+
            }
            $(fn $field<'a>(&'a self,row:&Hot)->Option<&'a $ty> {
                owned_get(&self.$field,row.slots.$field)
            })+



            fn validate(&self,rows:&[Hot])->Result<(),SparseSeasonError> {
                $(for (index,payload) in self.$field.iter().enumerate() {
                    let row=rows.get(payload.owner).ok_or(SparseSeasonError::InvalidIndex)?;
                    if row.slots.$field!=Some(next_slot(index)?) {return Err(SparseSeasonError::InvalidIndex);}
                }
                for (owner,row) in rows.iter().enumerate() {
                    if let Some(slot)=row.slots.$field {
                        if self.$field.get(slot.get() as usize-1).is_none_or(|p|p.owner!=owner) {
                            return Err(SparseSeasonError::InvalidIndex);
                        }
                    }
                })+
                Ok(())
            }
        }
    }
}
mutable_tables! {
    realtime:RealtimeStats,advanced:AdvancedStats,goalie:GoalieSeasonStats,
    time_on_ice:TimeOnIceStats,goals_for_against:GoalsForAgainstStats,
    goalie_advanced:GoalieAdvancedStats,goalie_saves_by_strength:GoalieSavesByStrengthStats,goalie_bios:GoalieBios,
}

/// Prepared-window ownership without an assembled SeasonStats clone cache.
/// BrowserEngine owns window replacement/eviction. This store intentionally has
/// no roster overlays, Tier2 report cache or independent window LRU. Identities
/// and contracts follow existing merge/borrow rules within this owner.
///
/// ```compile_fail,E0502
/// use icelines_core::{fixtures, sparse_season::SparseSeasonRepository, identity::PlayerId, model::Season, season_stats::SeasonType};
/// let mut store = SparseSeasonRepository::new();
/// store.upsert_identity(fixtures::identity(1).build()).unwrap();
/// store.upsert_stats(fixtures::stats(1, 20252026, "EDM").build()).unwrap();
/// let view = store.view(PlayerId(1), Season(20252026), SeasonType::Regular).unwrap();
/// store.remove(PlayerId(1), Season(20252026), SeasonType::Regular);
/// std::hint::black_box(view.gp());
/// ```
#[derive(Default)]
pub struct SparseSeasonRepository {
    identities: HashMap<PlayerId, PlayerIdentity>,
    contracts: HashMap<PlayerId, PlayerContract>,
    rows: Vec<Hot>,
    index: HashMap<Key, usize>,
    tables: OwnedTables,
    _not_send: PhantomData<*const ()>,
}
static_assertions::assert_not_impl_any!(SparseSeasonRepository: Send, Sync);
impl SparseSeasonRepository {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn identity(&self, id: PlayerId) -> Option<&PlayerIdentity> {
        self.identities.get(&id)
    }
    pub fn upsert_identity(&mut self, identity: PlayerIdentity) -> Result<(), RepoError> {
        match self.identities.get_mut(&identity.id) {
            Some(old) => old.merge_with(identity)?,
            None => {
                self.identities.insert(identity.id, identity);
            }
        }
        Ok(())
    }
    pub fn upsert_contract(&mut self, id: PlayerId, contract: PlayerContract) {
        self.contracts.insert(id, contract);
    }
    pub fn upsert_stats(&mut self, row: SeasonStats) -> Result<(), SparseSeasonError> {
        if !self.identities.contains_key(&row.player_id) {
            return Err(RepoError::StatsWithoutIdentity {
                id: row.player_id,
                season: row.season,
                season_type: row.season_type,
            }
            .into());
        }
        let key = (row.player_id, row.season, row.season_type);
        let owner = self.index.get(&key).copied().unwrap_or(self.rows.len());
        self.tables
            .precheck(&row, self.rows.get(owner).map(|r| &r.slots))?;
        if owner < self.rows.len() {
            self.tables.release(&mut self.rows, owner);
            self.rows[owner] = self.tables.pack(owner, row);
        } else {
            self.rows.push(self.tables.pack(owner, row));
            self.index.insert(key, owner);
        }
        Ok(())
    }
    pub fn remove(&mut self, id: PlayerId, season: Season, kind: SeasonType) -> bool {
        let Some(owner) = self.index.remove(&(id, season, kind)) else {
            return false;
        };
        self.tables.release(&mut self.rows, owner);
        self.rows.swap_remove(owner);
        if let Some(moved) = self.rows.get(owner) {
            self.index.insert(hot_key(moved), owner);
            self.tables.moved_row(moved, owner);
        }
        true
    }
    pub fn view(
        &self,
        id: PlayerId,
        season: Season,
        kind: SeasonType,
    ) -> Option<ReadPlayerView<'_, SeasonRowRead<'_>>> {
        let identity = self.identities.get(&id)?;
        let row = &self.rows[*self.index.get(&(id, season, kind))?];
        Some(ReadPlayerView {
            identity,
            stats: SeasonRowRead(RowStorage::Sparse(row, &self.tables)),
            contract: self.contracts.get(&id),
        })
    }
    pub fn read_league(
        &self,
        season: Season,
        kind: SeasonType,
    ) -> impl Iterator<Item = ReadPlayerView<'_, SeasonRowRead<'_>>> {
        self.rows
            .iter()
            .filter(move |r| r.season == season && r.season_type == kind)
            .filter_map(|row| {
                Some(ReadPlayerView {
                    identity: self.identities.get(&row.player_id)?,
                    stats: SeasonRowRead(RowStorage::Sparse(row, &self.tables)),
                    contract: self.contracts.get(&row.player_id),
                })
            })
    }
    pub fn stats_len(&self) -> usize {
        self.rows.len()
    }
    pub fn validate(&self) -> Result<(), SparseSeasonError> {
        if self.rows.len() != self.index.len() {
            return Err(SparseSeasonError::InvalidIndex);
        }
        for (i, row) in self.rows.iter().enumerate() {
            if self.index.get(&hot_key(row)) != Some(&i)
                || !self.identities.contains_key(&row.player_id)
            {
                return Err(SparseSeasonError::InvalidIndex);
            }
        }
        self.tables.validate(&self.rows)
    }
}
#[derive(Clone, Copy)]
enum RowStorage<'a> {
    Inline(&'a SeasonStats),
    Sparse(&'a Hot, &'a OwnedTables),
}
/// Copyable borrowed handle. Payloads resolve lazily from their actual owners.
#[derive(Clone, Copy)]
pub struct SeasonRowRead<'a>(RowStorage<'a>);
impl<'a> SeasonRowRead<'a> {
    pub(crate) fn inline(row: &'a SeasonStats) -> Self {
        Self(RowStorage::Inline(row))
    }
}
macro_rules! row_reads {
    ($($field:ident:$ty:ty),+ $(,)?)=>{$(fn $field(&self)->Option<&$ty>{match self.0{RowStorage::Inline(r)=>r.$field.as_ref(),RowStorage::Sparse(r,t)=>t.$field(r)}})+};
}
impl SeasonStatsRead for SeasonRowRead<'_> {
    fn player_id(&self) -> PlayerId {
        match self.0 {
            RowStorage::Inline(r) => r.player_id,
            RowStorage::Sparse(r, _) => r.player_id,
        }
    }
    fn season(&self) -> Season {
        match self.0 {
            RowStorage::Inline(r) => r.season,
            RowStorage::Sparse(r, _) => r.season,
        }
    }
    fn season_type(&self) -> SeasonType {
        match self.0 {
            RowStorage::Inline(r) => r.season_type,
            RowStorage::Sparse(r, _) => r.season_type,
        }
    }
    fn position(&self) -> Position {
        match self.0 {
            RowStorage::Inline(r) => r.position,
            RowStorage::Sparse(r, _) => r.position,
        }
    }
    fn sweater_number(&self) -> Option<u32> {
        match self.0 {
            RowStorage::Inline(r) => r.sweater_number,
            RowStorage::Sparse(r, _) => r.sweater_number,
        }
    }
    fn totals(&self) -> &StatTotals {
        match self.0 {
            RowStorage::Inline(r) => &r.totals,
            RowStorage::Sparse(r, _) => &r.totals,
        }
    }
    fn team_stints(&self) -> &[TeamStint] {
        match self.0 {
            RowStorage::Inline(r) => &r.team_stints,
            RowStorage::Sparse(r, _) => &r.team_stints,
        }
    }
    row_reads! { realtime:RealtimeStats,advanced:AdvancedStats,goalie:GoalieSeasonStats,time_on_ice:TimeOnIceStats,goals_for_against:GoalsForAgainstStats,goalie_advanced:GoalieAdvancedStats,goalie_saves_by_strength:GoalieSavesByStrengthStats,goalie_bios:GoalieBios }
}

impl serde::Serialize for SeasonRowRead<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut row = serializer.serialize_struct("SeasonStats", 15)?;
        row.serialize_field("player_id", &self.player_id())?;
        row.serialize_field("season", &self.season())?;
        row.serialize_field("season_type", &self.season_type())?;
        row.serialize_field("position", &self.position())?;
        row.serialize_field("sweater_number", &self.sweater_number())?;
        row.serialize_field("team_stints", &self.team_stints())?;
        row.serialize_field("totals", &self.totals())?;
        row.serialize_field("realtime", &self.realtime())?;
        row.serialize_field("advanced", &self.advanced())?;
        row.serialize_field("goalie", &self.goalie())?;
        row.serialize_field("time_on_ice", &self.time_on_ice())?;
        row.serialize_field("goals_for_against", &self.goals_for_against())?;
        row.serialize_field("goalie_advanced", &self.goalie_advanced())?;
        row.serialize_field("goalie_saves_by_strength", &self.goalie_saves_by_strength())?;
        row.serialize_field("goalie_bios", &self.goalie_bios())?;
        row.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn payload_slot_width_is_checked_before_mutation() {
        assert_eq!(next_slot(u32::MAX as usize - 1).unwrap().get(), u32::MAX);
        assert_eq!(
            next_slot(u32::MAX as usize),
            Err(SparseSeasonError::SlotOverflow)
        );
        assert_eq!(next_slot(usize::MAX), Err(SparseSeasonError::SlotOverflow));
        assert!(checked_after_remove(u32::MAX as usize, true, true).is_ok());
        assert_eq!(
            checked_after_remove(0, true, true),
            Err(SparseSeasonError::SlotOverflow)
        );
    }
}
