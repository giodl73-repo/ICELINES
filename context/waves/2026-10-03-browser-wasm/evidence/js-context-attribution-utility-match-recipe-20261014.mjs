import {readFile,writeFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import vm from 'node:vm';
const bundle=await readFile('icelines-browser/node_modules/playwright-core/lib/coreBundle.js','utf8');
const match=bundle.match(/\n\s+source3 = ('(?:\\.|[^'\\])*');/);
if(!match)throw Error('Pinned generated utility source literal not found');
// Evaluate only the regex-constrained quoted string, never the module/code body.
const utility=vm.runInNewContext(match[1],{}, {timeout:1000});
if(typeof utility!=='string'||!utility.includes('UtilityScript')||!utility.includes('parseEvaluationResultValue'))throw Error('Unexpected utility source');
const candidates=[];
for(const underTest of [false,true]){
 const source=`
      (() => {
        
        const module = {};
        ${utility}
        return new (module.exports.UtilityScript())(globalThis, ${underTest});
      })();`;
 for(const text of [source,source.trim()])candidates.push({under_test:underTest,trimmed:text!==source,sha256:createHash('sha256').update(text,'utf8').digest('hex'),bytes:Buffer.byteLength(text,'utf8')});
}
const report=JSON.parse(await readFile('target/js-context-attribution-noOPvR/report.json','utf8'));
const matched=report.sampled_script_sources.page.filter(item=>candidates.some(c=>c.sha256===item.sha256));
const output={scope:'Exact whole-script SHA matching runtime Debugger sources against reconstructed installed Playwright UtilityScript wrapper. No live memory/acceptance claim; unmatched source-marker classifications remain heuristic.',playwright_version:JSON.parse(await readFile('icelines-browser/node_modules/playwright-core/package.json','utf8')).version,bundle_sha256:createHash('sha256').update(bundle,'utf8').digest('hex'),candidates,matches:matched};
await writeFile('target/js-context-attribution-noOPvR/utility-source-match.json',JSON.stringify(output,null,2)+'\n');
console.log('Exact Playwright UtilityScript source matches:',matched.length);
