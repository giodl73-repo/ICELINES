import type { PlayerRow } from './types.js';

// Reuse presentation nodes across queries. This is not a result limit: rows
// beyond the pool are still displayed, but are not retained for the next query.
const MAX_REUSABLE_ROWS = 1024;
type RowNodes = { element: HTMLTableRowElement; values: Text[]; button: HTMLButtonElement; playerId?: number };

export class PlayerTable {
  private pool: RowNodes[] = [];
  private visible: RowNodes[] = [];
  private goalie: boolean | undefined;
  constructor(private readonly body: HTMLElement, private readonly head: HTMLElement,
    private readonly openPlayer: (row: PlayerRow) => void) {}

  suspend(): void {
    // Hide stale results from sight and accessibility while preserving layout.
    this.body.style.visibility = 'hidden'; this.body.inert = true;
  }

  clear(release = true): void {
    // Detached buttons must not retain callbacks or names from a previous dataset.
    for (const nodes of this.visible) {
      nodes.button.onclick = null;
      nodes.playerId = undefined;
      for (const text of nodes.values) text.nodeValue = '';
    }
    this.body.replaceChildren();
    this.body.style.visibility = ''; this.body.inert = false;
    this.visible = [];
    if (release) { this.pool = []; this.goalie = undefined; this.head.replaceChildren(); }
  }

  render(rows: PlayerRow[], goalie: boolean): void {
    if (this.goalie !== goalie) {
      this.clear(); this.goalie = goalie;
      const header = document.createElement('tr');
      const columns = goalie ? ['#', 'Player', 'Team', 'GP', 'Wins', 'SV%', 'GAA'] : ['#', 'Player', 'Team', 'GP', 'G', 'A', 'PTS', 'Pace / 82'];
      for (const title of columns) {
        const cell = document.createElement('th'); cell.scope = 'col'; cell.textContent = title; header.append(cell);
      }
      this.head.replaceChildren(header);
    }
    // Keep common rows attached: moving them through a fragment discards their
    // browser layout objects even when the presentation nodes are reused.
    const common = Math.min(rows.length, this.visible.length, MAX_REUSABLE_ROWS);
    for (const nodes of this.visible.slice(common)) {
      nodes.button.onclick = null;
      nodes.playerId = undefined;
      for (const text of nodes.values) text.nodeValue = '';
      nodes.element.remove();
    }
    this.visible.length = common;
    const fragment = document.createDocumentFragment();
    rows.forEach((row, index) => {
      const values = goalie ? [index + 1, row.name, row.team, row.gp, row.wins, row.save_pct?.toFixed(3), row.gaa?.toFixed(2)] : [index + 1, row.name, row.team, row.gp, row.goals, row.assists, row.points, row.pace_82?.toFixed(1)];
      let nodes = this.pool[index];
      if (!nodes) {
        nodes = { element: document.createElement('tr'), values: [], button: document.createElement('button') };
        nodes.button.type = 'button';
        values.forEach((_, column) => {
          const cell = document.createElement('td'); const text = document.createTextNode('');
          nodes!.values.push(text);
          if (column === 1) { nodes!.button.append(text); cell.append(nodes!.button); }
          else cell.append(text);
          nodes!.element.append(cell);
        });
        if (index < MAX_REUSABLE_ROWS) this.pool[index] = nodes;
      }
      values.forEach((value, column) => {
        const text = value == null ? '—' : String(value);
        if (nodes!.values[column].nodeValue !== text) nodes!.values[column].nodeValue = text;
      });
      // A background refresh may change who occupies this rank. Do not leave
      // keyboard focus on a button that now opens a different player.
      if (nodes.button === document.activeElement && nodes.playerId !== row.player_id) nodes.button.blur();
      nodes.playerId = row.player_id;
      nodes.button.onclick = () => this.openPlayer(row);
      if (index >= common) {
        this.visible.push(nodes);
        fragment.append(nodes.element);
      }
    });
    this.body.append(fragment);
    // Experimental bounded cache: cleared detached rows carry no player data.
    // Reuse them when a larger season follows a smaller playoff/filter result.
    // Unload and column transitions still release the entire pool.
    this.body.style.visibility = ''; this.body.inert = false;
  }
}
