import {renderFeatRuleTrigger,featConditionText,featValue} from './rinkcraft-feat-rule.js';
export interface StatFeat {id:string;name:string;state:string;tier:string|null;placements:string[];note?:string|null;matched:number;required:number;conditions:{raw_thresholds?:Record<string,{value:number;inclusive:boolean}>;key:string;label:string;value:number|null;unit:string;numerator:number|null;denominator:number|null;percentile:number|null;peers:number;reference_sha256:string;qualifies:boolean|null}[];}
export interface TaskRepertoire {highlights?:{feat_id:string;name:string;tier:string}[];schema:string;player_id:number;season:number;season_type:string;group:string;games:number;skills:{feat_id:string;name:string;family:string}[];note:string;}
export interface AchievementProfile {task_repertoire?:TaskRepertoire;profile_observations?:import('./rinkcraft-observations.js').ProfileObservations;player_id:number;season:number;season_type:string;group:string;games:number;minimum_games:number;rules_sha256:string;source_sha256:string;source_urls:string[];earned:number;feats:StatFeat[];notes:string[];}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderStatSpecialties(profile:AchievementProfile):HTMLElement|undefined {
 const r=profile.task_repertoire;
 if(!r||r.schema!=='rinkcraft.task-repertoire.v2'||r.player_id!==profile.player_id||r.season!==profile.season||r.season_type!==profile.season_type||r.group!==profile.group||r.games!==profile.games||!r.highlights?.length||r.highlights.length>2||new Set(r.highlights.map(h=>h.feat_id)).size!==r.highlights.length||!r.highlights.every(h=>profile.feats.some(f=>f.id===h.feat_id&&f.state==='earned'&&f.tier===h.tier&&!f.placements.includes('risk_flags'))))return undefined;
 const row=node('div');row.className='rk-stat-specialties';row.append(node('small',`Stat specialties · ${Math.floor(profile.season/10000)}–${String(profile.season%100).padStart(2,'0')} · ${profile.games} GP`));
 for(const h of r.highlights){const chip=node('span',`${h.name} · ${h.tier}`);chip.dataset.specialtyFeat=h.feat_id;row.append(chip);}return row;
}
export function renderStatFeats(profile:AchievementProfile,placement:string,limit=3):HTMLElement {
 const root=node('div');root.className='rk-stat-feats';root.dataset.featPlacement=placement;
 const feats=profile.feats.filter(f=>f.placements.includes(placement));
 const earned=feats.filter(f=>f.state==='earned').sort((a,b)=>b.required-a.required||Math.min(...b.conditions.map(c=>c.percentile??-1))-Math.min(...a.conditions.map(c=>c.percentile??-1))||a.id.localeCompare(b.id));
 const r=profile.task_repertoire;
 const repertoire=placement==='task_skills'&&r&&['rinkcraft.task-repertoire.v1','rinkcraft.task-repertoire.v2'].includes(r.schema)&&r.player_id===profile.player_id&&r.season===profile.season&&r.season_type===profile.season_type&&r.group===profile.group&&r.games===profile.games&&r.skills.length<=5&&new Set(r.skills.map(s=>s.feat_id)).size===r.skills.length&&r.skills.every(s=>earned.some(f=>f.id===s.feat_id))?r:undefined;
 const visible=repertoire?repertoire.skills.flatMap(s=>earned.filter(f=>f.id===s.feat_id)):earned.slice(0,limit);
 if(repertoire){root.className+=' rk-task-repertoire';root.title=repertoire.note;root.append(node('small',`${visible.length} selected / ${earned.length} earned / ${Math.floor(profile.season/10000)}-${String(profile.season%100).padStart(2,'0')} / ${profile.games} GP`));}
 else root.append(node('small',`Stat feats · all strengths · ${earned.length}/${feats.length}`));
 const item=(feat:StatFeat,selected=false)=>{const d=node('details');d.className='rk-stat-feat';d.dataset.featId=feat.id;d.dataset.featState=feat.state;const summary=node('summary');const skill=selected?repertoire?.skills.find(s=>s.feat_id===feat.id):undefined;if(skill)d.className+=' rk-task-feat';summary.append(node('strong',skill?.name??feat.name),node('small',feat.tier??(feat.state==='unavailable'?'Needs data':feat.state==='ineligible'?`Needs ${profile.minimum_games} GP (${profile.games}/${profile.minimum_games})`:`${feat.matched}/${feat.required} conditions`)));const trigger=node('span',feat.conditions.map(c=>featConditionText(c,feat.tier)).join(' · '));trigger.className='rk-feat-trigger';summary.append(trigger);if(skill){const actual=node('span',feat.conditions.map(c=>`${c.label.replace(' / GP','')} ${featValue(c,feat.tier)}`).join(' / '));actual.className='rk-task-measured';summary.append(actual);}d.append(summary);
  d.append(renderFeatRuleTrigger(feat,profile));
  const audit=node('details');audit.className='rk-feat-audit';audit.append(node('summary','Sources & limits'));
  if(feat.note)audit.append(node('p',feat.note));audit.append(node('p',`Rules ${profile.rules_sha256} · Input ${profile.source_sha256}`));for(const note of profile.notes)audit.append(node('p',note));
  if(profile.source_urls.length){const links=node('details');links.append(node('summary','NHL player report links'));const paths=new Set<string>();for(const source of profile.source_urls){const url=new URL(source);if(paths.has(url.pathname)||(profile.group==='Goalies')!==url.pathname.includes('/goalie/'))continue;paths.add(url.pathname);url.searchParams.set('start','0');url.searchParams.set('cayenneExp',`${url.searchParams.get('cayenneExp')} and playerId=${profile.player_id}`);const a=node('a',url.pathname.split('/').slice(-2).join('/'));a.href=url.href;links.append(a);}audit.append(links);}else audit.append(node('p','Loaded canonical cohort; exact raw snapshot not audited.'));d.append(audit);return d;};
 for(const f of visible)root.append(item(f,true));
 if(!earned.length)root.append(node('p','No unlocked stat feat in this category.'));
 else if(!visible.length)root.append(node('p','Earned feats are available in the library; task mapping is pending.'));
 const others=feats.filter(f=>!visible.includes(f));
 if(others.length){const more=node('details');more.className='rk-feat-library';more.append(node('summary',`Browse ${others.length} more feats`));for(const f of others)more.append(item(f));root.append(more);}
 return root;
}
