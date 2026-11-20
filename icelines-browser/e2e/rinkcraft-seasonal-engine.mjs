// Actual native executable, actual pinned WASM, and actual browser worker RPC.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import assert from 'node:assert/strict';
import {chromium} from '../node_modules/@playwright/test/index.mjs';
const root=resolve('.'),out=resolve(process.env.ICELINES_SEASONAL_EVIDENCE??'target/compact-sheet/shared-seasonal-records-20261120/acceptance');await mkdir(out,{recursive:true});
const native=process.env.ICELINES_SEASONAL_NATIVE;if(!native)throw Error('Set ICELINES_SEASONAL_NATIVE to the built rinkcraft_seasonal_records executable.');
const pkg=resolve(process.env.ICELINES_SEASONAL_PKG??'target/compact-sheet/shared-seasonal-records-20261120/pkg');
const inputBytes=await readFile('target/compact-sheet/shared-seasonal-records-20261120/input.json'),base=JSON.parse(inputBytes);
const sha=raw=>createHash('sha256').update(raw).digest('hex');
await writeFile(join(pkg,'package.json'),'{"type":"module"}\n');
const bindings=await import(pathToFileURL(join(pkg,'icelines_wasm.js')).href);await bindings.default({module_or_path:await readFile(join(pkg,'icelines_wasm_bg.wasm'))});
const engine=new bindings.BrowserEngine(),cases=[];
try{
 assert.deepEqual(JSON.parse(engine.resident_revisions()),[]);
 for(const [gp,shots]of [[10,25],[5,25],[20,25],[10,10],[10,50]]){
  const request=structuredClone(base);request.settings.minimum_games=gp;request.settings.minimum_conversion_shots=shots;
  const bytes=gp===10&&shots===25?inputBytes:Buffer.from(JSON.stringify(request));
  const input=join(out,`input-${gp}-${shots}.json`),output=join(out,`native-${gp}-${shots}.json`);await writeFile(input,bytes);
  const run=spawnSync(native,[input,sha(bytes),output],{cwd:root,encoding:'utf8',timeout:120000});assert.equal(run.status,0,run.stderr);
  const expected=await readFile(output,'utf8'),actual=engine.seasonal_record_catalog(bytes);assert.ok(actual===expected,`${gp} GP / ${shots} SOG native-WASM byte equality; ${sha(actual)} vs ${sha(expected)}`);
  await writeFile(join(out,`wasm-${gp}-${shots}.json`),actual);
  const catalog=JSON.parse(actual);assert.equal(catalog.profiles.length,3565);assert.ok(catalog.reference_scope.includes('caller-supplied'));cases.push({minimum_games:gp,minimum_conversion_shots:shots,output_sha256:sha(actual),byte_identical:true});
 }
 // Compare only order: both variants use the same JS JSON numeric representation.
 const normalized=JSON.stringify(base),original=engine.seasonal_record_catalog(new TextEncoder().encode(normalized));
 const reversed=structuredClone(base);reversed.records.reverse();assert.ok(engine.seasonal_record_catalog(new TextEncoder().encode(JSON.stringify(reversed)))===original,'input-order byte equality');
 const invalid=[];
 const add=(name,mutate)=>{const bad=structuredClone(base);mutate(bad);invalid.push([name,Buffer.from(JSON.stringify(bad))]);};
 add('mixed competition',v=>v.season_type='playoff');
 add('duplicate actor window',v=>v.records.push(v.records[0]));
 add('zero exposure',v=>v.records[0].minutes=0);
 add('goalie in skater cohort',v=>v.records[0].group='Goalies');
 add('fractional official count',v=>v.records[0].shots=.5);
 add('unknown envelope field',v=>v.unregistered=true);
 add('tracking identity mismatch',v=>v.records.find(r=>r.edge).edge.player_id++);
 add('tracking unit mismatch',v=>v.records.find(r=>r.edge?.measures.some(m=>m.key==='max-skating-speed')).edge.measures.find(m=>m.key==='max-skating-speed').unit='Miles');
 for(const [name,bytes]of invalid)assert.throws(()=>engine.seasonal_record_catalog(bytes),undefined,name);
 assert.throws(()=>engine.seasonal_record_catalog(new Uint8Array(32*1024*1024+1)),/32 MiB/);
 assert.deepEqual(JSON.parse(engine.resident_revisions()),[]);
 const browser=await chromium.launch({channel:'msedge',headless:true}),context=await browser.newContext(),page=await context.newPage(),errors=[];
 page.on('pageerror',e=>errors.push(e.message));
 await context.route('http://seasonal.test/**',async route=>{
  const path=new URL(route.request().url()).pathname;
  if(path==='/'){await route.fulfill({contentType:'text/html',body:'<!doctype html><title>Seasonal engine acceptance</title>'});return;}
  const file=path.startsWith('/src/')?join(root,'icelines-browser/dist',path.slice(1)):path.startsWith('/pkg/')?join(pkg,path.slice(5)):null;
  if(!file){await route.abort();return;}
  await route.fulfill({contentType:path.endsWith('.wasm')?'application/wasm':'text/javascript',body:await readFile(file)});
 });
 let workerResult;
 try{
  await page.goto('http://seasonal.test/');
  workerResult=await page.evaluate(async raw=>{
   const {EngineClient}=await import('/src/engine-client.js'),failures=[];
   let ownedWorker;
   const client=new EngineClient(message=>failures.push(message),()=>{ownedWorker=new Worker('/src/worker.js',{type:'module'});return ownedWorker;},120000,{deferStart:true});
   try{
    const before=await client.request('residentRevisions',null);
    const bytes=new TextEncoder().encode(raw),buffer=bytes.buffer;
    const pending=client.request('seasonalRecords',buffer,[buffer]);const detached=buffer.byteLength===0;
    const catalog=await pending;
    const invalid=new TextEncoder().encode('{"schema":"unsupported"}').buffer;
    let errorReceived=false;try{await client.request('seasonalRecords',invalid,[invalid]);}catch{errorReceived=true;}
    const after=await client.request('residentRevisions',null);
    const current=await client.request('currentSeason',null);
    return {catalog,detached,errorReceived,before,after,current,failures};
   }finally{ownedWorker?.terminate();}
  },inputBytes.toString('utf8'));
  assert.deepEqual(workerResult.catalog,JSON.parse(await readFile(join(out,'native-10-25.json'),'utf8')));
  assert.equal(workerResult.detached,true);assert.equal(workerResult.errorReceived,true);assert.deepEqual(workerResult.before,[]);assert.deepEqual(workerResult.after,[]);assert.equal(workerResult.current,20262027);assert.deepEqual(workerResult.failures,[]);assert.deepEqual(errors,[]);
 }finally{await browser.close();}
 const result={profiles:3565,cases,native_wasm_byte_identical:true,order_invariant:true,invalid_inputs_rejected:invalid.length+1,worker_rpc:true,transfer_detached:true,resident_packages_unchanged:true,worker_recovers_after_input_error:true,page_errors:errors};
 await writeFile(join(out,'verification.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result));
}finally{engine.free();}
