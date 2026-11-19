// Rust supplies annotations; presentation neither awards feats nor scores abilities.
import type {AchievementProfile} from './rinkcraft-achievements.js';
export interface ProfileObservations {
 schema:string;policy:string;player_id:number;season:number;season_type:string;group:string;games:number;
 source_sha256:string;analysis_sha256:string;positive_count:number;caution_count:number;coverage_gaps:string[];note:string;
 low_callouts:{id:string;kind:string;label:string;percentile:number;reference_group:string;eligible_peers?:number;minimums?:Record<string,number>;condition:{key:string;value:number;unit?:string;numerator?:number;denominator?:number;percentile:number;peers?:number;reference_sha256?:string};interpretation:string}[];
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function matchesObservations(o:ProfileObservations|undefined,a:AchievementProfile):o is ProfileObservations {
 return !!o&&o.schema==='rinkcraft.profile-observations.v1'&&o.policy==='lower-tail-observations.v0.1-exploratory'
  &&o.player_id===a.player_id&&o.season===a.season&&o.season_type===a.season_type&&o.group===a.group&&o.games===a.games
  &&['Forwards','Defense'].includes(o.group)&&o.games>=20&&o.source_sha256===a.source_sha256&&/^[a-f0-9]{64}$/.test(o.analysis_sha256)
  &&[o.positive_count,o.caution_count].every(n=>Number.isSafeInteger(n)&&n>=0)
  &&Array.isArray(o.coverage_gaps)&&o.coverage_gaps.every(s=>typeof s==='string')&&Array.isArray(o.low_callouts)
  &&o.low_callouts.every(c=>!!c&&!!c.condition&&typeof c.id==='string'&&typeof c.label==='string'&&typeof c.interpretation==='string'&&c.reference_group===a.group
   &&['low_output','low_activity','low_opportunity_conditioned_output'].includes(c.kind)
   &&Number.isFinite(c.percentile)&&c.percentile>=0&&c.percentile<=20&&c.condition?.percentile===c.percentile
   &&Number.isFinite(c.condition.value)&&c.condition.value>=0&&(c.condition.peers??c.eligible_peers??0)>=20)
  &&new Set(o.low_callouts.map(c=>c.id)).size===o.low_callouts.length;
}
export function renderObservations(a:AchievementProfile):HTMLElement|null {
 const candidate=a.profile_observations;if(!matchesObservations(candidate,a))return null;const o=candidate;
 const root=node('div');root.className='rk-profile-observations';root.dataset.observationSeason=String(o.season);
 const counts=node('p',`${o.positive_count} positive · ${o.caution_count} caution · ${o.low_callouts.length} low`);counts.className='rk-observation-counts';root.append(counts);
 const compact=o.low_callouts.slice(0,2),more=o.low_callouts.slice(2);
 function item(c:ProfileObservations['low_callouts'][number]):HTMLElement {
  const d=node('details');d.className='rk-low-observation';d.dataset.observationId=c.id;d.dataset.observationKind=c.kind;
  const title=c.kind==='low_activity'?'Low activity':c.kind==='low_output'?'Low output':'Low rate';
  const s=node('summary');s.append(node('strong',c.label.replace(/^Low recorded /,'Low ')),node('small',`P${c.percentile.toFixed(1)}`));d.append(s);
  const unit=c.condition.unit==='PerGame'?'/ GP':c.condition.key==='pp_points_per60'?'/ 60 PP min':c.condition.key==='conversion'?'fraction':'';
  d.append(node('p',`${title} · ${c.condition.value.toFixed(3)} ${unit} · n=${c.condition.peers??c.eligible_peers} ${o.group} · ${o.games} GP.`),node('p',c.interpretation));
  if(c.condition.numerator!==undefined)d.append(node('small',`${c.condition.numerator} / ${c.condition.denominator??'unknown'} · Reference ${c.condition.reference_sha256??'see analysis'}`));
  if(c.minimums)d.append(node('small',`Exposure gates: ${Object.entries(c.minimums).map(([k,v])=>`${k} ≥ ${v}`).join(' · ')}`));
  d.append(node('small',`Report ${o.source_sha256} · Analysis ${o.analysis_sha256}`));return d;
 }
 compact.forEach(c=>root.append(item(c)));
 if(more.length){const library=node('details');library.className='rk-observation-library';library.append(node('summary',`${more.length} more low observations`));more.forEach(c=>library.append(item(c)));root.append(library);}
 const gaps=node('details');gaps.className='rk-observation-gaps';gaps.append(node('summary','Assessment gaps'),node('p',o.coverage_gaps.join(' · ')),node('p','Unmeasured here; not player weaknesses.'),node('p',o.note));root.append(gaps);
 return root;
}
