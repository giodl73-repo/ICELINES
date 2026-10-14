from pathlib import Path
s=Path('icelines-browser/e2e/offline-reopen.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'", "from '../icelines-browser/node_modules/@playwright/test/index.mjs'")
s=s.replace("'../..'","'..'").replace('target/browser-acceptance-','target/static-routing-check-')
needle='  await page.goto(url.href);'
assert s.count(needle)==1
s=s.replace(needle,"""  const network=await context.newCDPSession(page);const routed=[];
  network.on('Network.responseReceived',({response})=>{
    const path=new URL(response.url).pathname;
    if(path.includes('/data/')||path.endsWith('/style.css'))routed.push({url:response.url,status:response.status,from_service_worker:response.fromServiceWorker??false,router:response.serviceWorkerRouterInfo??null});
  });
  await network.send('Network.enable');
"""+needle)
needle="  await page.locator('#catalog').selectOption(seasons[1].id);"
assert s.count(needle)==1
s=s.replace(needle,"""  const shellStatus=await page.evaluate(async()=>{const r=await fetch('./style.css',{cache:'no-store'});await r.text();return r.status;});
  if(shellStatus!==200)throw Error('Cached style unavailable');
"""+needle)
needle='  await context.close();'
assert s.count(needle)==1
s=s.replace(needle,"""  report.static_routing_observer={scope:'Additional page Network observer for actual Chrome route source, not memory acceptance.',responses:routed};
  const data=routed.filter(r=>new URL(r.url).pathname.includes('/data/'));
  if(!data.some(r=>r.router?.matchedSourceType==='network'&&r.router?.actualSourceType==='network'))throw Error('Actual package network rule did not match');
  if(!routed.some(r=>r.url.endsWith('/style.css')&&r.from_service_worker))throw Error('Shell style did not retain service-worker cache path');
  report.evidence.actual_network_rule_matched=true;report.evidence.shell_cache_path_preserved=true;
  await network.detach();
"""+needle)
Path('target/static-routing-check.mjs').write_text(s,encoding='utf-8')
print('Prepared route-source plus actual offline reopen check; extra observer, not memory acceptance.')
