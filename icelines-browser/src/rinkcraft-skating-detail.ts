import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface SkatingDetail {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;headline:string;secondary:string;rows:{key:string;label:string;value:string;comparison:string|null}[];note:string;}
export function skatingDetail(p:StatProfile|undefined):SkatingDetail|undefined {
 const r=p?.skating_detail;return r&&r.schema==='rinkcraft.skating-detail.v1'&&r.player_id===p.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&r.games>0&&['Forwards','Defense'].includes(r.group)&&Number.isFinite(r.minutes)&&r.minutes>0&&typeof p.minutes==='number'&&Math.abs(r.minutes-p.minutes)<1e-9?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderSkatingDetail(r:SkatingDetail):HTMLElement {
 const body=el('div');body.className='rk-skating-summary';body.append(el('strong',r.headline),el('small',r.secondary),el('small','Recorded · all strengths'));
 const details=el('details');details.className='rk-skating-measures';details.append(el('summary','Measures & scope'));for(const row of r.rows){const item=el('div');item.dataset.skatingMetric=row.key;item.append(el('b',row.label),el('span',row.value));if(row.comparison)item.append(el('small',row.comparison));details.append(item);}details.append(el('p',r.note));body.append(details);return body;
}
