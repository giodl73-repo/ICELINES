from pathlib import Path
from concurrent.futures import ThreadPoolExecutor
import json,re,urllib.request,urllib.error,hashlib
profile=Path('target/viewport-runtime-attribution-5klpj9/report.json')
r=json.loads(profile.read_bytes());modules=r['native_profile']['profile']['modules'][:2]
queries=[]
for module in modules:
 assert re.fullmatch(r'[A-Za-z0-9._-]+',module['name']) and re.fullmatch(r'[A-F0-9]{33,40}',module['uuid'])
 for name in [module['name'],module['name'][:-1]+'_','file.ptr']:
  queries.append({'module':module['name'],'uuid':module['uuid'],'representation':name,'url':'https://chromium-browser-symsrv.commondatastorage.googleapis.com/'+module['name']+'/'+module['uuid']+'/'+name})
def probe(query):
 row=dict(query)
 try:
  with urllib.request.urlopen(urllib.request.Request(query['url'],method='GET' if query['representation']=='file.ptr' else 'HEAD'),timeout=15) as response:
   row.update(status=response.status,content_length=response.headers.get('Content-Length'))
   if query['representation']=='file.ptr':
    content=response.read(4097);assert len(content)<=4096
    row.update(pointer_sha256=hashlib.sha256(content).hexdigest(),pointer_bytes=len(content))
 except urllib.error.HTTPError as e:row['status']=e.code
 except Exception as e:row['error_type']=type(e).__name__
 return row
with ThreadPoolExecutor(max_workers=6) as pool:results=list(pool.map(probe,queries))
report={'scope':'Availability probe of exact historical Chromium153.0.8010.12 native-profile module IDs; uncompressed PDB, compressed extension and pointer forms. No browser workload rerun, no mismatched symbols, no OS/library/application modifications.404 means absent at this exact public URL now; not unavailable everywhere or forever.','recorded_browser':r['browser'],'profile_sha256':hashlib.sha256(profile.read_bytes()).hexdigest(),'primary_sources':['https://www.chromium.org/developers/how-tos/debugging-on-windows/','https://learn.microsoft.com/en-us/windows/win32/debug/using-symstore'],'results':results}
Path('target/native-symbol-fallback-probe.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps([{'module':x['module'],'representation':x['representation'],'status':x.get('status'),'error':x.get('error_type')} for x in results],indent=2))
