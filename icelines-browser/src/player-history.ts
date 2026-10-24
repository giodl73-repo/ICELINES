import { comparisonStats } from './comparison.js';
import type { PlayerRow } from './types.js';

export interface PlayerHistory {
  schema_version: 1; player_id: number;
  rows: {
    season: number; season_type: 'regular' | 'playoff'; revision: string;
    source: string; observed_at: string | null; fetched_at: string | null;
    goalie: boolean | null; player: PlayerRow | null;
  }[];
}

export function historySeasonLabel(season: number, type: string): string {
  const value = String(season);
  return `${value.slice(0, 4)}–${value.slice(-2)} · ${type === 'playoff' ? 'Playoffs' : 'Regular season'}`;
}

// One short projection per already-resident window; never load or sum seasons here.
export function renderPlayerHistory(history?: PlayerHistory): HTMLElement {
  const section = document.createElement('section'); section.id = 'player-history';
  section.setAttribute('aria-labelledby', 'history-title');
  const heading = document.createElement('h3'); heading.id = 'history-title'; heading.textContent = 'Loaded season history';
  const note = document.createElement('p');
  note.textContent = history
    ? 'Only seasons in your Session library are shown, up to eight windows. Load another season to expand this view. This is not a complete career.'
    : 'Season history is unavailable. Reload the application to try again.';
  section.append(heading, note);
  if (!history) return section;
  const list = document.createElement('ol'); list.className = 'history-windows'; list.setAttribute('role', 'list');
  for (const window of history.rows) {
    const item = document.createElement('li'); item.className = 'history-window';
    const title = document.createElement('h4'); title.textContent = historySeasonLabel(window.season, window.season_type);
    item.append(title);
    if (window.player) {
      const identity = document.createElement('p'); identity.textContent = `${window.player.name} · ${window.player.team} · ${window.player.position}`;
      const values = document.createElement('dl'); values.className = 'history-stats';
      for (const [label, value] of comparisonStats(window.player, window.goalie === true)) {
        const term = document.createElement('dt'); term.textContent = label;
        const data = document.createElement('dd'); data.textContent = value;
        values.append(term, data);
      }
      item.append(identity, values);
    } else {
      const missing = document.createElement('p'); missing.textContent = 'No player record in this loaded package. This does not mean zero games.';
      item.append(missing);
    }
    const source = document.createElement('p'); source.className = 'history-source';
    source.textContent = `${window.source} · Observed ${window.observed_at ?? 'unknown'} · Fetched ${window.fetched_at ?? 'unknown'}`;
    item.append(source); list.append(item);
  }
  section.append(list);
  return section;
}
