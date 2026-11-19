// Rust supplies scopes, states and ranks. Presentation never pools seasonal stats.
export interface ProfileTimeWindow {player_id:number;kind:'season'|'current'|'career';first_season?:number;season:number;season_type:string;as_of:string|null;label:string;coverage:string;choices:{kind:string;label:string;available:boolean}[];note:string;}
export interface ExperimentalPattern {id:string;name:string;placement:string;state:string;reference_mode:string;reference_label:string;eligible_peers:number;threshold:number;conditions:{key:string;value:number;percentile:number;direction:string}[];opportunities:Record<string,number|null>;minimums:Record<string,number>;history:{season:number;state:string;eligible_peers:number;conditions:{key:string;value:number;percentile:number}[]}[];note:string;}
export interface ExperimentalPatterns {schema:string;player_id:number;season:number;season_type:string;group:string;status:string;patterns:ExperimentalPattern[];scope_note:string;source_urls?:string[];source_sha256?:string;screen_sha256?:string;}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const season=(s:number)=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
const labels:Record<string,string>={conversion:'Goals / shots',shots_pg:'Shots / GP',pp_points_per60:'PP points / 60',pp_seconds:'PP minutes',net_penalties_pg:'Net penalties / GP',drawn_pg:'Drawn penalties / GP'};
const stateLabel=(state:string)=>({earned:'Qualifies',not_earned:'Below threshold',insufficient_exposure:'Needs opportunities',missing_inputs:'Needs data',insufficient_reference_population:'Needs reference peers'}[state]??'Unavailable');
export function matchesProfileWindow(window:ProfileTimeWindow|undefined,patterns:ExperimentalPatterns|undefined,actor:number|undefined,selectedSeason:number|undefined,type:string|undefined,group:string|undefined):boolean {
 const validScope=!!window&&((window.kind==='season'&&window.coverage==='selected_season')||((window.kind==='current'&&window.coverage==='season_to_date'||window.kind==='career'&&window.coverage==='career_reconciled'&&!!window.first_season&&window.first_season<=window.season)&&!!window.as_of&&Number.isFinite(Date.parse(window.as_of))));
 return !!window&&!!patterns&&validScope&&window.player_id===actor&&patterns.player_id===actor&&window.season===selectedSeason&&patterns.season===selectedSeason&&window.season_type===type&&patterns.season_type===type&&patterns.group===group&&patterns.schema==='rinkcraft.scoped-patterns.v1';
}
export function renderProfileWindow(window:ProfileTimeWindow,onChange?:(kind:string)=>void):HTMLElement{
 const root=node('section');root.className='rk-profile-window';root.setAttribute('aria-label','Whole profile time scope');root.dataset.windowSeason=String(window.season);
 const label=node('label','Profile scope '),select=node('select');select.setAttribute('aria-label','Whole profile time scope');
 for(const choice of window.choices){const option=node('option',choice.label);option.value=choice.kind;option.disabled=!choice.available||(!onChange&&choice.kind!==window.kind);select.append(option);}select.value=window.kind;
 select.onchange=()=>{if(onChange&&window.choices.some(c=>c.kind===select.value&&c.available))onChange(select.value);else select.value=window.kind;};label.append(select);root.append(label);
 const why=node('details');why.append(node('summary','Scope & coverage'),node('p',window.note),node('p','A reference population is separate from the player’s time scope. Career comparisons require an appropriate peer cohort; reconciled totals alone do not establish one.'));if(window.as_of)why.append(node('p',`Captured through ${window.as_of}. Reports may lag games; this is not a live feed.`));root.append(why);return root;
}
function patternRow(pattern:ExperimentalPattern,scope:ProfileTimeWindow,evidence:ExperimentalPatterns):HTMLElement{
 const d=node('details');d.className='rk-stat-feat rk-experimental-pattern';d.dataset.candidateId=pattern.id;d.dataset.candidateState=pattern.state;
 const summary=node('summary');summary.append(node('strong',pattern.name),node('small',`Experimental · ${stateLabel(pattern.state)}`));
 const trigger=node('span',pattern.conditions.length?pattern.conditions.map(c=>`${labels[c.key]??c.key} P${c.percentile.toFixed(0)}`).join(' · '):'Opportunity gate not met');trigger.className='rk-feat-trigger';summary.append(trigger);d.append(summary);
 d.append(node('p',`${scope.label} · ${scope.season_type} · ${pattern.id==='pp-efficiency'?'power play':'all strengths'} · Reference: ${pattern.reference_label}, n=${pattern.eligible_peers}. Every condition must reach P${pattern.threshold}.`));
 for(const c of pattern.conditions){const value=c.key==='conversion'?`${(c.value*100).toFixed(2)}%`:c.key==='pp_seconds'?`${(c.value/60).toFixed(1)} min`:c.value.toFixed(3);d.append(node('p',`${labels[c.key]??c.key}: ${value} · P${c.percentile.toFixed(1)}`));}
 d.append(node('p',`Opportunities: ${Object.entries(pattern.opportunities).map(([key,value])=>`${key.replaceAll('_',' ')} ${value??'unavailable'}`).join(' · ')}`),node('p',pattern.note),node('p','Seasonal descriptive pattern; not an ability grade or simulation bonus. Thresholds and exposure gates are experimental.'));
 const history=node('details');history.className='rk-pattern-history';history.append(node('summary','Earlier seasons'));
 const earlier=pattern.history.filter(h=>h.season<scope.season);
 for(const h of earlier)history.append(node('p',`${season(h.season)} · ${stateLabel(h.state)} · n=${h.eligible_peers} · ${h.conditions.map(c=>`${labels[c.key]??c.key} P${c.percentile.toFixed(0)}`).join(' · ')}`));
 if(!earlier.length)history.append(node('p','No earlier comparable season loaded.'));d.append(history);
 const sources=node('details');sources.append(node('summary','Sources & calculation inputs'),node('p',`Report input ${evidence.source_sha256??'unavailable'} · Screen input ${evidence.screen_sha256??'unavailable'}`));
 const paths=new Set<string>();for(const source of evidence.source_urls??[]){const url=new URL(source);if(url.protocol!=='https:'||url.hostname!=='api.nhle.com'||!url.pathname.includes('/skater/')||paths.has(url.pathname))continue;paths.add(url.pathname);url.searchParams.set('start','0');const filter=url.searchParams.get('cayenneExp')??'';if(!filter.includes('playerId='))url.searchParams.set('cayenneExp',`${filter} and playerId=${scope.player_id}`);const a=node('a',url.pathname.split('/').slice(-1)[0]);a.href=url.href;sources.append(a,node('span',' · '));}d.append(sources);return d;
}
export function applyScopedPatterns(front:HTMLElement,scope:ProfileTimeWindow,patterns:ExperimentalPatterns):void{
 front.dataset.windowKind=scope.kind;front.dataset.windowSeason=String(scope.season);
 const scopeLine=front.querySelector('.rk-front-scope');if(scopeLine)scopeLine.textContent=`${scope.kind==='current'?'Current':scope.kind==='career'?'Career':'Season'} profile · ${scopeLine.textContent}${scope.as_of?` · Captured ${new Date(scope.as_of).toISOString().slice(0,16).replace('T',' ')} UTC`:''}`;
 if(scope.kind==='current'||scope.kind==='career'){const classNote=front.querySelector('.rk-class-note');if(classNote)classNote.textContent=`Historical dossier · ${classNote.textContent}`;const snapshot=front.querySelector('.rk-front-snapshot');if(snapshot)snapshot.textContent=scope.kind==='career'?'Pooled NHL regular-season career records · no career peer ranks or seasonal feat conversion.':'Season-to-date records · historical dossier retained; current task continuity unverified.';}
 if(scope.kind==='career'){const recordNote=front.querySelector('.rk-front-record>p');if(recordNote)recordNote.textContent='Pooled NHL regular-season counts and rates. Comparative career ranks and feats await a suitable reference. Record milestones describe accumulated counts.';}
 for(const [placement,selector,limit]of [['task_skills','.rk-front-skills',3],['playing_tendencies','.rk-front-traits',2]] as const){
  const box=front.querySelector(selector);if(!box)continue;
  const candidates=patterns.patterns.filter(p=>p.placement===placement),earned=candidates.filter(p=>p.state==='earned').slice(0,limit);
  if(!candidates.length)continue;
  const block=node('div');block.className='rk-candidate-patterns';block.append(node('small','Experimental · selected-season patterns'));
  for(const candidate of earned)block.append(patternRow(candidate,scope,patterns));
  const other=candidates.filter(p=>!earned.includes(p));if(other.length){const more=node('details');more.className='rk-candidate-library';more.append(node('summary',`Review ${other.length} candidate${other.length===1?'':'s'}`));for(const candidate of other)more.append(patternRow(candidate,scope,patterns));block.append(more);}
  const active=box.querySelector('.rk-stat-feats');
  if(active&&earned.length){const primary=Array.from(active.children).filter(e=>e.matches('details.rk-stat-feat'));let more=active.querySelector<HTMLDetailsElement>(':scope > .rk-feat-library');if(!more){more=node('details');more.className='rk-feat-library';more.append(node('summary'));active.append(more);}for(const e of primary.slice(limit-earned.length))more.append(e);const count=Array.from(more.children).filter(e=>e.matches('details.rk-stat-feat')).length;more.querySelector('summary')!.textContent=`Browse ${count} more feats`;}
  box.insertBefore(block,active??box.children[1]??null);
 }
}
