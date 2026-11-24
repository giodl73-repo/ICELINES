import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface PassingRecord {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;primary:string;secondary:string;mix:string;rows:{attribute:string;label:string;value:string}[];formula:string;note:string;}
export function passingRecord(p:StatProfile|undefined):PassingRecord|undefined {
 const r=p?.skater_context?.passing;
 return r&&r.schema==='rinkcraft.passing-record.v1'&&r.player_id===p?.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&Array.isArray(r.rows)&&r.rows.length===8&&r.rows.every(row=>typeof row.label==='string'&&typeof row.value==='string')&&[r.primary,r.secondary,r.mix,r.formula,r.note].every(v=>typeof v==='string')?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function passingSummary(r:PassingRecord):HTMLElement {const b=el('div');b.className='rk-passing-summary';b.append(el('span',`Primary ${r.primary}`),el('span',`Secondary ${r.secondary}`));return b;}
export function renderPassingRecord(r:PassingRecord):HTMLElement {
 const b=el('section');b.className='rk-passing-record';b.append(el('h3','Passing record'),el('p',`${Math.floor(r.season/10000)}–${String(r.season%100).padStart(2,'0')} · ${r.games} GP · all strengths · NHL`));const dl=el('dl');for(const row of r.rows){const a=el('div');a.append(el('dt',row.label),el('dd',row.value));dl.append(a);}b.append(dl,el('p',r.formula),el('p',r.note));return b;
}
