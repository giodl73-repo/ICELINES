import { refreshSchedule } from './acquisition.js';
import { reconcileSavePolicy } from './save-policy.js';
import { makeScheduleDataset, importScheduleDataset, schedulePayload, listSavedSchedules, loadSavedSchedule, saveSchedule, removeSavedSchedule, savedActiveScheduleId } from './library.js';
export const scheduleStorage = { list: listSavedSchedules, load: loadSavedSchedule, save: saveSchedule, remove: removeSavedSchedule, active: savedActiveScheduleId };
export class ScheduleController {
    project;
    changed;
    acquire;
    now;
    storage;
    state = { loading: false, message: 'Choose a date and refresh. Scores and schedule stay in this tab until saved.', entries: [], storageMessage: 'Checking schedule storage…' };
    sequence = 0;
    controller;
    librarySequence = 0;
    constructor(project, changed, acquire = refreshSchedule, now = () => new Date().toISOString(), storage = scheduleStorage) {
        this.project = project;
        this.changed = changed;
        this.acquire = acquire;
        this.now = now;
        this.storage = storage;
    }
    snapshot(dataset, games) {
        return { requestedDate: dataset.metadata.requested_date, fetchedAt: dataset.metadata.fetched_at, revision: dataset.revision, games, dataset };
    }
    async reloadLibrary() {
        const sequence = ++this.librarySequence;
        try {
            const entries = await this.storage.list();
            if (sequence !== this.librarySequence)
                return;
            this.state = { ...this.state, entries, storageMessage: 'Schedule storage is available.', storageAvailable: true };
            const snapshot = this.state.snapshot;
            if (snapshot) {
                const wasSaving = snapshot.dataset.keepUpdated;
                reconcileSavePolicy(snapshot.dataset, entries);
                if (wasSaving && !snapshot.dataset.keepUpdated && !this.state.message.includes('saving failed'))
                    this.state.message = 'Local schedule refresh saving was turned off or the saved copy was removed. Your schedule remains in memory.';
            }
        }
        catch (error) {
            if (sequence !== this.librarySequence)
                return;
            this.state = { ...this.state, entries: [], storageMessage: `Schedule storage unavailable. In-memory scores remain usable; export a backup. ${error}`, storageAvailable: false };
        }
        this.changed(this.state);
    }
    async restoreActive() {
        const sequence = this.sequence;
        await this.reloadLibrary();
        try {
            const id = await this.storage.active();
            if (sequence !== this.sequence)
                return;
            const entry = this.state.entries.find(entry => entry.id === id);
            if (entry)
                await this.load(entry);
        }
        catch (error) {
            if (sequence === this.sequence) {
                this.state = { ...this.state, storageMessage: `Saved schedule recovery unavailable. ${error}` };
                this.changed(this.state);
            }
        }
    }
    async load(entry) {
        this.cancel();
        const sequence = ++this.sequence;
        this.state = { ...this.state, loading: true, message: 'Loading your saved schedule…' };
        this.changed(this.state);
        try {
            const dataset = await this.storage.load(entry);
            if (sequence !== this.sequence)
                return;
            const games = await this.project(schedulePayload(dataset));
            if (sequence !== this.sequence)
                return;
            this.state = { ...this.state, loading: false, snapshot: this.snapshot(dataset, games), message: 'Loaded your saved schedule. Refresh to check for changes.' };
        }
        catch (error) {
            if (sequence !== this.sequence)
                return;
            this.state = { ...this.state, loading: false, message: `Saved schedule could not load. In-memory data is unchanged. ${error}` };
        }
        this.changed(this.state);
    }
    async save(policy) {
        const snapshot = this.state.snapshot;
        if (!snapshot)
            return;
        const expected = this.state.entries.find(entry => entry.id === snapshot.dataset.id)?.revision ?? null;
        if (policy !== undefined && expected !== snapshot.revision) {
            this.state.message = 'Load or save the current schedule revision before changing its refresh-save policy.';
            this.changed(this.state);
            return;
        }
        const dataset = { ...snapshot.dataset, keepUpdated: policy ?? snapshot.dataset.keepUpdated };
        try {
            await this.storage.save(dataset, expected);
            if (this.state.snapshot === snapshot) {
                snapshot.dataset.keepUpdated = dataset.keepUpdated;
                this.state.message = 'Schedule saved locally. Export a package for backup.';
            }
        }
        catch (error) {
            if (this.state.snapshot === snapshot)
                this.state.message = `Schedule save failed. In-memory data remains usable; the earlier saved copy is unchanged. ${error}`;
        }
        await this.reloadLibrary();
    }
    async remove() {
        const snapshot = this.state.snapshot;
        if (!snapshot)
            return;
        const selected = this.state.entries.find(entry => entry.id === snapshot.dataset.id);
        if (!selected)
            return;
        try {
            await this.storage.remove(snapshot.dataset.id, selected.revision);
            if (this.state.snapshot === snapshot) {
                snapshot.dataset.keepUpdated = false;
                this.state.message = 'Saved schedule removed. Your in-memory schedule remains available.';
            }
        }
        catch (error) {
            this.state.message = `Removal failed; saved data is unchanged. ${error}`;
        }
        await this.reloadLibrary();
    }
    unload() { this.cancel(); this.state = { ...this.state, snapshot: undefined, message: 'Schedule unloaded from memory. Saved copies remain in the library.' }; this.changed(this.state); }
    async import(bytes) {
        this.cancel();
        const sequence = ++this.sequence;
        this.state = { ...this.state, loading: true, message: 'Validating imported schedule…' };
        this.changed(this.state);
        try {
            const dataset = await importScheduleDataset(bytes);
            if (sequence !== this.sequence)
                return;
            const games = await this.project(schedulePayload(dataset));
            if (sequence !== this.sequence)
                return;
            this.state = { ...this.state, loading: false, snapshot: this.snapshot(dataset, games), message: 'Schedule imported into memory. Save locally to keep it.' };
        }
        catch (error) {
            if (sequence !== this.sequence)
                return;
            this.state = { ...this.state, loading: false, message: `Schedule import failed. In-memory data is unchanged. ${error}` };
        }
        this.changed(this.state);
    }
    cancel() {
        this.controller?.abort();
        this.sequence++;
        const action = this.state.loading ? 'Refresh cancelled.' : 'Date changed.';
        this.state = { ...this.state, loading: false, message: `${action} ${this.state.snapshot ? 'The last loaded schedule remains in memory.' : 'No schedule has been loaded. Refresh to load this week.'}` };
        this.changed(this.state);
    }
    async refresh(date, signal) {
        signal?.throwIfAborted();
        this.controller?.abort();
        const controller = new AbortController();
        this.controller = controller;
        const stop = () => controller.abort(signal?.reason);
        signal?.addEventListener('abort', stop, { once: true });
        const sequence = ++this.sequence;
        const previous = this.state.snapshot;
        const retained = this.state.entries.find(entry => entry.id === 'nhl-game-week:' + date);
        this.state = { ...this.state, loading: true, message: `Loading NHL game week for ${date}…` };
        this.changed(this.state);
        try {
            const bytes = await this.acquire(date, controller.signal);
            if (sequence !== this.sequence || controller.signal.aborted)
                return;
            const games = await this.project(bytes);
            const dataset = await makeScheduleDataset(date, bytes, this.now());
            if (sequence !== this.sequence || controller.signal.aborted)
                return;
            dataset.keepUpdated = !!previous && previous.requestedDate === date && previous.dataset.keepUpdated;
            const snapshot = this.snapshot(dataset, games);
            this.state = { ...this.state, loading: false, snapshot,
                message: games.length ? 'Schedule loaded in memory. Refresh to check for changes.' : 'No games in the returned week. Schedule loaded in memory.' };
            this.changed(this.state);
            if (dataset.keepUpdated) {
                try {
                    await this.storage.save(dataset, retained?.revision ?? null, { requireRefreshConsent: true });
                    if (this.state.snapshot === snapshot)
                        this.state.message = 'Schedule refreshed and saved locally.';
                }
                catch (error) {
                    if (this.state.snapshot === snapshot)
                        this.state.message = `Fresh schedule is available in memory, but saving failed. Your earlier saved copy is unchanged. ${error}`;
                }
                await this.reloadLibrary();
            }
        }
        catch (error) {
            if (sequence !== this.sequence || controller.signal.aborted)
                return;
            this.state = { ...this.state, loading: false, message: `Schedule refresh failed. ${this.state.snapshot ? 'The previous schedule remains available and may be stale.' : 'No schedule has been loaded.'} ${error}` };
        }
        finally {
            signal?.removeEventListener('abort', stop);
            if (sequence === this.sequence) {
                if (controller.signal.aborted && this.state.loading)
                    this.state = { ...this.state, loading: false, message: 'Refresh cancelled. The previous schedule remains available if loaded.' };
                this.controller = undefined;
                this.changed(this.state);
            }
        }
    }
}
