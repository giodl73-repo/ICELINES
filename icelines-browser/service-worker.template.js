/* Generated with the exact UI/worker/WASM asset set. No dataset routes are cached. */
const manifest = __SHELL_MANIFEST__;
const base = new URL('./', self.location.href);
const prefix = 'icelines-shell:' + base.pathname + ':';
const cacheName = prefix + manifest.build;
const assets = new Map(manifest.assets.map(asset => [new URL(asset.path, base).href, asset]));
const ready = new Set();
let repair;

async function acquireShell(cache, signal) {
  for (const [url, asset] of assets) {
    const response = await fetch(url, { cache: 'no-store', credentials: 'omit', redirect: 'error', signal });
    if (!response.ok) throw new Error('Offline asset unavailable: ' + asset.path);
    const bytes = await response.clone().arrayBuffer();
    const hash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), value => value.toString(16).padStart(2, '0')).join('');
    if (bytes.byteLength !== asset.bytes || hash !== asset.sha256) throw new Error('Offline asset changed during install: ' + asset.path);
    await cache.put(url, response);
  }
}

async function repairShell() {
  const stagingName = cacheName + ':repair';
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 120_000);
  try {
    await caches.delete(stagingName);
    const staging = await caches.open(stagingName);
    await acquireShell(staging, controller.signal);
    // Verify the complete same-build replacement before touching existing files.
    const active = await caches.open(cacheName);
    for (const url of assets.keys()) await active.put(url, await staging.match(url));
  } finally {
    clearTimeout(timeout);
    await caches.delete(stagingName);
  }
}

function ownsClient(client) {
  if (!client?.id || !client.url) return false;
  const url = new URL(client.url);
  return url.origin === base.origin && url.pathname.startsWith(base.pathname);
}
async function reportShell(client) {
  if (!ownsClient(client)) return;
  let complete = false;
  try {
    if ((await caches.keys()).includes(cacheName)) {
      const cache = await caches.open(cacheName);
      complete = true;
      for (const [url, asset] of assets) {
        const response = await cache.match(url);
        if (!response?.ok) { complete = false; break; }
        const bytes = await response.arrayBuffer();
        if (bytes.byteLength !== asset.bytes) { complete = false; break; }
        const hash = Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256', bytes)), value => value.toString(16).padStart(2, '0')).join('');
        if (hash !== asset.sha256) { complete = false; break; }
      }
    }
  } catch { complete = false; }
  client.postMessage({ type: complete ? 'SHELL_READY' : 'SHELL_UNAVAILABLE', build: manifest.build });
}

self.addEventListener('install', event => {
  event.waitUntil((async () => {
    const cache = await caches.open(cacheName);
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), 120_000);
    try {
      await acquireShell(cache, controller.signal);
    } catch (error) { await caches.delete(cacheName); throw error; }
    finally { clearTimeout(timeout); }
  })());
});
self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    for (const name of await caches.keys()) if (name.startsWith(prefix) && name !== cacheName) await caches.delete(name);
    await self.clients.claim();
    for (const client of await self.clients.matchAll({ type: 'window' })) await reportShell(client);
  })());
});
self.addEventListener('fetch', event => {
  const url = new URL(event.request.url);
  // View fragments identify client state, not different application bytes.
  // Normalize before comparing the deployment root or shell allowlist.
  url.hash = '';
  if (event.request.method !== 'GET' || url.search || url.origin !== base.origin) return;
  const key = url.href === base.href ? new URL('index.html', base).href : url.href;
  if (!assets.has(key)) return;
  event.respondWith((async () => {
    const response = await (await caches.open(cacheName)).match(key);
    // Never mix a new network asset into an old UI/engine shell.
    return response ?? new Response('Offline shell incomplete. Reconnect and reload IceLines.', { status: 503 });
  })());
});
self.addEventListener('message', event => {
  const source = event.source;
  if (!ownsClient(source)) return;
  if (event.data?.type === 'CHECK_SHELL') { event.waitUntil(reportShell(source)); return; }
  if (event.data?.type === 'REPAIR_SHELL') {
    event.waitUntil((async () => {
      source.postMessage({ type: 'SHELL_REPAIRING', build: manifest.build });
      try {
        if (!repair) repair = repairShell().finally(() => { repair = undefined; });
        await repair;
        await reportShell(source);
      } catch {
        source.postMessage({ type: 'SHELL_REPAIR_FAILED', build: manifest.build });
      }
    })());
    return;
  }
  if (event.data?.type !== 'ACTIVATE_UPDATE') return;
  event.waitUntil((async () => {
    ready.add(source.id);
    const clients = (await self.clients.matchAll({ type: 'window', includeUncontrolled: true })).filter(ownsClient);
    if (clients.every(client => ready.has(client.id))) await self.skipWaiting();
    else for (const client of clients) if (!ready.has(client.id)) client.postMessage({ type: 'UPDATE_CONSENT_REQUIRED' });
  })());
});
