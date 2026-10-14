from pathlib import Path
s=Path('icelines-browser/e2e/memory.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'","from '../icelines-browser/node_modules/@playwright/test/index.mjs'").replace("from './memory-diagnostics.mjs'","from '../icelines-browser/e2e/memory-diagnostics.mjs'").replace("from './memory-scope.mjs'","from '../icelines-browser/e2e/memory-scope.mjs'").replace("'../..'","'..'").replace('target/browser-memory-','target/dom-counter-diagnostic-')
s=s.replace("const diagnostics = args.includes('--diagnose-gc');","const diagnostics = false;")
needle="  phase = 'startup';"
assert s.count(needle)==1
s=s.replace(needle,"""  report.dom_checkpoints=[];
  const captureDOM=async label=>{
    const session=await context.newCDPSession(page);
    try{
      const counters=await session.send('Memory.getDOMCounters');
      const live=await page.evaluate(()=>({elements:document.getElementsByTagName('*').length,mounted_players:document.querySelectorAll('#rows tr[data-result-index]').length,result_count:document.querySelector('#rows')?.dataset.resultCount??null,library_items:document.querySelectorAll('#resident-library > li').length}));
      report.dom_checkpoints.push({label,at:new Date().toISOString(),counters,live,latest_os_sample:samples.at(-1)});
      console.log('DOM checkpoint '+label+': '+JSON.stringify({counters,live}));
    }finally{await session.detach();}
  };
"""+needle)
needle="  const catalog = JSON.parse(await readFile(resolve(root, 'catalog.json'), 'utf8'));"
assert s.count(needle)==1
s=s.replace(needle,"  await expect(page.locator('#offline-state')).toContainText('Application shell available offline',{timeout:60000});\n  await captureDOM('ready_before_engine');\n"+needle)
needle="    await expect.poll(() => samples.filter(sample => sample.phase === phase).length, { timeout: 15000 }).toBeGreaterThanOrEqual(2);"
assert s.count(needle)==1
s=s.replace(needle,needle+"\n    if([0,7,9,10,25,74].includes(index))await captureDOM('loaded_'+(index+1));")
needle="  phase = 'engine_restarted';"
assert s.count(needle)==1
s=s.replace(needle,"  await captureDOM('after_twenty_queries');\n"+needle)
needle="  await pause(2000);"
assert s.count(needle)==1
s=s.replace(needle,needle+"\n  await captureDOM('after_restart');")
needle="  report.completed = true;"
s=s.replace(needle,"""  report.provisional_250_mib_target={sampled_private_bytes_below_target:null,sampled_sum_working_set_below_target:null,note:'Additional DOM-counter CDP and live-element observations affect timing; diagnostic only, never normal acceptance.'};
  report.target_scope='Full75/eight/two/20/restart with original sampler/flags/channel and no forced GC. Nine additional DOM-counter observations, not a normal benchmark. DOM counters may include collected-later objects and other contexts; live element count covers current document only. No heap snapshot or tracing.';
  if(report.dom_checkpoints.length!==9)throw Error('Incomplete DOM counter observations');
"""+needle)
Path('target/dom-counter-diagnostic.mjs').write_text(s,encoding='utf-8')
print('Prepared nine DOM-counter observations with null acceptance; original runner unchanged')
