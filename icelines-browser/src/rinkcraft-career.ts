export interface CareerAudit {
 same_provider:boolean;
 seasons:{season:number;games:number;seconds:number;teams:string;state:string}[];
 fields:{key:string;state:string;published?:number;landing_seasons?:number;rest_seasons?:number;derived_from_published?:number;published_save_fraction?:number;conflict_seasons?:{season:number;reported_saves:number;shots_against:number;goals_against:number;published_save_fraction:number}[]}[];
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const period=(s:number)=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
export function renderCareerAudit(audit:CareerAudit):HTMLElement{
 const root=node('details');root.className='rk-career-audit';root.append(node('summary','Career coverage & reconciliation'),node('p',`${audit.seasons.length} NHL regular seasons. Landing career totals, NHL season ledger and stats reports are surfaces of the same official provider—not independent corroboration.`));
 for(const field of audit.fields){const d=node('details');d.dataset.careerField=field.key;d.append(node('summary',`${field.key.replaceAll('_',' ')} · ${field.state.replaceAll('_',' ')}`),node('p',`Published ${field.published??'not directly supplied'} · ledger ${field.landing_seasons??'not directly supplied'} · stats seasons ${field.rest_seasons??'not supplied'}${field.derived_from_published===undefined?'':` · shots minus goals ${field.derived_from_published}`}`));
  for(const row of field.conflict_seasons??[])d.append(node('p',`${period(row.season)}: reported saves ${row.reported_saves}, shots against ${row.shots_against}, goals against ${row.goals_against}, published save fraction ${row.published_save_fraction}. No resolution inferred.`));root.append(d);}
 const years=node('details');years.append(node('summary','Included season ledger'));for(const y of audit.seasons)years.append(node('p',`${period(y.season)} · ${y.games} GP · ${y.seconds} exact seconds · ${y.teams}`));root.append(years,node('p','Career totals and rates pool compatible counts and opportunities. No career percentile or seasonal feat threshold has been applied.'));return root;
}
