// Display the simple attribute estimates computed by shared Rust.
import {renderShotContext} from './rinkcraft-goalie-shots.js';
import {renderSkaterContext} from './rinkcraft-skater-context.js';
import {defendingRecord,renderDefendingRecord} from './rinkcraft-defending-record.js';
import {handlingRecord,renderHandlingRecord} from './rinkcraft-handling-record.js';
import {readingRecord,renderReadingRecord} from './rinkcraft-reading-record.js';
import {passingRecord,renderPassingRecord} from './rinkcraft-passing-record.js';
import {shootingRecord,renderShootingRecord} from './rinkcraft-shooting-record.js';
export interface StatPart {key:string;label:string;unit:string;weight:number;lower_is_better:boolean;value:number|null;percentile:number|null;favorable_percentile:number|null;peers:number;}
export interface StatScore {label:string;score:number|null;stat_points:number|null;parts:StatPart[];note:string;}
export interface StatProfile {skating_detail?:import('./rinkcraft-skating-detail.js').SkatingDetail;unit_links?:import('./rinkcraft-unit-links.js').UnitLinks;under_pressure?:import('./rinkcraft-under-pressure.js').PressureRecord;estimated_record?:EstimatedRecord;schema:string;player_id:number;season:number;group:string;scope:string;games?:number;minutes?:number;small_sample:boolean;scores:StatScore[];shot_context?:import('./rinkcraft-goalie-shots.js').GoalieShotContext;skater_context?:import('./rinkcraft-skater-context.js').SkaterContext;}
export interface StatChange {player_id:number;season:number;previous_season:number;deltas:(number|null)[];}
export interface EstimatedRecord {observed_games?:number;season:number;target_games:number;basis:string;metrics:{recorded_count?:number;calculation?:string;key:string;label:string;value:number|null;unit:string;per_game:number|null;projected_total:boolean}[];}
const goalieLabels=['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play'];
const labels=['Skating','Handling','Passing','Shooting','Reading','Defending'];
export function matchesStatScores(p:StatProfile|undefined,id:number|undefined,season:number|undefined,group:string,games?:number):p is StatProfile {
 return !!p&&p.schema==='rinkcraft.simple-stat-attributes.v1'&&p.player_id===id&&p.season===season&&p.group===group&&(games===undefined||p.games===undefined||p.games===games)&&p.scores.length===6&&p.scores.every((s,i)=>s.label===(group==='Goalies'?goalieLabels:labels)[i]&&(s.score===null||(Number.isInteger(s.score)&&s.score>=1&&s.score<=20)));
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderStatScore(profile:StatProfile,score:StatScore):HTMLElement {
 const body=node('section');body.className='rk-stat-score-detail';
 body.append(node('small','RINKCRAFT · STAT ESTIMATE'),node('h2',`${score.label} ${score.score??'—'}/20`),node('p',score.note));
 body.append(node('p',`${profile.scope}${profile.games===undefined?'':` · ${profile.games} GP`} · same-season ${profile.group.toLowerCase()} comparison.`));
 if(profile.small_sample)body.append(node('p','Small sample: fewer than 10 games or 100 minutes. This estimate can move quickly.'));
 const handling=score.label==='Handling'?handlingRecord(profile):undefined;if(handling)body.append(renderHandlingRecord(handling));
 const reading=score.label==='Reading'?readingRecord(profile):undefined;if(reading)body.append(renderReadingRecord(reading));
 const passing=score.label==='Passing'?passingRecord(profile):undefined;if(passing)body.append(renderPassingRecord(passing));
 const shooting=score.label==='Shooting'?shootingRecord(profile):undefined;if(shooting)body.append(renderShootingRecord(shooting));
 const used=score.parts.filter(p=>p.favorable_percentile!==null).reduce((sum,p)=>sum+p.weight,0);
 const list=node('dl');list.className='rk-task-list';
 for(const part of score.parts){
  const row=node('div');row.className='rk-field';
  row.append(node('dt',part.label));
  if(part.value===null||part.favorable_percentile===null)row.append(node('dd','Unavailable'));
  else {
   const value=part.unit==='%'?`${(part.value*100).toFixed(1)}%`:`${part.value.toFixed(2)} ${part.unit}`;
   row.append(node('dd',`${value} · ${Math.round(part.weight/used*100)}% weight`),node('small',`Favorable P${part.favorable_percentile.toFixed(1)} · ${part.peers} peers${part.lower_is_better?' · lower is better':''}`));
  }
  list.append(row);
 }
 body.append(list,node('p',score.stat_points===null?'No ranked stats available.':`${score.stat_points.toFixed(1)} stat points ÷ 5 → ${score.score}/20.`),node('p','Average the favorable stat percentiles using these weights, divide by 5, round and cap at 1–20. Missing stats are omitted; remaining weights scale to 100%. Reference players have at least 10 games and 100 minutes.'));
 if(profile.group==='Goalies'&&profile.shot_context&&['Movement','Recovery','Rebounds'].includes(score.label))body.append(renderShotContext(profile.shot_context,score.label));
 if(profile.group!=='Goalies'&&profile.skater_context&&!handling&&!reading&&!passing&&!shooting)body.append(renderSkaterContext(profile.skater_context,score.label));
 if(score.label==='Defending'){const record=defendingRecord(profile);if(record)body.append(renderDefendingRecord(record));}
 return body;
}
export function openStatScore(profile:StatProfile,score:StatScore,player:string):void {
 const dialog=node('dialog');dialog.className='rk-capability-dialog';dialog.setAttribute('aria-label',`${score.label} stat estimate for ${player}`);
 const close=node('button','Close');close.type='button';close.onclick=()=>dialog.close();
 dialog.append(close,renderStatScore(profile,score));document.body.append(dialog);dialog.onclose=()=>dialog.remove();dialog.showModal();
}

export function renderStatHistory(profiles:StatProfile[],playerId:number|undefined,seasonType:string,player:string,changes:StatChange[]=[]):HTMLElement {
 const section=node('section');section.className='rk-stat-history';
 section.append(node('h3','Six-attribute season record'),node('p','1–20 estimates against each season’s position peers. Changes can reflect deployment, opportunity or peer changes. These supplied windows are not a complete career. Δ is the change from the previous supplied season; — means the comparison is unavailable. Select a score for its formula.'));
 const rows=seasonType==='regular'?profiles.filter(p=>['Forwards','Defense','Goalies'].includes(p.group)&&Number.isInteger(p.games)&&(p.games??0)>0&&matchesStatScores(p,playerId,p.season,p.group)):[];
 if(!rows.length){section.append(node('p','No season-matched attribute estimates supplied.'));return section;}
 const scroll=node('div');scroll.className='table-scroll';scroll.tabIndex=0;scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Six-attribute season record');
 const table=node('table'),head=node('thead'),header=node('tr');
 for(const label of ['Season','GP','Peers',...(rows[0].group==='Goalies'?goalieLabels:labels)]){const th=node('th',label);th.scope='col';header.append(th);}head.append(header);
 const body=node('tbody');
 for(const p of [...rows].sort((a,b)=>b.season-a.season)){
  const row=node('tr');row.dataset.attributeSeason=String(p.season);
  const change=changes.find(c=>c.player_id===playerId&&c.season===p.season&&rows.some(r=>r.season===c.previous_season)&&c.deltas.length===6);
  const season=node('th',`${Math.floor(p.season/10000)}–${String(p.season%100).padStart(2,'0')}`);season.scope='row';
  if(change)season.append(node('small',` · vs ${Math.floor(change.previous_season/10000)}–${String(change.previous_season%100).padStart(2,'0')}`));
  if(p.small_sample)season.append(node('small',' · Small sample'));
  row.append(season,node('td',String(p.games)),node('td',p.group));
  for(const [index,score] of p.scores.entries()){const cell=node('td'),button=node('button',score.score===null?'—':String(score.score));button.type='button';button.setAttribute('aria-label',`${score.label} ${score.score??'unrated'} in ${p.season}`);button.onclick=()=>openStatScore(p,score,player);cell.append(button);if(change){const delta=change.deltas[index];const badge=node('small',`Δ ${delta===null?'—':delta>0?'+'+delta:String(delta)}`);badge.dataset.attributeDelta=String(index);cell.append(badge);}row.append(cell);}
  body.append(row);
 }
 table.append(head,body);scroll.append(table);section.append(scroll);return section;
}

let catalogPromise:Promise<unknown|undefined>|undefined;
export function readStatAttributeCatalog():Promise<unknown|undefined> {
 catalogPromise??=(async()=>{
  let response:Response;try{response=await fetch(new URL('../rinkcraft-stat-attributes.json',import.meta.url));}catch{return undefined;}
  if(response.status===404)return undefined;if(!response.ok)throw new Error('Stat estimates could not be loaded.');
  const text=await response.text();if(text.length>32*1024*1024)throw new Error('Stat attribute catalog exceeds 32 MiB');
  return JSON.parse(text) as unknown;
 })();return catalogPromise;
}
