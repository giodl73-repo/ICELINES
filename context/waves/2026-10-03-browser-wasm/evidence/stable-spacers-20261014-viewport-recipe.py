from pathlib import Path
s=Path('icelines-browser/e2e/viewport.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'", "from '../icelines-browser/node_modules/@playwright/test/index.mjs'")
s=s.replace("'../..'","'..'").replace('target/viewport-browser-','target/stable-spacers-viewport-')
needle="    const download = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);"
assert s.count(needle)==1
s=s.replace(needle,"""    await page.evaluate(()=>{
      globalThis.__removedSpacers=[];
      globalThis.__spacerObserver=new MutationObserver(records=>{for(const record of records)for(const node of record.removedNodes)if(node.nodeType===1&&node.classList.contains('row-spacer'))globalThis.__removedSpacers.push(node.outerHTML);});
      globalThis.__spacerObserver.observe(document.querySelector('#rows'),{childList:true});
    });
    await page.locator('#query button[type="submit"]').click();
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count','933');
    await expect(page.locator('#rows')).toHaveJSProperty('inert',false);
    const removed=await page.evaluate(()=>{globalThis.__spacerObserver.disconnect();return globalThis.__removedSpacers;});
    report.spacer_mutations={same_context_query_removed_spacers:removed.length,scope:'Real tbody MutationObserver during another committed933-player query; mode/row kind unchanged.'};
    if(process.argv.includes('--expect-original-detaches'))assert.ok(removed.length>0,'Original control must expose its detach/reinsert behavior');
    else assert.equal(removed.length,0,'Routine query must keep virtual spacers attached');
"""+needle)
Path('target/stable-spacers-viewport.mjs').write_text(s,encoding='utf-8')
print('Prepared real spacer mutation check alongside full viewport regression.')
