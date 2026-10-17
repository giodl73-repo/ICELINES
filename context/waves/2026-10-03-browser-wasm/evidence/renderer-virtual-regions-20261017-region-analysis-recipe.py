import datetime,hashlib,json,sys
from pathlib import Path
root=Path(sys.argv[1]);r=json.loads((root/'report.json').read_text(encoding='utf-8'));s=json.loads((root/'postmap-process-summary.json').read_text(encoding='utf-8'))
assert r['completed'] and r['provisional_250_mib_target'] is None
info=r['renderer_virtual_regions'];raw=(root/info['report']['file']).read_bytes();assert hashlib.sha256(raw).hexdigest()==info['report']['sha256']
def time(v):return datetime.datetime.fromisoformat(v.replace('Z','+00:00'))
assert time(info['started_at_utc'])>=time(r['passive_process_inventory']['observed_at_utc'])
assert time(info['finished_at_utc'])<=time(r['post_sampling_mapping']['started_at_utc'])
assert r['post_sampling_mapping']['samples_before']==r['post_sampling_mapping']['samples_after']
pages=[t for t in r['target_process_mapping']['targets'] if t['type']=='page'];assert len(pages)==1
pid=pages[0]['renderer_pid'];census=json.loads(raw);rows=[p for p in census['renderers'] if p['pid']==pid];assert len(rows)==1 and rows[0]['completed'];app=rows[0]
identities=[p for p in r['passive_process_inventory']['processes'] if p['id']==pid];assert len(identities)==1
assert abs((time(app['created_at_utc'])-time(identities[0]['created_at_utc'])).total_seconds())<=0.001
groups=app['committed_by_type_and_protection'];private_executable=sum(v for key,v in groups.items() if key.startswith('MEM_PRIVATE:EXECUTE'))
result={'scope':'Post-workload, post-engine-restart, post-sampling metadata census before added trace. Not earlier peak composition, atomic snapshot, physical RAM, total private-usage partition or attribution to Rust/JS/native allocator owners.','artifact':r['artifact'],'acceptance':None,'census_boundary':info,'mapping_boundary':r['post_sampling_mapping'],'application_page_associated_renderer_pid':pid,'application_region_census':app,'private_readwrite_region_bytes':groups.get('MEM_PRIVATE:READWRITE',0),'private_executable_region_bytes':private_executable,'last_pre_census_os_sample':s['last'],'peak_pre_census_os_sample':s['peak'],'partial_renderer_censuses':[p for p in census['renderers'] if not p['completed']],'primary_reference':'https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-virtualqueryex','copy_on_write_limit':'Windows may continue reporting private copy-on-write pages as MEM_IMAGE/MEM_MAPPED. Do not subtract/sum all mapped/image committed sizes as private memory. No page touching/locking/working-set manipulation was used to detect COW.'}
(root/'virtual-region-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print('App renderer census complete:',app['regions_queried'],'regions; readwrite private MiB',result['private_readwrite_region_bytes']/1048576,'executable private MiB',private_executable/1048576)
print('Census is after engine restart and before trace; measured peak remains',r['sampled_peak_private_bytes']/1048576,'MiB')
