import {chromium} from '../icelines-browser/node_modules/@playwright/test/index.mjs';
import {createServer} from 'node:http';
import {readFile,mkdtemp,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
import {gzipSync} from 'node:zlib';
import assert from 'node:assert/strict';
const historical=JSON.parse(await readFile('target/viewport-runtime-attribution-5klpj9/report.json','utf8'));
const expectedModule=historical.native_profile.profile.modules.find(m=>m.name==='chrome.dll.pdb');
const raw=Buffer.alloc(1024*1024);let random=0x12345678;
for(let i=0;i<raw.length;i++){random^=random<<13;random^=random>>>17;random^=random<<5;raw[i]=random&255;}
const compressed=gzipSync(raw);
const server=createServer((req,res)=>{
 if(req.url==='/wire.gz')res.writeHead(200,{'Content-Type':'application/octet-stream'}).end(compressed);
 else res.writeHead(200,{'Content-Type':'text/html'}).end('<!doctype html><html><body><p>Owned allocation probe</p></body></html>');
});
await new Promise(r=>server.listen(0,'127.0.0.1',r));
const out=await mkdtemp(resolve('target/native-operation-fingerprints-'));
const report={completed:false,scope:'Artificial isolated allocator-path probes, not app workload/resource/latency acceptance. Extra native sampling at4096-byte intervals, no explicit GC/pressure commands. Stack matches identify shared native call paths only, not object ownership, leaks or removable bytes.',provisional_250_mib_target:null,operations:[]};
async function operation(kind){
 if(kind==='control')return 0;
 if(kind==='typed-array'){globalThis.__held=globalThis.__inputs.map(input=>new Uint8Array(input.length).fill(61));return globalThis.__held.reduce((n,x)=>n+x.length,0);}
 if(kind==='clone-buffer'){globalThis.__held=globalThis.__inputs.map(input=>structuredClone(input.buffer));return globalThis.__held.reduce((n,x)=>n+x.byteLength,0);}
 if(kind==='dom'){const fragment=document.createDocumentFragment();for(let i=0;i<5000;i++){const span=document.createElement('span');span.textContent=String(i);fragment.append(span);}document.body.append(fragment);return document.querySelectorAll('span').length;}
 if(kind==='gzip'){
  const wire=globalThis.__wire;let position=0;
  const reader=new ReadableStream({pull(controller){if(position===wire.length){controller.close();return;}const end=Math.min(wire.length,position+8192);controller.enqueue(wire.subarray(position,end));position=end;}}).pipeThrough(new DecompressionStream('gzip')).getReader();
  const bytes=new Uint8Array(1024*1024);let length=0;
  try{for(;;){const {done,value}=await reader.read();if(done)break;if(value.length>bytes.length-length)throw Error('Overflow');bytes.set(value,length);length+=value.length;}}
  finally{await reader.cancel();}
  globalThis.__held=[bytes];return length;
 }
 throw Error('Unknown probe operation');
}
try{
 for(const kind of ['control','typed-array','clone-buffer','dom','gzip']){
  const browser=await chromium.launch({channel:'chromium',headless:true});let session;
  try{
   const page=await browser.newPage();await page.goto(`http://127.0.0.1:${server.address().port}/`);
   await page.evaluate(async()=>{globalThis.__inputs=Array.from({length:8},(_,i)=>new Uint8Array(262144+i*4096).fill(61));globalThis.__held=[];globalThis.__wire=new Uint8Array(await(await fetch('/wire.gz')).arrayBuffer());});
   await page.evaluate(operation,'control');
   session=await page.context().newCDPSession(page);
   await session.send('Memory.startSampling',{samplingInterval:4096});
   const result=await page.evaluate(operation,kind);
   const profile=(await session.send('Memory.getSamplingProfile')).profile;
   await session.send('Memory.stopSampling');await session.detach();session=undefined;
   const row={kind,result,browser:browser.version(),profile};report.operations.push(row);
   const module=profile.modules.find(m=>m.name==='chrome.dll.pdb');
   row.chrome_module_identity_match=module?true:null;
   if(module){assert.equal(module.uuid,expectedModule.uuid);assert.equal(module.size,expectedModule.size);}
   if(kind==='typed-array'||kind==='clone-buffer')assert.equal(result,8*262144+4096*28);
   if(kind==='dom')assert.equal(result,5000);
   if(kind==='gzip'){
    assert.equal(result,raw.length);
    const hash=await page.evaluate(async()=>Array.from(new Uint8Array(await crypto.subtle.digest('SHA-256',globalThis.__held[0]))).map(x=>x.toString(16).padStart(2,'0')).join(''));
    assert.equal(hash,createHash('sha256').update(raw).digest('hex'));
   }
   console.log(kind,'samples',profile.samples.length);
  }finally{if(session){await session.send('Memory.stopSampling').catch(()=>{});await session.detach().catch(()=>{});}await browser.close();}
 }
 report.completed=true;
}finally{await new Promise(r=>server.close(r));await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');console.log('Probe report:',resolve(out,'report.json'));}
