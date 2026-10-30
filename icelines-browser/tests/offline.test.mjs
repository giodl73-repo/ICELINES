import test from 'node:test';
import assert from 'node:assert/strict';
import vm from 'node:vm';
import { webcrypto } from 'node:crypto';
import { readFile } from 'node:fs/promises';

const code = await readFile(new URL('../dist/sw.js', import.meta.url), 'utf8');
const manifest = JSON.parse(await readFile(new URL('../dist/shell-manifest.json', import.meta.url), 'utf8'));
const fixtureAssets = new Map(await Promise.all(manifest.assets.map(async asset => [asset.path, await readFile(new URL('../dist/' + asset.path, import.meta.url))])));
function harness({ corrupt = false, timers = {setTimeout,clearTimeout}, baseURL = 'https://example.test/ICELINES/' } = {}) {
  const listeners = new Map(); const storage = new Map(); const fetched = []; const clients = [];
  let activated = 0; let claimed = 0;
  let network = path => new Response(corrupt ? 'wrong build' : fixtureAssets.get(path));
  const cache = () => {
    const records = new Map();
    return { records, put: async (key, response) => records.set(key, response.clone()), match: async key => records.get(key)?.clone() };
  };
  const caches = { open: async name => { if (!storage.has(name)) storage.set(name, cache()); return storage.get(name); }, keys: async () => [...storage.keys()], delete: async name => storage.delete(name) };
  const self = { location: { href: new URL('sw.js',baseURL).href },
    addEventListener: (name, callback) => listeners.set(name, callback),
    skipWaiting: async () => { activated++; },
    clients: { claim: async () => { claimed++; }, matchAll: async () => clients } };
  vm.runInNewContext(code, { self, caches, URL, Response, Uint8Array, crypto: webcrypto, AbortController, ...timers,
    fetch: async (url, options) => {
      fetched.push({ url, options });
      const path = new URL(url).pathname.slice(new URL(baseURL).pathname.length);
      assert.ok(fixtureAssets.has(path), 'Must fetch only shell allowlist');
      return network(path, options);
    } });
  async function dispatch(name, fields = {}) {
    let pending; let response;
    listeners.get(name)({ ...fields, waitUntil: promise => { pending = promise; }, respondWith: promise => { response = promise; } });
    await pending;
    return response ? await response : undefined;
  }
  return { storage, fetched, clients, caches, dispatch, setNetwork: callback => { network = callback; }, activated: () => activated, claimed: () => claimed };
}
test('offline install verifies and caches the complete shell, excluding datasets', async () => {
  const h = harness(); await h.dispatch('install');
  assert.equal(h.fetched.length, manifest.assets.length);
  assert.ok(manifest.assets.some(asset => asset.path === 'pkg/icelines_wasm_bg.wasm'));
  assert.ok(!manifest.assets.some(asset => asset.path.startsWith('data/')));
  for (const { options } of h.fetched) assert.equal(options.cache, 'no-store');
  assert.equal(h.activated(), 0, 'Installing an update must not activate it silently');
});
test('changed deployment asset rejects installation and removes the incomplete cache', async () => {
  const h = harness({ corrupt: true }); await assert.rejects(h.dispatch('install'), /changed during install/);
  assert.equal(h.storage.size, 0);
});
test('root/hash shell reload resolves the cached index under the Pages project path', async () => {
  const h = harness(); await h.dispatch('install');
  const response = await h.dispatch('fetch', { request: new Request('https://example.test/ICELINES/') });
  assert.match(await response.text(), /Browser workbench/);
  assert.equal(h.fetched.length, manifest.assets.length, 'Cached requests must not fetch newer network files');
});

