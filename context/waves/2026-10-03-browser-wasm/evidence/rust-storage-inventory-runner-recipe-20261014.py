from pathlib import Path
s=Path('icelines-browser/e2e/memory.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'", "from '../icelines-browser/node_modules/@playwright/test/index.mjs'")
s=s.replace("from './memory-diagnostics.mjs'", "from '../icelines-browser/e2e/memory-diagnostics.mjs'")
s=s.replace("from './memory-scope.mjs'", "from '../icelines-browser/e2e/memory-scope.mjs'")
s=s.replace("'../..'", "'..'").replace('target/browser-memory-', 'target/rust-storage-inventory-')
needle='  let viewportRenderer = false;'
assert s.count(needle)==1
s=s.replace(needle,"""  await page.addInitScript(() => {
    globalThis.__rustStorageSnapshots=[];
    const OriginalWorker=globalThis.Worker;
    globalThis.Worker=class extends OriginalWorker {
      constructor(url,options){
        super(url,options);
        this.addEventListener('message',event=>{
          if(event.data?.value?.diagnostic)globalThis.__rustStorageSnapshots.push(event.data.value.diagnostic);
        });
      }
    };
  });
"""+needle)
needle='  report.completed = true;'
assert s.count(needle)==1
s=s.replace(needle,"""  report.provisional_250_mib_target=null;
  report.target_scope='Diagnostic instrumented Rust/worker plus extra response observer. Full75/eight/two/20/restart workload and original flags/sampler/network observer, no forced GC. Partial Rust requested-capacity inventory, not complete/additive OS memory or acceptance.';
  report.rust_storage_snapshots=await page.evaluate(()=>globalThis.__rustStorageSnapshots);
  expect(report.rust_storage_snapshots.map(s=>s.load)).toEqual([1,8,10,11,26,75,76,77]);
  for(const snapshot of report.rust_storage_snapshots){
    expect(snapshot.storage.windows.length).toBeLessThanOrEqual(8);
    expect(snapshot.wasm_linear_memory_bytes%65536).toBe(0);
    for(const window of snapshot.storage.windows){
      expect(window.source_vec_reserved_bytes).toBeGreaterThanOrEqual(window.source_vec_used_bytes);
      expect(window.repository.stint_vec_capacity_bytes).toBeGreaterThanOrEqual(window.repository.stint_vec_used_bytes);
    }
  }
"""+needle)
Path('target/rust-storage-inventory.mjs').write_text(s,encoding='utf-8')
print('Prepared scoped full-workload Rust inventory diagnostic; null acceptance target.')
