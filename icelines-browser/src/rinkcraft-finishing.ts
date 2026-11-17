// Display only: Rust selects evidence, computes the multiplier and its interval.
export interface FinishingReadout {
 status:string;note:string;input_sha256:string;model_id:string;model_config_sha256:string;
 target_season:number;calibration_season:number|null;history_seasons:number[];
 history_goals:number|null;history_xg:number|null;mean:number|null;lower:number|null;upper:number|null;probability:number|null;
 sources:{season:number;source:{url:string;sha256:string;bytes:number}}[];limits:string[];
 qualification?:string;
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
 }
 for(const limit of readout.limits)section.append(node('p',limit));
 if(readout.model_id){const details=node('details');details.append(node('summary','Model, sources & fingerprints'),node('p',`Model: ${readout.model_id}`));if(readout.qualification)details.append(node('p',readout.qualification));details.append(node('p',`Model config SHA256 ${readout.model_config_sha256}`),node('p',`Normalized research input SHA256 ${readout.input_sha256}`));for(const s of readout.sources){const a=node('a',`${period(s.season)} · source`);a.href=s.source.url;a.target='_blank';a.rel='noopener noreferrer';details.append(a,node('p',`SHA256 ${s.source.sha256}`));}section.append(details);}
 return section;
}
