import collections,datetime,json,sys
from pathlib import Path
root=Path(sys.argv[1]);r=json.loads((root/'report.json').read_text(encoding='utf-8'));samples=json.loads((root/'samples.json').read_text(encoding='utf-8'))
assert r['completed'] and r['provisional_250_mib_target'] is None and len(r['packages'])==75 and [x['rows'] for x in r['imports']]==[905,332] and len(r['warm_query_ms'])==20 and not r['missing_sampled_phases']
inventory=r['passive_process_inventory'];by_id={p['id']:p for p in inventory['processes']};assert len(by_id)==len(inventory['processes'])
def time(s):return datetime.datetime.fromisoformat(s.replace('Z','+00:00'))
started=time(inventory['root_created_at_utc'])
def classify(sample):
    earliest=started+datetime.timedelta(milliseconds=sample['elapsed_ms'])
    seen=set();groups=collections.defaultdict(int);processes=[]
    for p in sample['processes']:
        assert p['id'] not in seen;seen.add(p['id'])
        role=by_id.get(p['id']);reason=None
        if role is None:reason='PID no longer in post-sampling live inventory'
        elif time(role['created_at_utc'])>earliest:reason='Current PID creation is later than conservative earliest sample time'
        label='unmapped-owned-process' if reason else role['kind']+(':'+role['utility_subtype'] if role['utility_subtype'] else '')
        groups[label]+=p['private_bytes']
        processes.append({'pid':p['id'],'private_bytes':p['private_bytes'],'role':label,'unmapped_reason':reason})
    total=sum(p['private_bytes'] for p in sample['processes']);assert sum(groups.values())==total
    return {'phase':sample['phase'],'elapsed_ms':sample['elapsed_ms'],'total_private_bytes':total,'groups':dict(groups),'processes':processes}
peak=max(samples,key=lambda s:sum(p['private_bytes'] for p in s['processes']))
assert sum(p['private_bytes'] for p in peak['processes'])==r['sampled_peak_private_bytes']
result={'scope':'OS PID-kind classification using Windows command-line role tokens collected only after original sampler terminated. No tracing/heap/extra target observer or added flags. No app-versus-browser-UI renderer attribution. Exited/reused/uncertain PIDs stay in totals as unmapped. All groups form a disjoint sum of original private-byte counters, not physical RAM or removable bytes.','artifact':r['artifact'],'acceptance':None,'peak':classify(peak),'last':classify(samples[-1]),'late_inventory':inventory,'tracing_service_present_in_inventory':any(p['utility_subtype']=='tracing.mojom.TracingService' for p in inventory['processes'])}
(root/'process-role-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print('Peak phase:',result['peak']['phase'],'MiB:',result['peak']['total_private_bytes']/1048576)
print('Peak groups MiB:',{k:round(v/1048576,3) for k,v in result['peak']['groups'].items()})
print('Tracing utility present without explicit trace request:',result['tracing_service_present_in_inventory'])
