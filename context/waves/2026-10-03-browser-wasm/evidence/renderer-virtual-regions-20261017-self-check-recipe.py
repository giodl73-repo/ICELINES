import ctypes,datetime,importlib.util,json,subprocess,sys
from pathlib import Path
spec=importlib.util.spec_from_file_location('regions',Path('target/renderer-virtual-regions.py'));regions=importlib.util.module_from_spec(spec);spec.loader.exec_module(regions)
child=subprocess.Popen([sys.executable,'-c','import time; data=bytearray(8*1024*1024); print("ready",flush=True); time.sleep(30)'],stdout=subprocess.PIPE,text=True,creationflags=subprocess.CREATE_NO_WINDOW)
try:
    assert child.stdout.readline().strip()=='ready'
    handle=regions.k.OpenProcess(0x0400,False,child.pid);assert handle
    try:created=regions.creation(handle)
    finally:regions.k.CloseHandle(handle)
    try:regions.inspect(child.pid,(created+datetime.timedelta(days=1)).isoformat())
    except ValueError as e:assert 'creation time mismatch' in str(e)
    else:raise AssertionError('Mismatched process identity accepted')
    r=regions.inspect(child.pid,created.isoformat())
    assert r['completed'] and r['regions_queried']>10 and r['private_committed_region_bytes']>=8*1024*1024
    assert r['committed_by_type_and_protection'].get('MEM_PRIVATE:READWRITE',0)>=8*1024*1024
    Path('target/renderer-virtual-regions-self-check.json').write_text(json.dumps({'verified':True,'identity_guard_rejects_wrong_creation':True,'owned_child_known_8_mib_allocation_detected':True,'result':r},indent=2)+'\n',encoding='utf-8')
    print('Owned8 MiB child detected; process identity mismatch rejected; complete region walk')
finally:child.terminate();child.wait(timeout=10)
