export interface SkaterContext {source_credit:string;notes:string[];values:Record<string,number>;rows:{attribute:string;label:string;value:string}[];defending?:import('./rinkcraft-defending-record.js').DefendingRecord;handling?:import('./rinkcraft-handling-record.js').HandlingRecord;reading?:import('./rinkcraft-reading-record.js').ReadingRecord;shooting?:import('./rinkcraft-shooting-record.js').ShootingRecord;passing?:import('./rinkcraft-passing-record.js').PassingRecord;}
const el=(tag:string,text?:string)=>{const n=document.createElement(tag);if(text!==undefined)n.textContent=text;return n;};
export function renderSkaterContext(context:SkaterContext,attribute?:string):HTMLElement {
 const block=el('div');block.className='rk-skater-context rk-shot-context';
 const credit='NHL'+(context.source_credit.includes('EDGE')?' / EDGE':'')+(context.source_credit.includes('MoneyPuck.com')?' / MoneyPuck.com':'');
 block.append(el('strong',attribute?`${attribute} record`:'Attribute Record'),el('small',credit));
 const mainLabels=['Primary assists','Secondary assists','Goals minus expected (goalie in net)','Penalty-kill deployment'];
 const list=(rows:SkaterContext['rows'])=>{const dl=el('dl');for(const r of rows){const row=el('div');row.append(el('dt',r.label),el('dd',r.value));dl.append(row);}return dl;};
 const selected=attribute?context.rows.filter(r=>r.attribute===attribute):mainLabels.flatMap(label=>{const row=context.rows.find(r=>r.label===label);return row?[row]:[];});
 block.append(list(selected));
 const details=el('details');details.append(el('summary','All six attribute records'),el('p',context.source_credit));
 for(const attr of ['Skating','Handling','Passing','Shooting','Reading','Defending'])details.append(el('strong',attr),list(context.rows.filter(r=>r.attribute===attr)));
 for(const note of context.notes)details.append(el('p',note));block.append(details);return block;
}
