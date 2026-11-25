// Rendering only. Rust counts and validates the captured game ledger.
export interface GoaliePuckEvidence {
 player_id:number;season:number;season_type:string;coverage:string;captured_through:string;
 metrics:{label:string;count:number}[];penalty_types:{label:string;count:number}[];
 plays:{label:string;game_id:number;event_id:number;period:number;clock:string;game_url:string;source_url:string;following_attempt:string|null}[];
 note:string;
}
const el=(tag:string,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function renderGoaliePuck(p:GoaliePuckEvidence):HTMLElement {
 const block=el('div');block.className='rk-puck-record';block.dataset.puckPlayer=String(p.player_id);block.dataset.puckSeason=String(p.season);
 block.append(el('strong','Puck Record'),el('small','Recorded game logs'));
 const values=el('dl');values.className='rk-puck-values';
 for(const m of p.metrics){const pair=el('div');pair.append(el('dt',m.label),el('dd',String(m.count)));values.append(pair);}
 block.append(values);const details=el('details');details.append(el('summary','Games & penalties'),el('p',p.coverage),el('small',`Captured through ${p.captured_through}`),el('p',p.note));
 for(const m of p.penalty_types)details.append(el('p',`${m.label}: ${m.count}`));
 if(!p.plays.length)details.append(el('p','No credited puck events in these captured games.'));
 for(const play of p.plays){const row=el('p'),link=el('a',`${play.label} · P${play.period} ${play.clock}`) as HTMLAnchorElement;link.href=play.game_url;
  const raw=el('a',`Event ${play.event_id} source`) as HTMLAnchorElement;raw.href=play.source_url;
  row.append(link,el('small',`Game ${play.game_id}`),raw);if(play.following_attempt)row.append(el('small',play.following_attempt));details.append(row);}
 block.append(details);return block;
}
let pending:Promise<unknown|undefined>|undefined;
export function readGoaliePuckLedger():Promise<unknown|undefined> {
 pending??=(async()=>{let response:Response;try{response=await fetch(new URL('../rinkcraft-goalie-puck-events.json',import.meta.url));}catch{return undefined;}
  if(response.status===404)return undefined;if(!response.ok)throw new Error('Goalie puck events could not be loaded');
  const text=await response.text();if(text.length>16*1024*1024)throw new Error('Goalie puck ledger exceeds 16 MiB');return JSON.parse(text) as unknown;})();return pending;
}
