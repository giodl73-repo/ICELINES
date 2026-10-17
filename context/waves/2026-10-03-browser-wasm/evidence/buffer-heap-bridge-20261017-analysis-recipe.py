import hashlib,json,re,sys
from pathlib import Path

root=Path(sys.argv[1]);r=json.loads((root/'report.json').read_text(encoding='utf-8'))
assert r['completed'] and len(r['packages'])==75 and [x['rows'] for x in r['imports']]==[905,332] and len(r['warm_query_ms'])==20
assert r['provisional_250_mib_target'] is None
result={'scope':'Nine additional-observer checkpoint dumps, not normal acceptance. Main Blink committed/resident/used/pooled categories overlap and are not a partition of OS private bytes. Non-object resident difference is within this heap only, not reclaimable OS memory. No native-allocation lifetime/rate or GC-event attribution.','artifact':r['artifact'],'acceptance':None,'checkpoints':[]}
for c in r['heap_checkpoints']:
    m=c['mapping'];p=root/c['label']/m['trace']['file'];raw=p.read_bytes();assert hashlib.sha256(raw).hexdigest()==m['trace']['sha256']
    events=json.loads(raw)['traceEvents'];guid=m['memory_dump']['dumpGuid']
    begin=[e for e in events if e.get('name')=='GlobalMemoryDump' and e.get('ph')=='b' and e.get('args',{}).get('dump_guid')==guid];assert len(begin)==1
    b=begin[0];end=[e for e in events if e.get('name')=='GlobalMemoryDump' and e.get('ph')=='e' and e.get('id2')==b.get('id2') and e['pid']==b['pid'] and e['ts']>=b['ts']];assert len(end)==1
    pages=[t for t in m['targets'] if t['type']=='page'];assert len(pages)==1 and pages[0]['mapped'];pid=pages[0]['renderer_pid'];alloc={};edges=[]
    for e in events:
        if e['pid']!=pid or not b['ts']<=e.get('ts',-1)<=end[0]['ts']:continue
        d=e.get('args',{}).get('dumps',{});edges.extend(d.get('allocators_graph',[]))
        for name,value in d.get('allocators',{}).items():
            if name=='global' or name.startswith('global/'):continue
            assert name not in alloc;alloc[name]=value
    def scalar(data,name):return int(data['attrs'][name]['value'],16)
    heap=alloc['blink_gc/main/heap'];metrics={k:scalar(heap,k) for k in ['committed_size','size','allocated_objects_size','pooled_size']}
    assert metrics['size']>=metrics['allocated_objects_size']
    metrics['non_object_reported_resident_bytes']=metrics['size']-metrics['allocated_objects_size']
    by_guid={v['guid']:(k,v) for k,v in alloc.items()};outgoing={}
    for edge in edges:outgoing.setdefault(edge['source'],[]).append(edge['target'])
    classes=[];joins=[]
    for name,value in alloc.items():
        match=re.fullmatch(r'blink_objects/blink_gc/main/(.+) \(0x[0-9a-f]+\)',name)
        if not match:continue
        amount=scalar(value,'allocated_objects_size');assert amount==scalar(value,'size')
        classes.append({'name':match[1],'bytes':amount,'objects':scalar(value,'object_count')})
        if amount:
            sub=[by_guid[g] for g in outgoing.get(value['guid'],[]) if g in by_guid and by_guid[g][0].startswith('blink_gc/main/allocated_objects/')]
            assert len(sub)==1 and scalar(sub[0][1],'size')==amount
            joins.append({'class_guid':value['guid'],'suballocation_guid':sub[0][1]['guid'],'bytes':amount})
    assert sum(x['bytes'] for x in classes)==metrics['allocated_objects_size']==scalar(alloc['blink_gc/main/allocated_objects'],'size')
    buckets=[(name,scalar(value,'free_usable_size')) for name,value in alloc.items() if re.fullmatch(r'blink_gc/main/heap/[^/]+/freelist/bucket_\d+',name)]
    row={'label':c['label'],'renderer_pid':pid,'trace_sha256':m['trace']['sha256'],'metrics':metrics,'reported_freelist_bucket_bytes':sum(v for _,v in buckets),'freelist_buckets':buckets,'selected_classes':[x for x in classes if x['name'] in ['blink::LogicalLineItems','blink::ConstraintSpace::RareData','blink::LayoutResult']],'nonempty_class_joins':joins}
    names={e['pid']:e.get('args',{}).get('name') for e in events if e.get('name')=='process_name'}
    sample=c['latest_os_sample'];processes=sample['processes']
    assert len({p['id'] for p in processes})==len(processes)
    app=[p for p in processes if p['id']==pid];assert len(app)==1
    whole=sum(p['private_bytes'] for p in processes)
    row['latest_os_sample']={'phase':sample['phase'],'elapsed_ms':sample['elapsed_ms'],'app_renderer_private_bytes':app[0]['private_bytes'],'other_owned_private_bytes':whole-app[0]['private_bytes'],'whole_browser_private_bytes':whole,'not_simultaneous_with_native_dump':True,'processes':[{'pid':p['id'],'trace_process_name':names.get(p['id'],'unmapped owned process'),'private_bytes':p['private_bytes']} for p in processes]}
    result['checkpoints'].append(row)
    print(c['label'],{k:round(v/1048576,3) for k,v in metrics.items()})
assert len(result['checkpoints'])==9
result['maximum_sampled_main_heap_committed_bytes']=max(c['metrics']['committed_size'] for c in result['checkpoints'])
(root/'heap-bridge-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
