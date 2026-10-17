from pathlib import Path
s=Path('icelines-browser/e2e/memory.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'","from '../icelines-browser/node_modules/@playwright/test/index.mjs'").replace("from './memory-diagnostics.mjs'","from '../icelines-browser/e2e/memory-diagnostics.mjs'").replace("from './memory-scope.mjs'","from '../icelines-browser/e2e/memory-scope.mjs'").replace("'../..'","'..'").replace('target/browser-memory-','target/passive-process-inventory-')
s=s.replace("const diagnostics = args.includes('--diagnose-gc');","const diagnostics = false;")
needle="  if (samplerExit !== 0 || samplerError) throw new Error(`Memory sampler failed: ${samplerError}`);"
assert s.count(needle)==1
s=s.replace(needle,needle+"""
  // After the sampler is terminal: no tracing, heap or target observer during
  // workload. Classify only the still-live owned process tree through Windows.
  const inventory=spawnSync('powershell.exe',['-NoProfile','-File',resolve(repository,'target/passive-browser-process-inventory.ps1'),'-RootProcessId',String(rootProcessId)],{encoding:'utf8',windowsHide:true});
  if(inventory.status!==0)throw Error('Post-sampling process inventory failed: '+inventory.stderr);
  report.passive_process_inventory=JSON.parse(inventory.stdout);
  if(report.passive_process_inventory.root_id!==rootProcessId)throw Error('Inventory root differs');
""")
needle="  report.completed = true;"
assert s.count(needle)==1
s=s.replace(needle,"""  report.provisional_250_mib_target=null;
  report.target_scope='Full original75/eight/two/20/restart workload and original flags/channel/sampler/network observer, no forced GC or added in-workload observer. Post-sampling Windows process-role inventory only; supplemental diagnostic, not acceptance. End inventory cannot classify exited/reused PIDs; preserve unmatched samples and all owned process memory.';
"""+needle)
Path('target/passive-process-inventory.mjs').write_text(s,encoding='utf-8')
print('Prepared passive post-sampler inventory; official runner unchanged, no tracing')
