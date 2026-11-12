import type {EngineClient} from './engine-client.js';
export interface GameInspiration {player_id:number;player_name:string;evidence_season:number;documentary_catalog_sha256?:string|null;position?:string;class_id?:string;}
interface GameController {setInspiration(value:GameInspiration):Promise<void>;destroy():void;}
interface GameModule {mountGameStudio(root:HTMLElement,options:{inspiration?:GameInspiration;loadUnitBlueprint():Promise<unknown>;loadUnitRules():Promise<unknown>;unitStudio(command:unknown):Promise<unknown>;loadRules():Promise<unknown>;build(rules:unknown,request:unknown):Promise<unknown>;loadPossessionRules():Promise<unknown>;possession(characterRules:unknown,rules:unknown,request:unknown):Promise<unknown>;exportPossession(characterRules:unknown,rules:unknown,request:unknown):Promise<unknown>;importPossession(replay:unknown):Promise<unknown>;inspectPossession(replay:unknown,step:number):Promise<unknown>;branchPossession(replay:unknown,fork:unknown):Promise<unknown>;comparePossession(replay:unknown):Promise<unknown>;importCharacter(rules:unknown,character:unknown):Promise<unknown>}):Promise<GameController>;}
export async function mountBrowserGameStudio(root:HTMLElement,engine:EngineClient,inspiration?:GameInspiration):Promise<GameController>{
 const module=await import(new URL('../rinkcraft-game.js',import.meta.url).href) as GameModule;
 return module.mountGameStudio(root,{
  inspiration,
  loadUnitBlueprint:()=>loadUnitFile('rinkcraft-unit-blueprint.json',262144),
  loadUnitRules:()=>loadUnitFile('rinkcraft-unit-rules.json',8192),
  unitStudio:command=>engine.request('unitStudio',command),
  loadRules:async()=>{const response=await fetch(new URL('../rinkcraft-game-rules.json',import.meta.url));if(!response.ok)throw Error('No operator-provided playtest rules loaded.');const raw=await response.text();if(raw.length>65536)throw Error('Game rules too large.');return JSON.parse(raw) as unknown;},
  loadPossessionRules:async()=>{const r=await fetch(new URL('../rinkcraft-possession-rules.json',import.meta.url));if(!r.ok)throw Error('No operator-provided possession rules.');const raw=await r.text();if(raw.length>8192)throw Error('Rules too large.');return JSON.parse(raw) as unknown;},
  possession:(characterRules,rules,request)=>engine.request('possessionSession',{characterRules,rules,request}),
  exportPossession:(characterRules,rules,request)=>engine.request('exportPossessionReplay',{characterRules,rules,request}),
  importPossession:replay=>engine.request('importPossessionReplay',{replay}),
  inspectPossession:(replay,step)=>engine.request('inspectPossessionReplay',{replay,step}),
  branchPossession:(replay,fork)=>engine.request('branchPossessionReplay',{replay,fork}),
  comparePossession:(replay)=>engine.request('comparePossessionPlans',{replay}),
  build:(rules,request)=>engine.request('gameCharacter',{rules,request}),
  importCharacter:(rules,character)=>engine.request('importGameCharacter',{rules,character}),
 });
}

async function loadUnitFile(name:string,max:number):Promise<unknown>{const r=await fetch(new URL('../'+name,import.meta.url));if(!r.ok)throw Error('Authored unit setup unavailable: '+name);const bytes=await r.arrayBuffer();if(bytes.byteLength>max)throw Error('Unit setup file exceeds bounds');return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(bytes)) as unknown;}
