// Included inside the study report's private script scope.
function showFullPuckSequence(r,parent){
 const ref=r.full_puck_sequence;if(!ref)return;
 const detail=node('details','',parent);detail.className='full-puck-sequence';node('summary',`Follow the puck · all players · ${ref.frame_count} captured samples`,detail);
 let loaded=false;
 detail.ontoggle=async()=>{if(!detail.open||loaded)return;loaded=true;const loading=node('p','Loading the whole captured sequence…',detail);
  try{
   if(!new RegExp('^full-sequence/[0-9]{10}-[0-9]+[.]json$').test(ref.file))throw Error('Invalid sequence reference');
   const response=await fetch(new URL(ref.file,location.href));if(!response.ok)throw Error(`Sequence request failed (${response.status})`);
   const bytes=await response.arrayBuffer(),digest=await crypto.subtle.digest('SHA-256',bytes),hash=[...new Uint8Array(digest)].map(v=>v.toString(16).padStart(2,'0')).join('');
   if(hash!==ref.sha256)throw Error('Sequence content hash does not match this report');
   const data=JSON.parse(new TextDecoder().decode(bytes));if(data.schema!=='icelines.full-puck-sequence.v0.1'||data.target.game_id!==r.target.game_id||data.target.event_id!==r.target.event_id||data.frame_count!==ref.frame_count)throw Error('Sequence identity/count mismatch');
   const history=data.distance_history;if(history.frames.length!==ref.frame_count||data.phase.same_identity_motion.synchronized_scene.frames.length!==ref.frame_count)throw Error('Incomplete whole-sequence sample coverage');loading.remove();if(history.status!=='all_player_distances_available')throw Error(history.reason||'Distance history unavailable');
   const frames=history.frames,roster=history.roster,last=ref.frame_count-1,candidates=data.candidate_windows;
   const scopeLabel=node('label','Displayed end ',detail),scope=node('select','',scopeLabel);scope.className='sequence-end';
   let option=node('option',`Full captured clip: 0 → ${last}`,scope);option.value='full';
   candidates.forEach((c,i)=>{const o=node('option',`Candidate ${i+1}: ${c.from_frame_index} → ${c.to_frame_index} (goal timing unverified)`,scope);o.value='candidate:'+i});
   if(candidates.length===1)scope.value='candidate:0';
   const scopeNote=node('p','',detail);scopeNote.className='sequence-scope small';
   node('p','First captured sample may already be mid-possession. These are source samples, not calibrated seconds. Exact goal-frame alignment and active personnel remain unverified.',detail).className='small';
   const legend=node('p','',detail);legend.className='history-legend small';
   const layout=node('div','',detail);layout.className='distance-history-layout';
   const labels=document.createElementNS('http://www.w3.org/2000/svg','svg');labels.classList.add('history-labels-svg');labels.setAttribute('aria-label','Player labels for distance history');layout.append(labels);
   const scroll=node('div','',layout);scroll.className='distance-history-scroll';
   const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.classList.add('distance-history-svg');svg.setAttribute('role','img');svg.setAttribute('aria-label','Both teams and goalies: player distance to the puck across captured samples');scroll.append(svg);
   const currentLabel=node('strong','',detail);currentLabel.className='history-current';
   const table=node('table','',detail);table.className='all-player-distances';
   node('p','*Regulation-derived feet. Δ is the distance change since the preceding available, adjacent source sample; it reflects both puck and player movement, not skating speed or checking. Unknowns break continuity. Nearest body does not establish possession.',detail).className='small';
   const rinkHost=node('div','',detail),api=showMotionRink(data.phase,rinkHost);if(!api)throw Error('Sample scene unavailable');
   let current=0;
   const end=()=>scope.value==='full'?last:candidates[Number(scope.value.split(':')[1])].to_frame_index;
   const get=(frame,id)=>{const rows=frame?.players.filter(v=>v.player_id===id)||[];return rows.length===1?rows[0]:null};
   function draw(){
    const stop=end(),shown=frames.filter(f=>f.frame_index<=stop);svg.replaceChildren();labels.replaceChildren();
    scopeNote.textContent=scope.value==='full'?`Showing all captured samples 0 → ${last}. ${candidates.length} candidate net-plane crossing(s); none is a verified goal timestamp. Later motion is retained.`:`Showing 0 → ${stop} through an unverified candidate net-plane crossing. The entire ${ref.frame_count}-sample source remains available.`;
    const maximum=Math.max(1,...shown.flatMap(f=>f.players.map(p=>p.puck_separation_feet_derived).filter(Number.isFinite))),left=0,step=7,rowHeight=22,top=28,width=left+shown.length*step+12,height=top+roster.length*rowHeight+32;
    svg.setAttribute('viewBox',`0 0 ${width} ${height}`);svg.style.minWidth=width+'px';svg.style.width=width+'px';labels.setAttribute('viewBox',`0 0 130 ${height}`);labels.style.height=height+'px';
    const make=(tag,attrs,text)=>{const n=document.createElementNS('http://www.w3.org/2000/svg',tag);for(const [k,v] of Object.entries(attrs))n.setAttribute(k,v);if(text!=null)n.textContent=text;svg.append(n);return n};
    make('title',{},`All-player puck distances, samples 0–${stop}`);
    for(const [i,actor] of roster.entries()){
     const y=top+i*rowHeight,label=document.createElementNS('http://www.w3.org/2000/svg','text');label.setAttribute('x','2');label.setAttribute('y',String(y+14));label.setAttribute('font-size','10');label.setAttribute('fill','#18304a');label.setAttribute('data-player-id',actor.player_id);label.textContent=`${actor.name||actor.player_id} · ${actor.team||'?'}${actor.goalie===true?' G':actor.goalie==null?' ?':''}${actor.metadata_changes_within_window?' [varies]':''}`;labels.append(label);
     for(const [j,f] of shown.entries()){
      const player=get(f,actor.player_id),value=player?.puck_separation_feet_derived,available=Number.isFinite(value),rect=make('rect',{x:left+j*step,y,width:step-1,height:rowHeight-2,fill:available?`hsl(210 55% ${22+72*value/maximum}%)`:'#dedede','data-player-id':actor.player_id,'data-frame-index':f.frame_index,'data-distance':available?value:'unknown'});
      const title=document.createElementNS('http://www.w3.org/2000/svg','title');title.textContent=`${actor.name||actor.player_id}, sample ${f.frame_index}: ${available?value.toFixed(2)+' ft (derived)':'unavailable'}`;rect.append(title);rect.onclick=()=>api.setSample(f.frame_index);
     }
    }
    for(const [j,f] of shown.entries())if(j===0||j===shown.length-1||f.frame_index%20===0)make('text',{x:left+j*step,y:height-8,'font-size':10,fill:'#52687a'},f.frame_index);
    const column=shown.findIndex(f=>f.frame_index===current);if(column>=0)make('line',{x1:left+column*step+2,x2:left+column*step+2,y1:top-5,y2:height-25,stroke:'#b03047','stroke-width':2,'data-role':'history-cursor','data-frame-index':current,'pointer-events':'none'});
    legend.textContent=`Shared scale: 0 → ${maximum.toFixed(1)} ft*. Darker = smaller puck separation; grey = unavailable. All observed IDs, both teams and goalies. Select a cell to inspect that source sample; labels use first source metadata; changes are flagged.`;
    currentLabel.textContent=`All observed players · source sample ${current}`;table.replaceChildren();const head=node('tr','',table);for(const h of ['Player / team','Puck ft*','Δ ft*'])node('th',h,head);
    const selected=frames.find(f=>f.frame_index===current);
    for(const actor of roster){const p=get(selected,actor.player_id),tr=node('tr','',table);tr.dataset.playerId=actor.player_id;if(p?.goalie===true)tr.className='sample-goalie';node('td',`${actor.name||actor.player_id} · ${p?.team||actor.team||'?'}${p?.goalie===true?' G':''}`,tr);for(const key of ['puck_separation_feet_derived','change_since_previous_sample_feet_derived'])node('td',p?.[key]==null?'Unknown':p[key].toFixed(1),tr)}
   }
   api.element.addEventListener('rink-source-sample',e=>{const index=e.detail.frame_index;if(!Number.isInteger(index)||index===current)return;current=index;if(current>end())scope.value='full';draw()});
   scope.onchange=()=>{if(current>end())current=end();draw();api.setSample(current)};
   draw();api.setSample(0);
  }catch(error){for(const child of [...detail.children])if(child.tagName!=='SUMMARY')child.remove();detail.append(loading);loading.textContent=`Whole-sequence view unavailable: ${error.message}`;loading.className='bad';loaded=false}
 };
}
