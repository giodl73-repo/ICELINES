import type {StatProfile} from './rinkcraft-stat-scores.js';
interface Group {team:string;partners_display:string;time_display:string;share_display:string;}
export interface UnitLinks {schema:string;player_id:number;season:number;group:string;games:number;coverage:string;game_ids:number[];contexts:{key:string;label:string;seconds:number;time_display:string;total_groups:number;tied_leaders:number;groups:Group[]}[];partners:{team:string;member:{name:string};time_display:string;share_display:string}[];recent:{game_ids:number[];first_date:string;last_date:string;leading:Group|null;tied_leaders:number;different_from_full:boolean|null};remainder_display:string;note:string;}
export function unitLinks(p:StatProfile|undefined):UnitLinks|undefined {
 const r=p?.unit_links;return r&&r.schema==='rinkcraft.unit-links.v1'&&r.player_id===p.player_id&&r.season===p.season&&r.group===p.group&&r.games===p.games&&r.contexts?.length===3&&r.contexts.every((c,i)=>c.key===['5v5','5v4','4v5'][i]&&Number.isInteger(c.seconds)&&c.seconds>=0)?r:undefined;
}
const el=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const describe=(g:Group)=>`${g.team} · ${g.partners_display} · ${g.time_display} · ${g.share_display}`;
export function renderUnitLinks(r:UnitLinks):HTMLElement {
 const body=el('div');body.className='rk-unit-links';body.append(el('small',r.coverage));
 for(const c of r.contexts){if(!c.seconds)continue;const row=el('div');row.className='rk-unit-summary';row.dataset.unitState=c.key;row.append(el('strong',c.label));if(c.tied_leaders>1)row.append(el('p',`${c.tied_leaders} co-leading groups`));else if(c.groups.length){const g=c.groups[0];row.append(el('p',`${g.team} · ${g.partners_display}`),el('small',`${g.time_display} · ${g.share_display} of own ${c.key}`));}else row.append(el('p','No standard combination retained'));body.append(row);}
 const details=el('details');details.className='rk-unit-details';details.append(el('summary','Alternates, partners & recent'));
 for(const c of r.contexts){details.append(el('strong',`${c.label} · ${c.total_groups} observed groups`));for(const g of c.groups)details.append(el('p',describe(g)));if(!c.seconds)details.append(el('p','No verified time in this state'));}
 details.append(el('strong','Frequent 5v5 partners'));for(const s of r.partners)details.append(el('p',`${s.team} · ${s.member.name} · ${s.time_display} · ${s.share_display} of own 5v5`));
 const recent=r.recent;details.append(el('strong',`Last ${recent.game_ids.length} matched appearances`),el('p',`${recent.first_date} to ${recent.last_date}`));if(recent.leading)details.append(el('p',describe(recent.leading)));else details.append(el('p',recent.tied_leaders>1?`${recent.tied_leaders} co-leading recent groups`:'No standard recent combination retained'));
 if(recent.different_from_full!==null)details.append(el('p',recent.different_from_full?'Recent leading combination differs':'Same leading combination in both windows'));
 details.append(el('p',r.remainder_display),el('p',r.note),el('small','NHL box scores + native shift charts'));body.append(details);return body;
}
export function renderUnitDeployment(r:UnitLinks):HTMLElement {
 const body=el('div');body.className='rk-unit-deployment';body.append(el('strong','Matched shifts'),el('small',r.coverage));const list=el('dl');for(const c of r.contexts){const row=el('div');row.dataset.unitDeployment=c.key;row.append(el('dt',c.key),el('dd',c.time_display));list.append(row);}body.append(list);return body;
}
