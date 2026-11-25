import type {StatProfile} from './rinkcraft-stat-scores.js';
export interface PenaltyBalance {schema:string;player_id:number;season:number;group:string;games:number;minutes:number;drawn:string;taken:string;balance:string;formula:string;note:string;}
export function penaltyBalance(p:StatProfile|undefined):PenaltyBalance|undefined {
 const r=p?.penalty_balance;
 return r&&r.schema==='rinkcraft.penalty-balance.v1'&&r.player_id===p.player_id&&r.season===p.season&&r.group===p.group&&['Forwards','Defense'].includes(r.group)&&r.games===p.games&&Number.isFinite(r.minutes)&&Math.abs(r.minutes-(p.minutes??NaN))<=1e-9&&[r.drawn,r.taken,r.balance,r.formula,r.note].every(v=>typeof v==='string')?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderPenaltyBalance(r:PenaltyBalance):HTMLElement {
 const body=el('details');body.className='rk-penalty-balance';body.append(el('summary',`Penalty balance ${r.balance}`));
 const dl=el('dl');for(const [label,value]of [['Drawn',r.drawn],['Taken',r.taken],['Drawn − taken',r.balance]]){const row=el('div');row.append(el('dt',label),el('dd',value));dl.append(row);}
 body.append(dl,el('p',`${r.games} GP · all strengths · NHL recorded calls`),el('p',r.formula),el('p',r.note));return body;
}
