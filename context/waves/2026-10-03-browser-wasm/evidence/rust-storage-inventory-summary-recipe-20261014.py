from pathlib import Path
import json,hashlib
root=Path('target/rust-storage-inventory-artifact')
report=Path('target/rust-storage-inventory-AtLCvA/report.json')
r=json.loads(report.read_bytes());catalog=json.loads((root/'catalog.json').read_bytes())
assert r['completed'] and r['provisional_250_mib_target'] is None
assert len(r['packages'])==75 and len(r['imports'])==2 and len(r['warm_query_ms'])==20 and not r['missing_sampled_phases']
entries={p['sha256']:p for p in catalog['packages']}
summary=[];validated=0;unknown=[]
for snapshot in r['rust_storage_snapshots']:
    windows=snapshot['storage']['windows'];load=snapshot['load']
    if load<=75:
        expected=catalog['packages'][max(0,load-8):load]
        assert [w['revision'] for w in windows]==[p['sha256'] for p in expected]
    for window in windows:
        if window['revision'] in entries:
            entry=entries[window['revision']];data=(root/entry['url']).read_bytes()
            assert hashlib.sha256(data).hexdigest()==window['revision']
            package=json.loads(data)
            assert window['source_rows']==[len(package[k]) for k in ['bios','stats','goalies']]
            validated+=1
        else:unknown.append({'load':load,'season':window['season'],'season_type':window['season_type']})
    source_spare=sum(w['source_vec_reserved_bytes']-w['source_vec_used_bytes'] for w in windows)
    stint_spare=sum(w['repository']['stint_vec_capacity_bytes']-w['repository']['stint_vec_used_bytes'] for w in windows)
    summary.append({'load':load,'windows':len(windows),'wasm_linear_memory_bytes':snapshot['wasm_linear_memory_bytes'],'source_vec_spare_bytes':source_spare,'stint_vec_spare_bytes':stint_spare,'combined_vec_spare_bytes':source_spare+stint_spare,'stats_row_payload_bytes':sum(w['repository']['stats_row_payload_bytes'] for w in windows)})
output={'verified':True,'scope':'Partial requested Rust storage at8 instrumented checkpoints, not normal memory acceptance or a complete/additive allocator/OS partition. Source array row counts checked against exact retained bytes when revisions match the catalog. Imported revisions are not relabeled as catalog bytes.','source_windows_checked':validated,'imported_windows_without_catalog_revision_match':unknown,'measurements':summary,'maximum_combined_vec_spare_bytes':max(s['combined_vec_spare_bytes'] for s in summary),'decision':'Do not run a vector shrinking trial: measured source/stint spare storage is at most about1.46 MiB and does not explain the whole-browser budget gap. Focus subsequent investigation outside WASM. No implementation adopted;250 MiB still unmet, Android deferred.'}
Path('target/rust-storage-inventory-summary.json').write_text(json.dumps(output,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'verified':True,'source_windows_checked':validated,'unmatched_imported_windows':len(unknown),'maximum_vec_spare_mib':output['maximum_combined_vec_spare_bytes']/1048576},indent=2))
