from pathlib import Path
from collections import defaultdict
import json
root=Path('target/js-allocation-sampling-Gtmrdy')
r=json.loads((root/'report.json').read_bytes())
assert r['completed'] and r['provisional_250_mib_target'] is None
assert len(r['packages'])==75 and len(r['imports'])==2 and len(r['warm_query_ms'])==20 and not r['missing_sampled_phases']
summary={'scope':'Sampled cumulative JS allocation churn after first committed package, including naturally collected objects. Direct selfSize tree accounting, not retained/native/backing-store/Rust/OS bytes or leak evidence. Nearest application ancestor is a stack attribution heuristic; unattributed/internal allocations remain separate.','contexts':{}}
for context,profile in r['js_profiles'].items():
    direct=defaultdict(int);owners=defaultdict(int);nodes={}
    def walk(node,owner=None):
        frame=node['callFrame'];url=frame['url'];path=url.split('/ICELINES/workbench/',1)[-1] if '/ICELINES/workbench/' in url else url
        key=(path,frame['functionName'],frame['lineNumber']+1)
        if '/ICELINES/workbench/' in url:owner=key
        assert node['id'] not in nodes
        nodes[node['id']]=node
        direct[key]+=node['selfSize'];owners[owner or ('(unattributed/internal)','',0)]+=node['selfSize']
        for child in node.get('children',[]):walk(child,owner)
    walk(profile['head'])
    assert all(sample['nodeId'] in nodes for sample in profile['samples'])
    order=lambda values:[{'url':key[0],'function':key[1],'line':key[2],'sampled_self_size_bytes':value} for key,value in sorted(values.items(),key=lambda pair:pair[1],reverse=True) if value]
    row={'node_count':len(nodes),'sample_count':len(profile['samples']),'summed_node_self_size_bytes':sum(direct.values()),'summed_sample_size_bytes':sum(sample['size'] for sample in profile['samples']),'direct':order(direct),'nearest_application_ancestor':order(owners)}
    summary['contexts'][context]=row
    print(context,'node self-size sum MiB',row['summed_node_self_size_bytes']/1048576,'samples',row['sample_count'])
    print(json.dumps(row['nearest_application_ancestor'][:8],indent=2))
(root/'js-allocation-summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
