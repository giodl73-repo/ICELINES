import type {NumericalMeasurement} from './rinkcraft-numbers.js';
export interface HistoryCard {
  player_id:number;name:string;
  seasons:{season:number;group:string|null;situation:string|null;games:number|null;minutes:number|null;state:string;sources:{url:string;sha256:string;bytes:number}[]}[];
  rows:{key:string;label:string;points:{season:number;state:string;measurement:NumericalMeasurement|null}[];ranked_seasons:number;upper_quintile_seasons:number|null;comparability:string}[];
  limitations:string[];
}
let pending:Promise<unknown|undefined>|undefined;
export function readNumericalHistory():Promise<unknown|undefined>{
  pending??=(async()=>{let response:Response;try{response=await fetch(new URL('../rinkcraft-history.json',import.meta.url));}catch{return undefined;}if(response.status===404)return undefined;if(!response.ok)throw new Error('Numerical history could not be loaded.');const text=await response.text();if(text.length>8*1024*1024)throw new Error('Numerical history exceeds 8 MiB');return JSON.parse(text) as unknown;})();return pending;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const season=(s:number):string=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
export function renderNumericalHistory(h:HistoryCard):HTMLElement {
  const details=node('details');details.className='rh-history';details.append(node('summary',`Evidence over seasons · ${h.seasons.length} season snapshots · ${h.seasons.filter(s=>s.state==='available').length} reported for this player`),node('p',"MoneyPuck.com · historical regular-season snapshots. Raw rates and each season's own peer ranks. P ≥ 80 counts upper-quintile raw values; higher is not always better."));
  const scroll=node('div');scroll.className='rh-scroll';scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Seasonal numerical evidence table');scroll.tabIndex=0;const table=node('table');const header=node('tr');for(const label of ['Measurement',...h.seasons.map(s=>season(s.season)),'Seasons P ≥ 80 / ranked'])header.append(node('th',label));const head=node('thead');head.append(header);table.append(head);const body=node('tbody');
  for(const r of h.rows){const row=node('tr');row.dataset.historyKey=r.key;row.append(node('td',r.label));for(const p of r.points){const m=p.measurement;const raw=m?.value==null?'—':`${(m.value*(m.unit==='Pct'?100:1)).toFixed(2)}${m.unit==='Pct'?'%':m.unit==='Per60'?' /60':''}`;const rank=m?.percentile==null?(m?.state??p.state).replaceAll('_',' '):`P${(Math.round(m.percentile*10)/10).toFixed(1)}`;row.append(node('td',m?`${raw} · ${rank} · n=${m.measured_peers}`:rank));}const repeat=node('td',r.ranked_seasons===0?'Unranked':r.upper_quintile_seasons===null?'Not comparable':`${r.upper_quintile_seasons} / ${r.ranked_seasons}`);repeat.title=r.comparability;row.append(repeat);body.append(row);}table.append(body);scroll.append(table);details.append(scroll);
  const limits=node('details');limits.append(node('summary','Season exposure and interpretation limits'));for(const s of h.seasons)limits.append(node('p',`${season(s.season)} · ${s.group??'Missing player'} · ${s.games??'—'} GP · ${s.situation??'unavailable'} · ${s.minutes==null?'unavailable':`${s.minutes.toFixed(1)} minutes`}`));for(const season of h.seasons)for(const source of season.sources){const link=node('a','Source snapshot');link.href=source.url;link.target='_blank';link.rel='noopener noreferrer';limits.append(link,node('p',`${source.bytes} bytes · SHA256 ${source.sha256}`));}for(const limit of h.limitations)limits.append(node('p',limit));details.append(limits);return details;
}
