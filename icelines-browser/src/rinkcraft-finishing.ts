// Display only: Rust selects evidence, computes the multiplier and its interval.
export interface FinishingReadout {
 status:string;note:string;input_sha256:string;model_id:string;model_config_sha256:string;
 target_season:number;calibration_season:number|null;history_seasons:number[];
 history_goals:number|null;history_xg:number|null;mean:number|null;lower:number|null;upper:number|null;probability:number|null;
 sources:{season:number;source:{url:string;sha256:string;bytes:number}}[];limits:string[];
 qualification?:string;
 support?:{rows:{season:number;position:string;goals:number;provider_xg:number;position_factor:number;adjusted_xg:number}[];prior_shape:number;prior_rate:number;posterior_shape:number;posterior_rate:number;adjusted_history_xg:number;history_weight:number}|null;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const period=(s:number)=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
export function renderFinishing(readout:FinishingReadout):HTMLElement {
 const section=node('section');section.className='rk-finishing rk-panel';section.dataset.finishingState=readout.status;
 section.append(node('h4','Finishing · experimental component'),node('p',readout.note));
 section.append(node('p',`Selected snapshot: ${period(readout.target_season)} · model scope: five-on-five skaters`));
 const available=['experimental','baseline_only'].includes(readout.status)&&[readout.mean,readout.lower,readout.upper,readout.probability].every(v=>typeof v==='number'&&Number.isFinite(v));
 if(available){
  const values=node('dl');values.className='rk-finishing-values';
  for(const [label,value] of [['Mean multiplier',readout.mean!.toFixed(3)],[`${(readout.probability!*100).toFixed(0)}% conditional interval`,`${readout.lower!.toFixed(3)} – ${readout.upper!.toFixed(3)}`],['Position baseline','1.000']]){const field=node('div');field.append(node('dt',label),node('dd',value));values.append(field);}
  section.append(values,node('p',`Player history: ${readout.history_seasons.length?readout.history_seasons.map(period).join(', '):'None supplied'} · ${readout.history_goals??0} goals / ${readout.history_xg?.toFixed(2)??'—'} provider xG. Frozen calibration: ${readout.calibration_season?period(readout.calibration_season):'Unknown'}.`));
  if(readout.support){
   const s=readout.support,details=node('details');details.className='rk-finishing-support';
   details.append(node('summary','How this estimate is built'),node('p',`History weight in the posterior mean: ${(s.history_weight*100).toFixed(1)}%. This is model weighting, not confidence, reliability or a capability score. The remaining weight is on the position baseline of 1. These weights do not measure independent evidence.`));
   const scroll=node('div');scroll.className='rk-finishing-scroll';scroll.tabIndex=0;scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Earlier-season finishing evidence');
   const table=node('table'),head=node('thead'),header=node('tr');
   for(const label of ['Earlier season','Position','Goals','Provider xG','Position factor','Adjusted xG'])header.append(node('th',label));head.append(header);table.append(head);
   const body=node('tbody');for(const row of s.rows){const tr=node('tr');tr.dataset.supportSeason=String(row.season);for(const text of [period(row.season),row.position,String(row.goals),row.provider_xg.toFixed(2),row.position_factor.toFixed(3),row.adjusted_xg.toFixed(2)])tr.append(node('td',text));body.append(tr);}table.append(body);scroll.append(table);details.append(scroll);
   if(!s.rows.length)details.append(node('p','No earlier player rows; baseline and its conditional uncertainty only.'));
   details.append(node('p',`Adjusted history exposure: ${s.adjusted_history_xg.toFixed(2)}. Position-residual prior: Gamma(shape ${s.prior_shape.toFixed(3)}, rate ${s.prior_rate.toFixed(3)}). Posterior: Gamma(shape ${s.posterior_shape.toFixed(3)}, rate ${s.posterior_rate.toFixed(3)}). Rate is not scale.`),node('p','Adjusted xG = provider xG × that season’s position factor. Posterior mean = (prior shape + earlier goals) / (prior rate + adjusted history xG). History weight = adjusted history xG / posterior rate. Target-season goals do not enter this calculation; the selected target’s source and qualification are still checked.'));
   section.append(details);
  }
 }
 for(const limit of readout.limits)section.append(node('p',limit));
 if(readout.model_id){const details=node('details');details.append(node('summary','Model, sources & fingerprints'),node('p',`Model: ${readout.model_id}`));if(readout.qualification)details.append(node('p',readout.qualification));details.append(node('p',`Model config SHA256 ${readout.model_config_sha256}`),node('p',`Normalized research input SHA256 ${readout.input_sha256}`));for(const s of readout.sources){const a=node('a',`${period(s.season)} · source`);a.href=s.source.url;a.target='_blank';a.rel='noopener noreferrer';details.append(a,node('p',`SHA256 ${s.source.sha256}`));}section.append(details);}
 return section;
}
