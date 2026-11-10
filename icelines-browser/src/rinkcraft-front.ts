// Rinkcraft presentation categories. Unassessed capability fields stay unrated.
// Numerical evidence and documentary identity are supplied independently by Rust.
import type {CharacterSheet} from './rinkcraft-character.js';
import type {AttributeSeason} from './rinkcraft-attributes.js';
import {attributeValue} from './rinkcraft-attributes.js';
import type {NumericalEvidence, SnapshotSelection} from './rinkcraft-numbers.js';
import {numericalValue} from './rinkcraft-numbers.js';

interface SeasonRecord {
  games:number;
  position:string;
  metrics:{key:string;unit:string;value:number|null}[];
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{
  const element=document.createElement(tag);
  if(text!==undefined)element.textContent=text;
  return element;
};
const field=(label:string,value:string):HTMLElement=>{
  const cell=node('div');cell.className='rk-field';
  cell.append(node('dt',label),node('dd',value));
  return cell;
};
const box=(title:string,className:string):HTMLElement=>{
  const section=node('section');section.className=`rk-form-box ${className}`;
  section.append(node('h4',title));
  return section;
};
function taskBox(title:string,className:string,tasks:readonly string[]):HTMLElement {
  const section=box(title,className),list=node('dl');list.className='rk-task-list';
  for(const task of tasks)list.append(field(task,'Unassessed'));
  section.append(list);return section;
}
function namesBox(title:string,className:string,names:readonly string[],empty="Undocumented"):HTMLElement {
  const section=box(title,className),list=node('ul');
  for(const name of names)list.append(node('li',name));
  if(!names.length)list.append(node('li',empty));
  section.append(list);return section;
}

export function renderCharacterFront(
  name:string,sheet:CharacterSheet|undefined,assessment:AttributeSeason|undefined,
  record:SeasonRecord|undefined,numerical:NumericalEvidence|undefined,
  snapshot:SnapshotSelection|undefined,catalogAvailable:boolean|undefined,
):HTMLElement {
  const position=record?.position??numerical?.group??assessment?.group??'Unknown';
  const goalie=['Goalie','Goalies','G'].includes(position);
  const front=node('section');front.className='rk-front';front.dataset.layout='rinkcraft-profile-v2';
  front.setAttribute('aria-label',`${name} compact player sheet`);
  const header=node('header');header.className='rk-front-header';
  const identity=node('div');identity.append(node('small','ICELINES / RINKCRAFT'),node('h3',name));
  const positionField=node('div');positionField.className='rk-front-position';
  positionField.append(node('small','Position'),node('strong',position));header.append(identity,positionField);
  const fields=node('dl');fields.className='rk-front-identity';
  fields.append(field('Class',sheet?.primary.name??(catalogAvailable===false?'Class dossier unavailable':'Unscouted')),
    field('Specialty',sheet?.specialty??'Unknown'),field('Secondary',sheet?.secondary.map(c=>c.name).join(' / ')||'—'));

  const development=field('Development','Undocumented');development.className+=' rk-development-field';
  fields.append(development,field('Experience','Undocumented'));
  const season=numerical?.season??assessment?.season;
  const scope=node('p',numerical
    ?`${Math.floor(numerical.season/10000)}–${String(numerical.season%100).padStart(2,'0')} · ${numerical.situation} · ${numerical.group}`
    :season?`${Math.floor(season/10000)}–${String(season%100).padStart(2,'0')} · all strengths`:'Statistical window unavailable');
  scope.className='rk-front-scope';front.append(header,fields,scope);

  const columns=node('div');columns.className='rk-front-columns';
  const left=node('div');left.className='rk-front-left';
  const split=node('div');split.className='rk-left-split';
  const attributes=node('section');attributes.className='rk-front-attributes';
  attributes.append(node('h4','Attributes'));const scale=node('small','1–20 scale');scale.className='rk-rating-scale';attributes.append(scale);
  const scores=node('dl');scores.className='rk-front-values';
  const labels=goalie?['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play']:['Skating','Handling','Passing','Shooting','Reading','Defending'];
  for(const [index,label] of labels.entries()){
    const score=field(label,'—');score.dataset.capabilityIndex=String(index);score.dataset.ratingState='unassessed';
    score.append(node('small','Unrated'));scores.append(score);
  }
  attributes.append(scores);
  const tasks=node('div');tasks.className='rk-front-task-column';
  tasks.append(taskBox('Under Pressure','rk-front-pressure',goalie?['Screened Shot','Rebound Threat','Lateral Rush']:['Escape','Retain','Recover']),
    taskBox('Task Skills','rk-front-skills',goalie?['Set Depth','Track Traffic','Save Selection','Rebound Direction','Puck Exchange']:['Retrieval','Zone Exit','Zone Entry','Coverage','Screening']),
    taskBox('Read & React','rk-front-reading',['Anticipation']));
  split.append(attributes,tasks);left.append(split);
  const coverage=box('Role Coverage','rk-front-role-coverage');
  const positions=node('dl');positions.className='rk-task-list';positions.append(field('Position',position),field('Special Teams','Undocumented'));coverage.append(positions);left.append(coverage);

  const center=node('div');center.className='rk-front-center';
  const responses=node('div');responses.className='rk-response-row';
  for(const title of ['Threat Denial','Response Timing',goalie?'Movement Detail':'Skating Detail']){
    const response=box(title,'rk-front-response');response.append(node('strong','—'),node('small','Unassessed'));responses.append(response);
  }
  const counters=box('Record Counters','rk-front-counters'),counterValues=node('dl');counterValues.className='rk-counter-values';
  counters.append(counterValues);counterValues.append(field('GP',String(numerical?.games??record?.games??assessment?.games??'—')),
    field('MIN',numerical?numerical.minutes.toFixed(0):'—'));
  const methods=namesBox('Signature Methods','rk-front-methods',sheet?.features.map(f=>f.name)??[]);
  const measured=box('Measured Record','rk-front-record');
  const values=node('dl');values.className='rk-record-values';
  if(numerical)for(const facet of numerical.facets){
    const m=facet.measurement,cell=field(facet.label,numericalValue(m));cell.dataset.numericalKey=m.key;
    cell.append(node('small',m.percentile==null?'Unranked':`P${m.percentile.toFixed(1)} · n=${m.measured_peers}`));values.append(cell);
  }
  else if(assessment)for(const a of assessment.attributes){
    const cell=field(a.label,attributeValue(a));
    cell.append(node('small',a.percentile==null?'Unranked':`P${a.percentile.toFixed(1)} · n=${a.measured_peers}`));values.append(cell);
  }
  else values.append(field('Coverage','Unavailable'));
  measured.append(values,node('p','P = raw-value percentile; position peers. Rates and outcomes, not capability grades.'));
  const deployment=box('Deployment','rk-front-deployment'),assignment=node('dl');assignment.className='rk-task-list';
  assignment.append(field('Unit','Undocumented'),field('Station','Undocumented'));deployment.append(assignment);
  center.append(responses,counters,methods,measured,deployment);

  const right=node('div');right.className='rk-front-right';
  right.append(namesBox('Playing Tendencies','rk-front-traits',sheet?.traits.map(t=>t.name)??[]));
  for(const [title,className,text] of [
    ['Assigned Role','rk-front-role','Undocumented'],['Unit Links','rk-front-links','Undocumented'],
    ['Known Limitations','rk-front-limitations','Unassessed'],
  ]){const section=box(title,className);section.append(node('p',text));right.append(section);}
  const distinctive=box('Distinctive Features','rk-front-distinctive');
  distinctive.append(node('strong',sheet?.specialty??'Undocumented'));
  if(sheet)distinctive.append(node('p',sheet.summary));
  const questions=namesBox('Open Questions','rk-front-questions',sheet?.unknowns??[],sheet?'None recorded':'Undocumented');
  right.append(distinctive,questions);columns.append(left,center,right);front.append(columns);

  const sources=node('dl');sources.className='rk-front-coverage';
  sources.append(field('Sources',sheet?String(sheet.sources.length):'Unknown'),
    field('Open Questions',sheet?String(sheet.unknowns.length):'Unknown'),
    field('Evidence Through',sheet?.evidence_through??'Unknown'),field('Assessed',sheet?.assessed??'Unknown'));
  front.append(sources);
  if(snapshot){const note=node('p',snapshot.note);note.className='rk-front-snapshot';note.dataset.snapshotState=snapshot.state;front.append(note);}
  const footer=node('footer',sheet?'Provisional documentary class · capability ratings unassessed · evidence below':catalogAvailable===false?'Class dossier unavailable · statistics do not assign a class':'No supported class dossier · statistics do not assign a class');
  footer.className='rk-front-footer';front.append(footer);return front;
}
