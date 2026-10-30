// Local acceptance harness only. Fault controls never enter the publishable dist.
import { cp, readFile, writeFile, mkdir } from 'node:fs/promises';
const destination = new URL('../target/browser-recovery-preview/', import.meta.url);
await mkdir(destination, { recursive: true });
await cp(new URL('../icelines-browser/dist/', import.meta.url), destination, { recursive: true });
const index = await readFile(new URL('index.html', destination), 'utf8');
const controls = `<aside aria-label="Recovery test controls"><strong>Local acceptance harness</strong>
<button id="test-crash">Simulate worker crash</button>
<label><input type="checkbox" id="test-block"> Block worker restarts</label></aside>
<script src="./recovery-faults.js"></script>`;
await writeFile(new URL('index.html', destination), index.replace('<main>', controls + '<main>'));
await writeFile(new URL('recovery-faults.js', destination), `const NativeWorker = window.Worker;
let currentWorker;
window.Worker = class extends NativeWorker {
  constructor(...args) {
    if (document.getElementById('test-block').checked) throw new Error('Harness blocked worker construction');
    super(...args); currentWorker = this;
  }
};
document.getElementById('test-crash').onclick = () => {
  if (!currentWorker) return;
  currentWorker.terminate(); currentWorker.dispatchEvent(new Event('error'));
};
`);
console.log('Local recovery harness: target/browser-recovery-preview. Fault signal is synthetic; actual WASM worker termination and restoration are exercised.');
