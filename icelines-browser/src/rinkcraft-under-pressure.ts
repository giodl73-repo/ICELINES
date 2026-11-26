import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface PressureEntry {breakdown?:{key:string;name:string;opportunities:number;successes:number;result:string}[];id:string;name:string;challenge:string;response:string;opportunities:number;successes:number;result:string;definition:string;source:string;captured_at:string;coverage:string;note:string;}
export interface PressureAction {name:string;result:string;baseline:string;exposure?:string;definition:string;coverage:string;captured_at:string;note:string;}
export interface PressureRecord {schema:string;player_id:number;season:number;group:string;games:number;entries:PressureEntry[];actions?:PressureAction[];gaps:string[];note:string;}
export function pressureRecord(p:StatProfile|undefined):PressureRecord|undefined {
 const r=p?.under_pressure;return r&&r.schema==='rinkcraft.under-pressure.v1'&&r.player_id===p?.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&Array.isArray(r.entries)&&r.entries.every(e=>Number.isInteger(e.opportunities)&&e.opportunities>=0&&Number.isInteger(e.successes)&&e.successes>=0&&e.successes<=e.opportunities)?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function pressureEntries(r:PressureRecord):HTMLElement {
 const list=el('div');list.className='rk-pressure-list';if(r.entries.length)list.append(el('small','Observed responses'));
 for(const e of r.entries){
  const row=el('details');row.className='rk-pressure-entry';row.dataset.challengeId=e.id;const summary=el('summary');summary.append(el('strong',e.name),el('span',e.result));row.append(summary,el('p',`${e.challenge} → ${e.response}`),el('p',e.definition));
  if(e.breakdown?.length){const dl=el('dl');dl.className='rk-pressure-breakdown';for(const b of e.breakdown){const item=el('div');item.dataset.scoreState=b.key;item.append(el('dt',b.name),el('dd',b.result));dl.append(item);}row.append(el('p',e.id==='draws'?"Player team's skater balance at the draw (both goalies present for even/advantage/disadvantage)":"Player team's score before the draw"),dl);}
  row.append(el('small',`${e.source} · ${e.coverage} · captured ${e.captured_at.slice(0,10)}`),el('p',e.note));list.append(row);
 }
 if(r.actions?.length){list.append(el('small','Context actions · counts only'));for(const a of r.actions){const row=el('details');row.className='rk-pressure-entry rk-pressure-action';const summary=el('summary');summary.append(el('strong',a.name),el('span',a.result));row.append(summary,el('p',a.definition),el('p',a.baseline));if(a.exposure)row.append(el('small','NHL shift charts + box score'),el('p',a.exposure));row.append(el('small',`NHL public play-by-play · ${a.coverage} · captured ${a.captured_at.slice(0,10)}`),el('p',a.note));list.append(row);}}
 return list;
}
export function pressureGaps(r:PressureRecord):HTMLElement {
 const d=el('details');d.className='rk-pressure-gaps';d.append(el('summary','Challenge catalog & gaps'),el('p',r.note));for(const gap of r.gaps)d.append(el('p',gap));return d;
}
