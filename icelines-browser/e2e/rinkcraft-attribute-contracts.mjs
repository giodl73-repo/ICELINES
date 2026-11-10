import {chromium,expect} from '@playwright/test';
import {writeFile} from 'node:fs/promises';
const b=await chromium.launch({channel:'msedge',headless:true});
const p=await b.newPage({viewport:{width:1440,height:1000}});const errors=[];p.on('pageerror',e=>errors.push(String(e)));
try{
 await p.goto('http://127.0.0.1:8771/attributes/index.html',{timeout:15000});
 await expect(p.locator('article')).toHaveCount(12);await expect(p.locator('article details')).toHaveCount(53);
 await expect(p.locator('nav a')).toHaveCount(12);
 for(const id of ['CAP-SKATING','CAP-READING','CAP-G-REBOUNDS']){
  await p.locator(`#${id} summary`).first().click();await expect(p.locator(`#${id} details`).first()).toHaveAttribute('open','');
 }
 expect(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 await p.screenshot({path:'target/compact-sheet/attributes/desktop.png',fullPage:false});
 await p.setViewportSize({width:390,height:844});
 await p.locator('nav a[href="#CAP-READING"]').click();
 expect(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
 await expect(p.locator('#CAP-READING')).toBeInViewport();
 await p.screenshot({path:'target/compact-sheet/attributes/mobile.png',fullPage:false});
 expect(errors).toEqual([]);
 await writeFile('target/compact-sheet/attributes/browser-check.json',JSON.stringify({passed:true,contracts:12,facetDisclosures:41,evidenceDisclosures:12,viewports:[1440,390],errors},null,2));
 console.log('12 contracts, 41 facets, source/boundary disclosures, desktop/mobile geometry and no page errors passed.');
}finally{await b.close();}
