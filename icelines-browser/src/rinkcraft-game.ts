import type {EngineClient} from './engine-client.js';
export interface GameInspiration {player_id:number;player_name:string;evidence_season:number;documentary_catalog_sha256?:string|null;position?:string;class_id?:string;}
interface GameController {setInspiration(value:GameInspiration):Promise<void>;destroy():void;}
interface GameModule {mountGameStudio(root:HTMLElement,options:{inspiration?:GameInspiration;loadRules():Promise<unknown>;build(rules:unknown,request:unknown):Promise<unknown>;loadPossessionRules():Promise<unknown>;possession(characterRules:unknown,rules:unknown,request:unknown):Promise<unknown>;exportPossession(characterRules:unknown,rules:unknown,request:unknown):Promise<unknown>;importPossession(replay:unknown):Promise<unknown>;importCharacter(rules:unknown,character:unknown):Promise<unknown>}):Promise<GameController>;}
export async function mountBrowserGameStudio(root:HTMLElement,engine:EngineClient,inspiration?:GameInspiration):Promise<GameController>{
 const module=await import(new URL('../rinkcraft-game.js',import.meta.url).href) as GameModule;
 return module.mountGameStudio(root,{
  inspiration,
  loadRules:async()=>{const response=await fetch(new URL('../rinkcraft-game-rules.json',import.meta.url));if(!response.ok)throw Error('No operator-provided playtest rules loaded.');const raw=await response.text();if(raw.length>65536)throw Error('Game rules too large.');return JSON.parse(raw) as unknown;},
  loadPossessionRules:async()=>{const r=await fetch(new URL('../rinkcraft-possession-rules.json',import.meta.url));if(!r.ok)throw Error('No operator-provided possession rules.');const raw=await r.text();if(raw.length>8192)throw Error('Rules too large.');return JSON.parse(raw) as unknown;},
  possession:(characterRules,rules,request)=>engine.request('possessionSession',{characterRules,rules,request}),
  exportPossession:(characterRules,rules,request)=>engine.request('exportPossessionReplay',{characterRules,rules,request}),
  importPossession:replay=>engine.request('importPossessionReplay',{replay}),
  build:(rules,request)=>engine.request('gameCharacter',{rules,request}),
  importCharacter:(rules,character)=>engine.request('importGameCharacter',{rules,character}),
 });
}
