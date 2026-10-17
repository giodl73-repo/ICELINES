from pathlib import Path
s=Path('target/analyze-passive-process-inventory.py').read_text(encoding='utf-8')
needle="started=time(inventory['root_created_at_utc'])"
assert s.count(needle)==1
s=s.replace(needle,needle+"""
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
""")
needle="        groups[label]+=p['private_bytes']"
assert s.count(needle)==1
s=s.replace(needle,"""        if not reason and role['kind']=='renderer':
            kinds={t['type'] for t in targets_by_pid.get(p['id'],[])}
            if 'page' in kinds and 'browser_ui' in kinds:label='renderer:mixed-late-targets'
            elif 'page' in kinds:label='renderer:application-page-associated'
            elif 'browser_ui' in kinds:label='renderer:browser-ui-associated'
            elif kinds:label='renderer:other-late-targets'
            else:label='renderer:unmapped-to-late-target'
"""+needle)
needle="(root/'process-role-summary.json').write_text"
assert s.count(needle)==1
s=s.replace(needle,"""result['scope']='Full original measurements precede all added trace/target observations. Surviving process identity guarded by creation time. Late target association is not historical object ownership or a pure app-code allocation partition; include automation/shared contexts, unknown targets and all other owned processes. Do not exclude costs or baseline-subtract acceptance.'
result['post_sampling_mapping']=boundary
result['late_target_associations']=targets_by_pid
(root/'postmap-process-summary.json').write_text""")
s=s.replace('Tracing utility present without explicit trace request:','Tracing utility in pre-mapping inventory:')
Path('target/analyze-postsample-renderer-mapping.py').write_text(s,encoding='utf-8')
print('Prepared exact-sample grouping with guarded late target associations')
