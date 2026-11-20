// Package Rust-generated public card data and the existing shared renderer.
import {readFile,writeFile,mkdir,cp,rm} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
const [dataDirectory,outputDirectory]=process.argv.slice(2);
if(!outputDirectory)throw Error('Usage: RUST_CATALOG_DIRECTORY OUTPUT_DIRECTORY');
const source=resolve(dataDirectory),out=resolve(outputDirectory);
if(source===out)throw Error('Source and output must differ.');
const raw=await readFile(resolve(source,'catalog.json')),catalog=JSON.parse(raw);
if(catalog.schema!=='rinkcraft.public-preview.v1'||!catalog.windows.length)throw Error('Expected public preview catalog.');
await mkdir(out,{recursive:true});
await cp(resolve(source,'data'),resolve(out,'data'),{recursive:true});
await writeFile(resolve(out,'catalog.json'),raw);
try{await writeFile(resolve(out,'edge-coverage.json'),await readFile(resolve(source,'edge-coverage.json')));}catch(e){if(e.code!=='ENOENT')throw e;await rm(resolve(out,'edge-coverage.json'),{force:true});}
for(const [from,to] of [
 ['icelines-browser/rinkcraft-preview.html','index.html'],['icelines-browser/rinkcraft-preview.css','rinkcraft-preview.css'],
 ['icelines-web/static/rinkcraft-character.css','rinkcraft-character.css'],['icelines-web/static/rinkcraft-themes.css','rinkcraft-themes.css'],
 ['icelines-core/src/view_model/rinkcraft_capability_rules.json','capability-rules.json'],
])await writeFile(resolve(out,to),await readFile(resolve(from)));
await cp(resolve('icelines-browser/dist/src'),resolve(out,'src'),{recursive:true});
await writeFile(resolve(out,'build.json'),JSON.stringify({schema:'rinkcraft.preview-build.v1',catalog_sha256:createHash('sha256').update(raw).digest('hex'),windows:catalog.windows.map(w=>({season:w.season,players:w.players.length,captured_through:w.captured_through})),models:'Shared Rust feat and workload builders; all-strength records. No 1–20 capability estimates or inferred classes.'},null,2));
console.log(JSON.stringify({output:out,windows:catalog.windows.length,player_seasons:catalog.windows.reduce((n,w)=>n+w.players.length,0)}));
