import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface ShootingRecord {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;shots:string;conversion:string;rows:{attribute:string;label:string;value:string}[];formula:string;note:string;source_notes:string[];}
export function shootingRecord(p:StatProfile|undefined):ShootingRecord|undefined {
 const r=p?.skater_context?.shooting;
 return r&&r.schema==='rinkcraft.shooting-record.v1'&&r.player_id===p?.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&Array.isArray(r.rows)&&r.rows.length===11&&r.rows.every(row=>typeof row.label==='string'&&typeof row.value==='string')&&[r.shots,r.conversion,r.formula,r.note].every(v=>typeof v==='string')&&Array.isArray(r.source_notes)&&r.source_notes.every(n=>typeof n==='string')?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function shootingSummary(r:ShootingRecord):HTMLElement {const b=el('div');b.className='rk-shooting-summary';b.append(el('span',`Shots ${r.shots}`),el('span',`Conversion ${r.conversion}`));return b;}
export function renderShootingRecord(r:ShootingRecord):HTMLElement {
 const b=el('section');b.className='rk-shooting-record';b.append(el('h3','Shooting record'),el('p',`${Math.floor(r.season/10000)}–${String(r.season%100).padStart(2,'0')} · ${r.games} GP · all strengths · NHL`));const dl=el('dl');for(const row of r.rows){const a=el('div');a.append(el('dt',row.label),el('dd',row.value));dl.append(a);}b.append(dl,el('p',r.formula),el('p',r.note));for(const note of r.source_notes)b.append(el('small',note));return b;
}
