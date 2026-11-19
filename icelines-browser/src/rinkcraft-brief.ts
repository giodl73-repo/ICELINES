// Presentation only. Signal selection, repeated evidence and intervals are supplied by Rust.
import type {NumericalEvidence} from './rinkcraft-numbers.js';
import {numericalValue} from './rinkcraft-numbers.js';
import type {HistoryCard} from './rinkcraft-history.js';
import type {CharacterSheet} from './rinkcraft-character.js';
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
const period=(s:number)=>`${Math.floor(s/10000)}–${String(s%100).padStart(2,'0')}`;
export function renderEvidenceBrief(n:NumericalEvidence,h?:HistoryCard,sheet?:CharacterSheet):HTMLElement {
 const brief=node('section');brief.className='rk-brief';brief.setAttribute('aria-label','Evidence at a glance');
 brief.append(node('h4','Evidence at a glance'),node('p',`${period(n.season)} · ${n.group} · ${n.situation==='all'?'all strengths':n.situation} · ${n.games} GP · ${n.minutes.toFixed(1)} minutes. Documentary identity has its own evidence date${sheet?`: ${sheet.evidence_through}`:': unavailable'}.`));
 const grid=node('div');grid.className='rk-brief-grid';
 const signature=node('section');signature.append(node('h5','Measured signature'));const list=node('ul');
 for(const key of n.leading){const m=n.metrics.find(m=>m.key===key);if(!m)continue;const item=node('li');item.dataset.briefKey=key;item.append(node('b',m.label),node('span',`${numericalValue(m)} · ${m.percentile===null?'Unranked':`P${m.percentile.toFixed(1)}`} · n=${m.measured_peers}`));list.append(item);}
 if(!list.children.length)list.append(node('li','No selected measured signals.'));
 signature.append(list,node('small','Selected from a fixed involvement/output shortlist. High raw values are not always better; these are not class or ability scores.'));grid.append(signature);
 const history=node('section');history.append(node('h5','History support'));
 const a=h?.assessment,matched=h!=null&&h.player_id===n.player_id&&h.seasons.at(-1)?.season===n.season&&a?.last_season===n.season;
 history.dataset.briefHistory=matched?'matched':'unavailable';
 if(matched&&a){history.append(node('p',`${period(a.first_season)} → ${period(a.last_season)} · ${a.reported_seasons}/${a.supplied_seasons} supplied seasons reported.`));for(const r of a.repeated){history.append(node('p',`${r.label}: ${r.seasons} seasons at or above P80; annual range P${r.minimum_percentile.toFixed(1)}–P${r.maximum_percentile.toFixed(1)}.`));}if(!a.repeated.length)history.append(node('p',a.repeated_reason));}
 else history.append(node('p','No history summary matched to this player and selected season. Other supplied history retains its own dates.'));
 history.append(node('small','Repeated raw ranks use annual peers. Missing qualifying evidence does not mean missing skill.'));grid.append(history);
 const gaps=node('section');gaps.append(node('h5','Catalogued measurement gaps'));const needs=node('ul');
 const seen=new Set<string>();for(const f of sheet?.features??[])for(const gap of f.unmeasured_stats??[]){if(seen.has(gap))continue;seen.add(gap);needs.append(node('li',gap));}
 if(!needs.children.length)needs.append(node('li','No task-specific gap list supplied; capability coverage remains unestablished.'));
 gaps.append(needs,node('small','From the documentary methods catalogue; not a numerical ranking of research priorities. All six capability attributes remain unrated.'));grid.append(gaps);brief.append(grid);
 const f=n.finishing_research;if(f){const estimate=node('p');estimate.className='rk-brief-component';estimate.dataset.briefFinishing=f.status;
  estimate.textContent=f.status==='experimental'&&f.mean!==null&&f.lower!==null&&f.upper!==null&&f.probability!==null?`Experimental finishing: ${f.mean.toFixed(3)} × position baseline; ${(f.probability*100).toFixed(0)}% conditional interval ${f.lower.toFixed(3)}–${f.upper.toFixed(3)}. Earlier history only; not a complete Shooting grade.`:`Finishing component: ${f.status.replaceAll('_',' ')}. ${f.note}`;
  brief.append(estimate);
 }
 brief.append(node('p','Inspect the methods, formulas, sources, history and model details below before interpreting these observations. No causal, capability-change or simulation claim follows.'));return brief;
}
