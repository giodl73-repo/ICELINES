import {chromium,expect} from '../node_modules/@playwright/test/index.mjs';
import {readFile,writeFile,mkdir} from 'node:fs/promises';
import {resolve,sep} from 'node:path';
import assert from 'node:assert/strict';
const root=resolve('.'),out=resolve(root,'target/compact-sheet/profile-guide');
const fixtures=JSON.parse(await readFile(process.env.ICELINES_GUIDE_FIXTURES??resolve(out,'fixtures.json'),'utf8'));
await mkdir(out,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1280,height:1000}}),errors=[];
page.on('pageerror',e=>errors.push(e.message));
await page.route('http://guide.test/**',async route=>{
 const pathname=new URL(route.request().url()).pathname;
 if(pathname==='/'){await route.fulfill({contentType:'text/html',body:'<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><link rel="stylesheet" href="/sheet.css"><style>body{margin:0;padding:12px}#main{max-width:1200px;margin:auto}</style><main id="main"></main></html>'});return;}
 if(pathname==='/sheet.css'){await route.fulfill({contentType:'text/css',body:await readFile(resolve(root,'icelines-web/static/rinkcraft-character.css'),'utf8')});return;}
 const base=resolve(root,'icelines-browser/dist'),file=resolve(base,'.'+pathname);
 assert.ok(file.startsWith(base+sep));
 await route.fulfill({contentType:'text/javascript',body:await readFile(file,'utf8')});
});
const results=[];
try{
 await page.goto('http://guide.test/');
 for(const [key,topics]of [['fox',5],['shesterkin',3]]){
  const unchanged=await page.evaluate(async profile=>{
   const {renderCharacterCard}=await import('/src/rinkcraft-character.js');
   const args=[profile.player_name,profile.character_sheet,undefined,profile.latest_available,profile.numerical_evidence,profile.numerical_history,profile.numerical_snapshot,profile.numerical_comparison_choices,profile.documentary_catalog_available];
   const old=renderCharacterCard(...args),current=renderCharacterCard(...args,profile.evidence_guide);
   document.querySelector('#main').replaceChildren(current);
   return old.querySelector('.rk-front').outerHTML===current.querySelector('.rk-front').outerHTML;
  },fixtures[key]);
  assert.equal(unchanged,true);
  const guide=page.locator('.rk-evidence-guide');
  assert.equal(await guide.getAttribute('open'),null);
  await expect(guide.locator('[data-guide-topic]')).toHaveCount(topics);
  await expect(page.locator('.rk-front [data-rating-state="unassessed"]')).toHaveCount(6);
  await guide.locator('summary').click();
  await expect(guide.getByText(key==='fox'?'Passing · delivery':'Puck Play · goalie exchanges',{exact:true})).toBeVisible();
  await expect(guide.locator('[data-guide-topic="reception"]')).toHaveCount(key==='fox'?1:0);
  await page.screenshot({path:resolve(out,key+'-guide.png'),fullPage:true});
  await page.setViewportSize({width:390,height:844});
  assert.ok(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth));
  await page.screenshot({path:resolve(out,key+'-guide-mobile.png'),fullPage:true});
  await page.emulateMedia({media:'print'});await expect(guide).toBeHidden();
  await page.pdf({path:resolve(out,key+'-guide.pdf'),format:'Letter',printBackground:true});
  await page.emulateMedia({media:'screen'});await page.setViewportSize({width:1280,height:1000});
  results.push({key,topics,front_unchanged:true,mobile_overflow:false,guide_hidden_in_print:true});
 }
 await page.evaluate(async profile=>{
  const {renderRinkcraftProfile}=await import('/src/rinkcraft-profile.js');
  document.querySelector('#main').replaceChildren(renderRinkcraftProfile(profile));
 },fixtures.fox);
 await expect(page.locator('.rk-evidence-guide')).toHaveCount(1);
 assert.deepEqual(errors,[]);
 await writeFile(resolve(out,'verification.json'),JSON.stringify({status:'passed',results,profile_renderer_wired:true,page_errors:errors},null,2));
 console.log('Profile guide desktop/mobile/print and profile wiring passed.');
}finally{await browser.close();}
