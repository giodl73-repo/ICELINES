import type { PlayerRow } from './types.js';

export interface ComparisonContext {
  revision: string; season: number; season_type: string; goalies: boolean;
}
const key = (context: ComparisonContext): string => JSON.stringify([
  context.revision, context.season, context.season_type, context.goalies,
]);

// Own two small projections only; never retain a query result or dataset bytes.
export class PlayerComparison {
  private contextKey: string | undefined;
  private selected: PlayerRow[] = [];
  get players(): readonly PlayerRow[] { return this.selected; }
  setContext(context?: ComparisonContext): void {
    const next = context && key(context);
    if (next !== this.contextKey) this.clear();
    this.contextKey = next;
  }
  add(row: PlayerRow, context: ComparisonContext): void {
    if (!this.contextKey || key(context) !== this.contextKey) throw new Error('The dataset changed. Select this player again.');
    if (this.selected.some(player => player.player_id === row.player_id)) return;
    if (this.selected.length === 2) throw new Error('Remove a player before adding another.');
    this.selected.push({ ...row });
  }
  remove(id: number): void { this.selected = this.selected.filter(player => player.player_id !== id); }
  clear(): void { this.selected = []; }
}

// Display the shared Rust projection, with the same precision as the leaders table.
// No scores, winner, missing-value substitution or per-game calculations.
export function comparisonStats(player: PlayerRow, goalies: boolean): [string, string][] {
  const count = (value: number | null): string => value === null ? '—' : String(value);
  return goalies ? [
    ['Games played', String(player.gp)], ['Wins', count(player.wins)],
    ['Save percentage', player.save_pct === null ? '—' : player.save_pct.toFixed(3)],
    ['Goals against average', player.gaa === null ? '—' : player.gaa.toFixed(2)],
  ] : [
    ['Games played', String(player.gp)], ['Goals', String(player.goals)],
    ['Assists', String(player.assists)], ['Points', String(player.points)],
    ['Pace per 82', player.pace_82 === null ? '—' : player.pace_82.toFixed(1)],
  ];
}

export function renderComparison(
  panel: HTMLElement, selection: PlayerComparison, context: ComparisonContext | undefined,
  provenance: string, changed: () => void,
): void {
  const rendered = JSON.stringify([context, selection.players, provenance]);
  if (panel.dataset.comparison === rendered) return;
  panel.dataset.comparison = rendered;
  panel.hidden = !selection.players.length || !context;
  panel.replaceChildren();
  if (panel.hidden || !context) return;
  const heading = document.createElement('h2');
  heading.id = 'comparison-title'; heading.tabIndex = -1; heading.textContent = 'Head to head';
  const note = document.createElement('p'); note.id = 'comparison-context';
  note.textContent = `${context.season} · ${context.season_type} · ${context.goalies ? 'Goalies' : 'Skaters'} · ${provenance}`;
  const help = document.createElement('p'); help.id = 'comparison-help';
  help.textContent = selection.players.length === 1
    ? 'Open another player and choose Compare this player. Your selection stays while you filter this season.'
    : 'Same-season stats. — means unavailable. Compare games played when reading totals.';
  const clear = document.createElement('button'); clear.type = 'button'; clear.textContent = 'Clear comparison';
  clear.onclick = () => { selection.clear(); changed(); };
  const table = document.createElement('table'); table.className = 'comparison-table';
  table.setAttribute('aria-labelledby', 'comparison-title');
  table.setAttribute('aria-describedby', 'comparison-context comparison-help');
  const head = table.createTHead().insertRow();
  const metric = document.createElement('th'); metric.scope = 'col'; metric.textContent = 'Stat'; head.append(metric);
  for (const player of selection.players) {
    const cell = document.createElement('th'); cell.scope = 'col';
    const name = document.createElement('span'); name.textContent = player.name;
    const team = document.createElement('small'); team.textContent = `${player.team} · ${player.position}`;
    const remove = document.createElement('button'); remove.type = 'button'; remove.textContent = 'Remove';
    remove.setAttribute('aria-label', `Remove ${player.name} from comparison`);
    remove.onclick = () => { selection.remove(player.player_id); changed(); };
    cell.append(name, team, remove); head.append(cell);
  }
  const stats = selection.players.map(player => comparisonStats(player, context.goalies));
  const body = table.createTBody();
  for (let index = 0; index < stats[0].length; index++) {
    const row = body.insertRow();
    const label = document.createElement('th'); label.scope = 'row'; label.textContent = stats[0][index][0]; row.append(label);
    for (const player of stats) { const value = row.insertCell(); value.textContent = player[index][1]; }
  }
  const scroll = document.createElement('div'); scroll.className = 'table-scroll';
  scroll.tabIndex = 0; scroll.setAttribute('role', 'region'); scroll.setAttribute('aria-label', 'Player comparison table');
  scroll.append(table);
  panel.append(heading, note, help, clear, scroll);
}
