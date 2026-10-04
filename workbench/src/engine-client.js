export class EngineStoppedError extends Error {
    constructor(message) { super(message); this.name = 'EngineStoppedError'; }
}
export class EngineClient {
    onFailure;
    createWorker;
    timeoutMs;
    worker;
    epoch = 0;
    nextId = 0;
    generation = 0;
    get available() { return !!this.worker; }
    pending = new Map();
    constructor(onFailure, createWorker = () => new Worker(new URL('./worker.js', import.meta.url), { type: 'module' }), timeoutMs = 120000) {
        this.onFailure = onFailure;
        this.createWorker = createWorker;
        this.timeoutMs = timeoutMs;
        this.restart();
    }
    restart() {
        this.worker?.terminate();
        this.worker = undefined;
        this.changeContext();
        const epoch = ++this.epoch;
        let worker;
        try {
            worker = this.createWorker();
        }
        catch {
            queueMicrotask(() => this.fail(epoch, 'Engine could not start.'));
            return;
        }
        this.worker = worker;
        worker.onmessage = (event) => {
            if (epoch !== this.epoch || this.worker !== worker)
                return;
            const response = event.data;
            if (!response || response.schema_version !== 1 || !Number.isSafeInteger(response.request_id)
                || !Number.isSafeInteger(response.context_generation)
                || (!!response.error === Object.hasOwn(response, 'value'))
                || (response.error && (typeof response.error.kind !== 'string' || typeof response.error.message !== 'string'))) {
                this.fail(epoch, 'Engine returned an invalid response.');
                return;
            }
            const pending = this.pending.get(response.request_id);
            if (!pending)
                return;
            if (response.context_generation !== pending.generation) {
                this.fail(epoch, 'Engine response context was invalid.');
                return;
            }
            if (response.error?.kind === 'engine_failure') {
                this.fail(epoch, response.error.message);
                return;
            }
            clearTimeout(pending.timer);
            this.pending.delete(response.request_id);
            if (response.context_generation !== this.generation)
                pending.reject(new Error('cancelled: context changed'));
            else if (response.error)
                pending.reject(new Error(response.error.message));
            else
                pending.resolve(response.value);
        };
        worker.onerror = () => this.fail(epoch, 'Engine stopped unexpectedly.');
        worker.onmessageerror = () => this.fail(epoch, 'Engine response could not be decoded.');
    }
    fail(epoch, message) {
        if (epoch !== this.epoch)
            return;
        ++this.epoch;
        this.worker?.terminate();
        this.worker = undefined;
        ++this.generation;
        for (const pending of this.pending.values()) {
            clearTimeout(pending.timer);
            pending.reject(new EngineStoppedError(message));
        }
        this.pending.clear();
        this.onFailure(message);
    }
    changeContext() {
        ++this.generation;
        for (const pending of this.pending.values()) {
            clearTimeout(pending.timer);
            pending.reject(new Error('cancelled: context changed'));
        }
        this.pending.clear();
        return this.generation;
    }
    request(operation, payload) {
        const worker = this.worker;
        if (!worker)
            return Promise.reject(new EngineStoppedError('Engine unavailable. Restart it or reload the application.'));
        const request = { schema_version: 1, request_id: ++this.nextId, context_generation: this.generation, operation, payload };
        return new Promise((resolve, reject) => {
            const epoch = this.epoch;
            const timer = setTimeout(() => this.fail(epoch, 'Engine did not respond within two minutes.'), this.timeoutMs);
            this.pending.set(request.request_id, { generation: request.context_generation, timer, resolve: value => resolve(value), reject });
            try {
                worker.postMessage(request);
            }
            catch {
                this.fail(epoch, 'Engine request could not be sent.');
            }
        });
    }
}
