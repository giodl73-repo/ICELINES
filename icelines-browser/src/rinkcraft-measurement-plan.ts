// Read-only presentation of a validated Rust-exported authored plan.
export interface MeasurementPlans {
 schema:string;queue_authored:string;priorities_authored:string;status:string;purpose:string;scope:string;
 selection:string;counting:string;reference_policy:string;grade_gate:string;next_step:string;
 registry_sha256:string;queue_sha256:string;priorities_sha256:string;declared_registry:string;declared_queue:string;
 players:{player_id:number;name:string;tasks:{attribute_name:string;branch:string;definition:{task_id:string;attribute_id:string;component_id:string;opportunity:string;start:string;end:string;candidate_measure:string;unit:string;denominator_scope:string;primary_outcomes:string[];missingness_rule:string;current_gap:string;task_envelope_status:string;calibration_status:string;grade_release:boolean}}[]}[];
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderMeasurementPlan(c:MeasurementPlans,playerId:number):HTMLElement {
 const root=node('section');root.className='rk-measurement-plan';
 const p=c.players.find(p=>p.player_id===playerId);
 if(!p){root.append(node('h2','No authored measurement plan'),node('p','No task selection is supplied for this player. No fallback, rating or deficit is inferred.'));return root;}
 root.dataset.planPlayer=String(p.player_id);root.append(node('h2',p.name+' · next measurements'),node('p','Authored research priorities, not observed performance, assigned playing duties or player scores.'),node('p',`Queue: ${c.queue_authored} · player priorities: ${c.priorities_authored}. ${c.status}.`),node('p',`Scope: ${c.scope}`));
 for(const task of p.tasks){const t=task.definition,card=node('article');card.className='rk-plan-task';card.dataset.planTask=t.task_id;
  card.append(node('h3',t.task_id.replaceAll('-',' ')),node('p',`${task.attribute_name} · ${task.branch} · ${t.component_id}`));
  const fields=node('dl');for(const [label,value] of [['What counts as an opportunity',t.opportunity],['Denominator',t.denominator_scope],['Candidate measure',t.candidate_measure],['Why current evidence is insufficient',t.current_gap]]){const row=node('div');row.append(node('dt',label),node('dd',value));fields.append(row);}card.append(fields);
  const details=node('details');details.append(node('summary','Task boundaries, outcomes & unresolved rules'),node('p',`Start: ${t.start}`),node('p',`End: ${t.end}`),node('p',`Missingness: ${t.missingness_rule}`),node('p',`Outcome categories: ${t.primary_outcomes.join(' · ')}`),node('p',`Before collection: ${t.task_envelope_status}`),node('p',`Calibration: ${t.calibration_status}. Grade released: ${t.grade_release?'yes':'no'}. Unit: ${t.unit}.`));card.append(details);root.append(card);
 }
 const policy=node('details');policy.className='rk-plan-policy';policy.append(node('summary','Selection, counting, reference and release policy'));
 for(const text of [c.selection,c.counting,c.reference_policy,c.grade_gate,c.next_step])policy.append(node('p',text));
 policy.append(node('p','Fingerprints establish input consistency, not empirical validation. These selected tasks do not cover the full attribute battery. No task success rate, reliability estimate, complete capability score or simulation parameter is released.'),node('p',`Registry SHA256 ${c.registry_sha256}`),node('p',`Queue SHA256 ${c.queue_sha256}`),node('p',`Priorities SHA256 ${c.priorities_sha256}`));root.append(policy);return root;
}
