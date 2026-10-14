from pathlib import Path
from collections import defaultdict
import json,hashlib
root=Path('target/js-context-attribution-noOPvR')
r=json.loads((root/'report.json').read_bytes());artifact=Path('target/wasm-validation-75e6ba97')
assert r['completed'] and r['provisional_250_mib_target'] is None
assert len(r['packages'])==75 and len(r['imports'])==2 and len(r['warm_query_ms'])==20 and not r['missing_sampled_phases']
summary={'scope':'Sampled cumulative JS churn, not live memory/leaks/native/OS totals. Direct selfSize counted once. Native/unmapped nodes inherit nearest known script context as a stack heuristic. Named utility-world attribution is context evidence; source markers are heuristics. Only actual source SHA matching retained module bytes is labeled verified app module. Unknown nodes remain separate. No budget subtraction or acceptance claim.','contexts':{}}
utility_match=json.loads((root/'utility-source-match.json').read_bytes())
utility_hashes={s['sha256'] for s in utility_match['matches']}
for kind,profile in r['js_profiles'].items():
 contexts={c['id']:c for c in r['context_metadata'][kind]['contexts']}
 scripts={s['scriptId']:s for s in r['context_metadata'][kind]['scripts']}
 sources={s['scriptId']:s for s in r['sampled_script_sources'][kind]}
 buckets=defaultdict(int);locations=defaultdict(int);nodes=set();verified_modules={}
 def classify(script_id):
  s=scripts.get(script_id)
  if not s:return None
  c=contexts.get(s['executionContextId'],{})
  if '__playwright_utility_world_' in c.get('name',''):return 'named_playwright_utility_context'
  url=s.get('url','');source=sources.get(script_id,{})
  if '/ICELINES/workbench/' in url:
   relative=url.split('/ICELINES/workbench/',1)[1];path=artifact/relative
   if path.is_file() and source.get('sha256')==hashlib.sha256(path.read_bytes()).hexdigest():
    verified_modules[relative]=source['sha256'];return 'verified_application_module'
  if source.get('sha256') in utility_hashes:return 'verified_playwright_utility_script'
  markers=source.get('source_markers',{})
  if any(markers.values()):return 'automation_source_marker_heuristic'
  if c.get('auxData',{}).get('isDefault') is True:return 'default_context_unclassified_source'
  if c:return 'known_other_context'
  return 'script_with_unmapped_context'
 def walk(node,parent_bucket=None):
  assert node['id'] not in nodes;nodes.add(node['id'])
  f=node['callFrame'];own=classify(f['scriptId']);bucket=own or parent_bucket or 'unmapped_internal'
  buckets[bucket]+=node['selfSize'];locations[(bucket,f['url'],f['functionName'],f['lineNumber']+1)]+=node['selfSize']
  for child in node.get('children',[]):walk(child,bucket)
 walk(profile['head'])
 assert all(s['nodeId'] in nodes for s in profile['samples'])
 total=sum(buckets.values())
 row={'node_count':len(nodes),'sample_count':len(profile['samples']),'summed_node_self_size_bytes':total,'buckets':[{'bucket':k,'sampled_self_size_bytes':v,'fraction':v/total} for k,v in sorted(buckets.items(),key=lambda p:p[1],reverse=True)],'verified_modules':verified_modules,'sampled_sources_unavailable':sum('error' in s for s in sources.values()),'top_locations':[{'bucket':key[0],'url':key[1],'function':key[2],'line':key[3],'sampled_self_size_bytes':value} for key,value in sorted(locations.items(),key=lambda p:p[1],reverse=True)[:25]]}
 summary['contexts'][kind]=row
 print(kind,'MiB',round(total/1048576,3));print(json.dumps(row['buckets'],indent=2))
(root/'context-summary.json').write_text(json.dumps(summary,indent=2)+'\n',encoding='utf-8')
