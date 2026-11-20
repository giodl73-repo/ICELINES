// Display the simple attribute estimates computed by shared Rust.
export interface StatPart {key:string;label:string;unit:string;weight:number;lower_is_better:boolean;value:number|null;percentile:number|null;favorable_percentile:number|null;peers:number;}
export interface StatScore {label:string;score:number|null;stat_points:number|null;parts:StatPart[];note:string;}
export interface StatProfile {schema:string;player_id:number;season:number;group:string;scope:string;small_sample:boolean;scores:StatScore[];}
export interface EstimatedRecord {season:number;target_games:number;basis:string;metrics:{key:string;label:string;value:number|null;unit:string;per_game:number|null;projected_total:boolean}[];}
const labels=['Skating','Handling','Passing','Shooting','Reading','Defending'];
export function matchesStatScores(p:StatProfile|undefined,id:number|undefined,season:number|undefined,group:string):p is StatProfile {
 return !!p&&p.schema==='rinkcraft.simple-stat-attributes.v1'&&p.player_id===id&&p.season===season&&p.group===group&&p.scores.length===6&&p.scores.every((s,i)=>s.label===labels[i]&&(s.score===null||(Number.isInteger(s.score)&&s.score>=1&&s.score<=20)));
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderStatScore(profile:StatProfile,score:StatScore):HTMLElement {
 const body=node('section');body.className='rk-stat-score-detail';
 body.append(node('small','RINKCRAFT · STAT ESTIMATE'),node('h2',`${score.label} ${score.score??'—'}/20`),node('p',score.note));
 body.append(node('p',`${profile.scope} · same-season ${profile.group.toLowerCase()} comparison.`));
 if(profile.small_sample)body.append(node('p','Small sample: fewer than 10 games or 100 minutes. This estimate can move quickly.'));
 const used=score.parts.filter(p=>p.favorable_percentile!==null).reduce((sum,p)=>sum+p.weight,0);
 const list=node('dl');list.className='rk-task-list';
 for(const part of score.parts){
  const row=node('div');row.className='rk-field';
  row.append(node('dt',part.label));
  if(part.value===null||part.favorable_percentile===null)row.append(node('dd','Unavailable'));
  else {
   const value=part.unit==='%'?`${(part.value*100).toFixed(1)}%`:`${part.value.toFixed(2)} ${part.unit}`;
   row.append(node('dd',`${value} · ${Math.round(part.weight/used*100)}% weight`),node('small',`Favorable P${part.favorable_percentile.toFixed(1)} · ${part.peers} peers${part.lower_is_better?' · lower is better':''}`));
  }
  list.append(row);
 }
 body.append(list,node('p',score.stat_points===null?'No ranked stats available.':`${score.stat_points.toFixed(1)} stat points ÷ 5 → ${score.score}/20.`),node('p','Average the favorable stat percentiles using these weights, divide by 5, round and cap at 1–20. Missing stats are omitted; remaining weights scale to 100%. Reference players have at least 10 games and 100 minutes.'));
 return body;
}
export function openStatScore(profile:StatProfile,score:StatScore,player:string):void {
 const dialog=node('dialog');dialog.className='rk-capability-dialog';dialog.setAttribute('aria-label',`${score.label} stat estimate for ${player}`);
 const close=node('button','Close');close.type='button';close.onclick=()=>dialog.close();
 dialog.append(close,renderStatScore(profile,score));document.body.append(dialog);dialog.onclose=()=>dialog.remove();dialog.showModal();
}
