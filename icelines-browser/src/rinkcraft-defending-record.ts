import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface DefendingRecord {
 schema:string;player_id:number;season:number;group:string;games:number;minutes_5v5:number|null;
 activity:string;chances:string;on_off:string;rank:number|null;peers:number;
 rows:{attribute:string;label:string;value:string}[];note:string;
}
export function defendingRecord(p:StatProfile|undefined):DefendingRecord|undefined {
 const r=p?.skater_context?.defending;
 return r&&r.schema==='rinkcraft.defending-record.v1'&&r.player_id===p?.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&Array.isArray(r.rows)&&r.rows.length===8&&r.rows.every(row=>typeof row.label==='string'&&typeof row.value==='string')&&[r.activity,r.chances,r.on_off,r.note].every(v=>typeof v==='string')&&(r.minutes_5v5===null||Number.isFinite(r.minutes_5v5)&&r.minutes_5v5>=0&&r.minutes_5v5<=(p.minutes??Infinity)+1/60)?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function renderDefendingRecord(r:DefendingRecord):HTMLElement {
 const block=el('div');block.className='rk-defense-record';block.dataset.defenseCoverage=r.minutes_5v5===null?'unavailable':'measured';
 block.append(el('small','Chance Control · 5v5'),el('strong',r.chances),el('small',r.on_off),el('small','Defensive Work · all strengths'),el('p',r.activity));
 const details=el('details');details.className='rk-defense-details';details.append(el('summary','Defense record'));
 const body=el('div');body.append(el('p',`${Math.floor(r.season/10000)}–${String(r.season%100).padStart(2,'0')} · ${r.games} GP · ${r.group} · NHL / MoneyPuck.com`));
 const dl=el('dl');for(const row of r.rows){const entry=el('div');entry.append(el('dt',row.label),el('dd',row.value));dl.append(entry);}body.append(dl,el('p','Rates = source event totals × 60 / source minutes. On/off improvement = off-ice xGA/60 − on-ice xGA/60. Lower on-ice xGA/60 is favorable.'),el('p',r.note));details.append(body);block.append(details);return block;
}
