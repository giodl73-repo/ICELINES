// Disposable package transport fixture; never publish this modified shell.
import {cp,mkdir,readFile,writeFile} from 'node:fs/promises';
const destination = new URL('../target/browser-package-preview/',import.meta.url);
await mkdir(destination,{recursive:true});
await cp(new URL('../icelines-browser/dist/',import.meta.url),destination,{recursive:true});
const index = await readFile(new URL('index.html',destination),'utf8');
const controls = `<aside aria-label="Package test controls"><strong>Disposable package transport fixture</strong>
<label>Package transport <select id="test-package-mode"><option value="normal">Normal</option><option value="retry">Two HTTP 503 responses</option><option value="stall">Stalled response body</option></select></label>
<p id="test-package-state" role="status">Requests: 0; cancelled bodies: 0</p></aside><script src="./package-faults.js"></script>`;
await writeFile(new URL('index.html',destination),index.replace('<main>',controls+'<main>'));
await writeFile(new URL('package-faults.js',destination),`const mode = document.getElementById('test-package-mode');
let requests = 0; let cancelled = 0;
const announce = () => {document.getElementById('test-package-state').textContent = 'Requests: '+requests+'; cancelled bodies: '+cancelled;};
mode.onchange = () => {requests = 0; cancelled = 0; announce();};
const nativeFetch = window.fetch.bind(window);
window.fetch = (input,options) => {
  const url = new URL(input instanceof Request ? input.url : String(input),location.href);
  if (url.origin === location.origin && /\\/data\\/[a-f0-9]{64}\\.json$/.test(url.pathname)) {
    requests++; announce();
    if (mode.value === 'retry' && requests <= 2) return Promise.resolve(new Response('',{status:503}));
    if (mode.value === 'stall') return Promise.resolve(new Response(new ReadableStream({cancel(){cancelled++; announce();}})));
  }
  return nativeFetch(input,options);
};
`);
console.log('Package transport fixture prepared in target/browser-package-preview. Its modified shell cannot prove offline readiness.');
