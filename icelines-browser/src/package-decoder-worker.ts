import { expandPackage } from './package-download.js';

self.onmessage = async (event: MessageEvent<unknown>) => {
  try {
    const input = event.data as { schema_version?: unknown; bytes?: unknown; maximum?: unknown; deadline?: unknown };
    if (!input || input.schema_version !== 1 || !(input.bytes instanceof ArrayBuffer)
      || input.bytes.byteLength > 100 * 1024 * 1024
      || typeof input.maximum !== 'number' || typeof input.deadline !== 'number' || !Number.isFinite(input.deadline)) throw new Error('Invalid package decoder request');
    const bytes = await expandPackage(new Uint8Array(input.bytes), input.maximum,
      { signal: new AbortController().signal, deadline: input.deadline });
    self.postMessage({ schema_version: 1, bytes: bytes.buffer, length: bytes.length }, { transfer: [bytes.buffer] });
  } catch (error) {
    self.postMessage({ schema_version: 1, error: String(error) });
  } finally { self.close(); }
};
