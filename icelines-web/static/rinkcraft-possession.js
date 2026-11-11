// Presentation/orchestration only. Rust owns legality, dice, arithmetic and state transitions.
const el=(tag,text)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=String(text);return e};
const words=s=>s.replaceAll('_',' ');
export function mountPossession(hero,characterRules,options){
 const root=el('details');root.className='rk-possession';root.append(el('summary','Play a possession with this character'));
 const body=el('section');body.append(el('h3','One possession. Twelve players. Real choices.'),el('p','Fictional playtest: goalie outlet → carry → pass. Failed actions create loose pucks and possible counters. No shots or goals. Opponent responses follow this scene’s fixed policy.'));
 const status=el('p','Open the playtest to load its rules.');status.setAttribute('role','status');body.append(status);
 const form=el('form');form.className='rk-possession-controls';
 const label=(name,input)=>{const l=el('label',name);input.setAttribute('aria-label',name);l.append(input);form.append(l);return input};
 const seed=label('Possession seed',el('input'));seed.type='number';seed.min='0';seed.max='4294967295';seed.step='1';seed.required=true;seed.value='42';
 const plan=label('Support plan',el('select'));for(const [value,text]of [['hold','Hold F3 high cover'],['commit','Commit F3 to attack support; vacate cover']]){const o=el('option',text);o.value=value;plan.append(o)}
 const start=el('button','Start possession');start.type='submit';form.append(start);const download=el('button','Download possession replay');download.type='button';download.disabled=true;form.append(download);body.append(form);
 const output=el('div');output.className='rk-possession-result';body.append(output);root.append(body);
 let rules,request,session,busy=false,loaded=false,sequence=0;
 function render(){
  output.replaceChildren();if(!session)return;
  const state=el('div');state.className='rk-possession-state';state.append(el('strong','Phase: '+words(session.state.phase)),el('span','Puck: '+(session.state.loose?'loose':session.state.control)),el('span','High cover: '+(session.state.cover_owner??'VACATED')),el('span','Seed '+session.seed+' · '+(session.commit_support?'F3 supporting attack':'F3 holding cover')));output.append(state);
  output.append(el('p','Every contest uses d20 + floor((game attribute − 10) / 2). Support adds '+rules.support_bonus+' on home outlet/carry/pass; holding cover adds '+rules.support_bonus+' to home counter defense. Ties end the opportunity. These are game odds, not NHL probabilities.'));
  const actions=el('div');actions.className='rk-possession-actions';
  for(const action of ['outlet','carry','pass','recover','counter']){const button=el('button','Roll '+action);button.type='button';button.disabled=busy||!session.legal_actions.includes(action);button.onclick=()=>void run({...request,actions:[...request.actions,action]});actions.append(button)}output.append(actions);
  if(!session.legal_actions.length)output.append(el('p','This opportunity window is finished. Start a new possession to compare a plan or replay this seed.'));
  const units=el('div');units.className='rk-possession-units';
  for(const side of ['home','away']){const box=el('section');box.append(el('h4',side==='home'?'Your unit':'Opposition unit'));const list=el('ul');for(const p of session.participants.filter(p=>p.slot.startsWith(side+':'))){const row=el('li');row.append(el('strong',p.slot+(p.slot===session.hero_slot?' · your character':'')),el('span',p.character.name+' · '+p.character.class_name),el('small',p.station));if(session.state.control===p.slot)row.dataset.puck='true';list.append(row)}box.append(list);units.append(box)}output.append(units);
  const log=el('ol');log.className='rk-possession-log';
  for(const e of session.events){const row=el('li');row.append(el('h4',words(e.action)+' · '+words(e.resolution.outcome)),el('p',e.actor+' '+e.actor_capability+' '+e.contest.actor_score+' vs '+e.opposition+' '+e.opposition_capability+' '+e.contest.opposition_score),el('code',e.resolution.actor_die+' + ('+e.terms[0]+') = '+e.resolution.actor_total+' vs '+e.resolution.opposition_die+' + ('+e.terms[1]+') = '+e.resolution.opposition_total+' · margin '+e.resolution.margin),el('p',e.explanation),el('small','Exact game odds: win '+(e.counts.actor_wins/4).toFixed(2)+'% · tie '+(e.counts.stalemates/4).toFixed(2)+'% · opposition '+(e.counts.opposition_wins/4).toFixed(2)+'%'));log.append(row)}output.append(log,el('p',session.note));
 }
 async function run(next){const token=++sequence;busy=true;seed.disabled=true;plan.disabled=true;start.disabled=true;download.disabled=true;output.setAttribute('aria-busy','true');render();status.textContent='Replaying possession…';try{const result=await options.possession(characterRules,rules,next);if(token!==sequence)return;request=next;session=result;status.textContent='Possession replayed. Only enabled actions have a legal opportunity.';}catch(e){if(token===sequence)status.textContent='Possession rejected. '+String(e)}finally{if(token===sequence){busy=false;seed.disabled=false;plan.disabled=false;start.disabled=false;download.disabled=!session;render();output.setAttribute('aria-busy','false')}}}
 async function initialize(){if(loaded)return;loaded=true;try{if(!options.loadPossessionRules||!options.possession)throw Error('This app has no possession engine.');rules=await options.loadPossessionRules();await run({hero,seed:Number(seed.value),commit_support:plan.value==='commit',actions:[]});}catch(e){loaded=false;status.textContent='Possession rules unavailable. Character creation remains available. '+String(e)}}
 root.addEventListener('toggle',()=>{if(root.open)void initialize()});
 form.onsubmit=e=>{e.preventDefault();if(!form.reportValidity())return;if(!rules){void initialize();return}void run({hero,seed:Number(seed.value),commit_support:plan.value==='commit',actions:[]})};
 for(const input of [seed,plan])input.addEventListener('change',()=>{for(const b of output.querySelectorAll('.rk-possession-actions button'))b.disabled=true;status.textContent='Setup changed. Start a new possession to apply it.'});
 download.onclick=()=>{if(!session)return;const replay={schema:'rinkcraft.possession-replay.v1',character_rules:characterRules,rules,request,session};const url=URL.createObjectURL(new Blob([JSON.stringify(replay,null,2)+'\n'],{type:'application/json'}));const a=el('a');a.href=url;a.download='rinkcraft-possession-'+request.seed+'.json';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)};
 return root;
}
