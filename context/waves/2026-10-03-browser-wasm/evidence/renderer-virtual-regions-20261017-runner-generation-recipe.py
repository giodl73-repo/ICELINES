from pathlib import Path
s=Path('target/postsample-renderer-mapping.mjs').read_text(encoding='utf-8').replace('target/postsample-renderer-mapping-','target/renderer-virtual-regions-')
needle="  if(report.passive_process_inventory.root_id!==rootProcessId)throw Error('Inventory root differs');"
assert s.count(needle)==1
s=s.replace(needle,needle+"""
  const inventoryFile=resolve(output,'owned-process-inventory.json');
  await writeFile(inventoryFile,JSON.stringify(report.passive_process_inventory,null,2)+'\\n');
  const regionFile=resolve(output,'renderer-regions.json');
  const regionStart=new Date().toISOString();
  const regionRead=spawnSync('python',[resolve(repository,'target/renderer-virtual-regions.py'),inventoryFile,regionFile],{encoding:'utf8',windowsHide:true});
  if(regionRead.status!==0)throw Error('Post-sampling renderer census failed: '+regionRead.stderr);
  const regionBytes=await readFile(regionFile);
  const regionData=JSON.parse(regionBytes);
  if(!regionData.renderers.some(item=>item.completed))throw Error('No complete renderer region census');
  report.renderer_virtual_regions={started_at_utc:regionStart,finished_at_utc:new Date().toISOString(),report:{file:'renderer-regions.json',sha256:createHash('sha256').update(regionBytes).digest('hex')}};
""")
s=s.replace('then trace markers only after sampler termination','then virtual-region census and trace markers only after sampler termination')
Path('target/renderer-virtual-regions.mjs').write_text(s,encoding='utf-8')
print('Prepared full workload then metadata-only region census, before post-sampler trace mapping')
