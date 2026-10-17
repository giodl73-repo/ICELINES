from pathlib import Path
s=Path('target/passive-process-inventory.mjs').read_text(encoding='utf-8')
s="import {mapTargetProcesses} from './target-process-markers.mjs';\n"+s
s=s.replace('target/passive-process-inventory-','target/postsample-renderer-mapping-')
needle="  if(report.passive_process_inventory.root_id!==rootProcessId)throw Error('Inventory root differs');"
assert s.count(needle)==1
s=s.replace(needle,needle+"""
  report.post_sampling_mapping={started_at_utc:new Date().toISOString(),sampler_exit:samplerExit,samples_before:samples.length};
  if(samplerExit!==0)throw Error('Mapping attempted before clean sampler termination');
  const mappingSession=await browser.newBrowserCDPSession();
  try{
    const targets=(await mappingSession.send('Target.getTargets')).targetInfos;
    report.target_process_mapping=await mapTargetProcesses(mappingSession,targets,output);
    if(!report.target_process_mapping.all_mapped)throw Error('Post-sampling target mapping incomplete');
  }finally{await mappingSession.detach();}
  report.post_sampling_mapping.finished_at_utc=new Date().toISOString();
  report.post_sampling_mapping.samples_after=samples.length;
  if(report.post_sampling_mapping.samples_after!==report.post_sampling_mapping.samples_before)throw Error('Samples changed during post-measurement tracing');
""")
s=s.replace('Post-sampling Windows process-role inventory only;', 'Post-sampling Windows process-role inventory then trace markers only after sampler termination;')
Path('target/postsample-renderer-mapping.mjs').write_text(s,encoding='utf-8')
print('Prepared renderer target mapping strictly after sampler exit; no added in-workload observers')
