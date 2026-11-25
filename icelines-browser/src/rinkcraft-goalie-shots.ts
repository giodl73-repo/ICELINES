export interface ShotRow {attribute:string;label:string;value:string;}
export interface GoalieShotContext {source_url:string;captured_at:string;sha256:string;headline:ShotRow[];rows:ShotRow[];}
const el=(tag:string,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function renderShotContext(context:GoalieShotContext,attribute?:string):HTMLElement {
 const section=el('div');section.className='rk-shot-context';
 section.append(el('strong',attribute?'Observed shot contexts':'Shot Record'),el('small','MoneyPuck.com · recorded shot sequences'));
 const list=(rows:ShotRow[])=>{const dl=el('dl');for(const r of rows){const cell=el('div');cell.append(el('dt',r.label),el('dd',r.value));dl.append(cell);}return dl;};
 section.append(list(attribute?context.rows.filter(r=>r.attribute===attribute):context.headline));
 const details=el('details');details.append(el('summary','Sequence breakdown'),list(context.rows),el('p','Shot-sequence outcomes, not measured goalie travel or reset time. Follow-ups can follow missed shots. Expected rebounds and xG are model estimates.'));
 const source=el('a','MoneyPuck.com shot archive') as HTMLAnchorElement;source.href=context.source_url;
 details.append(source,el('small',`Captured ${context.captured_at}`));section.append(details);return section;
}
