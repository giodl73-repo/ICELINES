from pathlib import Path
s=Path('target/js-allocation-sampling.mjs').read_text(encoding='utf-8')
s=s.replace('target/js-allocation-sampling-','target/js-context-attribution-')
needle="      const settings={samplingInterval:16384,stackDepth:64,includeObjectsCollectedByMajorGC:true,includeObjectsCollectedByMinorGC:true};"
assert s.count(needle)==1
s=s.replace(needle,"""      report.context_metadata={page:{contexts:[],scripts:[]},worker:{contexts:[],scripts:[]}};
      const record=(kind,method,params)=>{
        if(method==='Runtime.executionContextCreated')report.context_metadata[kind].contexts.push(params.context);
        if(method==='Debugger.scriptParsed')report.context_metadata[kind].scripts.push({scriptId:params.scriptId,url:params.url,executionContextId:params.executionContextId,hash:params.hash});
      };
      pageProfiler.on('Runtime.executionContextCreated',context=>record('page','Runtime.executionContextCreated',context));
      pageProfiler.on('Debugger.scriptParsed',script=>record('page','Debugger.scriptParsed',script));
      rootProfiler.on('Target.receivedMessageFromTarget',event=>{if(event.sessionId!==sessionId)return;const message=JSON.parse(event.message);if(message.method)record('worker',message.method,message.params);});
      await pageProfiler.send('Runtime.enable');await pageProfiler.send('Debugger.enable');
      await workerProfiler.send('Runtime.enable');await workerProfiler.send('Debugger.enable');
"""+needle)
needle="  for(const profile of Object.values(report.js_profiles)){"
assert s.count(needle)==1
s=s.replace(needle,"""  report.sampled_script_sources={page:[],worker:[]};
  for(const [kind,profile] of Object.entries(report.js_profiles)){
    const ids=new Set();const visit=node=>{if(node.callFrame.scriptId!=='0')ids.add(node.callFrame.scriptId);for(const child of node.children??[])visit(child);};visit(profile.head);
    const endpoint=kind==='page'?pageProfiler:workerProfiler;
    for(const scriptId of ids){
      const item={scriptId};
      try{const {scriptSource}=await endpoint.send('Debugger.getScriptSource',{scriptId});item.sha256=createHash('sha256').update(scriptSource,'utf8').digest('hex');item.bytes=Buffer.byteLength(scriptSource,'utf8');item.source_markers={injected_script:scriptSource.includes('class InjectedScript'),playwright_evaluation_serializer:scriptSource.includes('parseEvaluationResultValue')};}
      catch(error){item.error=String(error);}
      report.sampled_script_sources[kind].push(item);
    }
  }
"""+needle)
s=s.replace('additional JS allocation samplers after first committed package','additional JS allocation samplers plus Runtime/Debugger context metadata observers after first committed package')
Path('target/js-context-attribution.mjs').write_text(s,encoding='utf-8')
print('Prepared ownership metadata diagnostic; no app/engine transformation or acceptance claim.')
