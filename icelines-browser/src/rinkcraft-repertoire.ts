// Curated, already date-selected repertoire. Never derive specialties from scores.
import type {CharacterSheet, Feature} from './rinkcraft-character.js';
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{
 const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;
};
export function renderRepertoire(title:string,className:string,features:Feature[]|undefined,visible:number,sheet:CharacterSheet|undefined):HTMLElement{
 const section=node('section');section.className=`rk-form-box rk-repertoire ${className}`;
 section.append(node('h4',title));
 const entries=features??[];
 if(!entries.length){
  const empty=node('p','Repertoire not yet catalogued.');empty.className='rk-repertoire-empty';section.append(empty);
  section.append(node('small','A missing entry does not imply missing ability.'));return section;
 }
 const item=(f:Feature):HTMLElement=>{
  const row=node('li');row.dataset.repertoireName=f.name;row.append(node('strong',f.name));
  const detail=node('span',f.detail);detail.className='rk-repertoire-detail';row.append(detail);
  const status=node('small',f.evidence_type);status.className='rk-repertoire-status';row.append(status);
  if(f.source_ids.length||(f.stat_evidence?.length??0)||(f.unmeasured_stats?.length??0)){
   const evidence=node('details');evidence.className='rk-method-evidence';evidence.append(node('summary','Sources & gaps'));
   evidence.append(node('p','Related records provide context; they are not a specialty score.'));
   if(f.stat_evidence?.length)evidence.append(node('p','Inspect related measurements in Measured Record and the dossier, with their original scope.'));
   for(const gap of f.unmeasured_stats??[])evidence.append(node('p',`Need: ${gap}`));
   for(const id of f.source_ids){const source=sheet?.sources.find(s=>s.id===id);if(source){const link=node('a',`${id} · ${source.title}`);link.href=source.url;const p=node('p');p.append(link,node('small',`${source.published} · ${source.locator}`));evidence.append(p);}else evidence.append(node('p',`${id} · source record unavailable`));}
   row.append(evidence);
  }
  return row;
 };
 const list=node('ul');list.className='rk-repertoire-list';for(const f of entries.slice(0,visible))list.append(item(f));section.append(list);
 if(entries.length>visible){const more=node('details');more.className='rk-repertoire-more';more.append(node('summary',`+${entries.length-visible} more`));const rest=node('ul');rest.className='rk-repertoire-list';for(const f of entries.slice(visible))rest.append(item(f));more.append(rest);section.append(more);}
 return section;
}
