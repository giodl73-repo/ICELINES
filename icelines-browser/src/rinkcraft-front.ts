import {skatingDetail,renderSkatingDetail} from './rinkcraft-skating-detail.js';
import {unitLinks,renderUnitLinks,renderUnitDeployment} from './rinkcraft-unit-links.js';
import {pressureRecord,pressureEntries,pressureGaps} from './rinkcraft-under-pressure.js';
import {renderGoaliePuck} from './rinkcraft-goalie-puck.js';
import {renderShotContext} from './rinkcraft-goalie-shots.js';
import {renderSkaterContext} from './rinkcraft-skater-context.js';
import {defendingRecord,renderDefendingRecord} from './rinkcraft-defending-record.js';
import {handlingRecord,handlingSummary} from './rinkcraft-handling-record.js';
import {readingRecord,readingSummary} from './rinkcraft-reading-record.js';
import {passingRecord,passingSummary} from './rinkcraft-passing-record.js';
import {shootingRecord,shootingSummary} from './rinkcraft-shooting-record.js';
// Rinkcraft presentation categories. Unassessed capability fields stay unrated.
// Numerical evidence and documentary identity are supplied independently by Rust.
import type {CharacterSheet} from './rinkcraft-character.js';
import type {AttributeSeason} from './rinkcraft-attributes.js';
import {attributeValue} from './rinkcraft-attributes.js';
import type {NumericalEvidence, SnapshotSelection} from './rinkcraft-numbers.js';
import {numericalValue} from './rinkcraft-numbers.js';
import {renderStatFeats} from './rinkcraft-achievements.js';
import {renderObservations} from './rinkcraft-observations.js';
import {openCapability} from './rinkcraft-capability.js';
import {matchesStatScores,openStatScore} from './rinkcraft-stat-scores.js';
import {matchesEdge} from './rinkcraft-edge.js';
import {matchesSkatingCandidate,renderSkatingCandidate,skatingRecordButton} from './rinkcraft-skating-candidate.js';
import {matchesShootingCandidate,renderShootingCandidate,shootingRecordButton} from './rinkcraft-shooting-candidate.js';
import type {AchievementProfile} from './rinkcraft-achievements.js';
import {renderRepertoire} from './rinkcraft-repertoire.js';
import {renderDeploymentUsage} from './rinkcraft-deployment.js';
import type {DeploymentEvidence} from './rinkcraft-deployment.js';

