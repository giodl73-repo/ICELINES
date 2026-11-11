// Real-app showcase audit; requires the owned 8772 native and 8773 browser servers.
import {chromium,expect} from '@playwright/test';
import {mkdir,readFile,writeFile} from 'node:fs/promises';
const native='http://127.0.0.1:8772',browser='http://127.0.0.1:8773',out='target/profile-studio';await mkdir(out,{recursive:true});
const catalog=await(await fetch(browser+'/rinkcraft-characters.json')).json();expect(catalog.assessments).toHaveLength(26);
const skater=['Skating','Handling','Passing','Shooting','Reading','Defending'],goalie=['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play'];
const b=await chromium.launch({channel:'msedge',headless:true});const errors=[],results=[];
try{
 const nojs=await b.newContext({javaScriptEnabled:false,viewport:{width:1440,height:1200}}),n=await nojs.newPage();
 await n.goto(native+'/rinkcraft');await expect(n.locator('.rk-atlas-card')).toHaveCount(26);await expect(n.locator('main')).toContainText('October 27, 2026');await n.screenshot({path:out+'/native-atlas.png',fullPage:true});
 const p=await b.newPage({viewport:{width:1440,height:1200},acceptDownloads:true});p.on('pageerror',e=>errors.push(String(e)));await p.goto(browser);await expect(p.locator('#rinkcraft-atlas-cards button')).toHaveCount(26);await p.locator('#rinkcraft-atlas').evaluate(e=>e.open=true);await expect(p.locator('#rinkcraft-atlas-notes')).toContainText('October 27, 2026');await p.locator('#rinkcraft-atlas').screenshot({path:out+'/browser-atlas.png'});
 await p.locator('#data-library').evaluate(e=>e.open=true);await p.locator('#catalog').selectOption('20252026-regular');await p.locator('#load').click();await expect(p.locator('#context')).toHaveText('20252026 · regular');
 for(const a of catalog.assessments){
  const api=await(await fetch(native+'/api/v1/player/'+a.player_id+'/rinkcraft')).json();const c=api.data.character_sheet;expect(c.primary.id).toBe(a.primary.class_id);expect(c.sources.length).toBeGreaterThan(0);
  await n.goto(native+'/player/'+a.player_id+'/rinkcraft');const nf=n.locator('.rk-front');await expect(nf).toBeVisible();await expect(nf.locator('.rk-class-field dd')).toHaveText(c.primary.name);await expect(nf.locator('.rk-front-methods li')).toHaveText(c.features.map(f=>f.name));await expect(nf.locator('.rk-front-values dd')).toHaveText(Array(6).fill('—'));
  // Barkov has no appearance in the newer package; use his available 2024-25 window explicitly.
  const season=a.player_id===8477493?'20242025-regular':'20252026-regular';if(await p.locator('#catalog').inputValue()!==season){await p.locator('#data-library').evaluate(e=>e.open=true);await p.locator('#catalog').selectOption(season);await p.locator('#load').click();await expect(p.locator('#context')).toHaveText(season.replace('-regular',' · regular'));}
  await p.locator('#rinkcraft-atlas-cards [data-player-id="'+a.player_id+'"]').click();const f=p.locator('#player-dialog .rk-front');await expect(f).toBeVisible();await expect(f.locator('.rk-class-field dd')).toHaveText(c.primary.name);await expect(f.locator('.rk-front-methods li')).toHaveText(c.features.map(f=>f.name));await expect(f.locator('.rk-front-values dd')).toHaveText(Array(6).fill('—'));const labels=await f.locator('.rk-front-values dt').allTextContents();expect(labels).toEqual(api.data.latest_available.position==='Goalie'?goalie:skater);
  const values=await f.locator('.rk-record-values dd').allTextContents();expect(values.length).toBeGreaterThan(0);expect(await f.locator('.rk-record-values').innerText()).not.toContain('NaN');
  results.push({id:a.player_id,name:a.player_name,class:c.primary.id,specialty:c.specialty,sourceCount:c.sources.length,capabilities:'unrated',package:season,record:values});
  if([8479323,8478402,8478048].includes(a.player_id)){
   const label=labels[0]==='Tracking'?'goalie':a.player_id===8479323?'defense':'forward';await nf.screenshot({path:out+'/nhl-native-'+label+'.png'});await f.screenshot({path:out+'/nhl-browser-'+label+'.png'});
   for(const [page,surface]of [[n,'native'],[p,'browser']]){if(surface==='browser'){await page.emulateMedia({media:'print'});await expect(page.locator('.rk-game-profile .rk-front')).not.toBeVisible();await expect(f).toBeVisible();await page.emulateMedia({media:'screen'});}const pdf=await page.pdf({path:out+'/nhl-'+surface+'-'+label+'.pdf',preferCSSPageSize:true});expect(pdf.toString('latin1').match(/\/Type\s*\/Page\b/g)?.length).toBe(1);await page.setViewportSize({width:390,height:844});expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);await page.screenshot({path:out+'/nhl-'+surface+'-'+label+'-mobile.png',fullPage:true});await page.setViewportSize({width:1440,height:1200});}
   await p.locator('#player-dialog').getByRole('button',{name:'Create inspired game character',exact:true}).click();await expect(p.locator('#rinkcraft-game-root .rk-front-scope')).toContainText(a.player_name);await expect(p.locator('#rinkcraft-game-root .rk-front-values dt')).toHaveText(labels);await expect(p.locator('#rinkcraft-game-root [data-rating-state="game_authored"]')).toHaveCount(6);await p.locator('#rinkcraft-game-studio').evaluate(e=>e.open=false);
  }else await p.locator('#close-player').click();
  console.log('Verified '+a.player_name+' / '+c.primary.name);
 }
 expect(new Set(results.map(r=>r.class)).size).toBe(7);expect(errors).toEqual([]);
 await writeFile(out+'/showcase-check.json',JSON.stringify({pass:true,native:26,browser:26,classes:7,capabilitiesUnrated:true,inspiredBranches:3,mobilePrintBranches:3,errors,results},null,2));console.log('26 native/browser profiles, seven classes, three inspired branches and mobile/Letter exports passed.');
}finally{await b.close()}
