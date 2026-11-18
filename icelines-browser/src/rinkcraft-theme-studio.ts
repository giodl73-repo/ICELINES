// Local design playground. Themes never compute or modify NHL assessments.
import {renderCharacterCard} from './rinkcraft-character.js';
import type {CharacterSheet} from './rinkcraft-character.js';
import {renderCharacterFront} from './rinkcraft-front.js';
import type {RinkcraftProfile} from './rinkcraft-profile.js';

const themes=[
  {id:'ice',name:'Ice Sheet',tag:'The current favorite',detail:'Crisp rink lines, compact boxes and a familiar vertical attribute rail.'},
  {id:'ledger',name:'Guild Ledger',tag:'Bookish · ornamental',detail:'Serif typography, parchment, double rules and shield-shaped attribute fields.'},
  {id:'bureau',name:'Scouting Bureau',tag:'Archival · utilitarian',detail:'Typed records, numbered sections, ruled cells and a stamped position badge.'},
  {id:'tactics',name:'Tactical Board',tag:'Coaching · diagrammatic',detail:'Dark board, chalk-like lines and a horizontal six-attribute strip on desktop.'},
  {id:'broadcast',name:'Broadcast',tag:'Athletic · high impact',detail:'A jersey-stripe masthead, bold condensed type and cut-corner number blocks.'},
  {id:'editorial',name:'Editorial',tag:'Quiet · typographic',detail:'Serif masthead, large numerals and fine rules in place of most boxes.'},
] as const;
interface Specimen {
 key:string;name:string;position:string;class:string;specialty:string;secondary:string;purpose:string;
 development:string;experience:string;attributes:number[];pressure:string[];skills:string[];reading:string;responses:string[];
 games:number;minutes:number;methods:string[][];record:string[][];tendencies:string[];assignment:string;links:string;
 limitations:string;distinctive:string;questions:string[];deployment:string[];coverage:string;
}
interface Specimens {kind:string;notice:string;players:Specimen[];}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{
 const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;
};
const setText=(root:Element,selector:string,text:string)=>{
 const e=root.querySelector(selector);if(!e)throw Error(`Missing specimen field: ${selector}`);e.textContent=text;
};
function specimenCard(s:Specimen):HTMLElement {
 const sheet:CharacterSheet={schema:'fictional-design-only',protocol:'not an assessment',catalog_sha256:'',player_name:s.name,
  assessed:'Not assessed',evidence_through:'Fictional',as_of_season:0,
  primary:{id:s.class.toLowerCase(),name:s.class,analogy:s.class==='Warden'?'Paladin':'Wizard',purpose:s.purpose},
  secondary:[{id:s.secondary.toLowerCase(),name:s.secondary,analogy:'',purpose:''}],specialty:s.specialty,summary:s.distinctive,
  features:s.methods.map(([name,detail])=>({name,detail,evidence_type:'Fictional design sample',source_ids:[]})),
  traits:s.tendencies.map(name=>({name,detail:'',evidence_type:'Fictional design sample',source_ids:[]})),sources:[],unknowns:s.questions};
 const front=renderCharacterFront(s.name,sheet,undefined,{position:s.position,games:s.games,metrics:[]},undefined,undefined,true);
 front.dataset.profileKind='fictional_design_sample';
 setText(front,'.rk-front-header>div:first-child>small','RINKCRAFT / FICTIONAL DESIGN SAMPLE');
 setText(front,'.rk-front-scope','Fictional season · 5v5 · every number and assessment is illustrative');
 const fields=front.querySelectorAll('.rk-front-identity .rk-field');
 fields[3].querySelector('dd')!.textContent=s.development;fields[4].querySelector('dd')!.textContent=s.experience;
 front.querySelectorAll<HTMLElement>('[data-capability-index]').forEach((field,i)=>{
  field.dataset.ratingState='illustrative';field.querySelector('dd')!.textContent=String(s.attributes[i]);
  field.querySelector('small')!.textContent='Illustrative';
 });
 for(const [selector,values] of [
  ['.rk-front-pressure',s.pressure],['.rk-front-skills',s.skills],['.rk-front-reading',[s.reading]],
  ['.rk-front-deployment',s.deployment],
 ] as const)front.querySelectorAll(`${selector} dd`).forEach((e,i)=>e.textContent=values[i]??'Illustrative');
 front.querySelectorAll('.rk-front-response').forEach((e,i)=>{
  e.querySelector('strong')!.textContent=s.responses[i];e.querySelector('small')!.textContent='Illustrative';
 });
 const counters=front.querySelectorAll('.rk-counter-values dd');counters[0].textContent=String(s.games);counters[1].textContent=String(s.minutes);
 const record=front.querySelector('.rk-record-values')!;record.replaceChildren();
 for(const [label,value,rank]of s.record){const field=node('div');field.className='rk-field';field.append(node('dt',label),node('dd',value),node('small',rank));record.append(field);}
 setText(front,'.rk-front-record>p','Fictional record · illustrative percentiles · no measured reference cohort.');
 for(const [selector,text]of [['.rk-front-role',s.assignment],['.rk-front-links',s.links],['.rk-front-limitations',s.limitations]])setText(front,`${selector}>p`,text);
 const coverage=front.querySelectorAll('.rk-front-role-coverage .rk-field');coverage[1].querySelector('dt')!.textContent='Situations';coverage[1].querySelector('dd')!.textContent=s.coverage;
 front.querySelectorAll('.rk-method-evidence').forEach(e=>e.remove());
 setText(front,'.rk-front-footer','FICTIONAL DESIGN SAMPLE · all ratings, records and duties are illustrative · not an NHL assessment or calibrated game character');
 return front;
}

