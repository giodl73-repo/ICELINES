// Frozen historical acceptance inputs, projected only through the shared Rust engine.
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath,pathToFileURL} from 'node:url';
const repo=fileURLToPath(new URL('../../',import.meta.url));
const assets=resolve(process.env.ICELINES_WINDOW_ASSETS??resolve(repo,'icelines-browser/public'));
const pkg=resolve(process.env.ICELINES_WINDOW_WASM??resolve(repo,'target/compact-sheet/inputs/pkg-finishing'));
const output=resolve(process.env.ICELINES_PROFILE_FIXTURES??resolve(repo,'target/compact-sheet/profile-windows/fixtures.json'));
const {BrowserEngine,initSync}=await import(pathToFileURL(resolve(pkg,'icelines_wasm.js')).href);
initSync({module:await readFile(resolve(pkg,'icelines_wasm_bg.wasm'))});
const e=new BrowserEngine(),catalog=JSON.parse(await readFile(resolve(assets,'catalog.json'),'utf8'));
try{
 for(const id of ['20242025-regular','20252026-regular']){
  const p=catalog.packages.find(p=>p.id===id);
  if(!p||!/^data\/[a-f0-9]{64}\.json$/.test(p.url))throw Error('Missing or invalid supplied package '+id);
  e.load_package(await readFile(resolve(assets,p.url)));
 }
 const chars=await readFile(resolve(assets,'rinkcraft-characters.json'),'utf8');
 const history=await readFile(resolve(assets,'rinkcraft-history.json'),'utf8'),fixtures={};
 for(const [key,id]of [['mcdavid',8478402],['barkov',8477493],['fox',8479323],['makar',8480069],['shesterkin',8478048]]){
  for(const mode of ['20242025','20252026','latest']){
   const selected=JSON.parse(e.numerical_snapshot(history,id,undefined,20252026,mode==='latest'?undefined:Number(mode)));
   const year=selected.selection.selected_season??Number(mode);
   const p=JSON.parse(e.rinkcraft_profile(JSON.stringify({player_id:id,season_type:'regular',first_season:year,last_season:year})));
   p.character_sheet=JSON.parse(e.character_sheet(chars,id,20262027));
   p.numerical_evidence=selected.card;p.numerical_snapshot=selected.selection;
   p.numerical_history=JSON.parse(e.numerical_history(history,id,undefined,20252026));
   fixtures[key+':'+mode]={key,id,mode,profile:p};
  }
 }
 await mkdir(resolve(output,'..'),{recursive:true});await writeFile(output,JSON.stringify(fixtures));
 console.log(JSON.stringify({fixtures:Object.keys(fixtures).length,output}));
}finally{e.free();}
