export interface CatalogEntry {
  id: string; season: number; season_type: 'regular' | 'playoff'; source: string;
  url: string; sha256: string; bytes: number; skaters: number; goalies: number;
}
export interface PackageData {
  schema_version: number; season: number; season_type: 'regular' | 'playoff';
  source: string; observed_at: string | null; fetched_at: string | null;
  bios: unknown[]; stats: unknown[]; goalies: unknown[];
}
export interface Dataset {
  id: string; revision: string; bytes: Uint8Array; metadata: PackageData;
  keepUpdated: boolean;
}
export interface SavedDatasetEntry {
  id: string; revision: string; keepUpdated: boolean; byteLength: number;
  metadata: Pick<PackageData, 'schema_version' | 'season' | 'season_type' | 'source' | 'observed_at' | 'fetched_at'>;
}
export interface ScheduleMetadata {
  schema_version: 1; kind: 'nhl-game-week'; requested_date: string;
  source: 'NHL schedule'; observed_at: null; fetched_at: string;
}
export interface ScheduleDataset {
  id: string; revision: string; bytes: Uint8Array; metadata: ScheduleMetadata; keepUpdated: boolean;
}
export type SavedScheduleEntry = Omit<ScheduleDataset, 'bytes'> & { byteLength: number };
export interface PlayerRow {
  player_id: number; name: string; team: string; position: string; gp: number;
  goals: number; assists: number; points: number; pace_82: number | null;
  save_pct: number | null; gaa: number | null; wins: number | null;
}
export interface QueryResult {
  schema_version: number; season: number; season_type: string; revision: string;
  source: string; observed_at: string | null; fetched_at: string | null;
  missing_sources: string[]; minimum_games: number; pace_minimum_games: number; rows: PlayerRow[];
}
export interface QueryRequest {
  filter: string; sort: string; goalies: boolean; minimum_games: number | null; today: string;
}
export interface ScheduledGame {
  game_id: number; date: string; game_type: number; away_abbrev: string; away_name: string;
  home_abbrev: string; home_name: string; start_time_utc: string; away_score: number | null;
  home_score: number | null; game_state: string | null; last_period: string | null;
  series_game: string | null; away_wins: number | null; home_wins: number | null;
}
export type Operation = 'load' | 'query' | 'detail' | 'currentSeason' | 'unload' | 'importArchive' | 'schedule' | 'selectResident' | 'residentRevisions';
export interface EngineRequest {
  schema_version: 1; request_id: number; context_generation: number;
  operation: Operation; payload: unknown;
}
export interface EngineResponse {
  schema_version: 1; request_id: number; context_generation: number;
  value?: unknown; error?: { kind: string; message: string };
}
