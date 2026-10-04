export const STATS_POLL_MS = 24 * 60 * 60 * 1000;
export const SCHEDULE_POLL_MS = 60 * 1000;
export interface PollingEnvironment {
  visible(): boolean; online(): boolean; coordinated(): boolean; busy(): boolean;
}
export interface PollingClock {
  set(callback: () => void, delay: number): unknown; clear(handle: unknown): void; random(): number;
}
const clock: PollingClock = { set: (callback, delay) => setTimeout(callback, delay), clear: handle => clearTimeout(handle as number), random: Math.random };
export interface PollingState { enabled: boolean; message: string; }
export class PollingController {
  enabled = false;
  private context?: string;
  private timer?: unknown;
  private running?: AbortController;
  constructor(private readonly interval: number, private readonly run: (signal: AbortSignal) => Promise<void>,
    private readonly environment: PollingEnvironment, private readonly changed: (state: PollingState) => void,
    private readonly timing = clock) {
    if (!Number.isFinite(interval) || interval < 1000) throw new Error('Polling interval must be at least one second');
  }
  private show(message: string): void { this.changed({ enabled: this.enabled, message }); }
  private clear(): void { if (this.timer !== undefined) this.timing.clear(this.timer); this.timer = undefined; }
  setContext(context: string | undefined): void {
    if (context === this.context) return;
    this.context = context; this.setEnabled(false);
  }
  setEnabled(enabled: boolean): void {
    this.clear(); this.enabled = enabled && !!this.context && this.environment.coordinated();
    if (!this.enabled) {
      this.running?.abort(new Error('cancelled: automatic refresh stopped'));
      this.show(!this.environment.coordinated() ? 'Automatic refresh is unavailable in this browser. Use manual refresh.' : 'Automatic refresh is off.');
      return;
    }
    this.environmentChanged();
  }
  environmentChanged(): void {
    this.clear();
    if (!this.enabled) return;
    if (!this.environment.visible() || !this.environment.online()) {
      this.running?.abort(new Error('cancelled: automatic refresh paused'));
      this.show('Automatic refresh paused while this tab is hidden or offline.'); return;
    }
    if (!this.running) this.schedule();
  }
  private schedule(error?: unknown): void {
    if (!this.enabled) return;
    if (!this.environment.visible() || !this.environment.online()) { this.environmentChanged(); return; }
    // Completion-based timing plus positive jitter prevents catch-up bursts and
    // tight error loops. A failure waits the same complete interval.
    this.timer = this.timing.set(() => { this.timer = undefined; void this.tick(); }, this.interval * (1 + Math.max(0, Math.min(1, this.timing.random())) * 0.1));
    this.show(error ? `Automatic refresh failed; next check after the normal interval. ${error}` : 'Automatic refresh is waiting for the next check.');
  }
  private async tick(): Promise<void> {
    if (!this.enabled || !this.context) return;
    if (!this.environment.visible() || !this.environment.online()) { this.environmentChanged(); return; }
    if (this.running || this.environment.busy()) { this.schedule(); return; }
    const controller = new AbortController(); this.running = controller; this.show('Automatic refresh is running…');
    let failure: unknown;
    try { await this.run(controller.signal); } catch (error) { if (!controller.signal.aborted) failure = error; }
    finally {
      this.running = undefined;
      if (this.enabled) { this.clear(); this.schedule(failure); }
    }
  }
}
