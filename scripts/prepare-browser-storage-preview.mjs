// Local acceptance harness only; fault controls are never published.
import { cp, mkdir, readFile, writeFile } from 'node:fs/promises';
const destination = new URL('../target/browser-storage-preview/', import.meta.url);
await mkdir(destination, { recursive: true });
await cp(new URL('../icelines-browser/dist/', import.meta.url), destination, { recursive: true });
const index = await readFile(new URL('index.html', destination), 'utf8');
const controls = `<aside aria-label="Storage test controls"><strong>Local acceptance harness</strong>
<label><input type="checkbox" id="test-denied"> Simulate denied storage</label>
<label><input type="checkbox" id="test-quota"> Simulate quota exhaustion</label>
<button id="test-corrupt">Simulate corrupt saved digest</button>
<button id="test-restore">Restore original saved record</button><p id="test-result" role="status"></p></aside>
<script src="./storage-faults.js"></script>`;
await writeFile(new URL('index.html', destination), index.replace('<main>', controls + '<main>'));
await writeFile(new URL('storage-faults.js', destination), `const nativeDatabase = window.indexedDB;
Object.defineProperty(window, 'indexedDB', { configurable: true, get() {
  if (document.getElementById('test-denied').checked) throw new DOMException('Harness denied storage access', 'SecurityError');
  return nativeDatabase;
} });
const nativePut = IDBObjectStore.prototype.put;
IDBObjectStore.prototype.put = function(...args) {
  if (document.getElementById('test-quota').checked && this.name === 'settings') throw new DOMException('Harness quota exhausted', 'QuotaExceededError');
  return nativePut.apply(this, args);
};
let original;
async function changeRecord(restore) {
  const db = await new Promise((resolve, reject) => {
    const request = nativeDatabase.open('icelines-v1:' + new URL('./', location.href).pathname, 2);
    request.onsuccess = () => resolve(request.result); request.onerror = () => reject(request.error);
  });
  try {
    await new Promise((resolve, reject) => {
      const transaction = db.transaction(['datasets', 'settings'], 'readwrite');
      const store = transaction.objectStore('datasets');
      if (restore) { if (!original) throw new Error('No original test record'); store.put(original); }
      else {
        const pointer = transaction.objectStore('settings').get('active');
        pointer.onsuccess = () => {
          const reading = store.get(pointer.result);
          reading.onsuccess = () => {
            original = structuredClone(reading.result);
            store.put({ ...original, revision: '0'.repeat(64) });
          };
        };
      }
      transaction.oncomplete = resolve; transaction.onabort = () => reject(transaction.error);
    });
  } finally { db.close(); }
  document.getElementById('test-result').textContent = restore ? 'Original saved record restored. Retry local storage.' : 'Saved digest changed for this test. Retry local storage.';
}
document.getElementById('test-corrupt').onclick = () => { void changeRecord(false); };
document.getElementById('test-restore').onclick = () => { void changeRecord(true); };
`);
console.log('Local storage harness: target/browser-storage-preview. Denial/quota signals are synthetic; real IndexedDB transactions and the real engine remain in use.');
