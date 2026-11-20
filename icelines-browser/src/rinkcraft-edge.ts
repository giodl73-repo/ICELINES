// Rust parses EDGE; these are provider observations, not capability grades.
export interface EdgeMeasure{key:string;label:string;value:number;unit:string;provider_percentile:number|null;provider_average:number|null;source_field:string;attribute_id:string|null;component_id:string|null;interpretation:string;}
export interface EdgeLocation{code:string;shots:number|null;goals:number|null;saves:number|null;goals_against:number|null;fraction:number|null;provider_percentile:number|null;}
export interface EdgeEvidence{schema:string;player_id:number;season:number;season_type:string;group:string;games:number;captured_at:string;strength_scope:string;source:{url:string;sha256:string;bytes:number};measures:EdgeMeasure[];locations:EdgeLocation[];notes:string[];}
export function matchesEdge(e:EdgeEvidence|undefined,id:number|undefined,season:number|undefined,group:string|undefined,games:number|undefined):e is EdgeEvidence{
 const fraction=(v:number|null)=>v===null||Number.isFinite(v)&&v>=0&&v<=1;
 return !!e&&e.schema==='icelines.rinkcraft-edge.v1'&&e.player_id===id&&e.season===season&&e.season_type==='regular'&&e.group===group&&e.games===games&&e.games>0&&e.strength_scope==='all strengths'&&Number.isFinite(Date.parse(e.captured_at))
  &&e.source?.url===`https://api-web.nhle.com/v1/edge/${group==='Goalies'?'goalie-detail':'skater-detail'}/${id}/${season}/2`&&/^[a-f0-9]{64}$/.test(e.source.sha256)&&e.source.bytes>0&&e.source.bytes<=2*1024*1024
  &&Array.isArray(e.measures)&&e.measures.every(m=>m&&typeof m.label==='string'&&typeof m.interpretation==='string'&&Number.isFinite(m.value)&&m.value>=0&&['Mph','Miles','Count','Pct'].includes(m.unit)&&fraction(m.provider_percentile)&&(m.provider_average===null||Number.isFinite(m.provider_average)&&m.provider_average>=0)&&(m.unit!=='Pct'||m.value<=1)&&(m.unit!=='Count'||Number.isSafeInteger(m.value))&&(!['Mph','Miles'].includes(m.unit)||m.value>0))
  &&Array.isArray(e.locations)&&e.locations.every(l=>l&&typeof l.code==='string'&&fraction(l.fraction)&&fraction(l.provider_percentile)&&[l.shots,l.goals,l.saves,l.goals_against].every(n=>n===null||Number.isSafeInteger(n)&&n>=0))&&Array.isArray(e.notes);
}
export const edgeValue=(m:Pick<EdgeMeasure,'unit'|'value'>):string=>m.unit==='Mph'?`${m.value.toFixed(2)} mph`:m.unit==='Miles'?`${m.value.toFixed(2)} mi`:m.unit==='Pct'?`${(m.value*100).toFixed(1)}%`:m.value.toLocaleString('en-US');
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderEdgeEvidence(e:EdgeEvidence,attribute:string):HTMLElement{
 const section=node('section');section.className='rk-edge-evidence';section.dataset.edgeSeason=String(e.season);
 section.append(node('h3','This player’s NHL EDGE evidence'),node('p',`${Math.floor(e.season/10000)}–${String(e.season%100).padStart(2,'0')} · ${e.games} GP · all strengths · captured ${e.captured_at.slice(0,19).replace('T',' ')} UTC`));
 const selected=e.measures.filter(m=>m.attribute_id===attribute||attribute==='CAP-DEFENDING'&&m.key.endsWith('zone-puck-time'));
 if(selected.length){const grid=node('div');grid.className='rk-edge-measures';for(const m of selected){const tile=node('section');tile.append(node('h4',m.label),node('strong',edgeValue(m)));if(m.provider_percentile!==null)tile.append(node('small',`NHL percentile P${(m.provider_percentile*100).toFixed(1)}`));if(m.provider_average!==null)tile.append(node('small',`NHL reported average ${edgeValue({...m,value:m.provider_average})}`));tile.append(node('p',m.interpretation));grid.append(tile);}section.append(grid);}
 else if(e.group!=='Goalies')section.append(node('p','This EDGE detail does not directly measure the required facets of this attribute.'));
 if((attribute==='CAP-SHOOTING'||e.group==='Goalies')&&e.locations.length){
  section.append(node('h4',e.group==='Goalies'?'Shot-stopping context · not a technique grade':'Shot-location context · not target accuracy'));
  const scroll=node('div');scroll.className='rk-edge-location-scroll';scroll.tabIndex=0;scroll.setAttribute('role','region');scroll.setAttribute('aria-label','NHL EDGE shot-location summary');
  const table=node('table'),head=node('tr');for(const text of ['NHL band',e.group==='Goalies'?'Saves / GA':'Goals / shots',e.group==='Goalies'?'Save %':'Shooting %','NHL percentile'])head.append(node('th',text));table.append(head);
  for(const l of e.locations){const row=node('tr');for(const text of [l.code,e.group==='Goalies'?`${l.saves??'—'} / ${l.goals_against??'—'}`:`${l.goals??'—'} / ${l.shots??'—'}`,l.fraction===null?'—':`${(l.fraction*100).toFixed(2)}%`,l.provider_percentile===null?'Unranked':`P${(l.provider_percentile*100).toFixed(1)}`])row.append(node('td',text));table.append(row);}scroll.append(table);section.append(scroll);
 }
 const source=node('details');source.append(node('summary','NHL benchmark, scope & source'));for(const note of e.notes)source.append(node('p',note));const link=node('a','NHL EDGE source response');link.href=e.source.url;link.target='_blank';link.rel='noopener noreferrer';source.append(link,node('small',`Capture SHA-256 ${e.source.sha256}`));section.append(source);return section;
}
