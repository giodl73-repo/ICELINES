import collections,ctypes,datetime,json,os,sys,time
from pathlib import Path
from ctypes import wintypes as w

assert os.name=='nt' and ctypes.sizeof(ctypes.c_void_p)==8
k=ctypes.WinDLL('kernel32',use_last_error=True)
class MBI(ctypes.Structure):
    _fields_=[('BaseAddress',ctypes.c_void_p),('AllocationBase',ctypes.c_void_p),('AllocationProtect',w.DWORD),('PartitionId',w.WORD),('RegionSize',ctypes.c_size_t),('State',w.DWORD),('Protect',w.DWORD),('Type',w.DWORD)]
class SI(ctypes.Structure):
    _fields_=[('architecture',w.DWORD),('page_size',w.DWORD),('minimum',ctypes.c_void_p),('maximum',ctypes.c_void_p),('mask',ctypes.c_size_t),('processors',w.DWORD),('processor_type',w.DWORD),('granularity',w.DWORD),('level',w.WORD),('revision',w.WORD)]
assert ctypes.sizeof(MBI)==48 and ctypes.sizeof(SI)==48
k.OpenProcess.argtypes=[w.DWORD,w.BOOL,w.DWORD];k.OpenProcess.restype=w.HANDLE
k.CloseHandle.argtypes=[w.HANDLE];k.CloseHandle.restype=w.BOOL
k.GetProcessTimes.argtypes=[w.HANDLE]+[ctypes.POINTER(w.FILETIME)]*4;k.GetProcessTimes.restype=w.BOOL
k.VirtualQueryEx.argtypes=[w.HANDLE,ctypes.c_void_p,ctypes.POINTER(MBI),ctypes.c_size_t];k.VirtualQueryEx.restype=ctypes.c_size_t
k.GetNativeSystemInfo.argtypes=[ctypes.POINTER(SI)];k.GetNativeSystemInfo.restype=None
def creation(handle):
    values=[w.FILETIME() for _ in range(4)]
    if not k.GetProcessTimes(handle,*[ctypes.byref(x) for x in values]):raise ctypes.WinError(ctypes.get_last_error())
    ticks=(values[0].dwHighDateTime<<32)|values[0].dwLowDateTime
    return datetime.datetime(1601,1,1,tzinfo=datetime.timezone.utc)+datetime.timedelta(microseconds=ticks//10)
def inspect(pid,expected_created):
    handle=k.OpenProcess(0x0400,False,pid)  # PROCESS_QUERY_INFORMATION only.
    if not handle:raise ctypes.WinError(ctypes.get_last_error())
    try:
        actual=creation(handle)
        expected=datetime.datetime.fromisoformat(expected_created.replace('Z','+00:00'))
        if abs((actual-expected).total_seconds())>0.001:raise ValueError('Process creation time mismatch; refusing region query')
        system=SI();k.GetNativeSystemInfo(ctypes.byref(system));address=0;count=0;groups=collections.defaultdict(int);allocations={};started=time.monotonic()
        states=collections.defaultdict(int)
        while address<=system.maximum:
            if time.monotonic()-started>20:raise TimeoutError('Region walk exceeded20 seconds')
            region=MBI();size=k.VirtualQueryEx(handle,ctypes.c_void_p(address),ctypes.byref(region),ctypes.sizeof(region))
            if size!=ctypes.sizeof(region):raise ctypes.WinError(ctypes.get_last_error())
            base=region.BaseAddress or 0;end=base+region.RegionSize
            if not base<=address<end:raise ValueError('Non-advancing region walk')
            count+=1;states[hex(region.State)]+=region.RegionSize
            if region.State==0x1000:
                kind={0x20000:'MEM_PRIVATE',0x40000:'MEM_MAPPED',0x1000000:'MEM_IMAGE'}.get(region.Type,hex(region.Type))
                protection=region.Protect&0xff
                name={1:'NOACCESS',2:'READONLY',4:'READWRITE',8:'WRITECOPY',16:'EXECUTE',32:'EXECUTE_READ',64:'EXECUTE_READWRITE',128:'EXECUTE_WRITECOPY'}.get(protection,hex(protection))
                groups[kind+':'+name]+=region.RegionSize
                allocation=allocations.setdefault(region.AllocationBase,{'committed_bytes':0,'private_committed_bytes':0,'executable_committed_bytes':0,'regions':0})
                allocation['committed_bytes']+=region.RegionSize;allocation['regions']+=1
                if region.Type==0x20000:allocation['private_committed_bytes']+=region.RegionSize
                if protection in [16,32,64,128]:allocation['executable_committed_bytes']+=region.RegionSize
            address=end
        assert creation(handle)==actual
        private=sum(v for key,v in groups.items() if key.startswith('MEM_PRIVATE:'))
        return {'pid':pid,'created_at_utc':actual.isoformat(),'completed':True,'regions_queried':count,'walk_ms':(time.monotonic()-started)*1000,'state_bytes':dict(states),'committed_by_type_and_protection':dict(groups),'private_committed_region_bytes':private,'largest_committed_allocations':sorted(allocations.values(),key=lambda x:x['committed_bytes'],reverse=True)[:20],'scope':'VirtualQueryEx metadata only; no memory contents, page touching, GC or working-set intervention. No addresses emitted. MEM_IMAGE/MEM_MAPPED can include private copy-on-write pages: MEM_PRIVATE committed sum is not complete OS private usage or physical RAM. Region walk is not an atomic snapshot.'}
    finally:k.CloseHandle(handle)
def main():
    inventory=json.loads(Path(sys.argv[1]).read_text(encoding='utf-8'));results=[]
    assert inventory['root_id'] in {p['id'] for p in inventory['processes']}
    for p in inventory['processes']:
        if p['kind']!='renderer':continue
        try:results.append(inspect(p['id'],p['created_at_utc']))
        except Exception as error:results.append({'pid':p['id'],'completed':False,'error':str(error)})
    assert results
    Path(sys.argv[2]).write_text(json.dumps({'scope':'Post-sampler/pre-trace renderer region census; source inventory contains owned descendants only. Partial/failed walks are retained. No peak-memory or native-allocation ownership claim.','renderers':results},indent=2)+'\n',encoding='utf-8')
if __name__=='__main__':main()
