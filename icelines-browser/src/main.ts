import { initializeAppearance } from './appearance.js';
import { EngineClient, EngineStoppedError } from './engine-client.js';
import { registerOfflineShell } from './offline.js';
import { ScheduleController } from './schedule.js';
import { PollingController, STATS_POLL_MS, SCHEDULE_POLL_MS } from './polling.js';
import { liveReads } from './live-coordination.js';
import { queryExport, queryCsv } from './query-export.js';
import { readCatalog } from './catalog.js';
import { reconcileSavePolicy } from './save-policy.js';
import { readPublicBytes } from './network.js';
import { MAX_ARCHIVE_BYTES } from './archive.js';
import { digest, MAX_BYTES, refreshStats } from './acquisition.js';
import { listSaved, loadSaved, onLibraryChange, packageMetadata, removeSaved, savedActiveId, saveDataset, MAX_SCHEDULE_BYTES } from './library.js';
import { SORT_OPTIONS, parseViewHash, serializeViewHash, isPublicDataset, localDatasetForView } from './view-state.js';
import type { ViewState } from './view-state.js';
import type { CatalogEntry, Dataset, PackageData, PackageMetadata, PlayerRow, QueryRequest, QueryResult, SavedDatasetEntry, ScheduledGame } from './types.js';

initializeAppearance();

