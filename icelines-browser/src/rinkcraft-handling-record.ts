import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface HandlingRecord {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;giveaways:string;drawn:string;rows:{attribute:string;label:string;value:string}[];formula:string;note:string;}
export function handlingRecord(p:StatProfile|undefined):HandlingRecord|undefined {
 const r=p?.skater_context?.handling;
 return r&&r.schema==='rinkcraft.handling-record.v1'&&r.player_id===p?.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&Array.isArray(r.rows)&&r.rows.length===6&&r.rows.every(row=>typeof row.label==='string'&&typeof row.value==='string')&&[r.giveaways,r.drawn,r.formula,r.note].every(v=>typeof v==='string')?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function handlingSummary(r:HandlingRecord):HTMLElement {const b=el('div');b.className='rk-handling-summary';b.append(el('span',`Giveaways ${r.giveaways}`),el('span',`Penalties drawn ${r.drawn}`));return b;}
export function renderHandlingRecord(r:HandlingRecord):HTMLElement {
 const b=el('section');b.className='rk-handling-record';b.append(el('h3','Handling record'),el('p',`${Math.floor(r.season/10000)}–${String(r.season%100).padStart(2,'0')} · ${r.games} GP · all strengths · NHL`));
 const axes=el('div');axes.className='rk-handling-axes';for(const [label,value]of [['Recorded Giveaways',r.giveaways],['Penalty Drawing',r.drawn]]){const a=el('div');a.append(el('h4',label),el('strong',value));axes.append(a);}b.append(axes);
 const dl=el('dl');for(const row of r.rows){const a=el('div');a.append(el('dt',row.label),el('dd',row.value));dl.append(a);}b.append(dl,el('p',r.formula),el('p',r.note));return b;
}
