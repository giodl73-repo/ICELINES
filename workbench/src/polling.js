export const STATS_POLL_MS = 24 * 60 * 60 * 1000;
export const SCHEDULE_POLL_MS = 60 * 1000;
const clock = { set: (callback, delay) => setTimeout(callback, delay), clear: handle => clearTimeout(handle), random: Math.random };
export class PollingController {
    interval;
    run;
    environment;
    changed;
    timing;
    enabled = false;
    context;
    timer;
    running;
    constructor(interval, run, environment, changed, timing = clock) {
        this.interval = interval;
        this.run = run;
        this.environment = environment;
        this.changed = changed;
        this.timing = timing;
        if (!Number.isFinite(interval) || interval < 1000)
            throw new Error('Polling interval must be at least one second');
    }
    show(message) { this.changed({ enabled: this.enabled, message }); }
    clear() { if (this.timer !== undefined)
        this.timing.clear(this.timer); this.timer = undefined; }
    setContext(context) {
        if (context === this.context)
            return;
        this.context = context;
        this.setEnabled(false);
    }
    setEnabled(enabled) {
        this.clear();
        this.enabled = enabled && !!this.context && this.environment.coordinated();
        if (!this.enabled) {
            this.running?.abort(new Error('cancelled: automatic refresh stopped'));
            this.show(!this.environment.coordinated() ? 'Automatic refresh is unavailable in this browser. Use manual refresh.' : 'Automatic refresh is off.');
            return;
        }
        this.environmentChanged();
    }
    environmentChanged() {
        this.clear();
        if (!this.enabled)
            return;
        if (!this.environment.visible() || !this.environment.online()) {
            this.running?.abort(new Error('cancelled: automatic refresh paused'));
            this.show('Automatic refresh paused while this tab is hidden or offline.');
            return;
        }
        if (!this.running)
            this.schedule();
    }
    schedule(error) {
        if (!this.enabled)
            return;
        if (!this.environment.visible() || !this.environment.online()) {
            this.environmentChanged();
            return;
        }
        // Completion-based timing plus positive jitter prevents catch-up bursts and
        // tight error loops. A failure waits the same complete interval.
        this.timer = this.timing.set(() => { this.timer = undefined; void this.tick(); }, this.interval * (1 + Math.max(0, Math.min(1, this.timing.random())) * 0.1));
        this.show(error ? `Automatic refresh failed; next check after the normal interval. ${error}` : 'Automatic refresh is waiting for the next check.');
    }
    async tick() {
        if (!this.enabled || !this.context)
            return;
        if (!this.environment.visible() || !this.environment.online()) {
            this.environmentChanged();
            return;
        }
        if (this.running || this.environment.busy()) {
            this.schedule();
            return;
        }
        const controller = new AbortController();
        this.running = controller;
        this.show('Automatic refresh is running…');
        let failure;
        try {
            await this.run(controller.signal);
        }
        catch (error) {
            if (!controller.signal.aborted)
                failure = error;
        }
        finally {
            this.running = undefined;
            if (this.enabled) {
                this.clear();
                this.schedule(failure);
            }
        }
    }
}
