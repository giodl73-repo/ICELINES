// Literal record markers are evaluated in Rust, never percentile or ability tiers.
import type {ProfileTimeWindow} from './rinkcraft-scope.js';
export interface RecordTrack {id:string;name:string;metric:string;unit:string;priority:number;raw_value:number|null;divisor:number;value:number|null;state:string;highest:number|null;next:number|null;remaining_raw:number|null;steps:{threshold:number;required_raw:number;earned:boolean|null}[];}
export interface RecordMilestones {schema:string;player_id:number;scope_kind:string;first_season:number;last_season:number;season_type:string;as_of:string;group:string;rules_sha256:string;source_input_sha256:string;tracks:RecordTrack[];notice:string;}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const number=(n:number)=>n.toLocaleString('en-US',{maximumFractionDigits:1});
export function matchesRecordWindow(r:RecordMilestones|undefined,w:ProfileTimeWindow|undefined,group:string|undefined,sourceHash:string|undefined):boolean {
 return !!r&&!!w&&w.kind==='career'&&w.coverage==='career_reconciled'&&r.schema==='rinkcraft.career-records.v1'&&r.scope_kind==='career'&&r.player_id===w.player_id&&r.first_season===w.first_season&&r.last_season===w.season&&r.season_type===w.season_type&&r.as_of===w.as_of&&r.group===group&&r.source_input_sha256===sourceHash;
}
function row(t:RecordTrack):HTMLElement {
 const d=node('details');d.className='rk-record-track';d.dataset.recordTrack=t.id;d.dataset.recordState=t.state;
 const summary=node('summary');summary.append(node('strong',t.highest===null?t.name:`${number(t.highest)} ${t.unit}`),node('small',t.value===null?'Unavailable':`${number(t.value)} recorded${t.next===null?' · highest defined marker':` · next ${number(t.next)}`}`));d.append(summary);
 const steps=node('div');steps.className='rk-record-steps';for(const s of t.steps){const e=node('span',`${s.earned===null?'?':s.earned?'✓':'○'} ${number(s.threshold)}`);e.dataset.earned=String(s.earned);steps.append(e);}d.append(steps,node('p',t.raw_value===null?'Required career count is unavailable.':`${t.raw_value.toLocaleString('en-US')} verified ${t.divisor===60?'seconds':t.unit}. ${t.remaining_raw===null?'No further marker defined.':`${t.remaining_raw.toLocaleString('en-US')} ${t.divisor===60?'seconds':t.unit} to next marker.`}`));return d;
}
export function renderRecordMilestones(records:RecordMilestones):HTMLElement {
 const root=node('section');root.className='rk-record-milestones';root.append(node('small','Career record milestones'));
 const primary=records.tracks.filter(t=>t.highest!==null).slice(0,2);for(const t of primary){const e=row(t);e.dataset.recordPrimary='true';root.append(e);}
 const library=node('details');library.className='rk-record-library';library.append(node('summary','Browse record tracks'));for(const t of records.tracks)library.append(row(t));library.append(node('p',records.notice),node('p',`Rules ${records.rules_sha256} · source ${records.source_input_sha256}`));root.append(library);return root;
}
