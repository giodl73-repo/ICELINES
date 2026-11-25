import type {StatProfile} from './rinkcraft-stat-scores.js';
interface Method {id:string;name:string;result:string;share:string;outcomes:string;}
export interface ShotMethods {schema:string;player_id:number;season:number;group:string;games:number;typed_attempts:number;total_attempts:number;coverage:string;primary:Method[];additional:Method[];unknown_display:string;captured_at:string;note:string;}
export function shotMethods(p:StatProfile|undefined):ShotMethods|undefined {const r=p?.shot_methods;return r&&r.schema==='rinkcraft.shot-methods.v1'&&r.player_id===p.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&['Forwards','Defense'].includes(r.group)&&Array.isArray(r.primary)&&r.primary.length<=3?r:undefined;}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderShotMethods(r:ShotMethods):HTMLElement {
 const body=el('div');body.className='rk-shot-methods';body.append(el('small','Recorded shot repertoire'),el('small',r.coverage));
 for(const m of r.primary){const row=el('div');row.className='rk-shot-method';row.dataset.shotMethod=m.id;row.append(el('strong',m.name),el('span',m.result),el('small',m.share));body.append(row);}if(!r.primary.length)body.append(el('p','No supported shot types recorded in this sample'));
 const details=el('details');details.className='rk-shot-method-scope';details.append(el('summary',r.additional.length?`${r.additional.length} more types & scope`:'Counts & scope'));for(const m of [...r.primary,...r.additional])details.append(el('strong',m.name),el('p',`${m.result} · ${m.share}`),el('p',m.outcomes));details.append(el('p',r.unknown_display),el('p',r.note),el('small',`NHL public play-by-play · captured ${r.captured_at.slice(0,10)}`));body.append(details);return body;
}
