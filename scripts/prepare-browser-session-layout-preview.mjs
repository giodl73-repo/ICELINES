// Local narrow-context acceptance; production files stay unmodified.
import { cp, mkdir, writeFile } from 'node:fs/promises';
const destination=new URL('../target/browser-responsive-preview-32/',import.meta.url);
await mkdir(new URL('ICELINES/workbench/',destination),{recursive:true});
await cp(new URL('../icelines-browser/dist/',import.meta.url),new URL('ICELINES/workbench/',destination),{recursive:true});
await writeFile(new URL('mobile-test.html',destination),`<!doctype html><html lang="en"><meta charset="utf-8"><title>IceLines 360px session layout</title>
<style>body{margin:0;padding:16px;font:14px system-ui;background:#e5edf5}h1{font-size:18px}p,pre{max-width:650px}button{padding:10px;margin:0 8px 8px 0}iframe{display:block;width:360px;height:800px;border:0;background:white}pre{white-space:pre-wrap;overflow-wrap:anywhere}</style>
<h1>IceLines — 360px embedded viewport</h1><p>Production UI in a measured narrow browsing context. Local controls use normal app handlers. This is layout evidence, not physical phone hardware, touch or screen-reader coverage.</p>
<button id="seasons">Load nine public windows and open session library</button><button id="long-source">Import long-source layout fixture</button><button id="inspect">Inspect containment</button><button id="session">Show session library</button><p id="status" role="status">Loading application.</p><pre id="report"></pre>
<iframe title="IceLines narrow layout" src="./ICELINES/workbench/#/leaders?season=20242025&amp;type=regular&amp;kind=skaters&amp;sort=points&amp;gp=&amp;filter=p%3E%3D100"></iframe><script type="module" src="./mobile-controls.js"></script></html>`);
await writeFile(new URL('mobile-controls.js',destination),`
const frame=document.querySelector('iframe'),status=document.getElementById('status');
const wait=(doc,test)=>new Promise((resolve,reject)=>{
  let observer;const timer=setTimeout(()=>{observer?.disconnect();reject(new Error('Application state timeout'));},20000);
  const check=()=>{if(test()){clearTimeout(timer);observer?.disconnect();resolve();}};
  observer=new MutationObserver(check);observer.observe(doc.documentElement,{subtree:true,childList:true,characterData:true,attributes:true});check();
});
const run=action=>async()=>{try{await action();}catch(error){status.textContent=String(error);}};
const showSession=()=>{const details=frame.contentDocument.getElementById('resident-library').closest('details');details.open=true;details.scrollIntoView();};
document.getElementById('seasons').onclick=run(async()=>{
  const doc=frame.contentDocument;
  await wait(doc,()=>doc.getElementById('context')?.textContent==='20242025 · regular'&&!doc.getElementById('load').disabled);
  for(const id of ['20252026-regular','20242025-regular','20242025-playoff','20232024-regular','20232024-playoff','20222023-regular','20222023-playoff','20212022-regular','20212022-playoff']){
    doc.getElementById('catalog').value=id;doc.getElementById('load').click();
    await wait(doc,()=>doc.getElementById('context').textContent===id.replace('-',' · ')&&doc.getElementById('status').textContent.includes('memory'));
  }
  showSession();status.textContent='Nine windows loaded; session count '+doc.getElementById('resident-count').textContent;
});
document.getElementById('long-source').onclick=run(async()=>{
  const doc=frame.contentDocument;
  const catalog=await(await fetch('./ICELINES/workbench/catalog.json',{cache:'no-store'})).json();
  const entry=catalog.packages.find(row=>row.season===20242025&&row.season_type==='regular');
  const value=await(await fetch('./ICELINES/workbench/'+entry.url,{cache:'no-store'})).json();
  value.source='Layout fixture '+'X'.repeat(512);
  const transfer=new DataTransfer();transfer.items.add(new File([JSON.stringify(value)],'layout-fixture.json',{type:'application/json'}));
  const input=doc.getElementById('import');input.files=transfer.files;input.dispatchEvent(new Event('change',{bubbles:true}));
  await wait(doc,()=>doc.getElementById('status').textContent==='Imported into memory. Save locally to keep it.');
  showSession();status.textContent='Long-source fixture imported through the production handler.';
});
document.getElementById('session').onclick=showSession;
document.getElementById('inspect').onclick=()=>{
  const doc=frame.contentDocument;
  document.getElementById('report').textContent=JSON.stringify({viewport:frame.contentWindow.innerWidth,client:doc.documentElement.clientWidth,scroll:doc.documentElement.scrollWidth,
    sessionCount:doc.getElementById('resident-count').textContent,sourceFixture:doc.getElementById('resident-library').textContent.includes('X'.repeat(512)),
    tables:Array.from(doc.querySelectorAll('.table-scroll')).map(node=>({name:node.getAttribute('aria-label'),client:node.clientWidth,scroll:node.scrollWidth})),
    pageContained:doc.documentElement.scrollWidth===doc.documentElement.clientWidth},null,2);
};
frame.onload=()=>{status.textContent='Application document loaded. Its normal engine/query startup is running.';};
`);
console.log('Prepared target/browser-responsive-preview-32/mobile-test.html; controls are outside production worker scope.');
