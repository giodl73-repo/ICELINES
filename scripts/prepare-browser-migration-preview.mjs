// Disposable local acceptance setup. The application's files stay byte-identical
// to dist; these controls seed/inspect a legacy library at the same base path.
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
const destination = new URL('../target/browser-migration-preview-29/ICELINES/workbench/', import.meta.url);
await mkdir(destination, { recursive: true });
await cp(new URL('../icelines-browser/dist/', import.meta.url), destination, { recursive: true });
const catalog = JSON.parse(await readFile(new URL('catalog.json',destination),'utf8'));
const selected = catalog.packages.find(row => row.season === 20242025 && row.season_type === 'regular');
if (!selected) throw new Error('Required real season package is missing');
await writeFile(new URL('migration-setup.html',destination), `<!doctype html>
<html lang="en"><meta charset="utf-8"><title>IceLines local migration acceptance</title>
<main><h1>Local migration acceptance</h1>
<p>Seeds the real 2024–25 regular-season package into a schema-1 library.
Existing libraries are refused. No records are deleted or overwritten.</p>
<button id="seed">Seed legacy library and hold connection</button>
<button id="close" disabled>Close legacy connection</button>
<button id="inspect">Inspect saved bytes and schema</button>
<button id="old-version">Attempt older schema-one application open</button>
<a href="./">Open production application</a>
<pre id="result" role="status">Ready to seed an unused local preview.</pre></main>
<script type="module" src="./migration-setup.js"></script></html>`);
await writeFile(new URL('migration-setup.js',destination), `
const entry = ${JSON.stringify(selected)};
const namespace = 'icelines-v1:' + new URL('./',location.href).pathname;
const output = document.getElementById('result');
let held;
const hash = async bytes => Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',bytes)),n => n.toString(16).padStart(2,'0')).join('');
const run = action => async () => { try { await action(); } catch(error) { output.textContent = String(error); } };
document.getElementById('seed').onclick = run(async () => {
  if ((await indexedDB.databases()).some(db => db.name === namespace)) throw new Error('Preview library already exists; original records preserved. Use a fresh local origin.');
  const response = await fetch(entry.url,{cache:'no-store'});
  if (!response.ok) throw new Error('Package unavailable');
  const bytes = new Uint8Array(await response.arrayBuffer());
  const revision = await hash(bytes);
  if (revision !== entry.sha256) throw new Error('Package checksum mismatch');
  const metadata = JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes));
  const original = {id:'legacy:20242025',revision,bytes,metadata,keepUpdated:false};
  await new Promise((resolve,reject) => {
    const request = indexedDB.open(namespace,1);
    request.onupgradeneeded = () => {
      request.result.createObjectStore('datasets',{keyPath:'id'}).put(original);
      request.result.createObjectStore('settings').put(original.id,'active');
    };
    request.onerror = () => reject(request.error);
    request.onsuccess = () => { held = request.result; resolve(); };
  });
  held.onversionchange = () => { output.textContent = 'Upgrade requested; legacy connection deliberately held. Close it to allow migration.'; };
  document.getElementById('seed').disabled = true;
  document.getElementById('close').disabled = false;
  output.textContent = 'Schema 1 seeded; legacy connection held. SHA-256 ' + revision + '; bytes ' + bytes.length;
});
document.getElementById('close').onclick = () => {
  held?.close(); held = undefined; document.getElementById('close').disabled = true;
  output.textContent = 'Legacy connection closed. Retry storage in the production application.';
};
document.getElementById('inspect').onclick = run(async () => {
  const database = await new Promise((resolve,reject) => {
    const request = indexedDB.open(namespace);
    request.onerror = () => reject(request.error); request.onsuccess = () => resolve(request.result);
  });
  let row; let active;
  try {
    await new Promise((resolve,reject) => {
      const tx = database.transaction(['datasets','settings'],'readonly');
      const reading = tx.objectStore('datasets').get('legacy:20242025');
      const pointer = tx.objectStore('settings').get('active');
      tx.oncomplete = () => { row = reading.result; active = pointer.result; resolve(); };
      tx.onabort = () => reject(tx.error);
    });
    const revision = row?.bytes instanceof Uint8Array ? await hash(row.bytes) : null;
    output.textContent = JSON.stringify({version:database.version,stores:Array.from(database.objectStoreNames),
      id:row?.id,active,revision,expected:entry.sha256,byteLength:row?.bytes?.length,
      originalPreserved:revision === entry.sha256 && row?.revision === entry.sha256 && active === 'legacy:20242025'},null,2);
  } finally { database.close(); }
});
document.getElementById('old-version').onclick = run(async () => {
  const name = await new Promise((resolve,reject) => {
    const request = indexedDB.open(namespace,1);
    request.onerror = () => resolve(request.error?.name);
    request.onsuccess = () => { request.result.close(); reject(new Error('Older schema unexpectedly opened')); };
  });
  if (name !== 'VersionError') throw new Error('Unexpected older-schema result: ' + name);
  output.textContent = 'VersionError: older schema-one application refused. Inspect records to verify preservation.';
});
`);
console.log('Prepared target/browser-migration-preview-29. Setup controls are local-only; production shell bytes are unchanged.');
