// Local acceptance only: measured production-worker imports of verified release bytes.
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { gunzipSync } from 'node:zlib';
const destination = new URL('../target/browser-release-benchmark-preview-31/',import.meta.url);
const archive = await readFile(new URL('../target/browser-releases/data-20242025.tar.gz',import.meta.url));
const archive_sha256 = createHash('sha256').update(archive).digest('hex');
if (archive_sha256 !== '23ab90d4306c18299ac68d35389bc75edea5ce454765c40b6c66c6248b34d8ca') throw new Error('Real release archive digest mismatch');
await mkdir(destination,{recursive:true});
await cp(new URL('../icelines-browser/dist/',import.meta.url),destination,{recursive:true});
await writeFile(new URL('data-20242025.tar.gz',destination),archive);
await writeFile(new URL('release-input.json',destination),JSON.stringify({archive_sha256,compressed_bytes:archive.length,
  expanded_tar_bytes:gunzipSync(archive,{maxOutputLength:100*1024*1024}).length}));
await writeFile(new URL('release-benchmark.html',destination),`<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>IceLines real release import measurements</title><link rel="stylesheet" href="./style.css"><main>
<h1>Real release import measurements</h1><p>Local production worker/client/WASM, verified downloaded release bytes. Measures archive conversion separately from the file picker. No live acquisition or saving.</p>
<p>WASM linear memory excludes archive-worker JavaScript, browser overhead and transient staging peaks.</p>
<button id="measure">Measure regular and playoff imports</button><p id="status" role="status">Ready.</p><p id="summary"></p><details><summary>Measured report</summary><pre id="report" style="white-space:pre-wrap;overflow-wrap:anywhere"></pre></details></main><script type="module" src="./release-main.js"></script></html>`);
await writeFile(new URL('release-worker.js',destination),`import init from './pkg/icelines_wasm.js';
const queued = [], starts = new Map(); self.onmessage = event => queued.push(event);
self.addEventListener('message',event => starts.set(event.data.request_id,performance.now()));
const output = await init(); const send = self.postMessage.bind(self);
self.postMessage = (response,...args) => { const start = starts.get(response.request_id); starts.delete(response.request_id);
  send({...response,benchmark:{linear_memory_bytes:output.memory.buffer.byteLength,worker_ms:start === undefined ? null : performance.now()-start}},...args); };
await import('./src/worker.js'); for(const event of queued) self.onmessage(event); queued.length = 0;`);
await writeFile(new URL('release-main.js',destination),`import {EngineClient} from './src/engine-client.js';
import {digest} from './src/acquisition.js';
const status = document.getElementById('status');
document.getElementById('measure').onclick = async () => {
  document.getElementById('measure').disabled = true;
  const report = {schema_version:1,measured_at:new Date().toISOString(),user_agent:navigator.userAgent,
    note:'Local conversion round trips; file-picker latency excluded. Linear memory is not total browser or worker-JS peak memory.',samples:[],peak_worker_linear_memory_bytes:0};
  let client; let trace; let measuredWorker;
  try {
    Object.assign(report,await (await fetch('./release-input.json',{cache:'no-store'})).json());
    report.shell_build = (await (await fetch('./shell-manifest.json',{cache:'no-store'})).json()).build;
    const bytes = new Uint8Array(await (await fetch('./data-20242025.tar.gz',{cache:'no-store'})).arrayBuffer());
    if (await digest(bytes) !== report.archive_sha256) throw new Error('Archive changed');
    client = new EngineClient(message => {status.textContent=message;},() => {
      const worker = new Worker('./release-worker.js',{type:'module'});
      measuredWorker = worker;
      worker.addEventListener('message',event => { if (event.data.benchmark) { trace = event.data.benchmark;
        report.peak_worker_linear_memory_bytes = Math.max(report.peak_worker_linear_memory_bytes,trace.linear_memory_bytes); } });
      return worker;
    });
    await client.request('currentSeason',null);
    for(const seasonType of ['regular','playoff']) for(let repetition=0;repetition<6;repetition++) {
      status.textContent='Measuring '+seasonType+' import '+(repetition+1)+'/6'; client.changeContext();
      const beginning = performance.now();
      const packageBytes = await client.request('importArchive',{bytes:bytes.slice().buffer,filename:'data-20242025.tar.gz',seasonType});
      const conversion_ms=performance.now()-beginning,conversion_worker_ms=trace.worker_ms;
      const loading=performance.now(); const loaded = await client.request('load',packageBytes); const load_ms=performance.now()-loading;
      const skaters=await client.request('query',{filter:'',sort:'points',goalies:false,minimum_games:0,today:'2026-10-04'});
      const goalies=await client.request('query',{filter:'',sort:'wins',goalies:true,minimum_games:0,today:'2026-10-04'});
      if (skaters.season !== 20242025 || skaters.season_type !== seasonType || skaters.rows.length !== (seasonType==='regular'?905:332)
        || goalies.rows.length !== (seasonType==='regular'?103:27) || loaded.revision !== await digest(packageBytes)) throw new Error('Import context/count/integrity mismatch');
      report.samples.push({season_type:seasonType,repetition,conversion_ms,conversion_worker_ms,load_ms,package_bytes:packageBytes.byteLength,
        revision:loaded.revision,resident_windows:loaded.residents.length,skaters:skaters.rows.length,goalies:goalies.rows.length,leader:skaters.rows[0],
        linear_memory_bytes:trace.linear_memory_bytes});
    }
    status.textContent='Complete: 12 real archive conversions and loads.';
    document.getElementById('summary').textContent='Archive: '+report.compressed_bytes+' compressed bytes, '+report.expanded_tar_bytes+' expanded tar bytes. Slowest conversion: '+Math.max(...report.samples.map(s=>s.conversion_ms)).toFixed(1)+' ms; slowest package load: '+Math.max(...report.samples.map(s=>s.load_ms)).toFixed(1)+' ms. Peak WASM linear memory: '+(report.peak_worker_linear_memory_bytes/1048576).toFixed(2)+' MiB. Total peak memory remains unproven.';
  } catch(error) {report.error=String(error);status.textContent='Failed: '+error;}
  finally {measuredWorker?.terminate();document.getElementById('report').textContent=JSON.stringify(report,null,2);}
};`);
console.log('Prepared target/browser-release-benchmark-preview-31/release-benchmark.html; real release input and controls are local-only.');
