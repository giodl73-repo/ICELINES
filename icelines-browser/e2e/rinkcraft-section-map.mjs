// Local comparison preview acceptance; prepare reference assets and serve it first.
import {chromium,expect} from '@playwright/test';
import {writeFile,mkdir} from 'node:fs/promises';
import {fileURLToPath} from 'node:url';
const output=new URL('../../target/compact-sheet/',import.meta.url);await mkdir(output,{recursive:true});
const base=process.env.ICELINES_COMPARISON_URL??'http://127.0.0.1:8771/comparison.html';
const b=await chromium.launch({channel:'msedge',headless:true});
const p=await b.newPage({viewport:{width:1680,height:1200}});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
try{
 await p.goto(base);
 const f=p.frameLocator('iframe');await expect(f.locator('.rk-front')).toBeVisible();await expect(p.locator('img')).toBeVisible();expect(await p.locator('img').evaluate(e=>e.naturalWidth)).toBeGreaterThan(600);
 for(const [key,count,sections] of [['identity',1,2],['signals',1,1],['tasks',2,3],['methods',1,1],['traits',2,6],['record',1,5],['clear',0,0]]){
  await p.locator(`[data-map="${key}"]`).click();await expect(p.locator('.region.active')).toHaveCount(count);await expect(f.locator('.rk-map-selected')).toHaveCount(sections);await expect(p.locator('[aria-pressed="true"]')).toHaveCount(1);
 }
 await p.locator('[data-map="signals"]').click();
 const boxes=await f.locator('[data-capability-index]').evaluateAll(es=>es.map(e=>{const b=e.getBoundingClientRect();return{x:b.x,y:b.y,bottom:b.bottom}}));expect(boxes.length).toBe(6);expect(new Set(boxes.map(b=>b.x)).size).toBe(1);for(let i=1;i<boxes.length;i++)expect(boxes[i].y).toBeGreaterThan(boxes[i-1].bottom);
 expect(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 await p.screenshot({path:fileURLToPath(new URL('comparison-desktop.png',output)),fullPage:true});
 await p.setViewportSize({width:390,height:844});expect(await p.locator('.workspace').evaluate(e=>e.scrollWidth>e.clientWidth)).toBe(true);expect(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 await p.screenshot({path:fileURLToPath(new URL('comparison-mobile.png',output)),fullPage:false});
 expect(errors).toEqual([]);await writeFile(new URL('comparison-check.json',output),JSON.stringify({pass:true,signalBoxes:boxes.length,mappingButtons:7,errors},null,2));console.log('Side-by-side: reference loaded; all seven controls passed; six capability fields; desktop and mobile geometry passed.');
}finally{await b.close();}
