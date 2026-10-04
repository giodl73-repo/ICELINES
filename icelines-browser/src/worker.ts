import type { EngineRequest, EngineResponse, QueryRequest } from './types.js';
import { importArchive } from './archive.js';
interface Engine {
  load_package(bytes: Uint8Array): string; query(request: string): string;
  player_detail(id: number): string; current_season(): number; free(): void;
  schedule_week(bytes: Uint8Array): string;
  select_resident(revision: string): void; resident_revisions(): string; unload_active(): void;
}
interface Bindings { default(): Promise<unknown>; BrowserEngine: new () => Engine; }
const ready = (async () => {
  const bindings = await import(new URL('../pkg/icelines_wasm.js', import.meta.url).href) as Bindings;
  await bindings.default();
  return bindings;
})();
// Keep initialization rejection handled until a request can receive a typed error.
void ready.catch(() => undefined);
let engine: Engine | undefined;
let generation = 0;
let chain = Promise.resolve();
self.onmessage = (event: MessageEvent<EngineRequest>) => {
  const request = event.data;
  // Advance generation on arrival, before queued initialization/execution.
  if (request.context_generation > generation) generation = request.context_generation;
  chain = chain.then(async () => {
    const response: EngineResponse = { schema_version: 1, request_id: request.request_id, context_generation: request.context_generation };
    try {
      if (request.schema_version !== 1) throw new Error('unsupported schema version');
      let bindings: Bindings;
      try { bindings = await ready; }
      catch { response.error = { kind: 'engine_failure', message: 'WASM engine initialization failed.' }; self.postMessage(response); return; }
      if (request.context_generation !== generation) throw new Error('cancelled: context changed');
      engine ??= new bindings.BrowserEngine();
      switch (request.operation) {
        case 'importArchive': {
          const input = request.payload as { bytes: ArrayBuffer; filename: string; seasonType: 'regular' | 'playoff' };
          const bytes = await importArchive(new Uint8Array(input.bytes), input.filename, input.seasonType);
          if (request.context_generation !== generation) throw new Error('cancelled: context changed');
          response.value = bytes.buffer; break;
        }
        case 'load': {
          const revision = engine.load_package(new Uint8Array(request.payload as ArrayBuffer));
          response.value = { revision, residents: JSON.parse(engine.resident_revisions()) }; break;
        }
        case 'query': response.value = JSON.parse(engine.query(JSON.stringify(request.payload as QueryRequest))); break;
        case 'detail': response.value = JSON.parse(engine.player_detail(Number(request.payload))); break;
        case 'currentSeason': response.value = engine.current_season(); break;
        case 'schedule': response.value = JSON.parse(engine.schedule_week(new Uint8Array(request.payload as ArrayBuffer))); break;
        case 'selectResident': engine.select_resident(String(request.payload)); response.value = true; break;
        case 'residentRevisions': response.value = JSON.parse(engine.resident_revisions()); break;
        case 'unload': engine.unload_active(); response.value = true; break;
        default: throw new Error('invalid operation');
      }
    } catch (error) {
      const message = String(error);
      const kind = error instanceof WebAssembly.RuntimeError ? 'engine_failure' : message.includes('cancelled') ? 'cancelled' : message.includes('missing data') ? 'missing_data' : 'engine_error';
      response.error = { kind, message };
    }
    self.postMessage(response);
  });
};
