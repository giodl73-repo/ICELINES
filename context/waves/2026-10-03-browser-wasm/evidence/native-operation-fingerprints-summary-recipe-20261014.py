from pathlib import Path
from collections import defaultdict
import json
root=Path('target/native-operation-fingerprints-n7AlWE')
probe=json.loads((root/'report.json').read_bytes())
old=json.loads(Path('target/viewport-runtime-attribution-5klpj9/report.json').read_bytes())['native_profile']['profile']
assert probe['completed'] and probe['provisional_250_mib_target'] is None
def prefix(sample,profile,depth):
 if len(sample['stack'])<depth:return None
 frames=[]
 for address in sample['stack'][:depth]:
  if not address.startswith('0x'):return None
  pc=int(address,16);matches=[m for m in profile['modules'] if int(m['baseAddress'],16)<=pc<int(m['baseAddress'],16)+m['size']]
  if len(matches)!=1:return None
  m=matches[0];frames.append((m['name'],m['uuid'],pc-int(m['baseAddress'],16)))
 return tuple(frames)
summary={'scope':'Exact module debug-ID plus relative-PC prefix correlations. Controlled operations establish shared native allocator paths, not actual object ownership, leaks, removable/retained/OS bytes or current-release memory. Reference profile is historical. Overlapping operations remain ambiguous; missing samples/coverage do not mean zero allocation.','operations':[],'prefix_comparisons':[]}
for op in probe['operations']:
 summary['operations'].append({'kind':op['kind'],'samples':len(op['profile']['samples']),'chrome_module_identity_match':op['chrome_module_identity_match'],'sample_sizes':sorted({s['size'] for s in op['profile']['samples'] if s['size']>=262144})})
for depth in [4,6,8]:
 index=defaultdict(set)
 for op in probe['operations']:
  for sample in op['profile']['samples']:
   key=prefix(sample,op['profile'],depth)
   if key:index[key].add(op['kind'])
 categories=defaultdict(lambda:{'samples':0,'sample_total_weight_bytes':0,'sample_sizes':set()})
 unmapped=0
 for sample in old['samples']:
  key=prefix(sample,old,depth);operations=index.get(key,set()) if key else set()
  label='|'.join(sorted(operations)) if operations else 'unmatched'
  row=categories[label];row['samples']+=1;row['sample_total_weight_bytes']+=sample['total'];row['sample_sizes'].add(sample['size'])
  if key is None:unmapped+=1
 rows=[{'operations':label,**{k:(sorted(v) if isinstance(v,set) else v) for k,v in row.items()}} for label,row in categories.items()]
 assert sum(row['samples'] for row in rows)==len(old['samples'])
 summary['prefix_comparisons'].append({'depth':depth,'historical_samples':len(old['samples']),'unmapped_prefixes':unmapped,'categories':rows})
 print('depth',depth,[(x['operations'],x['samples']) for x in rows])
(root/'summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
print('Operation coverage',[(x['kind'],x['samples']) for x in summary['operations']])
