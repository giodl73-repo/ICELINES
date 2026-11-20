// Inspect the versioned Rust capability recipe; never turn feat ranks into abilities.
import {facetCopy} from './rinkcraft-capability-copy.js';
import {renderEdgeEvidence} from './rinkcraft-edge.js';
import type {EdgeEvidence} from './rinkcraft-edge.js';
interface Component{id:string;name:string;weight:number;measure:string;}
interface Definition{id:string;name:string;branch:string;construct:string;current_coverage:string;components:Component[];}
interface Rules{schema:string;status:string;source_registry_version:string;source_registry_sha256:string;reference_scope:string;quantile_knots:number[];score_knots:number[];release_gates:string[];attributes:Definition[];}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
let cached:Promise<Rules>|undefined;
async function loadRules():Promise<Rules>{
 cached??=(async()=>{const r=await fetch(new URL('../capability-rules.json',import.meta.url));if(!r.ok)throw Error('Attribute rulebook unavailable.');const v=await r.json() as Rules;if(v.schema!=='rinkcraft.capability-rules.v1'||v.status!=='unvalidated_candidate'||v.attributes.length!==12)throw Error('Unsupported attribute rulebook.');return v;})().catch(e=>{cached=undefined;throw e;});return cached;
}
export function renderCapabilityRecipe(d:Definition,r:Rules,edge?:EdgeEvidence):HTMLElement{
 const root=node('div');root.className='rk-capability-recipe';root.dataset.capabilityRule=d.id;
 root.append(node('small','RINKCRAFT ATTRIBUTE · DRAFT VALUATION'),node('h2',d.name),node('p',d.construct));
 const badge=node('p','— / 20 · Unrated');badge.className='rk-capability-grade';root.append(badge);
 if(edge)root.append(renderEdgeEvidence(edge,d.id));
 root.append(node('h3','Attribute recipe'));
 const facets=node('div');facets.className='rk-capability-facets';
 for(const c of d.components){const tile=node('section');tile.append(node('h4',c.name),node('strong',`${(c.weight*100).toFixed(c.weight===0.25?0:1)}%`),node('p',facetCopy[c.id]??c.measure));const detail=node('details');detail.append(node('summary','Measurement rule'),node('p',c.measure));tile.append(detail);facets.append(tile);}root.append(facets);
 root.append(node('h3','How the score is built'));
 const steps=node('ol');for(const text of ['Measure every facet in comparable tasks, with context and opportunity counts.','Normalize each facet against the frozen reference; combine them using the weights above.','Compare the combined result with the complete NHL reference, then use the 1–20 anchors below.'])steps.append(node('li',text));root.append(steps);
 const anchors=node('div');anchors.className='rk-capability-anchors';
 for(const [q,s,label]of [[0.25,8,'Below typical'],[0.5,10,'Typical'],[0.7,12,'Above typical'],[0.9,16,'Strong'],[0.97,18,'Upper tail'],[0.995,20,'Extreme upper tail']] as const){const x=node('div');x.append(node('strong',String(s)),node('span',label),node('small',`P${q*100} anchor`));anchors.append(x);}root.append(anchors,node('p','These are proposed NHL-reference anchors. A 16 is not twice the skill of an 8.'));
 root.append(node('h3','Why this player is still unrated'),node('p',edge?'NHL EDGE observations are available above. A full, calibrated facet battery and its uncertainty are not loaded. Maximums and outcome splits do not complete the required task estimates.':'A full, calibrated facet battery and its uncertainty are not loaded. Missing facets stay unknown; they are never filled with zero or an average.'));
 const exact=node('details');exact.append(node('summary','Exact formula & assessment requirements'),node('p','Facet quantile qⱼ = Fⱼ(θⱼ). Normalize zⱼ = Φ⁻¹(clip(qⱼ, 0.005, 0.995)). Balanced index H = Σ(wⱼ × zⱼ). Combined quantile Q = Fᴴ(H). Interpolate the frozen anchors, round half up, and clamp to 1–20.'),node('p','The candidate formula does not certify the measurements or release a player grade.'),node('p',`Capability task scope: ${r.reference_scope}`),node('p','Equal facet weights are an authored starting definition. They are not fitted estimates of what matters most to winning. Feat percentiles remain separate all-strength comparisons.'));
 const gates=node('ul');for(const gate of r.release_gates)gates.append(node('li',gate.replaceAll('_',' ')));exact.append(gates,node('small',`Rules ${r.source_registry_version} · Source registry ${r.source_registry_sha256}`));root.append(exact);return root;
}
export async function openCapability(name:string,goalie:boolean,player:string,edge?:EdgeEvidence):Promise<void>{
 const dialog=node('dialog');dialog.className='rk-capability-dialog';dialog.setAttribute('aria-label',`${name} attribute rule for ${player}`);
 const close=node('button','Close rulebook');close.type='button';close.onclick=()=>dialog.close();
 const heading=node('header');heading.append(node('small',player),close);const body=node('div','Loading attribute rulebook…');dialog.append(heading,body);document.body.append(dialog);dialog.onclose=()=>dialog.remove();dialog.showModal();
 try{const rules=await loadRules(),d=rules.attributes.find(d=>d.name===name&&d.branch===(goalie?'goalie':'skater'));if(!d)throw Error('Attribute definition unavailable.');body.replaceChildren(renderCapabilityRecipe(d,rules,edge));}catch(e){body.textContent=String(e);}
}
