from pathlib import Path
s=Path('icelines-browser/e2e/memory.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'", "from '../icelines-browser/node_modules/@playwright/test/index.mjs'")
s=s.replace("from './memory-diagnostics.mjs'", "from '../icelines-browser/e2e/memory-diagnostics.mjs'")
s=s.replace("from './memory-scope.mjs'", "from '../icelines-browser/e2e/memory-scope.mjs'")
s=s.replace("'../..'", "'..'").replace('target/browser-memory-', 'target/assertion-overhead-')
s=s.replace("const diagnostics = args.includes('--diagnose-gc');", "const diagnostics = false;\nconst batched = args.includes('--batched-assertions');")
needle='  const queryCommitted = async expectedRows => {'
assert s.count(needle)==1
s=s.replace(needle,"""  const batchedCommitted = async (expectedRows, expectedContext = null) => {
    const handle=await page.waitForFunction(({expectedRows,expectedContext,virtual})=>{
      const body=document.querySelector('#rows');
      const summary=document.querySelector('#query-summary')?.textContent??'';
      const context=document.querySelector('#context')?.textContent??'';
      const load=document.querySelector('#load');
      const submit=document.querySelector('#query button[type="submit"]');
      if(!body||body.inert!==false)return false;
      if(virtual ? body.dataset.resultCount!==String(expectedRows)||!summary.includes(`${expectedRows} skaters`) : body.querySelectorAll('tr').length!==expectedRows)return false;
      if(expectedContext ? !context.includes(String(expectedContext.season))||!context.includes(expectedContext.season_type)||load.disabled : submit.disabled)return false;
      return {mounted:body.querySelectorAll('tr[data-result-index]').length,residents:document.querySelectorAll('#resident-library > li').length};
    },{expectedRows,expectedContext,virtual:viewportRenderer},{timeout:60000});
    try{
      const state=await handle.jsonValue();
      if(viewportRenderer)expect(state.mounted).toBeLessThanOrEqual(44);
      expect(state.residents).toBeLessThanOrEqual(8);
      return state;
    }finally{await handle.dispose();}
  };
"""+needle+"\n    if(batched){await batchedCommitted(expectedRows);return;}")
start="    await expect(page.locator('#context')).toContainText(String(entry.season));"
end='    expect(residents).toBeLessThanOrEqual(8);'
a=s.index(start);b=s.index(end,a)+len(end)
original=s[a:b]
s=s[:a]+"""    let residents;
    if(batched){residents=(await batchedCommitted(entry.skaters,entry)).residents;}
    else{
"""+original.replace('    const residents =','    residents =')+"\n    }"+s[b:]
needle="    await expect(page.locator('#query button[type=\"submit\"]')).toBeEnabled();"
assert s.count(needle)==1
s=s.replace(needle,"    if(!batched)"+needle.strip())
s=s.replace('  report.completed = true;',"""  report.provisional_250_mib_target=null;
  report.target_scope='Diagnostic assertion-driver comparison only, not official acceptance. Same75/eight/two905/332/20/restart, UI actions, original browser flags/OS sampler/network observer and no forced GC. Batched mode changes DOM assertion/poll timing and can change allocation/GC timing. No app/engine/data transformation, no budget subtraction.';
  report.assertion_driver=batched?'batched query/context/commit/residency checks':'original separate locator assertions';
  report.completed = true;""")
Path('target/assertion-overhead.mjs').write_text(s,encoding='utf-8')
print('Prepared explicit control/batched diagnostic; official runner unchanged.')
