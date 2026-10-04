// Disposable localhost fixture. Production index/assets remain byte-for-byte intact.
import { cp, mkdir, writeFile } from 'node:fs/promises';
const source = new URL('../icelines-browser/dist/', import.meta.url);
const destination = new URL('../target/browser-offline-preview/', import.meta.url);
await mkdir(destination, { recursive: true });
await cp(source, destination, { recursive: true });
await writeFile(new URL('offline-test.html', destination), `<!doctype html>
<html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>IceLines offline recovery test</title><link rel="stylesheet" href="./style.css"></head>
<body><header><h1>IceLines offline recovery test</h1><p>Disposable localhost shell-cache fixture. Dataset storage is untouched.</p></header><main>
<aside aria-label="Application availability"><p id="offline-state" role="status">Offline preparation has not started.</p><button id="repair-offline" hidden>Repair offline files</button><button id="reload-app">Reload application</button><div id="update-notice" hidden><p>An application update is ready.</p><button id="update-app">Reload with update</button></div></aside>
<section><h2>Recovery controls</h2><button id="remove-style">Remove cached stylesheet (test)</button><button id="check-shell">Check cached files</button><p id="test-state" role="status">No test action performed.</p><a href="./">Open unchanged production application</a></section>
</main><script type="module">
import { registerOfflineShell } from './src/offline.js';
await registerOfflineShell();
document.getElementById('remove-style').onclick = async () => {
  try {
    const manifest = await (await fetch('./shell-manifest.json', {cache:'no-store'})).json();
    const base = new URL('./',location.href);
    const name = 'icelines-shell:' + base.pathname + ':' + manifest.build;
    const cache = await caches.open(name);
    const removed = await cache.delete(new URL('style.css',base).href);
    document.getElementById('test-state').textContent = removed ? 'Removed this test build’s cached stylesheet. Datasets untouched.' : 'Test stylesheet was not cached.';
    navigator.serviceWorker.controller?.postMessage({type:'CHECK_SHELL'});
  } catch (error) {document.getElementById('test-state').textContent = String(error);}
};
document.getElementById('check-shell').onclick = () => navigator.serviceWorker.controller?.postMessage({type:'CHECK_SHELL'});
</script></body></html>`);
console.log('Offline fixture prepared at target/browser-offline-preview; serve only on localhost for cache-loss testing.');
