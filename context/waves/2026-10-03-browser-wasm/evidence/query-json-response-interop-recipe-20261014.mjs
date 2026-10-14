import {chromium,expect} from '../icelines-browser/node_modules/@playwright/test/index.mjs';
import {createServer} from 'node:http';
import {readFile,stat,mkdtemp,writeFile} from 'node:fs/promises';
import {resolve,extname,sep} from 'node:path';
import {pathToFileURL} from 'node:url';
import {spawnSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
const root=resolve('target/query-json-response-artifact');
const control=resolve('target/chunked-decoder-input-artifact');
const out=await mkdtemp(resolve('target/query-json-interoperability-'));
for(const folder of [root,control]){
 const info=JSON.parse(await readFile(resolve(folder,'build-info.json'),'utf8'));
 const shell=JSON.parse(await readFile(resolve(folder,'shell-manifest.json'),'utf8'));
 const verified=spawnSync('python',['scripts/verify-browser-artifact.py',folder,info.commit,shell.build],{encoding:'utf8'});
 if(verified.status!==0)throw Error(verified.stderr);
}
const catalog=JSON.parse(await readFile(resolve(root,'catalog.json'),'utf8'));
const entry=catalog.packages[0];
const raw=await readFile(resolve(root,entry.url));
const bindings=await import(pathToFileURL(resolve(root,'pkg/icelines_wasm.js')));
await bindings.default({module_or_path:await readFile(resolve(root,'pkg/icelines_wasm_bg.wasm'))});
const engine=new bindings.BrowserEngine();engine.load_package(raw);
const request={filter:'',sort:'points',goalies:false,minimum_games:0,today:'2026-10-14'};
const expectedSkaters=JSON.parse(engine.query(JSON.stringify(request)));
const expectedGoalies=JSON.parse(engine.query(JSON.stringify({...request,goalies:true,sort:'wins'})));
const expectedDetail=JSON.parse(engine.player_detail(expectedSkaters.rows[0].player_id));engine.free();
const report={verified:false,scope:'Owned real-worker query protocol fixture; new/new, old-worker/new-client and new-worker/old-client. SW is blocked for mixed module delivery; not production identity, offline, memory or publication proof. Expected complete payloads are produced by the exact retained WASM in Node, not independent domain algorithm validation.',variants:[]};
for(const variant of ['new-new','old-worker','old-client']){
 const replacement=variant==='old-worker' ? 'src/worker.js' : variant==='old-client' ? 'src/engine-client.js' : null;
 const server=createServer(async(req,res)=>{try{
  const prefix='/ICELINES/workbench/';const path=decodeURIComponent(new URL(req.url,'http://localhost').pathname);
  if(!path.startsWith(prefix))throw Error();const relative=path.slice(prefix.length)||'index.html';
  const folder=relative===replacement ? control : root;const file=resolve(folder,relative);
  if(!file.startsWith(folder+sep)||!(await stat(file)).isFile())throw Error();
  const mime={'.js':'application/javascript','.wasm':'application/wasm','.json':'application/json','.html':'text/html','.css':'text/css'};
  res.writeHead(200,{'Content-Type':mime[extname(file)]||'application/octet-stream','Cache-Control':'no-store'}).end(await readFile(file));
 }catch{res.writeHead(404).end();}});
 await new Promise(r=>server.listen(0,'127.0.0.1',r));
 const browser=await chromium.launch({channel:'chromium',headless:true});
 try{
  const context=await browser.newContext({serviceWorkers:'block'});const page=await context.newPage();
  await page.addInitScript(()=>{
   globalThis.__queryProtocol={requests:[],responses:[]};const Original=globalThis.Worker;
   globalThis.Worker=class extends Original{
    constructor(...args){super(...args);this.addEventListener('message',event=>{const d=event.data;globalThis.__queryProtocol.responses.push({request_id:d.request_id,encoding:d.response_encoding??null,value_type:typeof d.value,error:d.error??null});});}
    postMessage(message,transfer=[]){globalThis.__queryProtocol.requests.push({request_id:message.request_id,operation:message.operation,encoding:message.response_encoding??null});super.postMessage(message,transfer);}
   };
  });
  const url=`http://127.0.0.1:${server.address().port}/ICELINES/workbench/`;
  await page.goto(url);await expect(page.locator('#catalog option').first()).toBeAttached();
  const result=await page.evaluate(async({packagePath,request})=>{
   const {EngineClient}=await import(new URL('src/engine-client.js',location.href).href);
   const failures=[];const client=new EngineClient(message=>failures.push(message));
   const bytes=await(await fetch(new URL(packagePath,location.href))).arrayBuffer();
   const load=await client.request('load',bytes,[bytes]);
   const skaters=await client.request('query',request);
   const goalies=await client.request('query',{...request,goalies:true,sort:'wins'});
   const detail=await client.request('detail',skaters.rows[0].player_id);
   return {load,skaters,goalies,detail,failures,protocol:globalThis.__queryProtocol};
  },{packagePath:entry.url,request});
  assert.deepEqual(result.skaters,expectedSkaters);assert.deepEqual(result.goalies,expectedGoalies);assert.deepEqual(result.detail,expectedDetail);assert.deepEqual(result.failures,[]);
  const queries=result.protocol.requests.filter(r=>r.operation==='query');assert.equal(queries.length,2);
  for(const query of queries){
   assert.equal(query.encoding,variant==='old-client'?null:'json');
   const reply=result.protocol.responses.find(r=>r.request_id===query.request_id);assert.ok(reply);
   assert.equal(reply.encoding,variant==='new-new'?'json':null);assert.equal(reply.value_type,variant==='new-new'?'string':'object');
  }
  report.variants.push({variant,replaced_module:replacement,replacement_sha256:replacement?createHash('sha256').update(await readFile(resolve(control,replacement))).digest('hex'):null,skater_rows:result.skaters.rows.length,goalie_rows:result.goalies.rows.length,all_query_fields_and_order_match:true,detail_match:true,protocol:result.protocol});
 }finally{await browser.close();await new Promise(r=>server.close(r));}
}
report.verified=true;await writeFile(resolve(out,'report.json'),JSON.stringify(report,null,2)+'\n');
console.log('All3 real-worker protocol variants preserve complete skater/goalie/detail payloads:',resolve(out,'report.json'));
