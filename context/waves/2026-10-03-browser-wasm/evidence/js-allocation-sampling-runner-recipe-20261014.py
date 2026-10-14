from pathlib import Path
s=Path('icelines-browser/e2e/memory.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'", "from '../icelines-browser/node_modules/@playwright/test/index.mjs'")
s=s.replace("from './memory-diagnostics.mjs'", "from '../icelines-browser/e2e/memory-diagnostics.mjs'")
s=s.replace("from './memory-scope.mjs'", "from '../icelines-browser/e2e/memory-scope.mjs'")
s=s.replace("'../..'", "'..'").replace('target/browser-memory-', 'target/js-allocation-sampling-')
s=s.replace('let server, browserServer, browser, sampler;', 'let server, browserServer, browser, sampler, pageProfiler, rootProfiler, workerProfiler;')
needle="    console.log(`Loaded ${index + 1}/${catalog.packages.length}: ${entry.id}, residents ${residents}`);"
assert s.count(needle)==1
s=s.replace(needle,needle+"""
    if(index===0){
      pageProfiler=await context.newCDPSession(page);
      rootProfiler=await browser.newBrowserCDPSession();
      const targets=(await rootProfiler.send('Target.getTargets')).targetInfos.filter(t=>t.type==='worker'&&t.url===new URL('src/worker.js',url).href);
      expect(targets.length).toBe(1);
      const sessionId=(await rootProfiler.send('Target.attachToTarget',{targetId:targets[0].targetId,flatten:false})).sessionId;
      let id=0;
      workerProfiler={sessionId,send:(method,params={})=>new Promise((accept,reject)=>{
        const next=++id;
        const received=event=>{if(event.sessionId!==sessionId)return;const m=JSON.parse(event.message);if(m.id!==next)return;clearTimeout(timer);rootProfiler.off('Target.receivedMessageFromTarget',received);m.error?reject(Error(JSON.stringify(m.error))):accept(m.result);};
        const timer=setTimeout(()=>{rootProfiler.off('Target.receivedMessageFromTarget',received);reject(Error(method+' timed out'));},10000);
        rootProfiler.on('Target.receivedMessageFromTarget',received);
        rootProfiler.send('Target.sendMessageToTarget',{sessionId,message:JSON.stringify({id:next,method,params})}).catch(error=>{clearTimeout(timer);rootProfiler.off('Target.receivedMessageFromTarget',received);reject(error);});
      })};
      const settings={samplingInterval:16384,stackDepth:64,includeObjectsCollectedByMajorGC:true,includeObjectsCollectedByMinorGC:true};
      await pageProfiler.send('HeapProfiler.startSampling',settings);
      await workerProfiler.send('HeapProfiler.startSampling',settings);
      report.js_sampling={settings,start:'After first package has loaded and committed its query; bootstrap allocations are excluded.',scope:'Sampled cumulative JS allocation churn including naturally collected objects, not live retained memory, a leak finding, backing-store/Rust/native allocator totals or OS acceptance. Extra page/worker observers; no explicit GC or pressure commands.',worker_url:targets[0].url};
    }
""")
needle="  phase = 'resident_settled';"
assert s.count(needle)==1
s=s.replace(needle,"""  report.js_profiles={page:(await pageProfiler.send('HeapProfiler.getSamplingProfile')).profile,worker:(await workerProfiler.send('HeapProfiler.getSamplingProfile')).profile};
  for(const profile of Object.values(report.js_profiles)){
    expect(profile.samples.length).toBeGreaterThan(0);
    expect(profile.head.children.length).toBeGreaterThan(0);
  }
  await pageProfiler.send('HeapProfiler.stopSampling');await pageProfiler.detach();pageProfiler=undefined;
  await workerProfiler.send('HeapProfiler.stopSampling');
  await rootProfiler.send('Target.detachFromTarget',{sessionId:workerProfiler.sessionId});workerProfiler=undefined;
  await rootProfiler.detach();rootProfiler=undefined;
"""+needle)
s=s.replace('  report.completed = true;',"""  report.provisional_250_mib_target=null;
  report.target_scope='Full75/eight/two905/332/20/restart workload with additional JS allocation samplers after first committed package; original Chromium flags/OS sampler/network observer, no explicit GC/pressure commands. Not normal memory acceptance; allocation samples are not retained/OS/native/Rust totals.';
  report.completed = true;""")
needle='  await browser?.close();'
assert s.count(needle)==1
s=s.replace(needle,"""  if(pageProfiler){await pageProfiler.send('HeapProfiler.stopSampling').catch(()=>{});await pageProfiler.detach().catch(()=>{});}
  if(workerProfiler&&rootProfiler){await workerProfiler.send('HeapProfiler.stopSampling').catch(()=>{});await rootProfiler.send('Target.detachFromTarget',{sessionId:workerProfiler.sessionId}).catch(()=>{});}
  if(rootProfiler)await rootProfiler.detach().catch(()=>{});
"""+needle)
Path('target/js-allocation-sampling.mjs').write_text(s,encoding='utf-8')
print('Prepared page and owned-engine JS churn sampling, null acceptance target.')
