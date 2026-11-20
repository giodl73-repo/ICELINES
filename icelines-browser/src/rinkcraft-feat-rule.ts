// Readable rule text for Rust-projected feats. No qualification or tier calculation.
import type {StatFeat,AchievementProfile} from './rinkcraft-achievements.js';
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export const featCategories:Record<string,string>={task_skills:'Task Skills',playing_tendencies:'Playing Tendencies',distinctive_features:'Distinctive Features',role_coverage:'Role Coverage',risk_flags:'Caution Flags'};
export function featThreshold(tier:string|null):number|null{
 if(tier===null)return 60;
 return ({Bronze:60,Silver:75,Gold:90,Notable:60,Elevated:75,Marked:90} as Record<string,number>)[tier]??null;
}
function displayDigits(c:StatFeat['conditions'][number],cut:{value:number;inclusive:boolean}|undefined):number {
 let digits=c.unit==='Count'?0:c.unit==='Pct'?1:2;
 if(!cut||c.value===null)return digits;
 const scale=c.unit==='Pct'?100:1,actual=c.value*scale,limit=cut.value*scale;
 while(digits<5&&actual!==limit&&actual.toFixed(digits)===limit.toFixed(digits))digits++;
 return digits;
}
export function featValue(c:StatFeat['conditions'][number],tier:string|null=null):string {
 if(c.value===null)return 'Unavailable';
 const digits=displayDigits(c,c.raw_thresholds?.[String(featThreshold(tier))]);
 return c.unit==='Count'?c.value.toLocaleString('en-US'):c.unit==='Pct'?`${(c.value*100).toFixed(digits)}%`:`${c.value.toFixed(digits)}${c.unit==='PerGame'?'/game':''}`;
}
export function featConditionText(c:StatFeat['conditions'][number],tier:string|null):string {
 const name=c.label.replace(' / GP',''),cut=c.raw_thresholds?.[String(featThreshold(tier))];
 if(!cut)return `${name}: ${featValue(c)}`;
 const scale=c.unit==='Pct'?100:1,limit=cut.value*scale,digits=displayDigits(c,cut);
 const amount=c.unit==='Count'?limit.toLocaleString('en-US'):`${Number(limit.toFixed(digits))}${c.unit==='Pct'?'%':c.unit==='PerGame'?'/game':''}`;
 return `${name} ${cut.inclusive?'≥':'>'} ${amount}`;
}
export function featArithmetic(c:StatFeat['conditions'][number]):string{
 if(c.value===null||!Number.isFinite(c.value))return 'Measurement unavailable';
 if(c.unit==='Count')return `Season total = ${c.value.toLocaleString('en-US')}`;
 if(c.numerator===null||c.denominator===null||c.denominator<=0)return 'Calculation inputs unavailable';
 const num=c.numerator.toLocaleString('en-US'),den=c.denominator.toLocaleString('en-US');
 if(c.unit==='Pct')return `${num} ÷ ${den} × 100 = ${(c.value*100).toFixed(2)}%`;
 return `${num} ÷ ${den} = ${c.value.toFixed(3)}${c.unit==='PerGame'?' / GP':''}`;
}
export function renderFeatRule(feat:StatFeat,profile:AchievementProfile):HTMLElement{
 const box=node('div');box.className='rk-feat-rulebook';
 const kind=feat.placements.includes('risk_flags')?'Caution':'Feat',threshold=featThreshold(feat.tier);
 box.append(node('small',`${kind} · ${feat.placements.map(p=>featCategories[p]??p).join(' / ')}`));
 const rule=threshold===null?'Threshold unavailable':feat.conditions.map(c=>featConditionText(c,feat.tier)).join(' AND ');
 box.append(node('h5','Prerequisites'),node('p',rule));
 const results=node('dl');results.className='rk-feat-roll';
 for(const c of feat.conditions){
  const row=node('div');row.append(node('dt',c.label));
  const value=c.value===null?'Unavailable':c.unit==='Pct'?`${(c.value*100).toFixed(2)}%`:c.unit==='Count'?c.value.toLocaleString('en-US'):c.value.toFixed(3);
  row.append(node('dd',`${value}${c.unit==='PerGame'?' / game':''}`),node('small',featArithmetic(c)));results.append(row);
 }
 box.append(node('h5','Your record'),results);
 const verdict=feat.state==='earned'?`Earned ${feat.tier??''}`:feat.state==='ineligible'?`Needs exposure: ${profile.minimum_games} GP minimum`:feat.state==='unavailable'?'Needs measurements or reference peers':`${feat.matched} of ${feat.required} base conditions met`;
 box.append(node('p',`Result: ${verdict}.`));
 box.append(node('h5',kind==='Caution'?'Flag bands':'Tier ladder'),node('p',kind==='Caution'?'Notable → Elevated → Marked':'Bronze → Silver → Gold'),node('p','Higher tiers require higher cutoffs for every stat. Rounded display values do not decide qualification.'));
 box.append(node('small',`${Math.floor(profile.season/10000)}–${String(profile.season%100).padStart(2,'0')} · ${profile.season_type} · all strengths · ${profile.group} · ${profile.games} GP`),node('small',`Eligibility: ${profile.minimum_games} GP and at least 20 measured peers for each condition.`));
 const math=node('details');math.append(node('summary','How cutoffs are set'),node('p','Tier cutoffs correspond to position-group percentiles 60, 75 and 90. P = 100 × (players below + ½ × players tied) ÷ measured eligible peers. Each player is counted once in the selected season and position group.'));
 for(const c of feat.conditions)math.append(node('p',`${c.label}: ${c.percentile===null?'Unranked':`P${c.percentile.toFixed(1)}`} · ${c.peers} peers`));
 for(const c of feat.conditions)math.append(node('small',`${c.label}: ${c.numerator??'unknown'} / ${c.denominator??'unknown'} · Reference ${c.reference_sha256}`));
 box.append(math);return box;
}
export function renderFeatRuleTrigger(feat:StatFeat,profile:AchievementProfile):HTMLElement{
 const root=node('div');root.className='rk-feat-rule-trigger';
 const threshold=featThreshold(feat.tier);root.append(node('p',`Prerequisites: ${threshold===null?'unavailable':feat.conditions.map(c=>featConditionText(c,feat.tier)).join(' + ')}`));
 for(const c of feat.conditions)root.append(node('small',`Your ${c.label.replace(' / GP','')}: ${featValue(c,feat.tier)}`));
 const button=node('button','Open feat rulebook');button.type='button';button.onclick=()=>{
  const dialog=node('dialog');dialog.className='rk-capability-dialog rk-feat-dialog';dialog.setAttribute('aria-label',`${feat.name} feat rulebook`);
  const close=node('button','Close rulebook');close.type='button';close.onclick=()=>dialog.close();
  const owner=root.closest('.rk-front')?.querySelector('h3')?.textContent??'Player record';const header=node('header');header.append(node('small',`${owner} · ${Math.floor(profile.season/10000)}–${String(profile.season%100).padStart(2,'0')}`),close);
  dialog.append(header,node('h2',feat.name),renderFeatRule(feat,profile));document.body.append(dialog);dialog.onclose=()=>dialog.remove();dialog.showModal();
 };root.append(button);return root;
}
