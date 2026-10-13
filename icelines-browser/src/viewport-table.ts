import { PlayerTable } from './player-table.js';
import type { PlayerRow } from './types.js';

// Presentation only: retain the complete engine result for browsing and exports.
const ROW_HEIGHT = 72;
const OVERSCAN = 16;
export function rowWindow(count: number, scrollOffset: number, viewportHeight: number): [number, number] {
  const start = Math.max(0, Math.min(count, Math.floor(Math.max(0, scrollOffset) / ROW_HEIGHT) - OVERSCAN));
  return [start, Math.min(count, start + Math.ceil(viewportHeight / ROW_HEIGHT) + 2 * OVERSCAN)];
}

export class ViewportTable {
  private readonly table: PlayerTable;
  private rows: PlayerRow[] = [];
  private goalie = false;
  private pending = false;
  private start = 0;
  private end = 0;
  private frame = 0;
  private readonly htmlTable: HTMLTableElement;
  private readonly top: HTMLTableRowElement;
  private readonly bottom: HTMLTableRowElement;

  constructor(private readonly body: HTMLElement, head: HTMLElement, open: (row: PlayerRow) => void,
    private readonly mode: HTMLSelectElement) {
    this.table = new PlayerTable(body, head, open);
    this.htmlTable = body.closest('table')!;
    this.top = this.spacer(); this.bottom = this.spacer();
    window.addEventListener('scroll', () => this.schedule(), { passive: true });
    window.addEventListener('resize', () => this.schedule(), { passive: true });
    mode.addEventListener('change', () => {
      if (this.pending) return;
      const active = document.activeElement?.closest<HTMLTableRowElement>('tr[data-result-index]');
      const index = active ? Number(active.dataset.resultIndex) : undefined;
      this.draw();
      if (index !== undefined) this.focus(index);
    });
    body.addEventListener('keydown', event => {
      if (event.key !== 'Tab' || this.mode.value === 'all' || this.pending) return;
      const button = (event.target as HTMLElement).closest('button');
      const row = button?.closest<HTMLTableRowElement>('tr[data-result-index]');
      if (!row) return;
      const next = Number(row.dataset.resultIndex) + (event.shiftKey ? -1 : 1);
      if (next < 0 || next >= this.rows.length || (next >= this.start && next < this.end)) return;
      event.preventDefault();
      this.draw(Math.max(0, next - OVERSCAN));
      this.focus(next);
    });
  }

  private spacer(): HTMLTableRowElement {
    const row = document.createElement('tr');
    row.className = 'row-spacer'; row.setAttribute('aria-hidden', 'true'); row.setAttribute('role', 'presentation');
    row.append(document.createElement('td'));
    return row;
  }

  private schedule(): void {
    if (this.frame || this.pending || this.mode.value === 'all' || !this.rows.length) return;
    this.frame = requestAnimationFrame(() => { this.frame = 0; if (!this.pending) this.draw(); });
  }

  suspend(): void {
    this.pending = true; this.mode.disabled = true; this.table.suspend();
  }

  clear(release = true): void {
    cancelAnimationFrame(this.frame); this.frame = 0;
    this.rows = []; this.start = this.end = 0; this.pending = false; this.mode.disabled = false;
    this.top.remove(); this.bottom.remove(); this.table.clear(release);
    this.htmlTable.removeAttribute('aria-rowcount');
    this.body.dataset.resultCount = '0';
  }

  render(rows: PlayerRow[], goalie: boolean): void {
    const tableTop = this.body.getBoundingClientRect().top + window.scrollY;
    this.rows = rows; this.goalie = goalie; this.pending = false; this.mode.disabled = false;
    this.body.dataset.resultCount = String(rows.length);
    // A new sort/filter starts at rank one, including when the old result was long.
    if (window.scrollY > tableTop + ROW_HEIGHT) window.scrollTo({ top: Math.max(0, tableTop - 160) });
    this.draw(0, false);
  }

  private focus(index: number): void {
    const button = this.body.querySelector<HTMLButtonElement>(`tr[data-result-index="${index}"] button`);
    button?.focus({ preventScroll: true }); button?.scrollIntoView({ block: 'nearest' });
  }

  private draw(forcedStart?: number, preserveFocus = true): void {
    if (this.pending) return;
    const virtual = this.mode.value !== 'all';
    const offset = -this.body.getBoundingClientRect().top;
    const [start, end] = virtual ? (forcedStart === undefined ? rowWindow(this.rows.length, offset, window.innerHeight)
      : [forcedStart, Math.min(this.rows.length, forcedStart + Math.ceil(window.innerHeight / ROW_HEIGHT) + 2 * OVERSCAN)])
      : [0, this.rows.length];
    // Scroll events that leave the window unchanged must not allocate or rewrite rows.
    if (forcedStart === undefined && start === this.start && end === this.end
      && this.htmlTable.classList.contains('viewport-table') === virtual) return;
    this.start = start; this.end = end;
    const focused = document.activeElement?.closest<HTMLTableRowElement>('tr[data-result-index]');
    const focusedIndex = preserveFocus && focused && this.body.contains(focused) ? Number(focused.dataset.resultIndex) : undefined;
    // Blurring a recycled focused button can force layout during the update.
    // Keep document height stable while spacers are temporarily detached.
    const container = this.htmlTable.parentElement!;
    container.style.minHeight = `${this.htmlTable.getBoundingClientRect().height}px`;
    this.top.remove(); this.bottom.remove();
    this.htmlTable.classList.toggle('viewport-table', virtual);
    this.htmlTable.setAttribute('aria-rowcount', String(this.rows.length + 1));
    this.table.render(this.rows.slice(start, end), this.goalie, start);
    const elements = Array.from(this.body.querySelectorAll<HTMLTableRowElement>('tr'));
    elements.forEach((element, index) => {
      const rank = start + index;
      element.dataset.resultIndex = String(rank);
      element.dataset.rank = String(rank + 1);
      element.setAttribute('aria-rowindex', String(rank + 2));
      element.classList.toggle('even-rank', rank % 2 === 1);
    });
    if (virtual && this.rows.length) {
      for (const [spacer, height] of [[this.top, start * ROW_HEIGHT], [this.bottom, (this.rows.length - end) * ROW_HEIGHT]] as const) {
        const cell = spacer.cells[0]; cell.colSpan = this.goalie ? 7 : 8; cell.style.height = `${height}px`;
      }
      if (start) this.body.prepend(this.top);
      if (end < this.rows.length) this.body.append(this.bottom);
    }
    container.style.minHeight = '';
    if (focusedIndex !== undefined) {
      if (focusedIndex >= start && focusedIndex < end) {
        this.body.querySelector<HTMLButtonElement>(`tr[data-result-index="${focusedIndex}"] button`)?.focus({ preventScroll: true });
      } else this.htmlTable.parentElement?.focus({ preventScroll: true });
    }
  }
}
