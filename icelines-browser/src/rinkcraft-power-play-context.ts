import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface PowerPlayContext {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;summary:string;time_share:string;assist_share:string;pp_assists:string;outside_assists:string;formula:string;note:string;}
export function powerPlayContext(p:StatProfile|undefined):PowerPlayContext|undefined {
 const r=p?.power_play_context;
 return r&&r.schema==='rinkcraft.power-play-context.v1'&&r.player_id===p.player_id&&r.season===p.season&&r.group===p.group&&['Forwards','Defense'].includes(r.group)&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&[r.summary,r.time_share,r.assist_share,r.pp_assists,r.outside_assists,r.formula,r.note].every(v=>typeof v==='string')?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderPowerPlayContext(r:PowerPlayContext):HTMLElement {
 const body=el('details');body.className='rk-power-play-context';body.append(el('summary',r.summary));const dl=el('dl');
 for(const [label,value]of [['PP time',r.time_share],['PP assists',r.assist_share],['Assists /60 PP',r.pp_assists],['Assists /60 outside PP',r.outside_assists]]){const row=el('div');row.append(el('dt',label),el('dd',value));dl.append(row);}
 body.append(dl,el('p',`${r.games} GP · NHL matched all-strength / PP inputs`),el('p',r.note));const formula=el('details');formula.append(el('summary','Calculation'),el('p',r.formula));body.append(formula);return body;
}
