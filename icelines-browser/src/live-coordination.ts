// A deployment-wide lock covers acquisition only, never imported/local bytes.
// No response, private query, or persistence preference is shared or cached.
const scope = 'icelines-live:' + new URL('../', import.meta.url).pathname;
function browserLocks(): Pick<LockManager, 'request'> | undefined {
  try { return typeof navigator !== 'undefined' ? navigator.locks : undefined; } catch { return undefined; }
}
export class LiveReadCoordinator {
  private tail: Promise<unknown> = Promise.resolve();
  readonly crossTab: boolean;
  constructor(private readonly locks = browserLocks(), private readonly timeoutMs = 120000) { this.crossTab = !!locks; }
  async run<T>(signal: AbortSignal, action: (signal: AbortSignal) => Promise<T>): Promise<T> {
    signal.throwIfAborted(); const controller = new AbortController();
    const stop = (): void => controller.abort(signal.reason);
    signal.addEventListener('abort', stop, { once: true });
    const timer = setTimeout(() => controller.abort(new Error('Live acquisition deadline exceeded')), this.timeoutMs);
    let abort: (() => void) | undefined;
    const cancelled = new Promise<never>((_resolve, reject) => {
      abort = () => reject(controller.signal.reason);
      controller.signal.addEventListener('abort', abort, { once: true });
    });
    try {
      const read = async (): Promise<T> => { controller.signal.throwIfAborted(); return Promise.race([action(controller.signal), cancelled]); };
      let work: Promise<T>;
      if (this.locks) work = (async () => await this.locks!.request(scope, { mode: 'exclusive', signal: controller.signal }, read))();
      else {
        work = this.tail.catch(() => undefined).then(read);
        this.tail = work.catch(() => undefined);
      }
      return await Promise.race([work, cancelled]);
    } finally {
      clearTimeout(timer); signal.removeEventListener('abort', stop);
      if (abort) controller.signal.removeEventListener('abort', abort);
    }
  }
}
export const liveReads = new LiveReadCoordinator();
