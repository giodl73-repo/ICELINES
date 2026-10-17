import collections,datetime,json,sys
from pathlib import Path
root=Path(sys.argv[1]);r=json.loads((root/'report.json').read_text(encoding='utf-8'));samples=json.loads((root/'samples.json').read_text(encoding='utf-8'))
assert r['completed'] and r['provisional_250_mib_target'] is None and len(r['packages'])==75 and [x['rows'] for x in r['imports']]==[905,332] and len(r['warm_query_ms'])==20 and not r['missing_sampled_phases']
inventory=r['passive_process_inventory'];by_id={p['id']:p for p in inventory['processes']};assert len(by_id)==len(inventory['processes'])
def time(s):return datetime.datetime.fromisoformat(s.replace('Z','+00:00'))
started=time(inventory['root_created_at_utc'])
mapping=r['target_process_mapping'];boundary=r['post_sampling_mapping']
assert mapping['all_mapped'] and boundary['sampler_exit']==0
assert boundary['samples_before']==boundary['samples_after']==len(samples)
assert time(boundary['started_at_utc'])>=time(inventory['observed_at_utc'])
import hashlib
assert hashlib.sha256((root/mapping['trace']['file']).read_bytes()).hexdigest()==mapping['trace']['sha256']
targets_by_pid={}
for target in mapping['targets']:
    assert target['mapped'] and target['marker_executed']
    targets_by_pid.setdefault(target['renderer_pid'],[]).append({'type':target['type'],'url':target['url']})

def classify(sample):
    earliest=started+datetime.timedelta(milliseconds=sample['elapsed_ms'])
    seen=set();groups=collections.defaultdict(int);processes=[]
    for p in sample['processes']:
        assert p['id'] not in seen;seen.add(p['id'])
        role=by_id.get(p['id']);reason=None
        if role is None:reason='PID no longer in post-sampling live inventory'
        elif time(role['created_at_utc'])>earliest:reason='Current PID creation is later than conservative earliest sample time'
        label='unmapped-owned-process' if reason else role['kind']+(':'+role['utility_subtype'] if role['utility_subtype'] else '')
        if not reason and role['kind']=='renderer':
            kinds={t['type'] for t in targets_by_pid.get(p['id'],[])}
            if 'page' in kinds and 'browser_ui' in kinds:label='renderer:mixed-late-targets'
            elif 'page' in kinds:label='renderer:application-page-associated'
            elif 'browser_ui' in kinds:label='renderer:browser-ui-associated'
            elif kinds:label='renderer:other-late-targets'
            else:label='renderer:unmapped-to-late-target'
        groups[label]+=p['private_bytes']
        processes.append({'pid':p['id'],'private_bytes':p['private_bytes'],'role':label,'unmapped_reason':reason})
    total=sum(p['private_bytes'] for p in sample['processes']);assert sum(groups.values())==total
    return {'phase':sample['phase'],'elapsed_ms':sample['elapsed_ms'],'total_private_bytes':total,'groups':dict(groups),'processes':processes}
peak=max(samples,key=lambda s:sum(p['private_bytes'] for p in s['processes']))
assert sum(p['private_bytes'] for p in peak['processes'])==r['sampled_peak_private_bytes']
result={'scope':'OS PID-kind classification using Windows command-line role tokens collected only after original sampler terminated. No tracing/heap/extra target observer or added flags. No app-versus-browser-UI renderer attribution. Exited/reused/uncertain PIDs stay in totals as unmapped. All groups form a disjoint sum of original private-byte counters, not physical RAM or removable bytes.','artifact':r['artifact'],'acceptance':None,'peak':classify(peak),'last':classify(samples[-1]),'late_inventory':inventory,'tracing_service_present_in_inventory':any(p['utility_subtype']=='tracing.mojom.TracingService' for p in inventory['processes'])}
result['scope']='Full original measurements precede all added trace/target observations. Surviving process identity guarded by creation time. Late target association is not historical object ownership or a pure app-code allocation partition; include automation/shared contexts, unknown targets and all other owned processes. Do not exclude costs or baseline-subtract acceptance.'
result['post_sampling_mapping']=boundary
result['late_target_associations']=targets_by_pid
(root/'postmap-process-summary.json').write_text(json.dumps(result,indent=2)+'\n',encoding='utf-8')
print('Peak phase:',result['peak']['phase'],'MiB:',result['peak']['total_private_bytes']/1048576)
print('Peak groups MiB:',{k:round(v/1048576,3) for k,v in result['peak']['groups'].items()})
print('Tracing utility in pre-mapping inventory:',result['tracing_service_present_in_inventory'])
