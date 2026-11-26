import type {StatProfile} from './rinkcraft-stat-scores.js';
interface Row {label:string;rate:string;share:string;}
export interface ProductionMix {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;headline:string;rows:Row[];assist_detail:Row[];note:string;}
export function productionMix(p:StatProfile|undefined):ProductionMix|undefined {
 const r=p?.production_mix;
 return r&&r.schema==='rinkcraft.production-mix.v1'&&r.player_id===p.player_id&&r.season===p.season&&r.group===p.group&&['Forwards','Defense'].includes(r.group)&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&typeof r.headline==='string'&&typeof r.note==='string'&&Array.isArray(r.rows)&&r.rows.length===2&&Array.isArray(r.assist_detail)&&[0,2].includes(r.assist_detail.length)?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderProductionMix(r:ProductionMix):HTMLElement {
 const body=el('details');body.className='rk-production-mix';body.append(el('summary',`Point mix · ${r.headline}`));
 const list=(rows:Row[])=>{const dl=el('dl');for(const row of rows){const d=el('div');d.append(el('dt',row.label),el('dd',row.rate),el('small',row.share));dl.append(d);}return dl;};
 body.append(list(r.rows));if(r.assist_detail.length){body.append(el('small','Assist detail · included above'),list(r.assist_detail));}else body.append(el('p','Matched primary/secondary assist detail unavailable'));
 body.append(el('p',`${r.games} GP · all strengths · NHL credited points`),el('p',r.note));return body;
}
