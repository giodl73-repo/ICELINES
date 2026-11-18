// Explicit local fixture projection. No network, model changes or invented NHL grades.
import {readFile,writeFile,mkdir,cp} from 'node:fs/promises';
import {resolve} from 'node:path';
import {createHash} from 'node:crypto';
const [input,output]=process.argv.slice(2);
if(!input||!output)throw Error('Usage: node tools/prepare_rinkcraft_theme_studio.mjs PROFILE_FIXTURES_JSON OUTPUT_DIRECTORY');
const root=resolve('.'),out=resolve(output),raw=await readFile(resolve(input));
const profiles=JSON.parse(raw);if(!Object.values(profiles).every(p=>p.player_name&&p.request&&p.latest_available))throw Error('Expected explicit retained profile fixtures.');
await mkdir(out,{recursive:true});
for(const [source,target]of [
 ['design/previews/rinkcraft-theme-studio.html','index.html'],['design/previews/theme-studio.css','theme-studio.css'],
 ['icelines-web/static/rinkcraft-character.css','rinkcraft-character.css'],['icelines-web/static/rinkcraft-themes.css','rinkcraft-themes.css'],
 ['design/fixtures/rinkcraft-theme-specimens.json','specimens.json'],
])await writeFile(resolve(out,target),await readFile(resolve(root,source)));
await writeFile(resolve(out,'fixtures.json'),raw);
await cp(resolve(root,'icelines-browser/dist/src'),resolve(out,'src'),{recursive:true});
await writeFile(resolve(out,'projection.json'),JSON.stringify({kind:'local_theme_playground',input:resolve(input),sha256:createHash('sha256').update(raw).digest('hex'),profile_count:Object.keys(profiles).length,fictional_data:'Authored neutral design specimens, isolated from NHL profiles and all model inputs.'},null,2));
console.log(`Prepared theme studio: ${out}`);
