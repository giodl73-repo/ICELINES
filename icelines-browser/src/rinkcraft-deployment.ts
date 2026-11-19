// Rust supplies workload values, denominators and display strings.
export interface DeploymentEvidence {
 scope_label?:string;scope_kind?:string;player_id:number;season:number;season_type:string;games:number;goalie:boolean;starts:number|null;
 rows:{key:string;label:string;seconds:number;seconds_per_game:number;share:number|null;per_game_display:string;minutes_display:string;share_display:string}[];
 source_label:string;source_url:string|null;source_sha256:string|null;notes:string[];
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export function renderDeploymentUsage(card:DeploymentEvidence):HTMLElement {
 const section=node('div');section.className='rk-usage';section.dataset.usageSeason=String(card.season);
 section.append(node('small',`${card.scope_label??`${Math.floor(card.season/10000)}–${String(card.season%100).padStart(2,'0')}`} · ${card.season_type} · TOI / appearance`));
 const grid=node('dl');grid.className='rk-usage-grid';
 for(const row of card.rows){const field=node('div');field.dataset.usageKey=row.key;field.append(node('dt',row.label),node('dd',row.per_game_display));grid.append(field);}
 if(card.goalie){const f=node('div');f.append(node('dt','Starts / GP'),node('dd',`${card.starts??'—'} / ${card.games}`));grid.append(f);}
 section.append(grid,node('small','Unit / station undocumented'));
 const details=node('details');details.className='rk-usage-evidence';details.append(node('summary','Usage record & source'));
 for(const row of card.rows)details.append(node('p',`${row.label}: ${row.minutes_display} MIN · ${row.share_display} of own TOI · ${row.seconds} seconds / ${card.games} appearances`));
 for(const note of card.notes)details.append(node('p',note));
 if(card.source_url){const a=node('a',card.source_label);a.href=card.source_url;details.append(a);}else details.append(node('p',`${card.source_label} · catalog route; exact snapshot not audited.`));
 if(card.source_sha256)details.append(node('p',`Capture SHA256 ${card.source_sha256}`));
 section.append(details);return section;
}
