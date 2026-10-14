import hashlib, json
from pathlib import Path
root=Path('target/renderer-allocation-checkpoints-W9Zw60')
r=json.loads((root/'report.json').read_text(encoding='utf-8'))
out={'scope':'Reanalysis of nine historical supplemental native dumps. Extra trace observers; no new workload run, no forced GC, no memory acceptance. Categories are hierarchical and may share ownership: do not sum, subtract from private bytes, or infer application ownership from GPU process membership.','source_commit':r['artifact'],'acceptance':None,'checkpoints':[]}
for c in r['heap_checkpoints']:
    m=c['mapping'];p=root/c['label']/m['trace']['file'];raw=p.read_bytes()
    assert hashlib.sha256(raw).hexdigest()==m['trace']['sha256']
    events=json.loads(raw)['traceEvents'];guid=m['memory_dump']['dumpGuid']
    begin=[e for e in events if e.get('name')=='GlobalMemoryDump' and e.get('ph')=='b' and e.get('args',{}).get('dump_guid')==guid]
    assert len(begin)==1
    b=begin[0]
    end=[e for e in events if e.get('name')=='GlobalMemoryDump' and e.get('ph')=='e' and e.get('id2')==b.get('id2') and e['pid']==b['pid'] and e['ts']>=b['ts']]
    assert len(end)==1
    names={}
    for e in events:
        if e.get('name')=='process_name':
            name=e.get('args',{}).get('name');pid=e['pid']
            assert pid not in names or names[pid]==name
            names[pid]=name
    gpu=[pid for pid,name in names.items() if name=='GPU Process'];assert len(gpu)==1
    pid=gpu[0];alloc={};global_entries=0
    for e in events:
        if e['pid']!=pid or not b['ts']<=e.get('ts',-1)<=end[0]['ts']:continue
        for name,data in e.get('args',{}).get('dumps',{}).get('allocators',{}).items():
            if name=='global' or name.startswith('global/'):
                global_entries+=1;continue
            assert name not in alloc,name
            alloc[name]=data
    assert alloc
    categories={}
    for name,data in alloc.items():
        attrs={k:int(v['value'],16) for k,v in data.get('attrs',{}).items() if v.get('type')=='scalar' and v.get('units')=='bytes'}
        if attrs:categories[name]=attrs
    os=[p for p in c['latest_os_sample']['processes'] if p['id']==pid];assert len(os)==1
    row={'label':c['label'],'gpu_pid':pid,'trace_sha256':m['trace']['sha256'],'dump_guid':guid,'categories':categories,'global_entries_separate':global_entries,'latest_os_sample':os[0],'os_sample_is_not_simultaneous_with_dump':True}
    out['checkpoints'].append(row)
    selected={k:{a:round(v/1048576,3) for a,v in categories[k].items()} for k in ['cc','cc/resource_memory','gpu','gpu/shared_images','gpu/shader_cache','skia','malloc','malloc/allocated_objects','partition_alloc'] if k in categories}
    print(c['label'],'OS private MiB',round(os[0]['private_bytes']/1048576,3),selected)
assert len(out['checkpoints'])==9
(root/'gpu-allocation-summary.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8')
