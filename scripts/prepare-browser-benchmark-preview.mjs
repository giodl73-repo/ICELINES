// Local measurement harness. No instrumentation enters the publishable app.
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
const destination = new URL('../target/browser-benchmark-preview/', import.meta.url);
await mkdir(destination, { recursive: true });
await cp(new URL('../icelines-browser/dist/', import.meta.url), destination, { recursive: true });
const shell = JSON.parse(await readFile(new URL('shell-manifest.json', destination), 'utf8'));
await writeFile(new URL('benchmark.html', destination), `<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>IceLines browser engine measurements</title><link rel="stylesheet" href="./style.css"></head><body><main>
<h1>IceLines browser engine measurements</h1><p>Local harness: actual production worker/client/WASM. One selected season and up to eight resident windows. Public package data only; no live requests or local saves.</p>
<p>Worker linear memory is its allocated high-water mark, not total browser memory. Query round trips include worker transport and JSON projection, but exclude table rendering. Startup includes harness instrumentation.</p>
<button id="run">Measure all public packages</button><p id="status" role="status">Ready to measure.</p><p id="summary" role="status"></p><details><summary>Raw benchmark report</summary><pre id="report" style="white-space:pre-wrap;overflow-wrap:anywhere"></pre></details>
</main><script type="module" src="./benchmark-main.js"></script></body></html>`);
await writeFile(new URL('benchmark-size.json', destination), JSON.stringify({
  shell_bytes: shell.assets.reduce((sum, row) => sum + row.bytes, 0),
  shell_gzip_bytes_sum: shell.assets.reduce((sum, row) => sum + row.gzip_bytes, 0),
  shell_build: shell.build,
}));
await writeFile(new URL('benchmark-worker.js', destination), `import init from './pkg/icelines_wasm.js';
const queued = [], starts = new Map();
self.onmessage = event => queued.push(event);
self.addEventListener('message', event => starts.set(event.data.request_id, performance.now()));
const output = await init();
const send = self.postMessage.bind(self);
self.postMessage = (response, ...args) => {
  const start = starts.get(response.request_id); starts.delete(response.request_id);
  send({ ...response, benchmark: { linear_memory_bytes: output.memory.buffer.byteLength,
    worker_ms: start === undefined ? null : performance.now() - start } }, ...args);
};
await import('./src/worker.js');
for (const event of queued) self.onmessage(event);
queued.length = 0;
`);
await writeFile(new URL('benchmark-main.js', destination), `import { EngineClient } from './src/engine-client.js';
import { boundedBytes, digest } from './src/acquisition.js';
const status = document.getElementById('status'), reportNode = document.getElementById('report');
const summary = values => {
  const sorted = values.slice().sort((a,b) => a-b);
  return { samples: sorted.length, minimum: sorted[0], median: sorted[Math.floor(sorted.length / 2)],
    p95: sorted[Math.min(sorted.length - 1, Math.ceil(sorted.length * .95) - 1)], maximum: sorted.at(-1) };
};
document.getElementById('run').onclick = async () => {
  document.getElementById('run').disabled = true;
  const result = { schema_version: 1, measured_at: new Date().toISOString(), user_agent: navigator.userAgent,
    note: 'Local desktop measurement; no mobile or total-browser-memory claim. One selected package; up to eight resident windows under the production input budget. No archive expansion measurement.',
    packages: [], peak_worker_linear_memory_bytes: 0, sampled_main_js_heap_peak_bytes: null };
  let client;
  const telemetry = [];
  const sample = () => { if (performance.memory) result.sampled_main_js_heap_peak_bytes = Math.max(result.sampled_main_js_heap_peak_bytes ?? 0, performance.memory.usedJSHeapSize); };
  try {
    Object.assign(result, await (await fetch('./benchmark-size.json', {cache:'no-store'})).json());
    const catalog = await (await fetch('./catalog.json', {cache:'no-store'})).json();
    const starting = performance.now();
    client = new EngineClient(message => { status.textContent = message; }, () => {
      const worker = new Worker('./benchmark-worker.js', {type:'module'});
      worker.addEventListener('message', event => {
        if (event.data.benchmark) {
          telemetry.push(event.data.benchmark);
          result.peak_worker_linear_memory_bytes = Math.max(result.peak_worker_linear_memory_bytes, event.data.benchmark.linear_memory_bytes);
        }
      });
      return worker;
    });
    result.engine_current_season = await client.request('currentSeason', null);
    result.instrumented_startup_ms = performance.now() - starting; sample();
    for (const entry of catalog.packages) {
      status.textContent = 'Measuring ' + entry.id + ' (' + (result.packages.length + 1) + '/' + catalog.packages.length + ')';
      const downloading = performance.now();
      const bytes = await boundedBytes(await fetch('./' + entry.url, {cache:'no-store',credentials:'omit'}));
      if (bytes.length !== entry.bytes || await digest(bytes) !== entry.sha256) throw new Error('Package integrity mismatch');
      const download_ms = performance.now() - downloading;
      client.changeContext();
      const loading = performance.now(); const loaded = await client.request('load', bytes.buffer); const load_ms = performance.now() - loading;
      if (loaded.revision !== entry.sha256 || loaded.residents.length > 8) throw new Error('Resident load contract mismatch');
      const request = {filter:'gp>=10 AND p>=20', sort:'points', goalies:false, minimum_games:0, today:'2026-10-03'};
      const cold = performance.now(); const initial = await client.request('query', request); const cold_query_ms = performance.now() - cold;
      const roundtrips = [], workerTimes = [];
      for (let i = 0; i < 20; i++) {
        const before = performance.now(); await client.request('query', request); roundtrips.push(performance.now() - before);
        workerTimes.push(telemetry.at(-1).worker_ms); sample();
      }
      result.packages.push({ id:entry.id, season:entry.season, season_type:entry.season_type, bytes:entry.bytes,
        download_and_integrity_ms:download_ms, load_ms, cold_query_ms, filtered_rows:initial.rows.length,
        warm_roundtrip_ms:summary(roundtrips), warm_worker_ms:summary(workerTimes),
        worker_linear_memory_bytes:telemetry.at(-1).linear_memory_bytes, resident_window_count:loaded.residents.length });
    }
    const acrossPackages = result.packages.map(row => row.warm_roundtrip_ms.p95);
    result.package_p95_roundtrip_ms = summary(acrossPackages);
    result.provisional_checks = { shell_under_10_mib_gzip: result.shell_gzip_bytes_sum <= 10 * 1024 * 1024,
      each_package_warm_roundtrip_p95_under_500_ms: acrossPackages.every(value => value <= 500),
      total_desktop_peak_memory_under_250_mib: 'unproven: linear memory and main heap samples exclude worker JS/browser/native overhead' };
    document.getElementById('summary').textContent = 'Shell: ' + (result.shell_gzip_bytes_sum / 1048576).toFixed(3) + ' MiB gzip (summed assets). Instrumented startup: ' + result.instrumented_startup_ms.toFixed(1) + ' ms. Worst package warm-query p95: ' + result.package_p95_roundtrip_ms.maximum.toFixed(1) + ' ms. Peak WASM linear memory: ' + (result.peak_worker_linear_memory_bytes / 1048576).toFixed(2) + ' MiB. Full browser peak memory remains unproven.';
    status.textContent = 'Measurements complete: ' + result.packages.length + ' public packages.';
  } catch (error) { result.error = String(error); status.textContent = 'Measurement failed: ' + error; }
  finally { sample(); reportNode.textContent = JSON.stringify(result, null, 2); }
};
`);
console.log('Local benchmark harness: target/browser-benchmark-preview/benchmark.html');
