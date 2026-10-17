import {createHash} from 'node:crypto';
import {writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';

export async function mapTargetHeaps(session,targets,output){
 const report={scope:'Post-measurement diagnostic trace markers identify renderer PID; no forced GC, not memory acceptance.',targets:[]};
 let next=0,tracing=false;
 const command=(sessionId,method,params={})=>new Promise((accept,reject)=>{
  const id=++next;
  const received=event=>{if(event.sessionId!==sessionId)return;const m=JSON.parse(event.message);if(m.id!==id)return;clearTimeout(timer);session.off('Target.receivedMessageFromTarget',received);m.error?reject(Error(JSON.stringify(m.error))):accept(m.result);};
  const timer=setTimeout(()=>{session.off('Target.receivedMessageFromTarget',received);reject(Error(method+' timed out'));},5000);
  session.on('Target.receivedMessageFromTarget',received);
  session.send('Target.sendMessageToTarget',{sessionId,message:JSON.stringify({id,method,params})}).catch(e=>{clearTimeout(timer);session.off('Target.receivedMessageFromTarget',received);reject(e);});
 });
 try{
  await session.send('Tracing.start',{transferMode:'ReturnAsStream',traceConfig:{recordMode:'recordUntilFull',traceBufferSizeInKb:16384,includedCategories:['devtools.timeline','blink.console','disabled-by-default-memory-infra'],excludedCategories:['*'],memoryDumpConfig:{triggers:[]}}});tracing=true;
  for(const target of targets){
   if(!['page','browser_ui','service_worker','worker'].includes(target.type))continue;
   const marker='icelines-target-pid-'+report.targets.length;
   const item={type:target.type,url:target.url,marker};report.targets.push(item);
   let attached;
   try{
    attached=(await session.send('Target.attachToTarget',{targetId:target.targetId,flatten:false})).sessionId;
    const result=await command(attached,'Runtime.evaluate',{expression:`console.timeStamp(${JSON.stringify(marker)}); true`,returnByValue:true});
    if(result.exceptionDetails||result.result?.value!==true)throw Error('Marker evaluation did not return true');
    item.marker_executed=true;
    item.heap=await command(attached,'Runtime.getHeapUsage');
    if(target.type==='worker'&&target.url.endsWith('/src/worker.js')){
      const memory=await command(attached,'Runtime.evaluate',{expression:'(async()=>{const bindings=await import("../pkg/icelines_wasm.js");const wasm=await bindings.default();return wasm.memory.buffer.byteLength;})()',awaitPromise:true,returnByValue:true});
      if(memory.exceptionDetails||typeof memory.result?.value!=='number')throw Error('WASM memory unavailable');
      item.wasm_linear_bytes=memory.result.value;
    }

   }catch(e){item.error=String(e);}
   finally{if(attached)await session.send('Target.detachFromTarget',{sessionId:attached}).catch(()=>{});}
  }
  report.memory_dump=await session.send('Tracing.requestMemoryDump',{deterministic:false,levelOfDetail:'detailed'});
  if(!report.memory_dump.success)throw Error('Allocation dump refused');
  const finished=new Promise(accept=>session.once('Tracing.tracingComplete',accept));
  await session.send('Tracing.end');tracing=false;
  const {stream}=await finished;let contents='';
  for(;;){const r=await session.send('IO.read',{handle:stream});contents+=r.base64Encoded?Buffer.from(r.data,'base64').toString('utf8'):r.data;if(r.eof)break;}
  await session.send('IO.close',{handle:stream});
  await writeFile(resolve(output,'target-process-markers-trace.json'),contents);
  report.trace={file:'target-process-markers-trace.json',sha256:createHash('sha256').update(contents).digest('hex'),bytes:Buffer.byteLength(contents)};
  const events=JSON.parse(contents).traceEvents;
  for(const item of report.targets){
   const found=events.filter(e=>e.name==='TimeStamp'&&e.args?.data?.message===item.marker);
   item.trace_markers=found.map(({pid,tid,ts})=>({pid,tid,ts}));
   const pids=[...new Set(found.map(e=>e.pid))];if(pids.length===1)item.renderer_pid=pids[0];
   item.mapped=item.marker_executed===true&&pids.length===1;
  }
  report.all_mapped=report.targets.length>0&&report.targets.every(t=>t.mapped&&t.heap&&!t.error);
  return report;
 }finally{if(tracing)await session.send('Tracing.end').catch(()=>{});}
}
