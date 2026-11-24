import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface ReadingRecord {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;takeaways:string;assists:string;taken:string;rows:{attribute:string;label:string;value:string}[];formula:string;note:string;}
export function readingRecord(p:StatProfile|undefined):ReadingRecord|undefined {
 const r=p?.skater_context?.reading;
 return r&&r.schema==='rinkcraft.reading-record.v1'&&r.player_id===p?.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&Array.isArray(r.rows)&&r.rows.length===6&&r.rows.every(row=>typeof row.label==='string'&&typeof row.value==='string')&&[r.takeaways,r.assists,r.taken,r.formula,r.note].every(v=>typeof v==='string')?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function readingSummary(r:ReadingRecord):HTMLElement {
 const b=el('div');b.className='rk-reading-summary';const dl=el('dl');for(const [label,value]of [['Takeaways',r.takeaways],['Penalties taken',r.taken]]){const a=el('div');a.append(el('dt',label),el('dd',value));dl.append(a);}b.append(dl,el('small','Recorded events · anticipation unmeasured'));return b;
}
export function renderReadingRecord(r:ReadingRecord):HTMLElement {
 const b=el('section');b.className='rk-reading-record';b.append(el('h3','Reading record'),el('p',`${Math.floor(r.season/10000)}–${String(r.season%100).padStart(2,'0')} · ${r.games} GP · all strengths · NHL`));
 const dl=el('dl');for(const row of r.rows){const a=el('div');a.append(el('dt',row.label),el('dd',row.value));dl.append(a);}b.append(dl,el('p',r.formula),el('p',r.note));return b;
}