interface SeasonRecord {
  season?:number;
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
function factField(label:string,key:'development'|'experience',sheet:CharacterSheet|undefined):HTMLElement {
  const fact=sheet?.card_facts?.find(f=>f.field===key);
  const cell=field(label,fact?.name??'Undocumented');
  cell.dataset.factField=key;
  if(!fact)return cell;
  cell.append(node('small',fact.context));
  const evidence=node('details');evidence.className='rk-fact-evidence';
  evidence.append(node('summary','Record source'),node('p',fact.detail));
  for(const id of fact.source_ids){
    const source=sheet?.sources.find(s=>s.id===id);
    const p=node('p');
    if(source){const a=node('a',`${id} · ${source.title}`);a.href=source.url;p.append(a,node('small',`${source.published} · ${source.locator}`));}
    else p.append(node('span',`${id} · source record unavailable`));
    evidence.append(p);
  }
  cell.append(evidence);return cell;
}
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
  snapshot:SnapshotSelection|undefined,catalogAvailable:boolean|undefined,deploymentEvidence?:DeploymentEvidence,usageRequest?:{player_id?:number;season_type:string},achievements?:AchievementProfile,statAttributeSeasons?:import('./rinkcraft-stat-scores.js').StatProfile[],goaliePuck?:import('./rinkcraft-goalie-puck.js').GoaliePuckEvidence,
):HTMLElement {
  const position=record?.position??numerical?.group??assessment?.group??'Unknown';
  const goalie=['Goalie','Goalies','G'].includes(position);
  const targetSeason=numerical?.season??record?.season??assessment?.season;
  const featGroup=goalie?'Goalies':['D','Defense'].includes(position)?'Defense':'Forwards';
  const statActor=usageRequest?.player_id??sheet?.player_id,targetGames=(numerical?.group===featGroup?numerical?.games:undefined)??record?.games;
  const usage=deploymentEvidence&&deploymentEvidence.season===targetSeason&&deploymentEvidence.goalie===goalie&&deploymentEvidence.player_id===(usageRequest?.player_id??sheet?.player_id)&&deploymentEvidence.season_type===usageRequest?.season_type&&(targetGames===undefined||deploymentEvidence.games===targetGames)?deploymentEvidence:undefined;
  const candidates=[...(statAttributeSeasons??[]).filter(p=>p.games!==undefined),numerical?.player_id===statActor?numerical?.stat_attributes:undefined];
  const statScores=usageRequest?.season_type==='regular'?candidates.find(p=>matchesStatScores(p,statActor,targetSeason,featGroup,targetGames)):undefined;
  const statFeats=achievements&&achievements.player_id===(usageRequest?.player_id??sheet?.player_id)&&achievements.season===targetSeason&&achievements.season_type===usageRequest?.season_type&&achievements.group===featGroup?achievements:undefined;
  const edge=usageRequest?.season_type==='regular'&&matchesEdge(numerical?.edge_evidence,usageRequest?.player_id??sheet?.player_id,targetSeason,featGroup,numerical?.games)?numerical!.edge_evidence:undefined;
  const skating=matchesSkatingCandidate(numerical?.edge_skating_candidate,edge,numerical?.minutes)?numerical!.edge_skating_candidate:undefined;
  const shooting=usageRequest?.season_type==='regular'&&numerical?.player_id===(usageRequest?.player_id??sheet?.player_id)&&matchesShootingCandidate(numerical?.edge_shooting_candidate,numerical,edge)?numerical!.edge_shooting_candidate:undefined;
  const front=node('section');front.className='rk-front';front.dataset.layout='rinkcraft-profile-v2';front.dataset.class=sheet?.primary.id??'unscouted';front.dataset.profileKind='nhl_evidence';
  front.setAttribute('aria-label',`${name} compact player sheet`);
  const header=node('header');header.className='rk-front-header';
  const identity=node('div');identity.append(node('small','ICELINES / RINKCRAFT'),node('h3',name));
  const positionField=node('div');positionField.className='rk-front-position';
  positionField.append(node('small','Position'),node('strong',position));header.append(identity,positionField);
  const fields=node('dl');fields.className='rk-front-identity';
  fields.append(field('Class',sheet?.primary.name??(catalogAvailable===false?'Class dossier unavailable':'Unscouted')),
    field('Specialty',sheet?.specialty??'Unknown'),field('Secondary',sheet?.secondary.map(c=>c.name).join(' / ')||'—'));

  const classCell=fields.children[0];classCell.className+=' rk-class-field';
  if(sheet){const emblem=node('span',sheet.primary.name.slice(0,2).toUpperCase());emblem.className='rk-class-emblem';emblem.setAttribute('aria-hidden','true');classCell.append(emblem);const identityNote=node('small',`${sheet.primary.analogy} · ${sheet.primary.purpose}`);identityNote.className='rk-class-note';classCell.append(identityNote);}
  const development=factField('Development','development',sheet);development.className+=' rk-development-field';
  fields.append(development,factField('Experience','experience',sheet));
  const season=numerical?.season??assessment?.season;
  const scope=node('p',numerical
    ?`${numerical.scope_label??`${Math.floor(numerical.season/10000)}–${String(numerical.season%100).padStart(2,'0')}`} · ${numerical.situation} · ${numerical.group}`
    :season?`${Math.floor(season/10000)}–${String(season%100).padStart(2,'0')} · all strengths`:'Statistical window unavailable');
  scope.className='rk-front-scope';front.append(header,fields,scope);
  if(snapshot){const note=node('p',snapshot.note);note.className='rk-front-snapshot';note.dataset.snapshotState=snapshot.state;front.append(note);}

  const columns=node('div');columns.className='rk-front-columns';
  const left=node('div');left.className='rk-front-left';
  const split=node('div');split.className='rk-left-split';
  const attributes=node('section');attributes.className='rk-front-attributes';
  attributes.append(node('h4','Attributes'));const scale=node('small',statScores?'1–20 estimates':'1–20 scale');scale.className='rk-rating-scale';attributes.append(scale);
  const scores=node('dl');scores.className='rk-front-values';
  const labels=goalie?['Tracking','Positioning','Movement','Rebounds','Recovery','Puck Play']:['Skating','Handling','Passing','Shooting','Reading','Defending'];
  for(const [index,label] of labels.entries()){
    const estimate=statScores?.scores[index];
    const score=field(label,estimate?.score===null||estimate?.score===undefined?'—':String(estimate.score));score.dataset.capabilityIndex=String(index);score.dataset.ratingState=estimate?.score!==null&&estimate?.score!==undefined?'stat_estimate':'unassessed';
    const rule=node('button',label);rule.type='button';rule.className='rk-capability-trigger';rule.setAttribute('aria-label',`How ${label} is scored`);rule.onclick=()=>{if(statScores&&estimate)openStatScore(statScores,estimate,name);else void openCapability(label,goalie,name,edge);};score.children[0].textContent='';score.children[0].append(rule);
    score.append(node('small',estimate?.score!==null&&estimate?.score!==undefined?(statScores?.small_sample?'Small sample':'Estimate'):'Unrated'));scores.append(score);
    if(label==='Handling'){const handling=handlingRecord(statScores);if(handling)score.append(handlingSummary(handling));}
    if(label==='Passing'){const passing=passingRecord(statScores);if(passing)score.append(passingSummary(passing));}
    if(label==='Shooting'){const shooting=shootingRecord(statScores);if(shooting)score.append(shootingSummary(shooting));}
  }
  attributes.append(scores);
  const tasks=node('div');tasks.className='rk-front-task-column';
  const pressure=pressureRecord(statScores);
  if(pressure&&(pressure.entries.length||pressure.actions?.length)){const section=box('Under Pressure','rk-front-pressure');section.append(pressureEntries(pressure));if(sheet?.under_pressure?.length){const scouting=node('details');scouting.className='rk-scouted-pressure';scouting.append(node('summary','Scouted responses'),renderRepertoire('Accounts','rk-pressure-accounts',sheet.under_pressure,3,sheet));section.append(scouting);}section.append(pressureGaps(pressure));tasks.append(section);}
  else {const section=renderRepertoire('Under Pressure','rk-front-pressure',sheet?.under_pressure,3,sheet);if(pressure)section.append(pressureGaps(pressure));tasks.append(section);}
  if(statFeats){const skills=box('Task Skills','rk-front-skills');skills.append(renderStatFeats(statFeats,'task_skills',5));const scouted=node('details');scouted.className='rk-scouted-tasks';scouted.append(node('summary','Scouted specialties'),renderRepertoire('Scouted tasks','rk-scouted-task-list',sheet?.task_skills,5,sheet));skills.append(scouted);tasks.append(skills);}
  else tasks.append(renderRepertoire('Task Skills','rk-front-skills',sheet?.task_skills,5,sheet));
  const reading=readingRecord(statScores);if(reading){const section=box('Read & React','rk-front-reading');section.append(readingSummary(reading));tasks.append(section);}else tasks.append(taskBox('Read & React','rk-front-reading',['Anticipation']));
  split.append(attributes,tasks);left.append(split);
  const coverage=box('Role Coverage','rk-front-role-coverage');
  const positions=node('dl');positions.className='rk-task-list';
  const pp=usage?.rows.find(r=>r.key==='pp'),sh=usage?.rows.find(r=>r.key==='sh');
  positions.append(field('Position',position),field('Special Teams',usage?(goalie?'See goalie workload':`PP ${pp?.per_game_display??'—'} / SH ${sh?.per_game_display??'—'} per GP`):'Undocumented'));coverage.append(positions);if(statFeats)coverage.append(renderStatFeats(statFeats,'role_coverage',2));left.append(coverage);

  const center=node('div');center.className='rk-front-center';
  const responses=node('div');responses.className='rk-response-row';
  for(const title of ['Threat Denial','Response Timing',goalie?'Movement Detail':'Skating Detail']){
    const response=box(title,'rk-front-response');
    const movement=title==='Skating Detail'||title==='Movement Detail';
    if(movement){response.className+=' rk-front-movement';response.dataset.movementState='unassessed';}
    const speed=!goalie&&title==='Skating Detail'&&['Forwards','Defense'].includes(numerical?.group??'')&&numerical?.style_evidence?.season===numerical?.season
      ?numerical?.style_evidence?.measured.find(m=>m.key==='max-skating-speed'&&m.unit==='Mph'&&Number.isFinite(m.value)&&m.value>=0&&typeof m.scope==='string'&&m.scope.startsWith('All strengths')):undefined;
    const observedSkating=title==='Skating Detail'?skatingDetail(statScores):undefined;
    const edgeSpeed=!goalie&&title==='Skating Detail'?edge?.measures.find(m=>m.key==='max-skating-speed'&&m.unit==='Mph'):undefined;
    const defense=title==='Threat Denial'&&!goalie?defendingRecord(statScores):undefined;
    if(defense){response.className+=' rk-front-defense';response.append(renderDefendingRecord(defense));}
    else if(observedSkating){response.dataset.movementState='observed_record';response.append(renderSkatingDetail(observedSkating));}
    else if(edgeSpeed){response.dataset.movementState='observed_maximum';response.append(node('strong',`${edgeSpeed.value.toFixed(2)} mph`),node('small','EDGE season max · all strengths'));response.title=`Captured ${edge!.captured_at} · ${edgeSpeed.interpretation}`;}
    else if(speed){response.dataset.movementState='observed_maximum';response.append(node('strong',`${speed.value.toFixed(2)} mph`),node('small','Season max · all strengths'));response.title=`${speed.scope}. Observed maximum; not puck-carrying speed, acceleration or a capability grade.`;}
    else response.append(node('strong','—'),node('small','Unassessed'));
    if(title==='Skating Detail'&&skating)response.append(skatingRecordButton(skating));
    responses.append(response);
  }
  const counters=box('Record Counters','rk-front-counters'),counterValues=node('dl');counterValues.className='rk-counter-values';
  counters.append(counterValues);counterValues.append(field('GP',String(numerical?.games??record?.games??assessment?.games??'—')),
    field('MIN',numerical?numerical.minutes.toFixed(0):statScores?.minutes===undefined?'—':statScores.minutes.toFixed(0)));
  const methods=namesBox('Signature Methods','rk-front-methods',sheet?.features.map(f=>f.name)??[]);
  // Keep the hockey action visible; the same short name can describe different tasks.
  for(const [index,feature] of (sheet?.features??[]).entries()){
    const item=methods.children[1].children[index];
    if(feature.detail){const detail=node('span',feature.detail);detail.className='rk-method-detail';item.append(detail);}
    const status=node('small',feature.evidence_type);status.className='rk-method-status';
    status.title=feature.source_ids.join(', ');item.append(status);
    const evidence=node('details');evidence.className='rk-method-evidence';
    evidence.append(node('summary','Stats, sources & gaps'));
    const body=node('div');body.className='rk-method-evidence-body';
    const contexts=(feature.stat_evidence??[]).flatMap(link=>numerical?.feature_context?.filter(c=>c.attribute_key===link.attribute_key)??[]);
    body.append(node('p','Related statistics do not establish this method or its effectiveness.'));
    if(numerical)body.append(node('p',`${Math.floor(numerical.season/10000)}–${String(numerical.season%100).padStart(2,'0')} · ${numerical.situation} · ${numerical.group}`));
    if(!contexts.length)body.append(node('p',numerical?'No matched statistic in this snapshot.':'No numerical snapshot available.'));
    for(const context of contexts){
      body.append(node('p',context.note));
      for(const m of context.measurements){
        const row=node('div');row.className='rk-method-measurement';row.dataset.measurementKey=m.key;
        row.append(node('span',`${m.label} · ${context.relation}`),node('strong',numericalValue(m)),
          node('small',`${m.percentile==null?'Unranked':`P${m.percentile.toFixed(1)}`} · n=${m.measured_peers}`));
        if(m.formula)row.append(node('small',m.formula));
        if(m.numerator!=null&&m.denominator!=null)row.append(node('small',`Numerator ${m.numerator} · denominator ${m.denominator} · scale ${m.scale??1}`));
        body.append(row);
      }
    }
    for(const gap of feature.unmeasured_stats??[])body.append(node('p',`Need: ${gap}`));
    for(const id of feature.source_ids){
      const source=sheet?.sources.find(s=>s.id===id);
      const p=node('p');if(source){const a=node('a',`${id} · ${source.title}`);a.href=source.url;p.append(a,node('small',`${source.published} · ${source.locator}`));}
      else p.append(node('span',`${id} · source record unavailable`));body.append(p);
    }
    evidence.append(body);item.append(evidence);
  }
  const measured=box('Estimated Record','rk-front-record');
  const projection=(statScores?.estimated_record?.season===statScores?.season?statScores?.estimated_record:undefined)??(numerical?.estimated_record?.season===numerical?.season?numerical?.estimated_record:undefined);
  measured.append(node('small',projection?.basis??'Recorded rates'));
  const values=node('dl');values.className='rk-record-values';
  if(projection)for(const m of projection.metrics){
    const total=m.value===null?'—':m.projected_total?Math.round(m.value).toLocaleString('en-US'):numericalValue({unit:m.unit,value:m.value} as import('./rinkcraft-numbers.js').NumericalMeasurement);
    const cell=field(m.label,total);cell.dataset.numericalKey=m.key;
    cell.append(node('small',m.per_game===null?(m.value===null?'Unavailable':'Current rate'):`${m.per_game.toFixed(2)} /game`));values.append(cell);
  }
  else if(numerical)for(const facet of numerical.facets){
    const m=facet.measurement,cell=field(facet.label,numericalValue(m));cell.dataset.numericalKey=m.key;cell.title=m.interpretation;
    cell.append(node('small',m.state==='conflicting_sources'?'Source conflict':m.value===null?'Unavailable':m.unit==='PerGame'?'Per game':'Recorded value'));values.append(cell);
  }
  else if(assessment)for(const a of assessment.attributes){
    const cell=field(a.label,attributeValue(a));
    cell.append(node('small','Recorded rate'));values.append(cell);
  }
  else values.append(field('Coverage','Unavailable'));
  measured.append(values);
  if(projection?.metrics.some(m=>m.calculation)){
    const details=node('details');details.className='rk-pace-detail';details.append(node('summary','Observed record & pace'));
    if(projection.observed_games!==undefined)details.append(node('p',`${projection.observed_games} observed GP / ${projection.target_games}-game pace`));
    for(const m of projection.metrics.filter(m=>m.calculation)){const row=node('div');row.dataset.paceKey=m.key;row.append(node('strong',m.label));if(m.recorded_count!==undefined)row.append(node('span',`${m.recorded_count} recorded`));row.append(node('small',m.calculation));details.append(row);}
    details.append(node('p','Projected totals round to whole counts. Ratios and TOI remain rates. Straight pace is not an availability forecast.'));measured.append(details);
  }

  const deployment=box('Deployment','rk-front-deployment'),assignment=node('dl');assignment.className='rk-task-list';
  if(usage)deployment.append(renderDeploymentUsage(usage));
  else {assignment.append(field('Unit','Undocumented'),field('Station','Undocumented'));deployment.append(assignment);}
  const observedUnits=unitLinks(statScores);if(observedUnits)deployment.append(renderUnitDeployment(observedUnits));
  center.append(responses,counters,methods,measured,deployment);

  const right=node('div');right.className='rk-front-right';
  const tendencies=statFeats?box('Playing Tendencies','rk-front-traits'):namesBox('Playing Tendencies','rk-front-traits',sheet?.traits.map(t=>t.name)??[]);if(statFeats){tendencies.append(renderStatFeats(statFeats,'playing_tendencies',2));if(sheet?.traits.length){const scouted=node('details');scouted.append(node('summary','Scouted tendencies'),namesBox('Accounts','rk-scouted-traits',sheet.traits.map(t=>t.name)));tendencies.append(scouted);}}right.append(tendencies);
  for(const [title,className,text] of [
    ['Assigned Role','rk-front-role','Undocumented'],['Unit Links','rk-front-links','Undocumented'],
    ['Known Limitations','rk-front-limitations','Unassessed'],
  ]){const section=box(title,className);if(className==='rk-front-links'&&observedUnits)section.append(renderUnitLinks(observedUnits));else section.append(node('p',text));if(statFeats&&className==='rk-front-limitations'){const observations=renderObservations(statFeats);if(observations){section.children[0].textContent='Cautions & Low Metrics';section.children[1].textContent='Measured context · not ability penalties';section.append(observations);}section.append(node('small','Stat flags · contextual, not diagnosed weaknesses.'),renderStatFeats(statFeats,'risk_flags',2));}right.append(section);}
  const distinctive=box('Distinctive Features','rk-front-distinctive');
  distinctive.append(node('strong',sheet?.specialty??'Undocumented'));
  if(sheet)distinctive.append(node('p',sheet.summary));
  if(statFeats)distinctive.append(renderStatFeats(statFeats,'distinctive_features',2));
  if(goalie&&statScores?.shot_context)distinctive.append(renderShotContext(statScores.shot_context));
  if(!goalie&&statScores?.skater_context)distinctive.append(renderSkaterContext(statScores.skater_context));
  if(goalie&&goaliePuck&&goaliePuck.player_id===statActor&&goaliePuck.season===targetSeason&&goaliePuck.season_type===usageRequest?.season_type)distinctive.append(renderGoaliePuck(goaliePuck));
  const seasonal=node('div');seasonal.className='rk-seasonal-records';
  if(skating){const candidate=renderSkatingCandidate(skating);if(candidate)seasonal.append(candidate);}
  if(shooting){const candidate=renderShootingCandidate(shooting);if(candidate)seasonal.append(candidate);}
  if(shooting||seasonal.children.length){const heading=node('div');heading.className='rk-seasonal-records-heading';heading.append(node('small','Seasonal records'));if(shooting)heading.append(shootingRecordButton(shooting));distinctive.append(heading,seasonal);}
  const questions=namesBox('Open Questions','rk-front-questions',sheet?.unknowns??[],sheet?'None recorded':'Undocumented');
  right.append(distinctive,questions);columns.append(left,center,right);front.append(columns);

  const sources=node('dl');sources.className='rk-front-coverage';
  sources.append(field('Dossier Sources',sheet?String(sheet.sources.length):'Unknown'),
    field('Open Questions',sheet?String(sheet.unknowns.length):'Unknown'),
    field('Dossier Through',sheet?.evidence_through??'Unknown'),field('Assessed',sheet?.assessed??'Unknown'));
  front.append(sources);
  const footer=node('footer',statScores?'Stat-based attribute estimates · selected season · click an attribute for its formula':sheet?'Provisional documentary class · capability ratings unassessed · evidence below':catalogAvailable===false?'Class dossier unavailable · statistics do not assign a class':'No supported class dossier · statistics do not assign a class');
  if(statScores?.group==='Goalies')footer.append(node('span',' · Sources: NHL / MoneyPuck.com'));
  footer.className='rk-front-footer';front.append(footer);return front;
}
