// Display only: Rust selects facets/leaders and computes comparison differences.
export interface NumericalMeasurement {
  key:string;label:string;domain:string;unit:string;value:number|null;percentile:number|null;
  measured_peers:number;interpretation:string;state:string;
}
export interface NumericalEvidence {
  style_evidence?:StyleEvidence|null;
  season:number;player_id:number;name:string;group:string;situation:string;games:number;minutes:number;
  eligible_peers:number;facets:{label:string;measurement:NumericalMeasurement}[];leading:string[];
  metrics:NumericalMeasurement[];source_credit:string;unknowns:string[];
}
interface StyleEvidence {
  season:number;limitations:string[];gaps:{axis:string;reason:string}[];
  measured:{key:string;label:string;value:number;unit:string;numerator:number;denominator:number;scope:string;meaning:string;sources:{url:string;sha256:string}[]}[];
}
export function renderStyleEvidence(s:StyleEvidence):HTMLElement {
  const section=node('section');section.className='rs-style';section.setAttribute('aria-label','Playing style evidence');section.append(node('h4','Playing style'),node('p',`${period(s.season)} · measured tendencies & open questions`));
  const measures=node('div');measures.className='rs-measures';for(const m of s.measured){const tile=node('div');tile.className='rs-measure';tile.dataset.styleKey=m.key;const suffix:Record<string,string>={Pct:'%',Mph:' mph',Per60:' /60',PerGame:' /game',MilesPerGame:' mi/game'};tile.append(node('span',m.label),node('strong',`${(m.value*(m.unit==='Pct'?100:1)).toFixed(2)}${suffix[m.unit]??''}`),node('small',m.scope),node('p',m.meaning));measures.append(tile);}section.append(measures);
  const gaps=node('div');gaps.className='rs-gaps';for(const g of s.gaps){const row=node('div');row.append(node('b',g.axis),node('span','Unmeasured'),node('p',g.reason));gaps.append(row);}section.append(gaps);
  const details=node('details');details.append(node('summary','Style evidence, calculations & limits'));for(const m of s.measured){details.append(node('p',`${m.label} · ${m.numerator} / ${m.denominator} = ${m.value}`));for(const src of m.sources){const link=node('a','Source');link.href=src.url;link.target='_blank';link.rel='noopener noreferrer';details.append(link,node('p',`SHA256 ${src.sha256}`));}}for(const limit of s.limitations)details.append(node('p',limit));section.append(details);return section;
}
export interface NumericalCatalog {season:number;profiles:{player_id:number;name:string;group:string}[];}
export interface NumericalComparison {
  style?:{rows:{key:string;label:string;unit:string;left:StyleEvidence['measured'][number]|null;right:StyleEvidence['measured'][number]|null;difference:number|null;status:string;reason:string}[];shared_gaps:{axis:string;reason:string}[]}|null;
  season:number;left:NumericalEvidence;right:NumericalEvidence;
  rows:{key:string;label:string;unit:string;left:number|null;right:number|null;left_percentile:number|null;right_percentile:number|null;difference:number|null;percentile_difference:number|null}[];
}
let catalogPromise:Promise<NumericalCatalog|undefined>|undefined;
export function readNumericalCatalog():Promise<NumericalCatalog|undefined>{
  catalogPromise??=(async()=>{let response:Response;try{response=await fetch(new URL('../rinkcraft-numbers.json',import.meta.url));}catch{return undefined;}
    if(response.status===404)return undefined;if(!response.ok)throw new Error('Numerical evidence could not be loaded.');
    const text=await response.text();if(text.length>2*1024*1024)throw new Error('Numerical evidence exceeds 2 MiB');return JSON.parse(text) as NumericalCatalog;
  })();return catalogPromise;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const period=(s:number):string=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
const rank=(v:number|null):string=>v===null?'Unranked':`P${(Math.round(v*10)/10).toFixed(1)}`;
const value=(unit:string,v:number|null,signed=false):string=>v===null?'—':`${signed&&v>=0?'+':''}${(v*(unit==='Pct'?100:1)).toFixed(2)}${unit==='Pct'?(signed?' pp':'%'):''}`;
const styleValue=(unit:string,v:number|null,signed=false):string=>v===null?'—':`${value(unit,v,signed)}${({Per60:' /60',Mph:' mph',PerGame:' /game',MilesPerGame:' mi/game'} as Record<string,string>)[unit]??''}`;
function renderStyleComparison(c:NumericalComparison):HTMLElement {
  const section=node('section');section.className='rs-comparison';section.id='playing-style-comparison';section.setAttribute('aria-label','Playing style comparison');section.append(node('h3','How their measured styles differ'),node('p','Each row keeps its own situation and denominator. Differences use full precision; percentage shares differ in percentage points.'));
  const s=c.style!;if(s.rows.length){const scroll=node('div');scroll.className='rn-comparison-scroll';scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Playing style comparison table');scroll.tabIndex=0;const table=node('table');const head=node('tr');for(const label of ['Measured tendency',c.left.name,c.right.name,'Left − right','Evidence coverage'])head.append(node('th',label));const thead=node('thead');thead.append(head);table.append(thead);const body=node('tbody');
    for(const r of s.rows){const row=node('tr');row.dataset.styleComparisonKey=r.key;const cell=(m:typeof r.left,cls:string):HTMLElement=>{const td=node('td',m?styleValue(m.unit,m.value):'Unmeasured');td.className=cls;if(m)td.append(node('small',m.scope));return td;};const status=node('td',({matched:'Matched definition',left_only:'Left player only',right_only:'Right player only'} as Record<string,string>)[r.status]??'Different context');status.title=r.reason;row.append(node('td',r.label),cell(r.left,'rn-left'),cell(r.right,'rn-right'),node('td',styleValue(r.unit,r.difference,true)),status);body.append(row);}table.append(body);scroll.append(table);section.append(scroll);
  }else section.append(node('p','No measured playing-method pair is available. Saving outcomes above remain separate.'));
  if(s.shared_gaps.length){section.append(node('h4','Still unmeasured for both players'));const gaps=node('div');gaps.className='rs-shared-gaps';for(const g of s.shared_gaps){const details=node('details');details.append(node('summary',g.axis),node('p',g.reason));gaps.append(details);}section.append(gaps);}return section;
}
export function renderNumericalPanel(card:NumericalEvidence):HTMLElement {
  const section=node('section');section.className='rk-panel rn-integrated';section.append(node('h4','Numerical profile'));
  const scope=node('p',`${period(card.season)} · ${card.situation} · ${card.games} GP · ${card.minutes.toFixed(1)} minutes · ${card.eligible_peers} peers`);scope.className='rk-stat-period';section.append(scope);
  const leading=node('p');leading.className='rn-leading';leading.append(node('small','Leading recorded signals'),document.createTextNode(card.leading.map(key=>card.facets.find(f=>f.measurement.key===key)?.label??key).join(' · ')));section.append(leading);
  const facets=node('div');facets.className='rn-facets';for(const f of card.facets){const m=f.measurement;const tile=node('div');tile.className='rn-facet';tile.setAttribute('data-numerical-key',m.key);tile.title=m.interpretation;
    tile.append(node('span',f.label),node('strong',rank(m.percentile)),node('b',`${value(m.unit,m.value)}${m.unit==='Per60'?' /60':''}`));
    if(m.percentile!==null){const track=node('i');track.className='rk-mini-track';track.setAttribute('aria-hidden','true');const fill=node('i');fill.style.width=`${m.percentile}%`;track.append(fill);tile.append(track);}tile.append(node('small',`n=${m.measured_peers}`));facets.append(tile);
  }section.append(facets);
  const note=node('p','Raw-value percentiles, not skill grades. Leading signals are the highest ranks among a fixed involvement/output shortlist.');note.className='rk-footnote';section.append(note);
  const details=node('details');details.append(node('summary',`All ${card.metrics.length} numerical measures`));const table=node('table');table.className='rn-inline-table';const body=node('tbody');for(const m of card.metrics){const row=node('tr');const label=node('td',m.label);label.title=m.interpretation;row.append(label,node('td',value(m.unit,m.value)),node('td',rank(m.percentile)));body.append(row);}table.append(body);details.append(table);section.append(details);
  const credit=node('p',card.source_credit);credit.className='rk-footnote';section.append(credit);
  const compare=node('button','Compare this numerical profile');compare.type='button';compare.onclick=()=>document.dispatchEvent(new CustomEvent('rinkcraft-numerical-compare',{detail:{player_id:card.player_id}}));section.append(compare);return section;
}
export function renderNumericalComparison(c:NumericalComparison):HTMLElement {
  const section=node('section');section.className='rn-comparison';section.append(node('h3',`${c.left.name} / ${c.right.name}`),node('p',`${period(c.season)} · ${c.left.group} · ${c.left.situation} · MoneyPuck.com`));
  const scroll=node('div');scroll.className='rn-comparison-scroll';scroll.setAttribute('role','region');scroll.setAttribute('aria-label','Numerical comparison table');scroll.tabIndex=0;const table=node('table');const head=node('tr');for(const label of ['Measurement',c.left.name,'Peer ranks · 0 → 100',c.right.name,'Left − right'])head.append(node('th',label));const thead=node('thead');thead.append(head);table.append(thead);const body=node('tbody');
  for(const r of c.rows.filter(r=>c.left.facets.some(f=>f.measurement.key===r.key))){const row=node('tr');const left=node('td',value(r.unit,r.left));left.className='rn-left';left.append(node('small',rank(r.left_percentile)));const right=node('td',value(r.unit,r.right));right.className='rn-right';right.append(node('small',rank(r.right_percentile)));const axis=node('td');axis.className='rn-comparison-axis';if(r.left_percentile!==null&&r.right_percentile!==null){const track=node('div');track.className='rn-dumbbell';for(const [cls,p]of [['rn-left-dot',r.left_percentile],['rn-right-dot',r.right_percentile]] as const){const dot=node('i');dot.className=cls;dot.style.left=`${p}%`;track.append(dot);}axis.append(track);}const difference=node('td',value(r.unit,r.difference,true));difference.append(node('small',r.percentile_difference===null?'—':`${r.percentile_difference>=0?'+':''}${r.percentile_difference.toFixed(1)} pp`));row.append(node('td',r.label),left,axis,right,difference);body.append(row);}table.append(body);scroll.append(table);section.append(scroll,node('p','Differences describe measured rates and raw percentile ranks. They are not differences in overall ability or class fit.'));if(c.style)section.append(renderStyleComparison(c));return section;
}