test('bookmarked fragment routes resolve cached HTML under the composed workbench path', async () => {
  const baseURL = 'https://example.test/ICELINES/workbench/';
  const h = harness({baseURL}); await h.dispatch('install');
  for (const path of ['','#/leaders?season=20242025&type=regular&filter=p+%3E%3D+100',
    'index.html#/leaders?data=local','style.css#theme']) {
    const response = await h.dispatch('fetch',{request:new Request(baseURL+path)});
    assert.ok(response,`Expected cached shell response for ${path}`);
    assert.equal(response.status,200);
    if (!path.startsWith('style')) assert.match(await response.text(),/Browser workbench/);
  }
  for (const path of ['?private=secret#/leaders','data/package.json#/leaders','../index.html#/leaders']) {
    assert.equal(await h.dispatch('fetch',{request:new Request(new URL(path,baseURL))}),undefined);
  }
  assert.equal(h.fetched.length,manifest.assets.length,'Fragment reload must not fetch or persist dataset bytes');
});
test('live/session packages, query-bearing URLs and other deployment paths are never intercepted', async () => {
  const h = harness();
  for (const path of ['https://api.nhle.com/stats/rest/en/skater/bios', 'https://example.test/ICELINES/data/abc.json', 'https://example.test/ICELINES/style.css?token=private', 'https://example.test/OTHER/style.css']) {
    assert.equal(await h.dispatch('fetch', { request: new Request(path) }), undefined);
  }
  assert.equal(await h.dispatch('fetch', { request: new Request('https://example.test/ICELINES/style.css', { method: 'POST' }) }), undefined);
  assert.equal(h.storage.size, 0);
});
test('missing cached engine cannot fall back to a mismatched network build', async () => {
  const h = harness();
  const response = await h.dispatch('fetch', { request: new Request('https://example.test/ICELINES/pkg/icelines_wasm_bg.wasm') });
  assert.equal(response.status, 503); assert.equal(h.fetched.length, 0);
});
test('activation removes only this deployment’s old shell caches', async () => {
  const h = harness(); await h.dispatch('install');
  await h.caches.open('icelines-shell:/ICELINES/:old');
  await h.caches.open('icelines-shell:/OTHER/:old');
  await h.caches.open('personal-data');
  await h.dispatch('activate');
  assert.equal(h.claimed(), 1);
  assert.ok(!h.storage.has('icelines-shell:/ICELINES/:old'));
  assert.ok(h.storage.has('icelines-shell:/OTHER/:old')); assert.ok(h.storage.has('personal-data'));
});

test('readiness checks actual cached bytes without network or unrelated-client messages', async () => {
  const h = harness(); await h.dispatch('install'); const messages = [];
  const source = {id:'one',url:'https://example.test/ICELINES/#/leaders',postMessage: message => messages.push(message)};
  await h.dispatch('message',{source,data:{type:'CHECK_SHELL'}});
  assert.deepEqual(messages.map(message => message.type),['SHELL_READY']);
  assert.equal(messages[0].build,manifest.build);
  assert.equal(h.fetched.length,manifest.assets.length,'Readiness must not acquire data or silently repair cache');
  await h.dispatch('message',{source:{...source,url:'https://example.test/OTHER/'},data:{type:'CHECK_SHELL'}});
  await h.dispatch('message',{source:{...source,url:'https://unrelated.test/ICELINES/'},data:{type:'CHECK_SHELL'}});
  assert.equal(messages.length,1);
});

test('registered worker cannot report ready after cache loss, missing asset or changed bytes', async () => {
  for (const corruption of ['cache','missing','changed']) {
    const h = harness(); await h.dispatch('install'); const messages = [];
    const shell = [...h.storage.values()][0]; const key = [...shell.records.keys()].find(key => key.endsWith('style.css'));
    if (corruption === 'cache') h.storage.clear();
    else if (corruption === 'missing') shell.records.delete(key);
    else {
      const bytes = fixtureAssets.get('style.css'); const changed = Buffer.from(bytes); changed[0] ^= 1;
      shell.records.set(key,new Response(changed)); // Same length; hash must catch it.
    }
    await h.dispatch('message',{source:{id:'one',url:'https://example.test/ICELINES/',postMessage: message => messages.push(message)},data:{type:'CHECK_SHELL'}});
    assert.deepEqual(messages.map(message => message.type),['SHELL_UNAVAILABLE'],corruption);
    assert.equal(h.fetched.length,manifest.assets.length);
  }
});

test('activation broadcasts readiness only to this deployment', async () => {
  const h = harness(); await h.dispatch('install'); const messages = [];
  h.clients.push({id:'one',url:'https://example.test/ICELINES/',postMessage: message => messages.push(message)},
    {id:'other',url:'https://example.test/OTHER/',postMessage() {assert.fail('Unrelated deployment received readiness');}});
  await h.dispatch('activate'); assert.deepEqual(messages.map(message => message.type),['SHELL_READY']);
});

test('explicit repair restores a lost cache using only verified same-build shell assets', async () => {
  const h = harness(); await h.dispatch('install'); h.storage.clear(); const messages = [];
  await h.dispatch('message', {source:{id:'one',url:'https://example.test/ICELINES/',postMessage: message => messages.push(message)},data:{type:'REPAIR_SHELL'}});
  assert.deepEqual(messages.map(message => message.type), ['SHELL_REPAIRING','SHELL_READY']);
  assert.equal(h.storage.size,1,'Staging cache is removed');
  assert.equal(h.fetched.length,manifest.assets.length * 2);
  const response = await h.dispatch('fetch',{request:new Request('https://example.test/ICELINES/')});
  assert.match(await response.text(), /Browser workbench/);
  assert.equal(h.activated(),0,'Repair must not activate a different build');
  for (const {options} of h.fetched) {
    assert.equal(options.credentials,'omit'); assert.equal(options.redirect,'error'); assert.ok(options.signal);
  }
});

