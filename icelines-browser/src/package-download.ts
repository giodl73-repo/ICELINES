import type { CatalogEntry } from './types.js';
import { digest, MAX_BYTES } from './acquisition.js';
import { readPublicBytes, type ReadOptions } from './network.js';

function releaseTemporaryBuffer(buffer: ArrayBufferLike): void {
  if (!(buffer instanceof ArrayBuffer)) return;
  // Optional early disposal; older browsers retain ordinary GC ownership.
  try { (buffer as ArrayBuffer & { transfer?: (length: number) => ArrayBuffer }).transfer?.(0); }
  catch { /* Disposal is optional and must not change acquisition results. */ }
}

export async function expandPackage(bytes: Uint8Array, maximum: number, options: ReadOptions): Promise<Uint8Array> {
  if (!Number.isSafeInteger(maximum) || maximum <= 0 || maximum > MAX_BYTES) throw new Error('Invalid expanded package limit');
  const now = options.now ?? Date.now;
  const check = (): void => {
    options.signal.throwIfAborted();
    if (now() >= options.deadline) throw new Error('Package decompression deadline exceeded');
  };
  check();
  // Feed the authenticated view directly, including its byte offset. Write each
  // decoded chunk into the bounded destination instead of retaining all chunks
  // alongside a second complete copy at the end.
  const input = bytes.buffer instanceof ArrayBuffer ? new Uint8Array(bytes.buffer,bytes.byteOffset,bytes.byteLength) : Uint8Array.from(bytes);
  const stream = new ReadableStream<BufferSource>({ start(controller) { controller.enqueue(input); controller.close(); } })
    .pipeThrough(new DecompressionStream('gzip'));
  const reader = stream.getReader();
  const expanded = new Uint8Array(maximum); let length = 0;
  const temporary = new Set<ArrayBufferLike>();
  const stop = (): void => { void reader.cancel().catch(() => {}); };
  const timer = setTimeout(stop, Math.max(0, options.deadline - now()));
  options.signal.addEventListener('abort', stop, {once:true});
  try {
    for (;;) {
      check(); const {done,value} = await reader.read(); check();
      if (done) {
        // Wait for EOF: queued output views may share their backing buffer.
        // Every chunk has been copied into our independent bounded destination.
        for (const buffer of temporary) releaseTemporaryBuffer(buffer);
        temporary.clear(); break;
      }
      if (value.length > maximum - length) throw new Error('Expanded package exceeds its advertised byte limit');
      expanded.set(value,length); length += value.length;
      temporary.add(value.buffer);
    }
    return expanded.subarray(0,length);
  } finally { clearTimeout(timer); options.signal.removeEventListener('abort',stop); await reader.cancel().catch(() => {}); }
}

// JSON remains the interchange format and revision source. Transport bytes are
// separately bounded/authenticated; decompression failures never downgrade reads.
export async function downloadPackage(entry: CatalogEntry, base: URL, options: ReadOptions): Promise<Uint8Array> {
  const compressed = entry.transport && typeof DecompressionStream !== 'undefined';
  const wire = compressed ? entry.transport! : entry;
  const bytes = await readPublicBytes(new URL(wire.url,base),wire.bytes,options);
  if (bytes.length !== wire.bytes || await digest(bytes) !== wire.sha256) throw new Error('Package transport size or checksum mismatch');
  const expanded = compressed ? await expandPackage(bytes,entry.bytes,options) : bytes;
  // readPublicBytes allocated this private transport buffer; decoding finished.
  // The raw fallback and expandPackage's caller-owned input are never detached.
  if (compressed) releaseTemporaryBuffer(bytes.buffer);
  if (expanded.length !== entry.bytes || await digest(expanded) !== entry.sha256) throw new Error('Package size or checksum mismatch');
  return expanded;
}
