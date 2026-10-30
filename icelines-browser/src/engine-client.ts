import type { EngineRequest, EngineResponse, Operation } from './types.js';
type WorkerHandle = Pick<Worker, 'onmessage' | 'onerror' | 'onmessageerror' | 'postMessage' | 'terminate'>;
export class EngineStoppedError extends Error { constructor(message: string) { super(message); this.name = 'EngineStoppedError'; } }
export class EngineClient {
  private worker?: WorkerHandle;
  private epoch = 0;
  private nextId = 0;
  generation = 0;
  get available(): boolean { return !!this.worker; }
  private pending = new Map<number, { generation: number; resolve: (value: unknown) => void; reject: (reason: Error) => void; timer: ReturnType<typeof setTimeout> }>();
  constructor(private onFailure: (message: string) => void,
    private createWorker: () => WorkerHandle = () => new Worker(new URL('./worker.js', import.meta.url), { type: 'module' }),
    private timeoutMs = 120000) { this.restart(); }
  restart(): void {
    this.worker?.terminate(); this.worker = undefined;
    this.changeContext(); const epoch = ++this.epoch;
    let worker: WorkerHandle;
    try { worker = this.createWorker(); }
    catch { queueMicrotask(() => this.fail(epoch, 'Engine could not start.')); return; }
    this.worker = worker;
    worker.onmessage = (event: MessageEvent<EngineResponse>) => {
      if (epoch !== this.epoch || this.worker !== worker) return;
      const response = event.data;
      if (!response || response.schema_version !== 1 || !Number.isSafeInteger(response.request_id)
        || !Number.isSafeInteger(response.context_generation)
        || (!!response.error === Object.hasOwn(response, 'value'))
        || (response.error && (typeof response.error.kind !== 'string' || typeof response.error.message !== 'string'))) {
        this.fail(epoch, 'Engine returned an invalid response.'); return;
      }
      const pending = this.pending.get(response.request_id);
      if (!pending) return;
      if (response.context_generation !== pending.generation) { this.fail(epoch, 'Engine response context was invalid.'); return; }
      if (response.error?.kind === 'engine_failure') { this.fail(epoch, response.error.message); return; }
      clearTimeout(pending.timer); this.pending.delete(response.request_id);
      if (response.context_generation !== this.generation) pending.reject(new Error('cancelled: context changed'));
      else if (response.error) pending.reject(new Error(response.error.message));
      else pending.resolve(response.value);
    };
    worker.onerror = () => this.fail(epoch, 'Engine stopped unexpectedly.');
    worker.onmessageerror = () => this.fail(epoch, 'Engine response could not be decoded.');
  }
  private fail(epoch: number, message: string): void {
    if (epoch !== this.epoch) return;
    ++this.epoch; this.worker?.terminate(); this.worker = undefined; ++this.generation;
    for (const pending of this.pending.values()) { clearTimeout(pending.timer); pending.reject(new EngineStoppedError(message)); }
    this.pending.clear(); this.onFailure(message);
  }
  changeContext(): number {
    ++this.generation;
    for (const pending of this.pending.values()) { clearTimeout(pending.timer); pending.reject(new Error('cancelled: context changed')); }
    this.pending.clear(); return this.generation;
  }
  request<T>(operation: Operation, payload: unknown): Promise<T> {
    const worker = this.worker;
    if (!worker) return Promise.reject(new EngineStoppedError('Engine unavailable. Restart it or reload the application.'));
    const request: EngineRequest = { schema_version: 1, request_id: ++this.nextId, context_generation: this.generation, operation, payload };
    return new Promise<T>((resolve, reject) => {
      const epoch = this.epoch;
      const timer = setTimeout(() => this.fail(epoch, 'Engine did not respond within two minutes.'), this.timeoutMs);
      this.pending.set(request.request_id, { generation: request.context_generation, timer, resolve: value => resolve(value as T), reject });
      try { worker.postMessage(request); }
      catch { this.fail(epoch, 'Engine request could not be sent.'); }
    });
  }
}
