import hashlib,json,re
from pathlib import Path
root=Path('target/renderer-allocation-checkpoints-W9Zw60')
r=json.loads((root/'report.json').read_text(encoding='utf-8'))
result={'scope':'Read-only reanalysis of nine historical additional-observer native dumps; not current release acceptance, byte ownership or reachability proof. Named blink_objects/blink_gc classes are an alternative view of allocated_objects, never additional bytes. No forced GC/new browser run.','artifact':r['artifact'],'acceptance':None,'checkpoints':[]}
for checkpoint in r['heap_checkpoints']:
    m=checkpoint['mapping'];p=root/checkpoint['label']/m['trace']['file'];raw=p.read_bytes()
    assert hashlib.sha256(raw).hexdigest()==m['trace']['sha256']
    events=json.loads(raw)['traceEvents'];guid=m['memory_dump']['dumpGuid']
    begin=[e for e in events if e.get('name')=='GlobalMemoryDump' and e.get('ph')=='b' and e.get('args',{}).get('dump_guid')==guid];assert len(begin)==1
    b=begin[0]
    end=[e for e in events if e.get('name')=='GlobalMemoryDump' and e.get('ph')=='e' and e.get('id2')==b.get('id2') and e['pid']==b['pid'] and e['ts']>=b['ts']];assert len(end)==1
    pages=[t for t in m['targets'] if t['type']=='page'];assert len(pages)==1 and pages[0]['mapped']
    pid=pages[0]['renderer_pid'];alloc={};edges=[]
    for e in events:
        if e['pid']!=pid or not b['ts']<=e.get('ts',-1)<=end[0]['ts']:continue
        d=e.get('args',{}).get('dumps',{})
        edges.extend(d.get('allocators_graph',[]))
        for name,value in d.get('allocators',{}).items():
            if name=='global' or name.startswith('global/'):continue
            assert name not in alloc,name
            alloc[name]=value
    def size(value):return int(value['attrs']['size']['value'],16)
    classes=[]
    for name,value in alloc.items():
        match=re.fullmatch(r'blink_objects/blink_gc/main/(.+) \(0x[0-9a-f]+\)',name)
        if not match:continue
        attrs=value['attrs'];count=int(attrs['object_count']['value'],16)
        amount=int(attrs['allocated_objects_size']['value'],16)
        assert size(value)==amount
        classes.append({'name':match[1],'bytes':amount,'objects':count,'guid':value['guid']})
    assert classes
    total=sum(c['bytes'] for c in classes)
    assert total==size(alloc['blink_gc/main/allocated_objects'])
    # These named classes and anonymous suballocations describe the same bytes.
    links=[]
    for item in classes:
        if not item['bytes']:continue
        outgoing=[e for e in edges if e.get('source')==item['guid']]
        targets={e['target'] for e in outgoing}
        matches=[(n,a) for n,a in alloc.items() if a['guid'] in targets and n.startswith('blink_gc/main/allocated_objects/')]
        assert len(matches)==1,(checkpoint['label'],item['name'],outgoing)
        anonymous_name,anonymous=matches[0]
        assert size(anonymous)==item['bytes']
        links.append({'class':item['name'],'source_guid':item['guid'],'target_guid':anonymous['guid'],'anonymous_name':anonymous_name,'bytes':item['bytes']})
    chosen=[c for c in classes if any(term in c['name'] for term in ['LogicalLineItems','ConstraintSpace::RareData','LayoutResult','Decompression','ReadableStream','TransformStream'])]
    row={'label':checkpoint['label'],'renderer_pid':pid,'targets_sharing_pid':[t['type'] for t in m['targets'] if t['renderer_pid']==pid],'trace_sha256':m['trace']['sha256'],'main_allocated_objects_bytes':total,'class_count':len(classes),'nonempty_class_edges_verified':len(links),'top_classes':sorted(classes,key=lambda c:c['bytes'],reverse=True)[:15],'selected_classes':chosen,'verified_ownership_edges':links}
    result['checkpoints'].append(row)
    print(row['label'],'total MiB',round(total/1048576,3),'largest',[(c['name'],round(c['bytes']/1048576,3),c['objects']) for c in row['top_classes'][:3]])
assert len(result['checkpoints'])==9
(root/'blink-layout-type-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
