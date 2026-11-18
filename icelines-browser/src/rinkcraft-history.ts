import type {NumericalMeasurement} from './rinkcraft-numbers.js';
import {renderMeasuredAssessment} from './rinkcraft-assessment.js';
import type {MeasuredAssessment} from './rinkcraft-assessment.js';
export interface HistoryCard {
  changes?:SeasonChange[];
  assessment?:MeasuredAssessment|null;
  player_id:number;name:string;
  seasons:{season:number;group:string|null;situation:string|null;games:number|null;minutes:number|null;state:string;sources:{url:string;sha256:string;bytes:number}[]}[];
  rows:{key:string;label:string;points:{season:number;state:string;measurement:NumericalMeasurement|null}[];ranked_seasons:number;upper_quintile_seasons:number|null;comparability:string}[];
  limitations:string[];
}
export interface SeasonChange {from_season:number;to_season:number;from_games:number|null;to_games:number|null;from_minutes:number|null;to_minutes:number|null;rows:{key:string;label:string;unit:string;state:string;from_value:number|null;to_value:number|null;difference:number|null;percentile_point_difference:number|null;note:string}[];}
let pending:Promise<unknown|undefined>|undefined;
export function readNumericalHistory():Promise<unknown|undefined>{
  pending??=(async()=>{let response:Response;try{response=await fetch(new URL('../rinkcraft-history.json',import.meta.url));}catch{return undefined;}if(response.status===404)return undefined;if(!response.ok)throw new Error('Numerical history could not be loaded.');const text=await response.text();if(text.length>8*1024*1024)throw new Error('Numerical history exceeds 8 MiB');return JSON.parse(text) as unknown;})();return pending;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const season=(s:number):string=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
const changeValue=(unit:string,v:number|null,signed=false):string=>{if(v===null)return '—';const n=v*(unit==='Pct'?100:1),rounded=Math.abs(n)<.005?0:n;return `${signed&&rounded>0?'+':''}${rounded.toFixed(2)}${unit==='Pct'?(signed?' pp':'%'):unit==='Per60'?' /60':''}`;};
const peerDifference=(v:number|null):string=>{if(v===null)return 'Unavailable';const rounded=Math.abs(v)<.05?0:v;return `${rounded>0?'+':''}${rounded.toFixed(1)} percentile points`;};
export function renderSeasonChanges(changes:SeasonChange[]):HTMLElement {
 const details=node('details');details.className='rh-changes';details.append(node('summary','What changed between snapshots?'),node('p','Later minus earlier recorded results. Workload, rates and annual peer ranks are separate. These differences do not establish ability changes, causes or statistical significance; higher is not always better. Display values are rounded.'));
 for(const c of changes){
  const group=node('section');group.dataset.changeWindow=String(c.to_season);group.append(node('h5',`${season(c.from_season)} → ${season(c.to_season)}`),node('p',`Exposure: ${c.from_games??'—'} → ${c.to_games??'—'} GP · ${c.from_minutes?.toFixed(1)??'—'} → ${c.to_minutes?.toFixed(1)??'—'} minutes. Adjacent supplied snapshots; no missing-year interpolation.`));
  const scroll=node('div');scroll.className='rh-scroll';scroll.tabIndex=0;scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Recorded snapshot differences');const table=node('table'),head=node('thead'),header=node('tr');for(const label of ['Measurement','Earlier','Later','Raw difference','Annual peer-rank difference','Context'])header.append(node('th',label));head.append(header);table.append(head);const body=node('tbody');
  for(const r of c.rows){const tr=node('tr');tr.dataset.changeKey=r.key;tr.dataset.changeState=r.state;tr.append(node('td',r.label),node('td',changeValue(r.unit,r.from_value)),node('td',changeValue(r.unit,r.to_value)),node('td',changeValue(r.unit,r.difference,true)),node('td',peerDifference(r.percentile_point_difference)));const context=node('td');const more=node('details');more.append(node('summary',r.state.replaceAll('_',' ')),node('p',r.note));context.append(more);tr.append(context);body.append(tr);}table.append(body);scroll.append(table);group.append(scroll);details.append(group);
 }return details;
}
export function renderNumericalHistory(h:HistoryCard):HTMLElement {
  const wrapper=node('section');wrapper.className='rh-wrapper';if(h.assessment){const latest=[...h.seasons].reverse().find(s=>s.group);const context=latest?`${season(latest.season)}: ${latest.group} · ${latest.situation==='all'?'all strengths':latest.situation??'unavailable'}`:'Unavailable';wrapper.append(renderMeasuredAssessment(h.assessment,h.rows,context));}
  const details=node('details');details.className='rh-history';details.append(node('summary',`Evidence over seasons · ${h.seasons.length} season snapshots · ${h.seasons.filter(s=>s.state==='available').length} reported for this player`),node('p',"MoneyPuck.com · historical regular-season snapshots. Raw rates and each season's own peer ranks. P ≥ 80 counts upper-quintile raw values; higher is not always better."));
  if(h.changes?.length)details.append(renderSeasonChanges(h.changes));
  const scroll=node('div');scroll.className='rh-scroll';scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Seasonal numerical evidence table');scroll.tabIndex=0;const table=node('table');const header=node('tr');for(const label of ['Measurement',...h.seasons.map(s=>season(s.season)),'Seasons P ≥ 80 / ranked'])header.append(node('th',label));const head=node('thead');head.append(header);table.append(head);const body=node('tbody');
  for(const r of h.rows){const row=node('tr');row.dataset.historyKey=r.key;row.append(node('td',r.label));for(const p of r.points){const m=p.measurement;const raw=m?.value==null?'—':`${(m.value*(m.unit==='Pct'?100:1)).toFixed(2)}${m.unit==='Pct'?'%':m.unit==='Per60'?' /60':''}`;const rank=m?.percentile==null?(m?.state??p.state).replaceAll('_',' '):`P${(Math.round(m.percentile*10)/10).toFixed(1)}`;row.append(node('td',m?`${raw} · ${rank} · n=${m.measured_peers}`:rank));}const repeat=node('td',r.ranked_seasons===0?'Unranked':r.upper_quintile_seasons===null?'Not comparable':`${r.upper_quintile_seasons} / ${r.ranked_seasons}`);repeat.title=r.comparability;row.append(repeat);body.append(row);}table.append(body);scroll.append(table);details.append(scroll);
  const limits=node('details');limits.append(node('summary','Season exposure and interpretation limits'));for(const s of h.seasons)limits.append(node('p',`${season(s.season)} · ${s.group??'Missing player'} · ${s.games??'—'} GP · ${s.situation??'unavailable'} · ${s.minutes==null?'unavailable':`${s.minutes.toFixed(1)} minutes`}`));for(const season of h.seasons)for(const source of season.sources){const link=node('a','Source snapshot');link.href=source.url;link.target='_blank';link.rel='noopener noreferrer';limits.append(link,node('p',`${source.bytes} bytes · SHA256 ${source.sha256}`));}for(const limit of h.limitations)limits.append(node('p',limit));details.append(limits);wrapper.append(details);return wrapper;
}