test('failed repair preserves existing files and never installs a newer network build', async () => {
  for (const failure of ['changed','network']) {
    const h = harness(); await h.dispatch('install'); const original = [...h.storage.values()][0];
    const before = new Map(await Promise.all([...original.records].map(async ([url,response]) => [url,Buffer.from(await response.clone().arrayBuffer())])));
    const lastPath = manifest.assets.at(-1).path;
    h.setNetwork(path => {
      if (path === lastPath) {
        if (failure === 'network') throw new Error('offline');
        return new Response('new deployment');
      }
      return new Response(fixtureAssets.get(path));
    });
    const messages = [];
    await h.dispatch('message',{source:{id:'one',url:'https://example.test/ICELINES/',postMessage: message => messages.push(message)},data:{type:'REPAIR_SHELL'}});
    assert.deepEqual(messages.map(message => message.type),['SHELL_REPAIRING','SHELL_REPAIR_FAILED']);
    assert.equal(h.storage.size,1);
    for (const [url,bytes] of before) assert.deepEqual(Buffer.from(await original.records.get(url).clone().arrayBuffer()),bytes);
  }
});

test('concurrent repair requests share one acquisition and report to each requesting tab', async () => {
  const h = harness(); const messages = []; let release; let entered;
  const gate = new Promise(resolve => {release = resolve;});
  const started = new Promise(resolve => {entered = resolve;});
  h.setNetwork(async path => {entered(); await gate; return new Response(fixtureAssets.get(path));});
  const source = id => ({id,url:'https://example.test/ICELINES/',postMessage: message => messages.push([id,message.type])});
  const first = h.dispatch('message',{source:source('one'),data:{type:'REPAIR_SHELL'}});
  await started;
  const second = h.dispatch('message',{source:source('two'),data:{type:'REPAIR_SHELL'}});
  release(); await Promise.all([first,second]);
  assert.equal(h.fetched.length,manifest.assets.length);
  for (const id of ['one','two']) assert.deepEqual(messages.filter(([sender]) => sender === id).map(([,type]) => type),['SHELL_REPAIRING','SHELL_READY']);
});

test('unrelated clients cannot trigger shell repair', async () => {
  const h = harness();
  await h.dispatch('message',{source:{id:'other',url:'https://example.test/OTHER/',postMessage(){assert.fail('Unrelated repair');}},data:{type:'REPAIR_SHELL'}});
  assert.equal(h.fetched.length,0); assert.equal(h.storage.size,0);
});

test('stalled repair aborts at the shell deadline and leaves a retryable failure', async () => {
  let deadline; let cleared = false;
  const h = harness({timers:{setTimeout(callback,milliseconds){assert.equal(milliseconds,120_000); deadline = callback; return 17;},clearTimeout(id){assert.equal(id,17); cleared = true;}}});
  h.setNetwork((_path,{signal}) => new Promise((_resolve,reject) => {
    signal.addEventListener('abort',() => reject(new Error('deadline')),{once:true});
    queueMicrotask(() => deadline());
  }));
  const messages = [];
  await h.dispatch('message',{source:{id:'one',url:'https://example.test/ICELINES/',postMessage: message => messages.push(message)},data:{type:'REPAIR_SHELL'}});
  assert.deepEqual(messages.map(message => message.type),['SHELL_REPAIRING','SHELL_REPAIR_FAILED']);
  assert.equal(cleared,true); assert.equal(h.storage.size,0); assert.equal(h.fetched.length,1);
});
test('update requires consent from every app tab and ignores unrelated sources', async () => {
  const h = harness(); const messages = [];
  h.clients.push({ id: 'one', url: 'https://example.test/ICELINES/', postMessage: message => messages.push(message) },
    { id: 'two', url: 'https://example.test/ICELINES/#/leaders', postMessage: message => messages.push(message) },
    { id: 'other', url: 'https://example.test/OTHER/', postMessage() { assert.fail('unrelated application'); } });
  await h.dispatch('message', { source: h.clients[2], data: { type: 'ACTIVATE_UPDATE' } });
  assert.equal(h.activated(), 0);
  await h.dispatch('message', { source: h.clients[0], data: { type: 'ACTIVATE_UPDATE' } });
  assert.equal(h.activated(), 0); assert.equal(messages[0].type, 'UPDATE_CONSENT_REQUIRED');
  await h.dispatch('message', { source: h.clients[1], data: { type: 'ACTIVATE_UPDATE' } });
  assert.equal(h.activated(), 1);
});
