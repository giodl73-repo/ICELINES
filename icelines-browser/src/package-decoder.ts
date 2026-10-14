import { expandPackage } from './package-download.js';
import type { ReadOptions } from './network.js';

// A decoder has no engine or persistence state. End its lifetime after each
// result, cancellation or failure instead of retaining its temporary heap.
export function expandInWorker(bytes: Uint8Array, maximum: number, options: ReadOptions): Promise<Uint8Array> {
  options.signal.throwIfAborted();
  if (!Number.isSafeInteger(maximum) || maximum <= 0 || maximum > 100 * 1024 * 1024) throw new Error('Invalid expanded package limit');
  const remaining = options.deadline - (options.now ?? Date.now)();
  if (!Number.isFinite(remaining) || remaining <= 0) throw new Error('Package decompression deadline exceeded');
  if (typeof Worker === 'undefined') return expandPackage(bytes, maximum, options);
  return new Promise((resolve, reject) => {
    const worker = new Worker(new URL('./package-decoder-worker.js', import.meta.url), { type: 'module' });
    let settled = false;
    const finish = (value?: Uint8Array, error?: unknown): void => {
      if (settled) return;
      settled = true; clearTimeout(timer); options.signal.removeEventListener('abort', stop);
      worker.terminate(); worker.onmessage = worker.onerror = worker.onmessageerror = null;
      if (value) resolve(value); else reject(error);
    };
    const stop = (): void => finish(undefined, options.signal.reason);
    const timer = setTimeout(() => finish(undefined, new Error('Package decompression deadline exceeded')), remaining);
    options.signal.addEventListener('abort', stop, { once: true });
    worker.onerror = () => finish(undefined, new Error('Package decoder failed'));
    worker.onmessageerror = () => finish(undefined, new Error('Package decoder response could not be decoded'));
    worker.onmessage = (event: MessageEvent<unknown>) => {
      const response = event.data as { schema_version?: unknown; error?: unknown; bytes?: unknown; length?: unknown };
      if (!response || response.schema_version !== 1) { finish(undefined, new Error('Invalid package decoder response')); return; }
      if (typeof response.error === 'string' && response.bytes === undefined) { finish(undefined, new Error(response.error)); return; }
      if (response.error !== undefined || !(response.bytes instanceof ArrayBuffer) || typeof response.length !== 'number'
        || response.bytes.byteLength > maximum || !Number.isSafeInteger(response.length) || response.length < 0 || response.length > maximum || response.length > response.bytes.byteLength) {
        finish(undefined, new Error('Invalid package decoder response')); return;
      }
      if (options.signal.aborted) { stop(); return; }
      if ((options.now ?? Date.now)() >= options.deadline) { finish(undefined, new Error('Package decompression deadline exceeded')); return; }
      finish(new Uint8Array(response.bytes, 0, response.length));
    };
    if (options.signal.aborted) { stop(); return; }
    try {
      const input = bytes.buffer instanceof ArrayBuffer && bytes.byteOffset === 0 && bytes.byteLength === bytes.buffer.byteLength ? bytes.buffer : bytes.slice().buffer;
      worker.postMessage({ schema_version: 1, bytes: input, maximum, deadline: Date.now() + remaining }, [input]);
    } catch (error) { finish(undefined, error); }
  });
}
