/* Generated with the exact UI/worker/WASM asset set. No dataset routes are cached. */
const manifest = {"schema_version":1,"build":"dd80c23b88c9646a4bcd78756ae3007a0a6724eab493f474c61826e65534fed1","worker_sha256":"0cf24ac769be0a1e0dfe20dc2b1069b3a23f1fb5596610dbc4debd87363e89b9","assets":[{"path":"build-info.json","sha256":"cc1afdf2759a91bc4a5798edd09e9602f21ffb1b7b5bb51bc28c084410f76d59","bytes":365,"gzip_bytes":268},{"path":"catalog.json","sha256":"f126628d167e3f1a52cf790b3753e57a535507e071eff8d28d6857d1f23fa377","bytes":76249,"gzip_bytes":14374},{"path":"index.html","sha256":"9b909fb8685ad835f63b4f8dd20610e6370381f02d6df71a3070b9aee6667a36","bytes":9358,"gzip_bytes":3074},{"path":"pkg/icelines_wasm.js","sha256":"9ae5be431a9bd1e78217a4650a3193a16920aab0d657dade86ba68c7c1baedbd","bytes":12348,"gzip_bytes":2914},{"path":"pkg/icelines_wasm_bg.wasm","sha256":"1344a015f6f61c6743185fabe92b37ecb39124f19826ab1d6fd4d55a7fdfe1f0","bytes":624177,"gzip_bytes":218423},{"path":"src/acquisition.js","sha256":"693a42d639e1cf543d5653301e5b319d93cb90e5a25fbcd4b416801de552265d","bytes":4917,"gzip_bytes":1790},{"path":"src/archive.js","sha256":"33b02bee6ab4d8bbc62c7ae5a4f74ea276ba15093b1cd933df5d20063187b03a","bytes":7736,"gzip_bytes":2425},{"path":"src/catalog.js","sha256":"0caf8d960ee2eb4951fa1149e3c99cc2c546eb9e7619df8c0e3bf27345edd0ae","bytes":2283,"gzip_bytes":939},{"path":"src/engine-client.js","sha256":"45ee8e9b2fddd16be789793e02a2b7b40a40ec8ec0e5dbd03bdce5666580290f","bytes":4355,"gzip_bytes":1187},{"path":"src/library.js","sha256":"d95e0942280af86bcb6660f181d17c809a9a3a04fbf6002a9691efba0e56ea96","bytes":17515,"gzip_bytes":4039},{"path":"src/live-coordination.js","sha256":"6ec18259c481b1539e9d0a098756f5e2cc6f61c6460e69009d54b792f9e9e327","bytes":2091,"gzip_bytes":806},{"path":"src/main.js","sha256":"fc92ac6e9e93e7bd888e62e35dd23edd5963539c1bbdd88072cb2c87f7d7fefe","bytes":38168,"gzip_bytes":9126},{"path":"src/network.js","sha256":"8f1102d36e112bd2d571e4e91f9cd8f08c3f4136579aa0909b53433489329d8a","bytes":5504,"gzip_bytes":1663},{"path":"src/offline.js","sha256":"7b3a90418d5a9877047ef8608ef2669b3da3cf42d25f359fdf62107dbcfecf50","bytes":4356,"gzip_bytes":1337},{"path":"src/polling.js","sha256":"5f77d7c9b3d2831e8f7cdaa3e39f48c4d2e6c82157ae4cb069af2d15611e067c","bytes":3568,"gzip_bytes":1099},{"path":"src/query-export.js","sha256":"2e4113d7c182d4dd4b73cfd05525e058d304e3fe16c1c9c99ca34d3f8c82ef0a","bytes":2084,"gzip_bytes":1037},{"path":"src/save-policy.js","sha256":"74d49598e1306777ebe19c5823eed6be4358e4dd22f127ff2ef14d96a258e1fe","bytes":334,"gzip_bytes":223},{"path":"src/schedule.js","sha256":"f103a83ba4befa4646dc17f869ff3e3206f3e180e4c810537204eedf216b492e","bytes":10496,"gzip_bytes":2154},{"path":"src/types.js","sha256":"8e609bb71c20b858c77f0e9f90bb1319db8477b13f9f965f1a1e18524bf50881","bytes":11,"gzip_bytes":31},{"path":"src/view-state.js","sha256":"a9496f24e717d31a1a06779bdd08c621fc6f052cac02f9f86ab1129e4746bcbd","bytes":3433,"gzip_bytes":1269},{"path":"src/worker.js","sha256":"ea3f8d4c649e09459ac945f9d8a5608a62bc1da094faae3a72d32b2ad33afda7","bytes":3699,"gzip_bytes":1069},{"path":"style.css","sha256":"b3f54b982d5b4f8f6ffc8977d0fd7fc990c00e02f7b6153d42817bb500e46fe6","bytes":4593,"gzip_bytes":1632}]};
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
