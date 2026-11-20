// Shared Rust supplies definitions; this renderer does not assess a player.
export interface EvidenceGuideTopic {
  key:string; scope:string; title:string; question:string; evidence:string; boundary:string;
}
const node=<K extends keyof HTMLElementTagNameMap>(tag:K,text?:string):HTMLElementTagNameMap[K]=>{
  const element=document.createElement(tag);if(text!==undefined)element.textContent=text;return element;
};
export function renderEvidenceGuide(topics:EvidenceGuideTopic[]|undefined,position:string):HTMLElement|undefined {
  const branch=['Goalie','Goalies','G'].includes(position)?'goalie':
    ['C','L','R','D','LW','RW','F','Center','LeftWing','RightWing','Defenseman','Forwards','Defense'].includes(position)?'skater':undefined;
  const selected=(topics??[]).filter(t=>t.scope==='all'||(branch!==undefined&&t.scope===branch));
  if(!selected.length)return undefined;
  const guide=node('details');guide.className='rk-evidence-guide';
  guide.append(node('summary','How to read this profile'));
  guide.append(node('p','Definition guide · not player observations. Capability attributes remain unrated.'));
  const grid=node('div');grid.className='rk-guide-topics';
  for(const topic of selected){
    const section=node('section');section.dataset.guideTopic=topic.key;
    section.append(node('h4',topic.title),node('p',topic.question),node('p',topic.evidence),node('small',topic.boundary));
    grid.append(section);
  }
  guide.append(grid);return guide;
}
