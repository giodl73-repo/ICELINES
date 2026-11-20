import {renderCharacterCard} from './rinkcraft-character.js';
import type {RinkcraftProfile} from './rinkcraft-profile.js';
import {matchesEdge} from './rinkcraft-edge.js';
export interface PreviewPlayer{id:number;name:string;position:string;group:string;teams:string[];games:number;url:string;}
export interface PreviewWindow{season:number;label:string;captured_through:string;reports_sha256:string;players:PreviewPlayer[];}
export const captureLabel=(stamp:string):string=>/^\d{4}-\d{2}-\d{2}$/.test(stamp)?`${stamp} (capture date)`:`${new Date(stamp).toLocaleString()} (capture time)`;
interface Catalog{schema:string;current_season:number;windows:PreviewWindow[];coverage:string;}
const normalize=(s:string)=>s.normalize('NFKD').replace(/[\u0300-\u036f]/g,'').toLowerCase();
export function filterPreviewPlayers(players:PreviewPlayer[],query:string,team:string,group:string):PreviewPlayer[]{const words=normalize(query.trim()).split(/\s+/).filter(Boolean);return players.filter(p=>(!team||p.teams.includes(team))&&(!group||p.group===group)&&words.every(w=>normalize(`${p.name} ${p.teams.join(' ')}`).includes(w)));}
export function validatePreviewCatalog(raw:Catalog):Catalog{
 if(raw?.schema!=='rinkcraft.public-preview.v1'||!Array.isArray(raw.windows)||!raw.windows.length)throw Error('Unsupported player catalog.');
 const seasons=new Set<number>();for(const w of raw.windows){if(!Number.isInteger(w.season)||Math.floor(w.season/10000)+1!==w.season%10000||seasons.has(w.season)||!Number.isFinite(Date.parse(w.captured_through))||!Array.isArray(w.players))throw Error('Invalid season coverage.');seasons.add(w.season);const ids=new Set<number>();for(const p of w.players){if(!Number.isSafeInteger(p.id)||p.id<=0||ids.has(p.id)||typeof p.name!=='string'||!['Forwards','Defense','Goalies'].includes(p.group)||!Array.isArray(p.teams)||!p.teams.every(t=>typeof t==='string')||!Number.isSafeInteger(p.games)||p.games<0||p.url!==`data/${w.season}/${p.id}.json`)throw Error('Invalid player record.');ids.add(p.id);}}
 if(!seasons.has(raw.current_season))throw Error('Current season is not supplied.');return raw;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=text;return e;};
