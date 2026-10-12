import type { PlayerRow } from './types.js';

// Reuse presentation nodes across queries. This is not a result limit: rows
// beyond the pool are still displayed, but are not retained for the next query.
const MAX_REUSABLE_ROWS = 1024;
type RowNodes = { element: HTMLTableRowElement; values: Text[]; button: HTMLButtonElement };

export class PlayerTable {
  private pool: RowNodes[] = [];
  private visible: RowNodes[] = [];
  private goalie: boolean | undefined;
  constructor(private readonly body: HTMLElement, private readonly head: HTMLElement,
    private readonly openPlayer: (row: PlayerRow) => void) {}

  clear(release = true): void {
    // Detached buttons must not retain callbacks or names from a previous dataset.
    for (const nodes of this.visible) {
      nodes.button.onclick = null;
      for (const text of nodes.values) text.nodeValue = '';
    }
    this.body.replaceChildren();
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
    } else this.clear(false);
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
      values.forEach((value, column) => { nodes!.values[column].nodeValue = value == null ? '—' : String(value); });
      nodes.button.onclick = () => this.openPlayer(row);
      this.visible.push(nodes);
      fragment.append(nodes.element);
    });
    this.body.append(fragment);
    // A smaller result must release spare nodes from a larger season/filter.
    this.pool.length = Math.min(rows.length, MAX_REUSABLE_ROWS);
  }
}
