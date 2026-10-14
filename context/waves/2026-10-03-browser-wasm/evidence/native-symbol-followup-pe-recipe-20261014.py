from pathlib import Path
import json,struct,uuid,hashlib,mmap,bisect
folder=Path('C:/Users/giodl/AppData/Local/ms-playwright/chromium-1243/chrome-win64')
profile=Path('target/viewport-runtime-attribution-5klpj9/report.json')
r=json.loads(profile.read_bytes());p=r['native_profile']['profile']
report={'scope':'Read-only exact PE debug-ID/image-size verification and export/unwind boundary inspection for historical native profile. No mismatched symbol use. Unnamed function boundaries are not function identities or allocation ownership; nearest export offsets are deliberately not used.','profile_sha256':hashlib.sha256(profile.read_bytes()).hexdigest(),'modules':[]}
for module in p['modules'][:2]:
 path=folder/module['name'].removesuffix('.pdb')
 with path.open('rb') as stream,mmap.mmap(stream.fileno(),0,access=mmap.ACCESS_READ) as data:
  u16=lambda o:struct.unpack_from('<H',data,o)[0]
  u32=lambda o:struct.unpack_from('<I',data,o)[0]
  assert data[:2]==b'MZ';pe=u32(60);assert data[pe:pe+4]==b'PE\0\0'
  assert u16(pe+4)==0x8664, 'Unwind inspection requires AMD64 PE format'
  sections_count=u16(pe+6);optional_size=u16(pe+20);optional=pe+24;assert u16(optional)==0x20b
  directories=optional+112;image_size=u32(optional+56);header_size=u32(optional+60)
  assert image_size==module['size']
  sections=[]
  for index in range(sections_count):
   off=optional+optional_size+index*40
   sections.append((u32(off+12),u32(off+8),u32(off+16),u32(off+20)))
  def offset(rva):
   if rva<header_size:return rva
   for address,virtual_size,raw_size,pointer in sections:
    if address<=rva<address+max(virtual_size,raw_size):
     assert rva-address<raw_size
     return pointer+rva-address
   raise ValueError('RVA outside file-backed sections')
  def cstring(off):
   end=data.find(b'\0',off,off+4096);assert end>=0
   return data[off:end].decode('utf-8')
  debug_rva,debug_bytes=struct.unpack_from('<II',data,directories+6*8);debug_ids=[]
  for i in range(debug_bytes//28):
   off=offset(debug_rva)+i*28
   _,_,_,_,kind,size,_,pointer=struct.unpack_from('<IIHHIIII',data,off)
   if kind==2 and data[pointer:pointer+4]==b'RSDS':
    identifier=uuid.UUID(bytes_le=data[pointer+4:pointer+20]).hex.upper()+format(u32(pointer+20),'X')
    debug_ids.append({'uuid':identifier,'pdb':cstring(pointer+24)})
  assert any(x['uuid']==module['uuid'] for x in debug_ids)
  export_rva,export_bytes=struct.unpack_from('<II',data,directories);exports=[]
  if export_rva:
   _,_,_,_,_,base,functions,names,function_table,name_table,ordinal_table=struct.unpack_from('<IIHHIIIIIII',data,offset(export_rva))
   for i in range(names):
    ordinal=u16(offset(ordinal_table)+i*2);assert ordinal<functions
    address=u32(offset(function_table)+ordinal*4)
    exports.append({'name':cstring(offset(u32(offset(name_table)+i*4))),'rva':address,'forwarded':export_rva<=address<export_rva+export_bytes})
  exception_rva,exception_bytes=struct.unpack_from('<II',data,directories+3*8);assert exception_bytes%12==0
  runtime=[struct.unpack_from('<III',data,offset(exception_rva)+i*12) for i in range(exception_bytes//12)] if exception_rva else []
  starts=[x[0] for x in runtime];assert starts==sorted(starts)
  by_start={}
  for item in exports:
   if not item['forwarded']:by_start.setdefault(item['rva'],[]).append(item['name'])
  base_address=int(module['baseAddress'],16);frames=[]
  addresses=sorted({int(frame,16) for sample in p['samples'] for frame in sample['stack'] if frame.startswith('0x')})
  for address in addresses:
   if not base_address<=address<base_address+image_size:continue
   relative=address-base_address;index=bisect.bisect_right(starts,relative)-1
   entry=runtime[index] if index>=0 and relative<runtime[index][1] else None
   frames.append({'rva':relative,'runtime_function_range':list(entry[:2]) if entry else None,'export_at_function_start':by_start.get(entry[0],[]) if entry else []})
  row={'name':path.name,'sha256':hashlib.sha256(data).hexdigest(),'image_size':image_size,'expected_uuid':module['uuid'],'debug_ids':debug_ids,'exact_match':True,'named_exports':exports,'runtime_function_count':len(runtime),'sampled_frame_count':len(frames),'frames_in_unwind_ranges':sum(x['runtime_function_range'] is not None for x in frames),'frames_with_exported_function_start':sum(bool(x['export_at_function_start']) for x in frames),'frames':frames}
  report['modules'].append(row)
Path('target/native-profile-pe-inspection.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps([{k:x[k] for k in ['name','exact_match','runtime_function_count','sampled_frame_count','frames_in_unwind_ranges','frames_with_exported_function_start']} for x in report['modules']],indent=2))
