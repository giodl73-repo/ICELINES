import {renderFeatRuleTrigger} from './rinkcraft-feat-rule.js';
export interface StatFeat {id:string;name:string;state:string;tier:string|null;placements:string[];note?:string|null;matched:number;required:number;conditions:{key:string;label:string;value:number|null;unit:string;numerator:number|null;denominator:number|null;percentile:number|null;peers:number;reference_sha256:string;qualifies:boolean|null}[];}
export interface AchievementProfile {profile_observations?:import('./rinkcraft-observations.js').ProfileObservations;player_id:number;season:number;season_type:string;group:string;games:number;minimum_games:number;rules_sha256:string;source_sha256:string;source_urls:string[];earned:number;feats:StatFeat[];notes:string[];}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderStatFeats(profile:AchievementProfile,placement:string,limit=3):HTMLElement {
 const root=node('div');root.className='rk-stat-feats';root.dataset.featPlacement=placement;
 const feats=profile.feats.filter(f=>f.placements.includes(placement));
 const earned=feats.filter(f=>f.state==='earned').sort((a,b)=>b.required-a.required||Math.min(...b.conditions.map(c=>c.percentile??-1))-Math.min(...a.conditions.map(c=>c.percentile??-1))||a.id.localeCompare(b.id));
 root.append(node('small',`Stat feats · all strengths · ${earned.length}/${feats.length}`));
 const item=(feat:StatFeat)=>{const d=node('details');d.className='rk-stat-feat';d.dataset.featId=feat.id;d.dataset.featState=feat.state;const summary=node('summary');summary.append(node('strong',feat.name),node('small',feat.tier??(feat.state==='unavailable'?'Needs data':feat.state==='ineligible'?'Needs GP':`${feat.matched}/${feat.required} conditions`)));const trigger=node('span',feat.conditions.map(c=>`${c.label.replace(' / GP','')}: ${c.percentile===null?'—':`P${c.percentile.toFixed(0)}`}`).join(' · '));trigger.className='rk-feat-trigger';summary.append(trigger);d.append(summary);
  d.append(renderFeatRuleTrigger(feat,profile));
  const audit=node('details');audit.className='rk-feat-audit';audit.append(node('summary','Sources & limits'));
  if(feat.note)audit.append(node('p',feat.note));audit.append(node('p',`Rules ${profile.rules_sha256} · Input ${profile.source_sha256}`));for(const note of profile.notes)audit.append(node('p',note));
  if(profile.source_urls.length){const links=node('details');links.append(node('summary','NHL player report links'));const paths=new Set<string>();for(const source of profile.source_urls){const url=new URL(source);if(paths.has(url.pathname)||(profile.group==='Goalies')!==url.pathname.includes('/goalie/'))continue;paths.add(url.pathname);url.searchParams.set('start','0');url.searchParams.set('cayenneExp',`${url.searchParams.get('cayenneExp')} and playerId=${profile.player_id}`);const a=node('a',url.pathname.split('/').slice(-2).join('/'));a.href=url.href;links.append(a);}audit.append(links);}else audit.append(node('p','Loaded canonical cohort; exact raw snapshot not audited.'));d.append(audit);return d;};
 for(const f of earned.slice(0,limit))root.append(item(f));
 if(!earned.length)root.append(node('p','No unlocked stat feat in this category.'));
 const others=feats.filter(f=>!earned.slice(0,limit).includes(f));
 if(others.length){const more=node('details');more.className='rk-feat-library';more.append(node('summary',`Browse ${others.length} more feats`));for(const f of others)more.append(item(f));root.append(more);}
 return root;
}
