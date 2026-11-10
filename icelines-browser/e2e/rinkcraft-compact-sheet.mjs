// Compile TypeScript first; reads an existing native API without restarting it.
import {chromium,expect} from '@playwright/test';
import {readFile,writeFile} from 'node:fs/promises';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {mkdir} from 'node:fs/promises';
const root=fileURLToPath(new URL('../../',import.meta.url));
const base=process.env.ICELINES_TEST_URL??'http://127.0.0.1:8765';
await mkdir(resolve(root,'target/compact-sheet'),{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
const page=await browser.newPage({viewport:{width:1280,height:1100}});
await page.route('http://sheet.test/**',async route=>{
 const path=new URL(route.request().url()).pathname;
 if(path==='/'){await route.fulfill({contentType:'text/html',body:'<!doctype html><meta charset="utf-8"><link rel="stylesheet" href="/base.css"><link rel="stylesheet" href="/sheet.css"><main id="main"></main>'});return;}
 const file=path==='/sheet.css'?'icelines-web/static/rinkcraft-character.css':path==='/base.css'?'icelines-web/static/style.css':'icelines-browser/dist'+path;
 await route.fulfill({contentType:path.endsWith('.js')?'text/javascript':'text/css',body:await readFile(resolve(root,file))});
});
const errors=[];page.on('pageerror',e=>errors.push(String(e)));
const results=[];
try{
 await page.goto('http://sheet.test/');
 for(const [label,id,query] of [['defense',8479323,''],['forward',8481789,''],['goalie',8478048,''],['unscouted',8479323,'?season=20202021']]){
  const profile=(await (await fetch(`${base}/api/v1/player/${id}/rinkcraft${query}`)).json()).data;
  await page.evaluate(async p=>{const {renderCharacterCard}=await import('/src/rinkcraft-character.js');document.querySelector('main').replaceChildren(renderCharacterCard(p.player_name,p.character_sheet??undefined,p.attribute_seasons.at(-1),p.latest_available,p.numerical_evidence??undefined,p.numerical_history??undefined,p.numerical_snapshot??undefined,[],true));},profile);
  await expect(page.locator('.rk-front')).toBeVisible();
  expect(await page.locator('.rk-evidence').evaluate(e=>e.open)).toBe(false);
  const box=await page.locator('.rk-front').boundingBox();expect(box.width).toBeLessThanOrEqual(816);
  expect(await page.locator('.rk-front .rk-field[data-numerical-key]').count()).toBe(profile.numerical_evidence?.facets.length??0);
  await page.locator('.rk-evidence > summary').click();await expect(page.locator('.rk-sheet')).toBeVisible();await page.locator('.rk-evidence > summary').click();
  await page.screenshot({path:resolve(root,`target/compact-sheet/${label}.png`),fullPage:true});
  const html=await page.locator('.rk-profile').evaluate(e=>e.outerHTML);
  await writeFile(resolve(root,`target/compact-sheet/${label}.html`),'<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Compact Rinkcraft profile</title><style>'+await readFile(resolve(root,'icelines-web/static/rinkcraft-character.css'),'utf8')+'</style>'+html+'</html>');
  const pdf=await page.pdf({path:resolve(root,`target/compact-sheet/${label}.pdf`),preferCSSPageSize:true});
  expect(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length).toBe(1);
  await page.setViewportSize({width:390,height:844});
  expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);
  await page.screenshot({path:resolve(root,`target/compact-sheet/${label}-mobile.png`),fullPage:true});
  await page.setViewportSize({width:1280,height:1100});
  results.push({label,width:box.width,height:box.height,facets:profile.numerical_evidence?.facets.length??0});
 }
 expect(errors).toEqual([]);await writeFile(resolve(root,'target/compact-sheet/results.json'),JSON.stringify({results,errors},null,2));console.log(JSON.stringify(results));
}finally{await browser.close();}
