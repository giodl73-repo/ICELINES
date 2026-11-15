// Uses an existing loopback report; does not start/replace any application server.
import {chromium,expect} from '@playwright/test';
import {mkdir,writeFile} from 'node:fs/promises';
const url=process.env.ICELINES_TOURNAMENT_URL??'http://127.0.0.1:8783/';
const out=process.env.ICELINES_TOURNAMENT_CHECK_DIR??'target/tournament-ui';
await mkdir(out,{recursive:true});
const browser=await chromium.launch({channel:'msedge',headless:true});
try{
 const page=await browser.newPage({viewport:{width:1280,height:1000}});const errors=[];
 page.on('pageerror',e=>errors.push(String(e)));
 const response=await page.request.get(new URL('tournament.json',url).href);expect(response.ok()).toBeTruthy();const r=await response.json();
 await page.goto(url,{waitUntil:'networkidle'});
 await expect(page.getByRole('heading',{name:'The first tournament',exact:true})).toBeVisible();
 await expect(page.locator('.winner')).toHaveText(r.illustrative.champion);
 const standings=page.getByRole('heading',{name:'Round-robin standings',exact:true}).locator('..');
 await expect(standings.getByRole('row')).toHaveCount(r.active_teams.length+1);
 const games=page.getByRole('heading',{name:'Game results',exact:true}).locator('..');
 await expect(games.getByRole('row')).toHaveCount(r.illustrative.games.length+1);
 const teams=page.getByRole('heading',{name:'Teams and character profiles',exact:true}).locator('..');
 const details=teams.locator('details');await expect(details).toHaveCount(r.active_teams.length);
 let members=0;
 for(let i=0;i<r.active_teams.length;i++){
  await details.nth(i).locator('summary').click();await expect(details.nth(i).locator('tbody tr')).toHaveCount(r.active_teams[i].roster.members.length+1);members+=r.active_teams[i].roster.members.length;
 }
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBeTruthy();
 await page.evaluate(()=>scrollTo(0,0));
 await page.screenshot({path:`${out}/desktop.png`,fullPage:false});
 await page.setViewportSize({width:390,height:844});
 expect(await page.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1)).toBeTruthy();
 await expect(details.nth(0).locator('summary')).toBeVisible();
 await page.evaluate(()=>scrollTo(0,0));await page.screenshot({path:`${out}/mobile.png`,fullPage:false});
 expect(errors).toEqual([]);
 await writeFile(`${out}/verification.json`,JSON.stringify({url,teams:r.active_teams.length,members,games:r.illustrative.games.length,desktop:'pass',mobile390:'pass',allRosterDisclosures:'pass',pageErrors:errors},null,2));
 console.log(`Tournament UI passed: ${r.active_teams.length} teams / ${members} characters / ${r.illustrative.games.length} games; desktop + mobile390`);
}finally{await browser.close();}