export async function mountThemeStudio(root:HTMLElement):Promise<void>{
 const responses=await Promise.all([fetch('./fixtures.json'),fetch('./specimens.json')]);
 if(responses.some(r=>!r.ok))throw Error('The local profile examples could not be loaded.');
 const profiles=await responses[0].json() as Record<string,RinkcraftProfile>;
 const specimens=await responses[1].json() as Specimens;
 if(specimens.kind!=='fictional_design_sample'||specimens.players.some(s=>s.attributes.length!==6))throw Error('Invalid design specimens.');
 const query=new URLSearchParams(location.search);
 let index=themes.findIndex(t=>t.id===query.get('theme'));if(index<0)index=0;
 const sample=root.querySelector<HTMLSelectElement>('#sample')!,stage=root.querySelector<HTMLElement>('#profile-stage')!;
 const choices=root.querySelector<HTMLElement>('#theme-choices')!,range=root.querySelector<HTMLInputElement>('#theme-dial')!;
 const name=root.querySelector<HTMLElement>('#theme-name')!,number=root.querySelector<HTMLElement>('#theme-number')!,description=root.querySelector<HTMLElement>('#theme-description')!;
 for(const s of specimens.players){const option=node('option',`Fictional · ${s.name} · ${s.position}`);option.value=s.key;sample.append(option);}
 for(const [key,p]of Object.entries(profiles)){const option=node('option',`NHL · ${p.player_name} · 2024–25`);option.value=key;sample.append(option);}
 if(Array.from(sample.options).some(o=>o.value===query.get('sample')))sample.value=query.get('sample')!;
 const buttons:HTMLButtonElement[]=[];
 themes.forEach((t,i)=>{
  const button=node('button');button.type='button';button.className='theme-choice';button.dataset.theme=t.id;
  const swatch=node('span',String(i+1).padStart(2,'0'));swatch.className=`theme-swatch swatch-${t.id}`;swatch.setAttribute('aria-hidden','true');
  const words=node('span');words.append(node('strong',t.name),node('small',t.tag));button.append(swatch,words);
  button.addEventListener('click',()=>selectTheme(i));choices.append(button);buttons.push(button);
 });
 function remember(){const url=new URL(location.href);url.searchParams.set('theme',themes[index].id);url.searchParams.set('sample',sample.value);history.replaceState(null,'',url);}
 function selectTheme(next:number){
  index=(next+themes.length)%themes.length;const t=themes[index];document.documentElement.dataset.rinkcraftTheme=t.id;
  name.textContent=t.name;number.textContent=`${String(index+1).padStart(2,'0')} / 06`;description.textContent=t.detail;
  range.value=String(index);range.setAttribute('aria-valuetext',t.name);
  buttons.forEach((b,i)=>b.setAttribute('aria-pressed',String(i===index)));remember();
 }
 function showSample(){
  const specimen=specimens.players.find(s=>s.key===sample.value),notice=root.querySelector<HTMLElement>('#data-notice')!;
  if(specimen){stage.replaceChildren(specimenCard(specimen));notice.textContent=specimens.notice;notice.dataset.kind='fictional';}
  else {
   const p=profiles[sample.value];if(!p)throw Error('Unknown local example.');
   stage.replaceChildren(renderCharacterCard(p.player_name,p.character_sheet,p.attribute_seasons?.at(-1),p.latest_available??undefined,p.numerical_evidence,p.numerical_history,p.numerical_snapshot,p.numerical_comparison_choices,p.documentary_catalog_available,p.evidence_guide));
   notice.textContent='Historical NHL evidence · 2024–25. Measured records and documentary methods are populated; unvalidated 1–20 capability scores remain unrated.';notice.dataset.kind='nhl';
  }
  remember();
 }
 root.querySelector('#previous-theme')!.addEventListener('click',()=>selectTheme(index-1));
 root.querySelector('#next-theme')!.addEventListener('click',()=>selectTheme(index+1));
 range.addEventListener('input',()=>selectTheme(Number(range.value)));sample.addEventListener('change',showSample);
 root.querySelector('#print-profile')!.addEventListener('click',()=>window.print());
 root.querySelector('#copy-link')!.addEventListener('click',async()=>{
  const status=root.querySelector<HTMLElement>('#link-status')!;
  try{await navigator.clipboard.writeText(location.href);status.textContent='Link copied';}catch{status.textContent='Copy the address above to keep this selection.';}
 });
 document.addEventListener('keydown',event=>{
  const target=event.target as HTMLElement;
  if(target.closest('input,select,textarea,button,[contenteditable]')||event.altKey||event.ctrlKey||event.metaKey)return;
  if(event.key==='ArrowRight'||event.key===']'){event.preventDefault();selectTheme(index+1);}
  if(event.key==='ArrowLeft'||event.key==='['){event.preventDefault();selectTheme(index-1);}
 });
 showSample();selectTheme(index);root.dataset.ready='true';
}
const root=document.querySelector<HTMLElement>('[data-theme-studio]');
if(root)void mountThemeStudio(root).catch(error=>{root.dataset.ready='error';root.querySelector('#profile-stage')!.textContent=String(error);});