export async function mountPreview(root:HTMLElement):Promise<void>{
 const get=<T extends HTMLElement>(id:string)=>document.getElementById(id) as T;
 const season=get<HTMLSelectElement>('season'),search=get<HTMLInputElement>('search'),team=get<HTMLSelectElement>('team'),group=get<HTMLSelectElement>('position'),player=get<HTMLSelectElement>('player'),theme=get<HTMLSelectElement>('theme'),stage=get('profile-stage'),notice=get('data-notice'),status=get('selection-status');
 const response=await fetch('./catalog.json');if(!response.ok)throw Error('Player catalog could not be loaded.');const catalog=validatePreviewCatalog(await response.json() as Catalog);
 const query=new URLSearchParams(location.search),requested=query.get('season')??'current';let window=catalog.windows.find(w=>w.season===(requested==='current'?catalog.current_season:Number(requested)));
 if(!window)throw Error('That season is not available in this preview.');
 let selected=Number(query.get('player')??8478402),serial=0;
 for(const w of catalog.windows){const o=node('option',`${w.season===catalog.current_season?'Current · ':''}${w.label}`);o.value=String(w.season);season.append(o);}season.value=String(window.season);
 const requestedTheme=query.get('theme')??'ice';if(Array.from(theme.options).some(o=>o.value===requestedTheme))theme.value=requestedTheme;
 function remember(){const url=new URL(location.href);url.searchParams.set('season',window!.season===catalog.current_season?'current':String(window!.season));if(selected)url.searchParams.set('player',String(selected));url.searchParams.set('theme',theme.value);history.replaceState(null,'',url);}
 function options(){const previous=team.value;team.replaceChildren(node('option','All teams'));team.options[0].value='';for(const t of [...new Set(window!.players.flatMap(p=>p.teams))].sort()){const o=node('option',t);o.value=t;team.append(o);}team.value=Array.from(team.options).some(o=>o.value===previous)?previous:'';}
 function choices(){const rows=filterPreviewPlayers(window!.players,search.value,team.value,group.value);player.replaceChildren();for(const p of rows){const o=node('option',`${p.name} · ${p.teams.join('/')} · ${p.games} GP`);o.value=String(p.id);player.append(o);}player.value=String(selected);get('player-count').textContent=`(${rows.length} / ${window!.players.length})`;status.textContent=rows.length?'Choose a player to open their sheet.':'No players match these filters.';}
 async function show(){const token=++serial;const p=window!.players.find(p=>p.id===selected);remember();stage.replaceChildren();get('source-detail').replaceChildren();if(!p){notice.textContent=`No captured record for player ${selected} in ${window!.label}. Choose another player or season.`;return;}
  notice.textContent=`Loading ${p.name} · ${window!.label}…`;
  try{const r=await fetch('./'+p.url);if(!r.ok)throw Error('Player sheet could not be loaded.');const profile=await r.json() as RinkcraftProfile;if(token!==serial)return;
   if(profile.request?.player_id!==p.id||profile.latest_available?.season!==window!.season||profile.achievements?.season!==window!.season||profile.latest_available.games!==p.games)throw Error('Player/window evidence mismatch.');
   stage.replaceChildren(renderCharacterCard(profile.player_name,profile.character_sheet,profile.attribute_seasons?.at(-1),profile.latest_available,profile.numerical_evidence,profile.numerical_history,profile.numerical_snapshot,profile.numerical_comparison_choices,profile.documentary_catalog_available,profile.evidence_guide,profile.deployment_evidence,profile.request,profile.achievements,profile.profile_window,profile.experimental_patterns,undefined,profile.record_milestones));
   const edge=profile.numerical_evidence?.edge_evidence;const edgeAvailable=matchesEdge(edge,p.id,window!.season,p.group,p.games);
   notice.textContent=`${p.name} · ${window!.season===catalog.current_season?'Current season to date':'Season'} ${window!.label} · ${p.games} GP. Season record captured ${captureLabel(window!.captured_through)}. ${p.games<(p.group==='Goalies'?15:20)?`Feat eligibility needs ${p.group==='Goalies'?15:20} GP; recorded stats remain visible. `:''}NHL regular season · all strengths. ${edgeAvailable?'NHL EDGE observations included. ':''}Capability ratings remain unrated.`;
   const sources=get('source-detail');sources.replaceChildren(node('p',catalog.coverage),node('p',`Selected report SHA-256: ${window!.reports_sha256}`));const paths=new Set<string>();for(const source of profile.achievements?.source_urls??[]){const u=new URL(source);if(u.protocol!=='https:'||u.hostname!=='api.nhle.com'||paths.has(u.pathname))continue;paths.add(u.pathname);u.searchParams.set('start','0');u.searchParams.set('cayenneExp',`seasonId=${window!.season} and gameTypeId=2 and playerId=${p.id}`);const a=node('a',`NHL ${u.pathname.split('/').slice(-2).join('/')}`);a.href=u.href;sources.append(a);}if(edgeAvailable){const a=node('a','NHL EDGE source response');a.href=edge.source.url;sources.append(a,node('p',`EDGE captured ${captureLabel(edge.captured_at)} · SHA-256 ${edge.source.sha256}`));}else if(profile.numerical_evidence?.edge_capture_status)sources.append(node('p',`EDGE unavailable in this capture: ${profile.numerical_evidence.edge_capture_status.reason??'No matching observations'}. No zero or low capability inferred.`));
  }catch(e){if(token!==serial)return;notice.textContent=String(e);stage.append(node('p','The selected sheet is unavailable. No other season was substituted.'));}
 }
 theme.onchange=()=>{document.documentElement.dataset.rinkcraftTheme=theme.value;remember();};document.documentElement.dataset.rinkcraftTheme=theme.value;
 for(const control of [search,team,group])control.addEventListener(control===search?'input':'change',choices);
 player.onchange=()=>{selected=Number(player.value);void show();};season.onchange=()=>{window=catalog.windows.find(w=>w.season===Number(season.value))!;options();choices();void show();};
 get('print-profile').onclick=()=>windowPrint();function windowPrint(){globalThis.print();}
 get('copy-link').onclick=()=>{void navigator.clipboard.writeText(location.href).then(()=>get('link-status').textContent='Profile link copied.').catch(()=>get('link-status').textContent='Copy the address from your browser.');};
 options();choices();await show();root.dataset.ready='true';
}
const root=typeof document==='undefined'?null:document.querySelector<HTMLElement>('[data-rinkcraft-preview]');if(root)void mountPreview(root).catch(e=>{root.dataset.ready='error';const notice=document.getElementById('data-notice');if(notice)notice.textContent=String(e);});
