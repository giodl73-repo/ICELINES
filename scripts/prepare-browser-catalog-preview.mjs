// Disposable fault fixture; do not publish its modified index or fetch adapter.
import {cp,mkdir,readFile,writeFile} from 'node:fs/promises';
const destination = new URL('../target/browser-catalog-preview/',import.meta.url);
await mkdir(destination,{recursive:true});
await cp(new URL('../icelines-browser/dist/',import.meta.url),destination,{recursive:true});
const index = await readFile(new URL('index.html',destination),'utf8');
const controls = `<aside aria-label="Catalog test controls"><strong>Disposable catalog failure fixture</strong>
<label><input id="test-catalog-failure" type="checkbox"> Simulate unavailable catalog</label>
<a href="./">Restart at application root</a><p id="test-catalog-state" role="status"></p></aside>
<script src="./catalog-faults.js"></script>`;
await writeFile(new URL('index.html',destination),index.replace('<main>',controls+'<main>'));
await writeFile(new URL('catalog-faults.js',destination),`const flag = document.getElementById('test-catalog-failure');
flag.checked = sessionStorage.getItem('catalog-failure') !== 'off';
const announce = () => {document.getElementById('test-catalog-state').textContent = flag.checked ? 'Catalog reads return HTTP 404 in this test tab. Packages, engine, and saved storage are real.' : 'Catalog reads restored. Use Retry public catalog.';};
flag.onchange = () => {sessionStorage.setItem('catalog-failure',flag.checked ? 'on' : 'off'); announce();};
announce();
const nativeFetch = window.fetch.bind(window);
window.fetch = (input,options) => {
  const url = new URL(input instanceof Request ? input.url : String(input),location.href);
  if (flag.checked && url.origin === location.origin && url.pathname.endsWith('/catalog.json')) return Promise.resolve(new Response('Fixture unavailable catalog',{status:404}));
  return nativeFetch(input,options);
};
`);
console.log('Catalog startup fixture prepared in target/browser-catalog-preview. Its modified index cannot prove offline shell integrity.');