function element<T extends HTMLElement>(id: string): T { const value = document.getElementById(id); if (!value) throw new Error(`Missing control ${id}`); return value as T; }
const catalogSelect = element<HTMLSelectElement>('catalog');
const kind = element<HTMLSelectElement>('kind');
const sort = element<HTMLSelectElement>('sort');
const filter = element<HTMLInputElement>('filter');
const minimumGames = element<HTMLInputElement>('minimum-games');
const keepUpdated = element<HTMLInputElement>('keep-updated');
const textDecoder = new TextDecoder('utf-8', { fatal: true });
let catalog: CatalogEntry[] = [];
let active: Dataset | undefined;
const resident = new Map<string, Dataset>();
let saved: SavedDatasetEntry[] = [];
let librarySequence = 0;
let result: QueryResult | undefined;
let resultRequest: QueryRequest | undefined;
let resultState: ViewState | undefined;
let routing = false;
let routeSequence = 0;
let started = false;
let acquisition: AbortController | undefined;
let acquisitionSequence = 0;
let querySequence = 0;
let recovering = false;
let engineUnavailable = false;
let recoveryCopy: Dataset | undefined;
let refreshingStats = false;
let statsPolling: PollingController | undefined;
let schedulePolling: PollingController | undefined;
function beginAcquisition(): { controller: AbortController; sequence: number } {
  acquisition?.abort();
  const controller = new AbortController(); acquisition = controller;
  return { controller, sequence: ++acquisitionSequence };
}
function cancelAcquisition(): void { acquisition?.abort(); acquisitionSequence++; element('cancel').hidden = true; }
function requireCurrentAcquisition(sequence: number, signal: AbortSignal): void {
  if (sequence !== acquisitionSequence || signal.aborted) throw new Error('cancelled: context changed');
}
function status(message: string, error = false): void { const node = element('status'); node.textContent = message; node.classList.toggle('error', error); }
function showAnalysis(): void {
  const panel = element<HTMLDetailsElement>('data-library');
  const transferFocus = panel.contains(document.activeElement);
  panel.open = false;
  if (transferFocus) element('analysis').focus();
}
const engine = new EngineClient(message => {
  resident.clear();
  engineUnavailable = true; cancelAcquisition(); ++querySequence;
  recoveryCopy = active ?? recoveryCopy; active = undefined; result = undefined;
  element('rows').replaceChildren(); element('evidence').textContent = '';
  const dialog = element<HTMLDialogElement>('player-dialog'); if (dialog.open) dialog.close();
  element('empty').textContent = 'Engine unavailable. Restart it or load your data again after recovery.';
  status(message, true); updateState();
  if (!recovering) void recoverEngine();
});
async function recoverEngine(): Promise<void> {
  if (recovering) return;
  recovering = true; engineUnavailable = true;
  resident.clear();
  cancelAcquisition(); ++querySequence;
  recoveryCopy = active ?? recoveryCopy; active = undefined; result = undefined;
  element('rows').replaceChildren(); element('evidence').textContent = '';
  const dialog = element<HTMLDialogElement>('player-dialog'); if (dialog.open) dialog.close();
  status('Restarting engine and checking saved data…'); updateState();
  try {
    engine.restart(); await engine.request('currentSeason', null);
    const records = await listSaved();
    const id = recoveryCopy?.id ?? await savedActiveId();
    const restored = records.find(item => item.id === id);
    engineUnavailable = false;
    if (restored) {
      await activateSaved(restored);
      if (recoveryCopy?.revision === restored.revision) recoveryCopy = undefined;
      status(recoveryCopy ? 'Restored the saved revision. Unsaved changes need reloading; export the recovery copy first.' : 'Engine restarted. Restored your saved dataset.');
    } else {
      element('empty').textContent = 'Load a package to explore players.';
      status('Engine restarted. Session-only data needs reloading. Load a season or import a package.');
    }
    await library();
  } catch (error) {
    engineUnavailable = !engine.available;
    status(`Recovery failed. You can export the recovery copy and try restarting or reload the application. ${error}`, true);
  } finally { recovering = false; updateState(); }
}
function savedVersion(): SavedDatasetEntry | undefined { return saved.find(item => item.id === active?.id); }
function updateState(): void {
  renderResidentLibrary();
  statsPolling?.setContext(active ? `${active.id}:${active.metadata.season}:${active.metadata.season_type}` : undefined);
  element<HTMLInputElement>('stats-auto-refresh').disabled = !active || !liveReads.crossTab || engineUnavailable || recovering;
  const matching = savedVersion();
  const isSaved = !!active && matching?.revision === active.revision;
  element('context').textContent = active ? `${active.metadata.season} · ${active.metadata.season_type}` : 'Choose a dataset';
  element('data-state').textContent = active ? `${isSaved ? 'Saved locally' : 'In memory'} · ${active.metadata.source}${matching && !isSaved ? ' · an earlier version is saved' : ''}` : 'No dataset in memory';
  element('data-state').className = 'state';
  for (const id of ['save', 'unload', 'export-package']) element<HTMLButtonElement>(id).disabled = !active;
  element<HTMLButtonElement>('remove').disabled = !matching;
  keepUpdated.disabled = !isSaved;
  keepUpdated.checked = active?.keepUpdated ?? false;
  element<HTMLButtonElement>('refresh').disabled = !active;
  for (const id of ['export-json', 'export-csv', 'create-link']) element<HTMLButtonElement>(id).disabled = !result;
  element<HTMLInputElement>('share-filter').disabled = !result || resultState?.data !== 'public';
  if (!result || resultState?.data !== 'public') element<HTMLInputElement>('share-filter').checked = false;
  element<HTMLButtonElement>('restart-engine').disabled = recovering;
  element('recovery-copy').hidden = !recoveryCopy;
  for (const id of ['load', 'refresh', 'import']) (element(id) as HTMLButtonElement | HTMLInputElement).disabled = routing || recovering || engineUnavailable || (id === 'load' && !catalog.length) || (id === 'refresh' && (!active || refreshingStats));
  element<HTMLFormElement>('query').querySelector<HTMLButtonElement>('button')!.disabled = routing || recovering || engineUnavailable || !active;
  for (const control of [kind, sort, filter, minimumGames]) control.disabled = routing || recovering || engineUnavailable;
  element('saved-library').querySelectorAll('button').forEach(button => { button.disabled = routing || recovering || engineUnavailable; });
  if (!result) { resultState = undefined; element<HTMLInputElement>('shared-link').value = ''; }
  if (!result) element('query-summary').textContent = '';
}
async function library(): Promise<void> {
  const sequence = ++librarySequence;
  try {
    const entries = await listSaved();
    if (sequence !== librarySequence) return;
    saved = entries;
    const wasSaving = active?.keepUpdated;
    for (const dataset of resident.values()) reconcileSavePolicy(dataset, saved);
    if (wasSaving && !active?.keepUpdated) status('Local refresh saving was turned off or the saved copy was removed. Your data remains in memory.');
    element('storage-status').textContent = 'Local storage is available.';
    element('saved-count').textContent = `(${saved.length})`;
    const node = element('saved-library'); node.replaceChildren();
    for (const dataset of saved) {
      const row = document.createElement('li');
      const label = document.createElement('span'); label.textContent = `${dataset.metadata.season} · ${dataset.metadata.season_type} · ${dataset.metadata.source}`;
      const button = document.createElement('button'); button.textContent = 'Load saved';
      button.disabled = recovering || engineUnavailable;
      button.onclick = () => { void perform(async () => { await activateSaved(dataset); status('Loaded your saved copy.'); }); };
      row.append(label, button); node.append(row);
    }
    if (!saved.length) node.textContent = 'No saved datasets yet.';
  } catch (error) {
    if (sequence !== librarySequence) return;
    saved = [];
    element('saved-count').textContent = '(unavailable)';
    element('saved-library').textContent = 'Saved records cannot be checked. Retry storage access or reload the application.';
    element('storage-status').textContent = `Local storage unavailable. In-memory data remains usable; export a package for backup. ${error}`;
  }
  updateState();
}
onLibraryChange(() => { void library(); void schedule.reloadLibrary(); });
element<HTMLButtonElement>('retry-storage').onclick = () => { void library(); };
async function perform(action: () => Promise<void>): Promise<void> {
  try { await action(); }
  catch (error) {
    const obsolete = error instanceof Error && error.message.startsWith('cancelled: ');
    if (!(error instanceof EngineStoppedError) && !obsolete) status(String(error), true);
  } finally { updateState(); }
}
async function activate(bytes: Uint8Array, id: string, policy = false): Promise<void> {
  if (bytes.length > MAX_BYTES) throw new Error('Package exceeds 100 MiB');
  const generation = engine.changeContext();
  const staging = bytes.slice().buffer;
  const { revision, residents: revisions, metadata } = await engine.request<{ revision: string; residents: string[]; metadata: PackageMetadata }>('load', staging, [staging]);
  if (generation !== engine.generation) throw new Error('cancelled: context changed');
  active = { id, bytes, revision, metadata, keepUpdated: policy };
  for (const key of resident.keys()) if (!revisions.includes(key)) resident.delete(key);
  resident.set(revision, active);
  result = undefined; updateState();
  await runQuery();
  showAnalysis();
}
function renderResidentLibrary(): void {
  element('resident-count').textContent = `(${resident.size}/8)`;
  const node = element('resident-library'); node.replaceChildren();
  for (const dataset of resident.values()) {
    const row = document.createElement('li');
    const label = document.createElement('span');
    label.textContent = `${dataset.metadata.season} · ${dataset.metadata.season_type} · ${dataset.metadata.source}${active?.revision === dataset.revision ? ' · selected' : ''}`;
    const button = document.createElement('button'); button.textContent = 'Use in-memory season';
    button.disabled = active?.revision === dataset.revision || routing || recovering || engineUnavailable;
    button.onclick = () => { void perform(async () => { await activateResident(dataset); status('Selected in-memory season. Saved copies are unchanged.'); }); };
    row.append(label,button); node.append(row);
  }
  if (!resident.size) node.textContent = 'No seasons in this tab yet.';
}
async function activateResident(dataset: Dataset): Promise<void> {
  cancelAcquisition(); const generation = engine.changeContext();
  await engine.request('selectResident', dataset.revision);
  if (generation !== engine.generation) throw new Error('cancelled: context changed');
  dataset.keepUpdated = saved.find(entry => entry.id === dataset.id && entry.revision === dataset.revision)?.keepUpdated ?? false;
  active = dataset; result = undefined; updateState(); await runQuery(); showAnalysis();
}
async function activateSaved(entry: SavedDatasetEntry): Promise<void> {
  const { controller, sequence } = beginAcquisition();
  const dataset = await loadSaved(entry.id, entry.revision);
  requireCurrentAcquisition(sequence, controller.signal);
  await activate(dataset.bytes, dataset.id, dataset.keepUpdated);
}
function renderRows(rows: PlayerRow[]): void {
  const goalie = kind.value === 'goalies';
  const columns = goalie ? ['#', 'Player', 'Team', 'GP', 'Wins', 'SV%', 'GAA'] : ['#', 'Player', 'Team', 'GP', 'G', 'A', 'PTS', 'Pace / 82'];
  const header = document.createElement('tr');
  for (const title of columns) { const cell = document.createElement('th'); cell.scope = 'col'; cell.textContent = title; header.append(cell); }
  element('head').replaceChildren(header);
  const body = element('rows'); body.replaceChildren();
  rows.forEach((row, index) => {
    const tr = document.createElement('tr');
    const values = goalie ? [index + 1, row.name, row.team, row.gp, row.wins, row.save_pct?.toFixed(3), row.gaa?.toFixed(2)] : [index + 1, row.name, row.team, row.gp, row.goals, row.assists, row.points, row.pace_82?.toFixed(1)];
    values.forEach((value, column) => {
      const cell = document.createElement('td');
      if (column === 1) { const button = document.createElement('button'); button.textContent = row.name; button.onclick = () => { void perform(() => player(row)); }; cell.append(button); }
      else cell.textContent = value == null ? '—' : String(value);
      tr.append(cell);
    }); body.append(tr);
  });
  element('empty').textContent = rows.length ? `${rows.length} players` : 'No players match. Adjust or clear the filter.';
}
async function player(row: PlayerRow): Promise<void> {
  const detail = await engine.request<{ identity: { bio: unknown }; stats: unknown }>('detail', row.player_id);
  element('player-name').textContent = row.name;
  const summary = document.createElement('p'); summary.textContent = `${row.team} · ${row.position} · ${row.gp} games · ${row.points} points`;
  const details = document.createElement('details'); const label = document.createElement('summary'); label.textContent = 'Player evidence and season record';
  const pre = document.createElement('pre'); pre.textContent = JSON.stringify(detail, null, 2); details.append(label, pre);
  element('player-body').replaceChildren(summary, details);
  element<HTMLDialogElement>('player-dialog').showModal();
}
async function runQuery(): Promise<void> {
  if (!active) return;
  const sequence = ++querySequence;
  const view: ViewState = { season: active.metadata.season, type: active.metadata.season_type,
    kind: kind.value as ViewState['kind'], sort: sort.value, gp: minimumGames.value,
    data: isPublicDataset(active, catalog) ? 'public' : 'local', filter: filter.value };
  serializeViewHash(view); // Validate bookmarkable controls before starting work.
  result = undefined; element('rows').replaceChildren(); element('evidence').textContent = '';
  element('empty').textContent = 'Running query…'; updateState();
  let next: QueryResult;
  const request: QueryRequest = { filter: view.filter, sort: view.sort,
    goalies: view.kind === 'goalies', minimum_games: view.gp === '' ? null : Number(view.gp), today: new Date().toISOString().slice(0, 10) };
  try {
    next = await engine.request<QueryResult>('query', request);
  } catch (error) {
    if (sequence === querySequence) element('empty').textContent = 'Query unavailable. Adjust the filter or load the required data.';
    throw error;
  }
  if (sequence !== querySequence) return;
  result = next; resultRequest = request; resultState = view; renderRows(next.rows);
  element('query-summary').textContent = `${next.rows.length} ${view.kind === 'goalies' ? 'goalies' : 'skaters'} · Minimum GP ${next.minimum_games} · Observed ${next.observed_at ?? 'unknown'} · ${next.missing_sources.length} source families unavailable`;
  element('evidence').textContent = `Source: ${next.source}. Observed: ${next.observed_at ?? 'unknown'}. Fetched: ${next.fetched_at ?? 'unknown'}. Minimum GP: ${next.minimum_games}. Pace requires ${next.pace_minimum_games} GP; values below that are unavailable. Missing: ${next.missing_sources.join(', ')}. Pace is descriptive; historical comparisons are not era adjusted.`;
  // Filter text may contain personal information: bookmarks omit it by default.
  history.replaceState(null, '', serializeViewHash(view));
  updateState();
}
function download(bytes: BlobPart, filename: string, type: string): void {
  const url = URL.createObjectURL(new Blob([bytes], { type }));
  const link = document.createElement('a'); link.href = url; link.download = filename; link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
element('restart-engine').onclick = () => { void recoverEngine(); };
element('export-recovery').onclick = () => { if (recoveryCopy) download(recoveryCopy.bytes.slice().buffer, `icelines-${recoveryCopy.metadata.season}-${recoveryCopy.metadata.season_type}-recovery.json`, 'application/json'); };
async function loadCatalog(entry: CatalogEntry): Promise<void> {
  const memory = resident.get(entry.sha256);
  if (memory && memory.id === entry.id) { await activateResident(memory); status('Selected in-memory season. Save locally to keep it.'); return; }
  const { controller, sequence } = beginAcquisition(); status('Loading season into memory…');
  element('cancel').hidden = false;
  try {
  const retained = saved.find(item => item.id === entry.id && item.revision === entry.sha256);
  const stored = retained ? await loadSaved(retained.id, retained.revision) : undefined;
  const bytes = stored ? stored.bytes : await readPublicBytes(new URL(entry.url, location.href.split('#')[0]), entry.bytes,
    {signal:controller.signal,deadline:Date.now() + 120000});
  if (bytes.length !== entry.bytes || await digest(bytes) !== entry.sha256) throw new Error('Package size or checksum mismatch');
  const context = packageMetadata(JSON.parse(textDecoder.decode(bytes)) as PackageData);
  if (context.season !== entry.season || context.season_type !== entry.season_type || context.source !== entry.source) throw new Error('Package does not match catalog context');
  requireCurrentAcquisition(sequence, controller.signal);
  await activate(bytes, entry.id, stored?.keepUpdated); status(retained ? 'Loaded your saved public season.' : 'Season loaded in memory. Save locally to keep it.');
  } catch (error) {
    if (sequence !== acquisitionSequence) throw new Error('cancelled: context changed');
    if (controller.signal.aborted) { status(active ? 'Season load cancelled. Your previous data remains available.' : 'Season load cancelled. Choose a season or import a package.'); throw new Error('cancelled: season load'); }
    throw error;
  } finally { if (acquisition === controller) element('cancel').hidden = true; }
}
element('load').onclick = () => { void perform(async () => {
  const entry = catalog.find(item => item.id === catalogSelect.value); if (!entry) throw new Error('Choose a package');
  await loadCatalog(entry);
}); };
element<HTMLInputElement>('import').onchange = event => { void perform(async () => {
  const file = (event.target as HTMLInputElement).files?.[0]; if (!file) return;
  (event.target as HTMLInputElement).value = ''; // Allows reimporting the same archive with a different season type.
  const { controller, sequence } = beginAcquisition();
  const archive = /\.tar\.gz$/i.test(file.name);
  if (file.size > (archive ? MAX_ARCHIVE_BYTES : MAX_BYTES)) throw new Error(archive ? 'Compressed archive exceeds 25 MiB' : 'Package exceeds 100 MiB');
  let bytes = new Uint8Array(await file.arrayBuffer());
  requireCurrentAcquisition(sequence, controller.signal);
  if (archive) {
    status('Validating and decompressing the local archive…'); engine.changeContext();
    bytes = new Uint8Array(await engine.request<ArrayBuffer>('importArchive', { bytes: bytes.buffer, filename: file.name,
      seasonType: element<HTMLSelectElement>('import-type').value }, [bytes.buffer]));
  }
  const id = 'import-' + (await digest(bytes)).slice(0, 16);
  requireCurrentAcquisition(sequence, controller.signal);
  await activate(bytes, id); status('Imported into memory. Save locally to keep it.');
}); };
element('save').onclick = () => { void perform(async () => {
  if (!active) return; const dataset = active; const expected = savedVersion()?.revision ?? null;
  await saveDataset(dataset, expected); await library(); status('Saved locally. Export a package for a portable backup.');
}); };
element('remove').onclick = () => { void perform(async () => {
  const dataset = active; const selected = savedVersion(); if (!dataset || !selected) return;
  try { await removeSaved(dataset.id, selected.revision); dataset.keepUpdated = false; }
  catch (error) { await library(); throw error; }
  await library(); status('Saved copy removed. Your in-memory data is still available.');
}); };
element('unload').onclick = () => { void perform(async () => {
  cancelAcquisition(); engine.changeContext(); await engine.request('unload', null);
  if (active) resident.delete(active.revision);
  active = undefined; result = undefined; element('rows').replaceChildren(); element('head').replaceChildren();
  element('empty').textContent = 'Load a package to explore players.'; element('evidence').textContent = ''; status('Unloaded from memory. Saved copies remain in your library.');
}); };
keepUpdated.onchange = () => { void perform(async () => {
  const current = savedVersion(); if (!active || !current || current.revision !== active.revision) return;
  const dataset = active;
  const policy = keepUpdated.checked;
  await saveDataset({ ...dataset, keepUpdated: policy }, current.revision);
  if (active === dataset) active.keepUpdated = policy;
  await library(); status(policy ? 'Successful refreshes will also update your saved copy.' : 'Refreshes will stay in memory.');
}); };
async function refreshActiveStats(signal?: AbortSignal): Promise<void> {
  if (refreshingStats) return;
  if (!active) return; const previous = active; const expected = savedVersion()?.revision ?? null;
  signal?.throwIfAborted(); refreshingStats = true; updateState();
  const { controller, sequence } = beginAcquisition();
  const stop = (): void => controller.abort(signal?.reason);
  signal?.addEventListener('abort', stop, { once: true });
  try {
  const timer = setTimeout(() => controller.abort(), 120000); element('cancel').hidden = false; status('Refreshing NHL season stats…');
  try {
    const bytes = await refreshStats(previous.metadata.season, previous.metadata.season_type, controller.signal);
    requireCurrentAcquisition(sequence, controller.signal);
    if (active !== previous) throw new Error('cancelled: context changed');
    await activate(bytes, previous.id, previous.keepUpdated);
  } catch (error) {
    const wasCancelled = controller.signal.aborted;
    controller.abort();
    if (active?.id === previous.id && active !== previous) throw new Error(`Fresh data was loaded into memory, but the query could not run. ${error}`);
    if (active !== previous) throw new Error('cancelled: context changed');
    if (wasCancelled) throw new Error('Refresh cancelled. Your previous good data remains available.');
    throw new Error(`Live refresh failed. Your previous good data remains available. ${error}`);
  }
  finally { clearTimeout(timer); element('cancel').hidden = true; }
  if (active?.keepUpdated) {
    try { await saveDataset(active, expected, { requireRefreshConsent: true }); await library(); status('Refreshed and saved locally.'); }
    catch (error) { await library(); throw new Error(`Fresh data is available in memory, but saving failed. The local library was not changed by this refresh. ${error}`); }
  } else status('Refreshed in memory. Save locally if you want to keep it.');
  } finally { signal?.removeEventListener('abort', stop); refreshingStats = false; updateState(); }
}
element('refresh').onclick = () => { void perform(() => refreshActiveStats()); };
element('cancel').onclick = () => acquisition?.abort();
element('export-package').onclick = () => { if (active) download(active.bytes.slice().buffer, `icelines-${active.metadata.season}-${active.metadata.season_type}.json`, 'application/json'); };
element('export-json').onclick = () => { if (result && resultRequest) download(JSON.stringify(queryExport(result, resultRequest), null, 2), `icelines-${result.season}-query.json`, 'application/json'); };
element('export-csv').onclick = () => {
  if (result && resultRequest) download(queryCsv(result, resultRequest), `icelines-${result.season}-query.csv`, 'text/csv;charset=utf-8');
};
element('query').onsubmit = event => { event.preventDefault(); void perform(runQuery); };
function setSortOptions(): void {
  sort.replaceChildren(...SORT_OPTIONS[kind.value === 'goalies' ? 'goalies' : 'skaters'].map(([value, label]) => new Option(label, value)));
}
kind.onchange = () => {
  setSortOptions();
  void perform(runQuery);
};
element('create-link').onclick = () => {
  if (!resultState) return;
  const includeFilter = element<HTMLInputElement>('share-filter').checked;
  try {
    const url = new URL(location.href); url.hash = serializeViewHash(resultState, includeFilter); url.search = '';
    element<HTMLInputElement>('shared-link').value = url.href;
    element('link-state').textContent = resultState.data === 'local' ? 'This link references local data only. Recipients must load or import their own copy; no dataset or filter is included.' : includeFilter ? 'This link includes the applied public filter. Review it before sharing.' : 'This link omits filter text. Only public view settings are included.';
  } catch (error) { status(String(error), true); }
};
element('share-filter').onchange = () => { element<HTMLInputElement>('shared-link').value = ''; };
async function applyViewRoute(view: ViewState): Promise<void> {
  const sequence = ++routeSequence; routing = true; cancelAcquisition(); ++querySequence;
  result = undefined; element('rows').replaceChildren(); element('evidence').textContent = '';
  kind.value = view.kind; setSortOptions(); sort.value = view.sort; minimumGames.value = view.gp; filter.value = view.filter;
  updateState();
  try {
    const entry = catalog.find(item => item.season === view.season && item.season_type === view.type);
    if (entry) catalogSelect.value = entry.id;
    if (view.data === 'public' && entry) await loadCatalog(entry);
    else if (view.data === 'local') {
      if (active && active.metadata.season === view.season && active.metadata.season_type === view.type && !isPublicDataset(active, catalog)) await runQuery();
      else {
        const id = await savedActiveId().catch(() => undefined);
        if (sequence !== routeSequence) throw new Error('cancelled: context changed');
        const local = localDatasetForView(saved, id, view, catalog);
        if (local) { await activateSaved(local); status('Restored local data for this view.'); }
        else await missingRouteData('This view needs local data. Load a saved copy or import the package; the link contains no dataset.');
      }
    } else await missingRouteData('This season/type is not in the public catalog. Choose a season or import a compatible package.');
    // Preserve only the filter explicitly supplied by an incoming public link.
    // Subsequent ordinary queries go back to filter-free bookmarks.
    if (sequence === routeSequence && result) history.replaceState(null, '', serializeViewHash(view, view.data === 'public' && !!view.filter));
  } finally { if (sequence === routeSequence) { routing = false; updateState(); } }
}
async function missingRouteData(message: string): Promise<void> {
  if (active && !savedVersion()) recoveryCopy = active;
  active = undefined; engine.changeContext(); await engine.request('unload', null);
  element('empty').textContent = message; status(message, true);
}
window.addEventListener('hashchange', () => {
  if (!started) return;
  void perform(async () => { const view = parseViewHash(location.hash); if (view) await applyViewRoute(view); });
});
element('close-player').onclick = () => element<HTMLDialogElement>('player-dialog').close();
async function reloadCatalog(): Promise<void> {
  const retry = element<HTMLButtonElement>('retry-catalog'); retry.disabled = true;
  element('catalog-status').textContent = 'Checking public season catalog…';
  try {
    const entries = await readCatalog(new URL('../catalog.json', import.meta.url));
    const selection = catalogSelect.value;
    catalog = entries;
    catalogSelect.replaceChildren(...entries.map(entry => new Option(`${entry.season} · ${entry.season_type} · ${entry.skaters} skaters`, entry.id)));
    if (entries.some(entry => entry.id === selection)) catalogSelect.value = selection;
    element('catalog-status').textContent = entries.length ? `Public catalog available: ${entries.length} season packages.` : 'Public catalog is empty. Load saved data or import a package.';
    retry.hidden = true;
  } catch (error) {
    element('catalog-status').textContent = `Public catalog unavailable. Saved data and file imports remain usable. Reconnect and retry the catalog. ${error}`;
    retry.hidden = false;
  } finally { retry.disabled = false; updateState(); }
}
element('retry-catalog').onclick = () => { void reloadCatalog(); };
async function start(): Promise<void> {
  await reloadCatalog();
  await engine.request('currentSeason', null); await library();
  started = true;
  const view = parseViewHash(location.hash);
  if (view) { await applyViewRoute(view); return; }
  const id = await savedActiveId().catch(() => undefined); const restored = saved.find(item => item.id === id);
  if (restored) { await activateSaved(restored); status('Restored your saved dataset.'); }
  else status('Choose a season or import a package. Data stays in memory until you save it.');
  updateState();
}
void perform(async () => { try { await start(); } finally { await schedule.restoreActive(); } });
void registerOfflineShell();

const scheduleDate = element<HTMLInputElement>('schedule-date');
const today = new Date();
scheduleDate.value = `${today.getFullYear()}-${String(today.getMonth() + 1).padStart(2, '0')}-${String(today.getDate()).padStart(2, '0')}`;
let renderedScheduleRevision: string | undefined;
const schedule = new ScheduleController(bytes => {
  const staging = bytes.slice().buffer;
  return engine.request<ScheduledGame[]>('schedule', staging, [staging]);
}, state => {
  element('schedule-status').textContent = state.message;
  element('schedule-cancel').hidden = !state.loading;
  element<HTMLButtonElement>('schedule-refresh').disabled = state.loading;
  const snapshot = state.snapshot;
  if (snapshot && snapshot.revision !== renderedScheduleRevision) scheduleDate.value = snapshot.requestedDate;
  renderedScheduleRevision = snapshot?.revision;
  schedulePolling?.setContext(snapshot && scheduleDate.value === snapshot.requestedDate ? snapshot.requestedDate : undefined);
  element<HTMLInputElement>('schedule-auto-refresh').disabled = !snapshot || !liveReads.crossTab || engineUnavailable || recovering;
  const matching = state.entries.find(entry => entry.id === snapshot?.dataset.id);
  const isSaved = !!snapshot && matching?.revision === snapshot.revision;
  element('schedule-evidence').textContent = snapshot ? `NHL schedule · requested ${snapshot.requestedDate} · fetched ${snapshot.fetchedAt} · source observation time unknown · revision ${snapshot.revision} · ${isSaved ? 'saved locally' : 'in memory'}. Game states are supplied by NHL; missing scores are unavailable.` : '';
  for (const id of ['schedule-save', 'schedule-unload', 'schedule-export']) element<HTMLButtonElement>(id).disabled = !snapshot || state.loading;
  element<HTMLButtonElement>('schedule-remove').disabled = !matching || state.loading;
  const policy = element<HTMLInputElement>('schedule-keep-updated'); policy.disabled = !isSaved || state.loading; policy.checked = snapshot?.dataset.keepUpdated ?? false;
  element('schedule-storage-status').textContent = state.storageMessage;
  element('schedule-count').textContent = state.storageAvailable === false ? '(unavailable)' : `(${state.entries.length})`;
  const entries = state.entries.map(entry => {
    const row = document.createElement('li'); const label = document.createElement('span');
    label.textContent = `${entry.metadata.requested_date} · fetched ${entry.metadata.fetched_at}`;
    const load = document.createElement('button'); load.textContent = 'Load saved schedule'; load.disabled = state.loading;
    load.onclick = () => { scheduleDate.value = entry.metadata.requested_date; void schedule.load(entry); };
    row.append(label, load); return row;
  });
  element('schedule-library').replaceChildren(...entries);
  const rows = (snapshot?.games ?? []).map(game => {
    const row = document.createElement('tr');
    for (const value of [game.date, `${game.away_name} (${game.away_abbrev})`, `${game.away_score ?? '—'} – ${game.home_score ?? '—'}`,
      `${game.home_name} (${game.home_abbrev})`, game.start_time_utc, game.game_state ?? 'Unavailable', game.series_game ?? '—']) {
      const cell = document.createElement('td'); cell.textContent = value; row.append(cell);
    }
    return row;
  });
  element('schedule-rows').replaceChildren(...rows);
});
element('schedule-form').onsubmit = event => { event.preventDefault(); void schedule.refresh(scheduleDate.value); };
element('schedule-cancel').onclick = () => schedule.cancel();
scheduleDate.onchange = () => { schedulePolling?.setContext(undefined); schedule.cancel(); };
element('schedule-save').onclick = () => { void schedule.save(); };
element('schedule-remove').onclick = () => { void schedule.remove(); };
element('schedule-unload').onclick = () => schedule.unload();
element('schedule-export').onclick = () => { const snapshot = schedule.state.snapshot; if (snapshot) download(snapshot.dataset.bytes.slice().buffer, `icelines-schedule-${snapshot.requestedDate}.json`, 'application/json'); };
element('schedule-retry-storage').onclick = () => { void schedule.reloadLibrary(); };
element<HTMLInputElement>('schedule-keep-updated').onchange = event => { void schedule.save((event.target as HTMLInputElement).checked); };
element<HTMLInputElement>('schedule-import').onchange = event => { void perform(async () => {
  const input = event.target as HTMLInputElement, file = input.files?.[0]; input.value = ''; if (!file) return;
  if (file.size > MAX_SCHEDULE_BYTES) throw new Error('Schedule package exceeds 3 MiB');
  await schedule.import(new Uint8Array(await file.arrayBuffer()));
  if (schedule.state.snapshot) scheduleDate.value = schedule.state.snapshot.requestedDate;
}); };

const pollingEnvironment = {
  visible: () => document.visibilityState === 'visible', online: () => navigator.onLine,
  coordinated: () => liveReads.crossTab,
};
statsPolling = new PollingController(STATS_POLL_MS, async signal => {
  try { await refreshActiveStats(signal); }
  catch (error) { if (!signal.aborted && !(error instanceof EngineStoppedError)) status(String(error), true); throw error; }
}, { ...pollingEnvironment, busy: () => refreshingStats || routing || recovering || engineUnavailable }, state => {
  element<HTMLInputElement>('stats-auto-refresh').checked = state.enabled;
  element('stats-poll-status').textContent = state.message;
});
schedulePolling = new PollingController(SCHEDULE_POLL_MS, signal => schedule.refresh(scheduleDate.value, signal),
  { ...pollingEnvironment, busy: () => schedule.state.loading || recovering || engineUnavailable || routing || refreshingStats }, state => {
    element<HTMLInputElement>('schedule-auto-refresh').checked = state.enabled;
    element('schedule-poll-status').textContent = state.message;
  });
statsPolling.setEnabled(false); schedulePolling.setEnabled(false);
element<HTMLInputElement>('stats-auto-refresh').onchange = event => statsPolling!.setEnabled((event.target as HTMLInputElement).checked);
element<HTMLInputElement>('schedule-auto-refresh').onchange = event => schedulePolling!.setEnabled((event.target as HTMLInputElement).checked);
const pollingEnvironmentChanged = (): void => { statsPolling?.environmentChanged(); schedulePolling?.environmentChanged(); };
document.addEventListener('visibilitychange', pollingEnvironmentChanged);
window.addEventListener('online', pollingEnvironmentChanged); window.addEventListener('offline', pollingEnvironmentChanged);
window.addEventListener('pagehide', () => { statsPolling?.setEnabled(false); schedulePolling?.setEnabled(false); });
