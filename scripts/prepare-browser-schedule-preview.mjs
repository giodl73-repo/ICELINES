// Deterministic localhost UI evidence; this harness is outside publishable dist.
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
const root = new URL('../', import.meta.url);
const target = new URL('target/browser-schedule-preview/', root);
await mkdir(target, { recursive: true });
await cp(new URL('icelines-browser/dist/', root), target, { recursive: true });
await cp(new URL('icelines-sources/tests/fixtures/browser-schedule-week.json', root), new URL('schedule-fixture.json', target));
let html = await readFile(new URL('index.html', target), 'utf8');
html = html.replace('<main>', '<main><aside aria-label="Local schedule test controls"><p>Local test harness: schedule responses are fixtures. This does not prove live NHL access. Minute polling is accelerated to one second. Visibility/offline controls are simulated; Web Locks are real.</p><button id="fixture-valid">Use valid schedule</button><button id="fixture-invalid">Use invalid schedule</button><span id="fixture-mode">Valid fixture</span><label><input id="fixture-quota" type="checkbox"> Simulate schedule quota failure</label><label><input id="fixture-hidden" type="checkbox"> Simulate hidden tab</label><label><input id="fixture-offline" type="checkbox"> Simulate offline tab</label><label><input id="fixture-hold" type="checkbox"> Hold schedule response</label><button id="fixture-release">Release schedule response</button><p>Schedule requests in this tab: <span id="fixture-requests">0</span></p></aside>');
html = html.replace('src="./src/main.js"', 'src="./schedule-test.js"');
html = html.replace('</main>', '<section aria-label="Local export inspection"><h2>Local export inspection</h2><p>Captures the Blob passed to the production download control; this does not prove a downloaded file exists.</p><label>Last exported contents<textarea id="fixture-export" readonly rows="12" style="width:100%"></textarea></label></section></main>');
await writeFile(new URL('index.html', target), html);
await writeFile(new URL('schedule-test.js', target), `
const original = window.fetch.bind(window);
const nativeObjectURL = URL.createObjectURL.bind(URL);
URL.createObjectURL = blob => {
  if (blob instanceof Blob) void blob.text().then(text => { document.getElementById('fixture-export').value = text; });
  return nativeObjectURL(blob);
};
const fixture = await (await original('./schedule-fixture.json', {cache:'no-store'})).json();
let valid = true;
const nativeTimeout = window.setTimeout.bind(window);
window.setTimeout = (callback, delay, ...args) => nativeTimeout(callback, delay >= 60000 && delay <= 66000 ? 1000 : delay, ...args);
Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => document.getElementById('fixture-hidden').checked ? 'hidden' : 'visible' });
Object.defineProperty(navigator, 'onLine', { configurable: true, get: () => !document.getElementById('fixture-offline').checked });
document.getElementById('fixture-hidden').onchange = () => document.dispatchEvent(new Event('visibilitychange'));
document.getElementById('fixture-offline').onchange = () => window.dispatchEvent(new Event(document.getElementById('fixture-offline').checked ? 'offline' : 'online'));
let requests = 0, release;
document.getElementById('fixture-release').onclick = () => release?.();
const nativePut = IDBObjectStore.prototype.put;
IDBObjectStore.prototype.put = function (...args) {
  if (this.name === 'settings' && args[1] === 'activeSchedule' && document.getElementById('fixture-quota').checked) throw new DOMException('Harness quota failure', 'QuotaExceededError');
  return nativePut.apply(this, args);
};
document.getElementById('fixture-valid').onclick = () => { valid = true; document.getElementById('fixture-mode').textContent = 'Valid fixture'; };
document.getElementById('fixture-invalid').onclick = () => { valid = false; document.getElementById('fixture-mode').textContent = 'Invalid fixture'; };
window.fetch = async (input, options) => {
  const url = String(input);
  if (url.startsWith('https://api-web.nhle.com/v1/schedule/')) {
    document.getElementById('fixture-requests').textContent = String(++requests);
    if (document.getElementById('fixture-hold').checked) await new Promise((resolve, reject) => {
      const stop = () => { release = undefined; reject(options.signal.reason); };
      release = () => { options.signal.removeEventListener('abort', stop); release = undefined; resolve(); };
      options.signal.addEventListener('abort', stop, {once:true});
    });
    return new Response(JSON.stringify(valid ? fixture : {gameWeek:[]}));
  }
  return original(input, options);
};
await import('./src/main.js');
`);
console.log('Prepared local fixture schedule preview at ' + target.pathname);
