// A deployment-wide lock covers acquisition only, never imported/local bytes.
// No response, private query, or persistence preference is shared or cached.
const scope = 'icelines-live:' + new URL('../', import.meta.url).pathname;
function browserLocks() {
    try {
        return typeof navigator !== 'undefined' ? navigator.locks : undefined;
    }
    catch {
        return undefined;
    }
}
export class LiveReadCoordinator {
    locks;
    timeoutMs;
    tail = Promise.resolve();
    crossTab;
    constructor(locks = browserLocks(), timeoutMs = 120000) {
        this.locks = locks;
        this.timeoutMs = timeoutMs;
        this.crossTab = !!locks;
    }
    async run(signal, action) {
        signal.throwIfAborted();
        const controller = new AbortController();
        const stop = () => controller.abort(signal.reason);
        signal.addEventListener('abort', stop, { once: true });
        const timer = setTimeout(() => controller.abort(new Error('Live acquisition deadline exceeded')), this.timeoutMs);
        let abort;
        const cancelled = new Promise((_resolve, reject) => {
            abort = () => reject(controller.signal.reason);
            controller.signal.addEventListener('abort', abort, { once: true });
        });
        try {
            const read = async () => { controller.signal.throwIfAborted(); return Promise.race([action(controller.signal), cancelled]); };
            let work;
            if (this.locks)
                work = (async () => await this.locks.request(scope, { mode: 'exclusive', signal: controller.signal }, read))();
            else {
                work = this.tail.catch(() => undefined).then(read);
                this.tail = work.catch(() => undefined);
            }
            return await Promise.race([work, cancelled]);
        }
        finally {
            clearTimeout(timer);
            signal.removeEventListener('abort', stop);
            if (abort)
                controller.signal.removeEventListener('abort', abort);
        }
    }
}
export const liveReads = new LiveReadCoordinator();
