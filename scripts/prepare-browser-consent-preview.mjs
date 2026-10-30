// Disposable local acceptance fixture; no controls or fixtures enter dist.
import { cp, mkdir, writeFile } from 'node:fs/promises';
const destination = new URL('../target/browser-consent-preview-35/', import.meta.url);
await mkdir(destination, { recursive: true });
await cp(new URL('../icelines-browser/dist/', import.meta.url), destination, { recursive: true });
await cp(new URL('../icelines-sources/tests/fixtures/browser-schedule-week.json', import.meta.url), new URL('schedule-fixture.json', destination));
await writeFile(new URL('consent.html', destination), `<!doctype html><html lang="en"><meta charset="utf-8"><title>IceLines refresh consent acceptance</title><link rel="stylesheet" href="./style.css"><main><h1>Refresh consent transaction check</h1><p>Local fixtures, real IndexedDB and production engine/controller/library. Messaging is deliberately not used. No live requests.</p><button id="run">Run missed-notification checks</button><p id="status" role="status">Ready.</p><pre id="report" style="white-space:pre-wrap;overflow-wrap:anywhere"></pre></main><script type="module" src="./consent-main.js"></script></html>`);
await writeFile(new URL('consent-main.js', destination), `import {EngineClient} from './src/engine-client.js';
import {ScheduleController,scheduleStorage} from './src/schedule.js';
import {saveDataset,loadSaved,removeSaved,listSaved,savedActiveId,savedActiveScheduleId} from './src/library.js';
import {digest} from './src/acquisition.js';
const check=(value,message)=>{if(!value)throw new Error(message)};
const reportNode=document.getElementById('report'),status=document.getElementById('status');
document.getElementById('run').onclick=async()=>{
 document.getElementById('run').disabled=true; let worker;
 const report={schema_version:1,measured_at:new Date().toISOString(),user_agent:navigator.userAgent,scope:'Local fixtures; actual IndexedDB and production WASM; no notification listeners or upstream calls'};
 try {
  report.build=(await(await fetch('./shell-manifest.json',{cache:'no-store'})).json()).build;
  const engine=new EngineClient(message=>{status.textContent=message},()=>worker=new Worker(new URL('./src/worker.js',import.meta.url),{type:'module'}));
  await engine.request('currentSeason',null);
  const catalog=await(await fetch('./catalog.json',{cache:'no-store'})).json();
  const entry=catalog.packages.find(row=>row.season===20242025&&row.season_type==='regular');
  const bytes=new Uint8Array(await(await fetch('./'+entry.url,{cache:'no-store'})).arrayBuffer());
  const loaded=await engine.request('load',bytes.slice().buffer);
  const original={id:'consent-acceptance-season',revision:loaded.revision,bytes,metadata:JSON.parse(new TextDecoder().decode(bytes)),keepUpdated:true};
  await saveDataset(original,null);
  await saveDataset({...original,keepUpdated:false},original.revision);
  let error;try{await saveDataset(original,original.revision,{requireRefreshConsent:true})}catch(value){error=String(value)}
  check(error?.includes('refresh saving is disabled'),'Revoked season consent was overwritten');
  check((await loadSaved(original.id,original.revision)).keepUpdated===false,'Season policy changed');
  await removeSaved(original.id,original.revision);
  let removedError;try{await saveDataset(original,null,{requireRefreshConsent:true})}catch(value){removedError=String(value)}
  check(removedError?.includes('refresh saving is disabled'),'Removed season was recreated');
  check((await listSaved()).length===0&&await savedActiveId()===undefined,'Season library/pointer changed');
  const query=await engine.request('query',{filter:'p>=100',sort:'points',goalies:false,minimum_games:0,today:'2026-10-04'});
  check(query.rows.length===6,'Memory season became unavailable');
  report.season={revocation_error:error,removal_error:removedError,saved_count:0,memory_query_rows:query.rows.length};
  await saveDataset({...original,keepUpdated:false},null);
  const metadata={...original.metadata,source:'Local removal fixture replacement'};
  const replacementBytes=new TextEncoder().encode(JSON.stringify(metadata));
  const replacement={...original,metadata,bytes:replacementBytes,revision:await digest(replacementBytes),keepUpdated:false};
  await saveDataset(replacement,original.revision);
  let staleRemoval;try{await removeSaved(original.id,original.revision)}catch(value){staleRemoval=String(value)}
  check(staleRemoval?.includes('changed or was removed'),'Stale season removal deleted replacement');
  check((await loadSaved(replacement.id,replacement.revision)).revision===replacement.revision&&await savedActiveId()===replacement.id,'Replacement season/pointer not preserved');
  await removeSaved(replacement.id,replacement.revision);
  report.season_removal={stale_error:staleRemoval,replacement_preserved:true,pointer_preserved:true,explicit_current_removal_succeeded:(await listSaved()).length===0};
  const fixture=new Uint8Array(await(await fetch('./schedule-fixture.json',{cache:'no-store'})).arrayBuffer());
  let release,held=false;const pending=new Promise(resolve=>release=resolve);let tick=0;
  const controller=new ScheduleController(value=>engine.request('schedule',value.slice().buffer),()=>{},async()=>held?pending:fixture,()=>tick++?'2026-04-29T13:00:00Z':'2026-04-29T12:00:00Z');
  await controller.refresh('2026-04-29');await controller.save();await controller.save(true);
  const saved=await scheduleStorage.load(controller.state.entries[0]);
  held=true;const refreshing=controller.refresh('2026-04-29');
  await scheduleStorage.save({...saved,keepUpdated:false},saved.revision);
  release(fixture);await refreshing;
  const retained=await scheduleStorage.load({...controller.state.entries[0],revision:saved.revision});
  check(retained.keepUpdated===false&&controller.state.snapshot.dataset.keepUpdated===false,'Schedule consent was overwritten');
  check(controller.state.snapshot.revision!==saved.revision,'Fresh schedule not in memory');
  check(controller.state.snapshot.games.length===2,'WASM schedule projection failed');
  report.schedule={saved_revision:retained.revision,fresh_memory_revision:controller.state.snapshot.revision,saved_policy:retained.keepUpdated,memory_policy:controller.state.snapshot.dataset.keepUpdated,games:controller.state.snapshot.games.length,message:controller.state.message};
  await scheduleStorage.save(controller.state.snapshot.dataset,retained.revision);
  let staleScheduleRemoval;try{await scheduleStorage.remove(retained.id,retained.revision)}catch(value){staleScheduleRemoval=String(value)}
  check(staleScheduleRemoval?.includes('changed or was removed'),'Stale schedule removal deleted replacement');
  const current=await scheduleStorage.load({...retained,revision:controller.state.snapshot.revision});
  check(current.revision===controller.state.snapshot.revision&&await savedActiveScheduleId()===current.id,'Replacement schedule/pointer not preserved');
  await scheduleStorage.remove(current.id,current.revision);
  report.schedule_removal={stale_error:staleScheduleRemoval,replacement_preserved:true,pointer_preserved:true,explicit_current_removal_succeeded:await savedActiveScheduleId()===undefined};
  status.textContent='Passed: consent and stale-removal conflicts preserved current saved versions/pointers; explicit current removal succeeded.';
 }catch(error){report.error=String(error);status.textContent='Failed: '+error}
 finally{worker?.terminate();reportNode.textContent=JSON.stringify(report,null,2)}
};`);
console.log('Prepared target/browser-consent-preview-35/consent.html; local-only transaction acceptance.');
