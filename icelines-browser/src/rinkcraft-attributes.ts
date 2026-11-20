// Rendering only: bands, rates and reference populations are supplied by Rust.
export interface Attribute {
  key:string;label:string;layer:string;unit:string;value:number|null;percentile:number|null;band:number|null;
  measured_peers:number;reference_sha256:string;state:string;formula:string;interpretation:string;
}
export interface AttributeSeason {
  protocol:string;season:number;group:string;games:number;minimum_games:number;minimum_peers:number;
  eligible_peers:number;strength_scope:string;attributes:Attribute[];
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{
  const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;
};
const seasonName=(s:number):string=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
export function attributeValue(a:Pick<Attribute,'unit'|'value'>):string {
  if(a.value===null)return 'Unknown';
  if(a.unit==='Pct')return `${(a.value*100).toFixed(2)}%`;
  if(a.unit==='Seconds')return `${(a.value/60).toFixed(1)} min / game`;
  if(a.unit==='Count')return `${a.value.toFixed(0)} starts`;
  return `${a.value.toFixed(2)} / game`;
}
export function renderAttributeSheet(current:AttributeSeason|undefined,history:AttributeSeason[]):HTMLElement {
  const section=node('section');section.className='rc-attribute-sheet';section.append(node('h4','Profile attributes'));
  if(!current){section.append(node('p','Attribute reference data unavailable in this profile.'));return section;}
  section.append(node('p',`${seasonName(current.season)} · ${current.group} · ${current.games} GP · ${current.strength_scope} · Peer minimum ${current.minimum_games} GP`),
    node('p','Bands 1–5 describe higher observed values, not better players or skill levels. Each season uses its own peers. Missing reference data stays unranked.'));
  const grid=node('div');grid.className='rc-attributes';
  for(const a of current.attributes){
    const card=node('article');card.className='rc-attribute';card.dataset.attribute=a.key;
    card.append(node('small',a.layer),node('h4',a.label));
    const level=node('strong',a.band===null?'Unranked':`${a.band} / 5`);level.className='rc-level';card.append(level);
    const track=node('div');track.className='rc-track';track.setAttribute('aria-hidden','true');
    for(let i=1;i<=5;i++){const step=node('span');if(a.band!==null&&i<=a.band)step.className='filled';track.append(step);}card.append(track);
    card.append(node('p',attributeValue(a)),node('p',a.percentile===null?a.state.replaceAll('_',' '):`${a.percentile.toFixed(1)} percentile`),
      node('p',`${a.measured_peers} measured / ${current.eligible_peers} eligible ${current.group} · ${current.games} GP`));
    const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 300 120');svg.setAttribute('role','img');svg.setAttribute('aria-label',`${a.label} seasonal bands`);svg.classList.add('rc-chart');
    const make=(tag:string,attrs:Record<string,string>,text?:string):SVGElement=>{const e=document.createElementNS('http://www.w3.org/2000/svg',tag);for(const [k,v]of Object.entries(attrs))e.setAttribute(k,v);if(text)e.textContent=text;svg.append(e);return e;};
    make('title',{},`${a.label}: historical seasonal bands. Exact values are in the evidence below.`);
    for(const [y,label]of [[22,'5'],[58,'3'],[94,'1']] as const){make('line',{x1:'24',x2:'276',y1:String(y),y2:String(y)});make('text',{x:'4',y:String(y+3)},label);}
    const first=Math.floor(history[0].season/10000);const last=Math.floor(history.at(-1)!.season/10000);
    let previous:number|undefined;let path='';const evidence=node('details');evidence.append(node('summary','Evidence, meaning and season history'));
    evidence.append(node('p',`${a.interpretation} Formula: ${a.formula}. Reference minimum: ${current.minimum_peers}. Fingerprint: ${a.reference_sha256}. Protocol: ${current.protocol}.`));const list=node('ul');
    for(const s of history){
      const at=s.attributes.find(m=>m.key===a.key);if(!at){previous=undefined;continue;}
      list.append(node('li',`${seasonName(s.season)}: ${attributeValue(at)} · band ${at.band??'unknown'} · ${s.games} GP · ${at.measured_peers}/${s.eligible_peers} ${s.group}`));
      const year=Math.floor(s.season/10000);if(at.band===null){previous=undefined;continue;}
      const x=24+252*(year-first)/Math.max(1,last-first),y=94-(at.band-1)*18;
      path+=previous!==undefined&&previous+1===year?` H ${x} V ${y}`:` M ${x} ${y}`;
      const circle=make('circle',{cx:String(x),cy:String(y),r:'3.5'});const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=`${s.season}: band ${at.band}, ${attributeValue(at)}`;circle.append(title);previous=year;
    }
    make('path',{d:path});make('text',{x:'24',y:'116'},String(first));make('text',{x:'247',y:'116'},String(last));
    evidence.append(list);card.append(svg,evidence);grid.append(card);
  }
  section.append(grid,node('p','Statistical aggregates do not measure directional style, assignments or persistent ability. The documentary identity above is a separate evidence layer.'));
  return section;
}
