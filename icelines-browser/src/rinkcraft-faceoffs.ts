import type {ProfileTimeWindow} from './rinkcraft-scope.js';
export const faceoffIds=new Set(['draw-specialist','oz-draw-record','dz-draw-record']);
export interface TaskEvidence {schema:string;scope_kind:string;player_id:number;season:number;season_type:string;group:string;source_urls:string[];source_sha256:string;screen_sha256:string;rules_sha256:string;captured_through:string;}
export interface FaceoffRecord {schema:string;context:string;wins:number;losses:number;attempts:number;games:number;fraction:number|null;attempts_per_game:number;signed_net:number;display:string;precision_display:string;}
export interface FaceoffBoundary {schema:string;state:string;notice:string;variants:{change:string;wins:number;losses:number;fraction:number;fraction_percentile:number;workload_percentile:number;qualifies:boolean}[];}
function sourceUrlsMatch(e:TaskEvidence,year:number):boolean {
 if(!Array.isArray(e.source_urls)||!e.source_urls.length)return false;
 try{return e.source_urls.every(s=>{const u=new URL(s),filters=(u.searchParams.get('cayenneExp')??'').split(' and ');return u.protocol==='https:'&&u.hostname==='api.nhle.com'&&u.pathname==='/stats/rest/en/skater/faceoffwins'&&filters.includes(`seasonId=${year}`)&&filters.includes('gameTypeId=2');});}catch{return false;}
}
export function matchesTaskEvidence(e:TaskEvidence|undefined,scope:ProfileTimeWindow,group:string,year=scope.season):boolean {
 return !!e&&scope.kind==='season'&&e.schema==='rinkcraft.task-evidence.v1'&&e.scope_kind==='season'&&e.player_id===scope.player_id&&e.season===year&&e.season_type===scope.season_type&&e.group===group&&sourceUrlsMatch(e,year)&&Number.isFinite(Date.parse(e.captured_through))&&[e.source_sha256,e.screen_sha256,e.rules_sha256].every(h=>/^[0-9a-f]{64}$/.test(h));
}
export function matchesFaceoffPattern(p:{id:string;evidence?:TaskEvidence;task_record?:FaceoffRecord;opportunities:Record<string,number|null>},scope:ProfileTimeWindow,group:string):boolean {
 if(!faceoffIds.has(p.id))return true;
 const r=p.task_record;return matchesTaskEvidence(p.evidence,scope,group)&&!!r&&r.schema==='rinkcraft.faceoff-record.v1'&&r.wins===p.opportunities.wins&&r.losses===p.opportunities.losses&&r.attempts===p.opportunities.draws&&r.games===p.opportunities.games;
}
