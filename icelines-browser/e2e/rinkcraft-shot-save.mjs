// Prepared CLI probes are authored outside IceLines; this file verifies transport and UI.
import {chromium, expect} from '@playwright/test';
import {readFile, writeFile, mkdir} from 'node:fs/promises';
const out='target/shot-save';await mkdir(out,{recursive:true});
const {BrowserEngine,initSync}=await import('../public/pkg/icelines_wasm.js');
initSync({module:await readFile('icelines-browser/public/pkg/icelines_wasm_bg.wasm')});
const engine=new BrowserEngine(),env=command=>({schema:'rinkcraft.unit-studio-command.v1',command,sheet_character:null});
const run=command=>JSON.parse(engine.unit_studio(JSON.stringify(env(command))));
const api=async command=>{const r=await fetch('http://127.0.0.1:8772/api/v1/rinkcraft/unit-scene',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(env(command))});const raw=await r.text();return {status:r.status,data:r.headers.get('content-type')?.includes('application/json')?JSON.parse(raw).data:null}};
const cases=[];for(const kind of ['shooting','tracking','rebounds'])for(const score of [1,20]){
 const id=kind+'-'+score,pending=JSON.parse(await readFile(out+'/'+id+'-pending.json','utf8')),expected=JSON.parse(await readFile(out+'/'+id+'.json','utf8'));
 const action=expected.input.request.actions.at(-1),command={kind:'step',replay:pending,action};
 expect(run({kind:'import',replay:pending}).replay).toEqual(pending);expect(run(command).replay).toEqual(expected);expect((await api(command)).data.replay).toEqual(expected);cases.push({id,pending,expected,action});
}
const old=JSON.parse(await readFile('icelines-core/tests/fixtures/rinkcraft-unit-v03.json','utf8'));
expect(run({kind:'import',replay:old}).replay).toEqual(old);expect((await api({kind:'import',replay:old})).data.replay).toEqual(old);
expect(()=>run({kind:'step',replay:old,action:'shoot'})).toThrow();
const upgraded=run({kind:'start_shooting',replay:old});expect((await api({kind:'start_shooting',replay:old})).data).toEqual(upgraded);
expect(upgraded.replay.input.request.actions).toEqual([]);expect(upgraded.replay.input.rules.version).toBe('0.4.0-playtest');expect(upgraded.replay.session.units_sha256).toBe(old.session.units_sha256);
const bad=structuredClone(cases[2].pending);bad.session.scene.state.scored_by='home:F2';expect(()=>run({kind:'import',replay:bad})).toThrow();expect((await api({kind:'import',replay:bad})).status).toBeGreaterThanOrEqual(400);await writeFile(out+'/invalid-goal.json',JSON.stringify(bad));
const browser=await chromium.launch({channel:'msedge',headless:true}),errors=[];
try{for(const surface of ['native','browser']){
 const p=await browser.newPage({viewport:{width:1440,height:1100},acceptDownloads:true});p.on('pageerror',e=>errors.push(String(e)));
 await p.goto(surface==='native'?'http://127.0.0.1:8772/rinkcraft/game':'http://127.0.0.1:8773/');if(surface==='browser')await p.locator('#rinkcraft-game-studio > summary').click();
 const root=p.locator('#rinkcraft-game-root');await expect(root.locator('.rk-game-profile')).toBeVisible();await root.getByRole('button',{name:'Arrange explicit units',exact:true}).click();
 const panel=root.locator('.rk-unit-scene'),result=panel.locator('.rk-unit-result'),idle=()=>expect(result).toHaveAttribute('aria-busy','false'),status=text=>expect(panel.getByRole('status')).toContainText(text);
 await status('Authored unit scene loaded');await idle();await expect(panel.getByRole('button',{name:'Roll unit shoot',exact:true})).toHaveCount(0);
 await panel.getByRole('button',{name:'Start shooting scene (0.4)',exact:true}).click();await status('New shooting scene started');await idle();
 await expect(panel.getByRole('button',{name:'Start shooting scene (0.4)',exact:true})).toBeHidden();await expect(panel.getByRole('button',{name:'Start receiving scene (0.3)',exact:true})).toBeHidden();
 await expect(panel.getByRole('button',{name:'Roll unit shoot',exact:true})).toBeDisabled();await expect(panel.getByRole('button',{name:'Roll unit save',exact:true})).toBeDisabled();
 const card=await root.locator('.rk-front').innerText();
 const download=async name=>{const wait=p.waitForEvent('download');await panel.getByRole('button',{name:'Download unit replay',exact:true}).click();const path=out+'/'+surface+'-'+name+'.json';await(await wait).saveAs(path);return JSON.parse(await readFile(path,'utf8'))};
 if(surface==='browser')await p.context().setOffline(true);
 for(const c of cases){await panel.getByLabel('Import unit replay',{exact:true}).setInputFiles(out+'/'+c.id+'-pending.json');await status('Unit replay imported');await idle();
  if(c.action==='save'){await expect(panel.locator('.rk-unit-state')).toContainText('in flight to away:G');await expect(panel.getByRole('button',{name:'Roll unit rebound',exact:true})).toBeDisabled();}
  if(c.action==='rebound')await expect(panel.locator('.rk-unit-state')).toContainText('Puck: loose');
  await expect(panel.getByRole('button',{name:'Roll unit '+c.action,exact:true})).toBeEnabled();await panel.getByRole('button',{name:'Roll unit '+c.action,exact:true}).click();await idle();
  expect(await download(c.id)).toEqual(c.expected);await expect(panel.locator('.rk-unit-state')).toContainText('Phase: '+c.expected.session.scene.state.phase.replaceAll('_',' '));
  if(c.expected.session.scene.state.phase==='goal')await expect(panel.locator('.rk-unit-state')).toContainText('in net · goal by home:F2');
  if(c.expected.session.scene.state.phase==='goalie_held')await expect(panel.locator('.rk-unit-state')).toContainText('Puck: away:G');
  expect(await root.locator('.rk-front').innerText()).toBe(card);
 }
 const before=await download('before-rejection');await panel.getByLabel('Import unit replay',{exact:true}).setInputFiles(out+'/invalid-goal.json');await status('rejected');await idle();expect(await download('after-rejection')).toEqual(before);
 const state=panel.locator('.rk-unit-state');await state.scrollIntoViewIfNeeded();await p.screenshot({path:out+'/'+surface+'-held.png'});
 await p.setViewportSize({width:390,height:844});expect(await p.evaluate(()=>document.documentElement.scrollWidth<=innerWidth)).toBe(true);await state.scrollIntoViewIfNeeded();await p.screenshot({path:out+'/'+surface+'-mobile.png'});
 await p.pdf({path:out+'/'+surface+'-character.pdf',preferCSSPageSize:true});await p.close();console.log(surface+' shot/save/rebound passed');
 }expect(errors).toEqual([]);await writeFile(out+'/results.json',JSON.stringify({pass:true,cliNativeWasmCases:6,frozenV03Exact:true,upgradeExplicit:true,threeSameDiceContrasts:true,goalOnlyAfterShotAndResponse:true,saveContactIsNotPossession:true,reboundFailureIsLoose:true,invalidGoalRejectedWithoutMutation:true,cardPreserved:true,browserOffline:true,mobile:390,errors},null,2));
}finally{engine.free();await browser.close()}
