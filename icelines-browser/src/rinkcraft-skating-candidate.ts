// Rust classifies the family. This module binds and displays the research result.
import type {EdgeEvidence} from './rinkcraft-edge.js';
const names={'top-gear':'Top Gear','burst-engine':'Burst Engine','two-gears':'Two Gears'} as const;
export interface SkatingCandidate {
 schema:string;status:string;name:string;player_id:number;season:number;season_type:string;group:string;games:number;minutes:number;
 min_games:number;min_minutes:number;state:string;candidate:keyof typeof names|null;
 measurements:{metric:string;value:number;percentile:number;peers:number}[];
 edge_sha256:string;rules_sha256:string;candidate_report_sha256:string;source_profile_sha256:string;
}
export function matchesSkatingCandidate(c:SkatingCandidate|undefined,e:EdgeEvidence|undefined,minutes:number|undefined):c is SkatingCandidate {
 if(!c||!e||e.group==='Goalies'||typeof c.name!=='string'||!c.name)return false;
 const hash=(s:string)=>typeof s==='string'&&/^[a-f0-9]{64}$/.test(s);
 if(c.schema!=='icelines.edge-skating-candidate.v1'||c.status!=='research_candidate'||c.player_id!==e.player_id||c.season!==e.season||c.season_type!=='regular'||c.group!==e.group||c.games!==e.games||!Number.isFinite(c.minutes)||!Number.isFinite(minutes)||Math.abs(c.minutes-minutes!)>1e-9||c.min_games!==10||c.min_minutes!==100||c.edge_sha256!==e.source.sha256||![c.rules_sha256,c.candidate_report_sha256,c.source_profile_sha256].every(hash))return false;
 if(!['assessed','unassessed','insufficient_exposure'].includes(c.state)||!(c.candidate===null||Object.prototype.hasOwnProperty.call(names,c.candidate))||c.candidate!==null&&c.state!=='assessed')return false;
 const eligible=c.games>=10&&c.minutes>=100;
 if((c.state==='insufficient_exposure')===eligible||!Array.isArray(c.measurements))return false;
 const keys=new Set<string>();
 for(const m of c.measurements){if(keys.has(m.metric)||!['peak_mph','bursts','bursts_per_60'].includes(m.metric)||!Number.isFinite(m.value)||m.value<0||!Number.isFinite(m.percentile)||m.percentile<0||m.percentile>100||!Number.isSafeInteger(m.peers)||m.peers<1)return false;keys.add(m.metric);}
 if(c.state==='insufficient_exposure'&&c.measurements.length)return false;
 if(c.state==='assessed'&&!['peak_mph','bursts_per_60'].every(k=>c.measurements.some(m=>m.metric===k&&m.peers>=20)))return false;
 return true;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const season=(s:number)=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
export function renderSkatingCandidateDetail(c:SkatingCandidate):HTMLElement {
 const body=node('div');body.className='rk-skating-candidate-detail';
 body.append(node('small','SEASONAL RECORD · RESEARCH CANDIDATE'),node('h2',c.candidate?names[c.candidate]:'Skating record'),node('p',`${c.name} · ${season(c.season)} · ${c.games} GP · ${c.minutes.toFixed(1)} official TOI minutes · all strengths · ${c.group}`));
 if(c.state==='insufficient_exposure')body.append(node('p','Insufficient exposure: this comparison requires at least 10 GP and 100 minutes. No qualification decision was made.'));
 else if(c.state==='unassessed')body.append(node('p','Unassessed: both measurements and at least 20 measured peers are required. Missing evidence is not a failed feat.'));
 else body.append(node('p',c.candidate?'This selected-season record meets the candidate formula below.':'Neither measure reaches P90 in this reference. No candidate is attached; this is not a negative feat or a low Skating grade.'));
 for(const [key,label,unit]of [['peak_mph','Peak speed','mph'],['bursts_per_60','High-speed burst rate','/60']] as const){const m=c.measurements.find(m=>m.metric===key),tile=node('section');tile.append(node('h3',label));if(m)tile.append(node('strong',`${m.value.toFixed(2)} ${unit}`),node('p',`IceLines P${m.percentile.toFixed(1)} · ${m.peers} measured ${c.group.toLowerCase()} · same season`));else tile.append(node('p','Not assessed'));body.append(tile);}
 const formula=node('details');formula.open=true;formula.append(node('summary','Candidate formulas'),node('p','Top Gear: peak-speed P ≥ 90; burst-rate P < 90.'),node('p','Burst Engine: burst-rate P ≥ 90; peak-speed P < 90.'),node('p','Two Gears: both P ≥ 90. Replaces the two individual labels; at most one candidate from this family.'),node('p','Burst rate = recorded 20-plus-mph bursts × 60 / official all-strength TOI minutes. Percentile = 100 × (peers below + ½ tied peers) / measured eligible peers, including this player. Minimum 10 GP and 100 minutes; at least 20 measured peers per metric.'));
 const bursts=c.measurements.find(m=>m.metric==='bursts'),rate=c.measurements.find(m=>m.metric==='bursts_per_60');if(bursts&&rate)formula.append(node('p',`${bursts.value.toFixed(0)} bursts × 60 / ${c.minutes.toFixed(2)} minutes = ${rate.value.toFixed(2)} bursts/60 (display rounded).`));body.append(formula);
 body.append(node('p','These are authored P90 hypotheses, not validated capability ratings. IceLines positional ranks differ from the NHL provider percentiles. Official TOI is not tracked sprint opportunity time. Deployment and tracking coverage remain uncontrolled. Peak speed is a sample maximum.'),node('p','Applies only to the selected season. No acceleration, endurance, puck-pursuit, overall Skating or simulation bonus is inferred. No production feat award.'));
 const sources=node('details');sources.append(node('summary','Evidence lineage'));for(const [label,value]of [['EDGE capture',c.edge_sha256],['Original profile',c.source_profile_sha256],['Candidate rules',c.rules_sha256],['Candidate report',c.candidate_report_sha256]])sources.append(node('p',`${label} SHA-256: ${value}`));body.append(sources);return body;
}
function open(c:SkatingCandidate):void {
 const dialog=node('dialog');dialog.className='rk-capability-dialog rk-skating-dialog';dialog.setAttribute('aria-label','Seasonal skating candidate');
 const close=node('button','Close skating record');close.type='button';close.onclick=()=>dialog.close();dialog.append(close,renderSkatingCandidateDetail(c));document.body.append(dialog);dialog.onclose=()=>dialog.remove();dialog.showModal();
}
export function skatingRecordButton(c:SkatingCandidate):HTMLButtonElement {const b=node('button','Skating record');b.type='button';b.className='rk-skating-record-trigger';b.onclick=()=>open(c);return b;}
export function renderSkatingCandidate(c:SkatingCandidate):HTMLElement|undefined {
 if(c.candidate===null)return undefined;
 const root=node('div');root.className='rk-skating-candidate';root.dataset.skatingCandidate=c.candidate;root.dataset.candidateSeason=String(c.season);
 const b=node('button',names[c.candidate]);b.type='button';b.setAttribute('aria-label',`${names[c.candidate]} skating research candidate`);b.onclick=()=>open(c);
 root.append(b,node('small',`${season(c.season)} · Research candidate`));return root;
}
