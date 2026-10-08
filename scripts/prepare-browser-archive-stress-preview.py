from pathlib import Path
import argparse, importlib.util, json, shutil, gzip, tarfile
parser = argparse.ArgumentParser(description='Prepare local browser archive-limit checks from a verified retained artifact.')
parser.add_argument('artifact', type=Path)
parser.add_argument('commit')
parser.add_argument('shell_sha256')
parser.add_argument('output', type=Path)
args = parser.parse_args()
artifact, out = args.artifact.resolve(), args.output.resolve()
if out.exists() or out == artifact or out.is_relative_to(artifact):
    raise ValueError('Output must be a new directory separate from the artifact')
spec = importlib.util.spec_from_file_location('browser_artifact', Path(__file__).with_name('verify-browser-artifact.py'))
verifier = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verifier)
identity = verifier.verify(artifact, args.commit, args.shell_sha256)
shutil.copytree(artifact, out)
(out/'stress-identity.json').write_text(json.dumps(identity), encoding='utf-8')
with gzip.open(out/'expanded-over-limit.tar.gz','wb') as stream:
    for _ in range(101): stream.write(bytes(1024*1024))
header = tarfile.TarInfo('bios.json'); header.size = 33*1024*1024
with gzip.open(out/'file-over-limit.tar.gz','wb') as stream: stream.write(header.tobuf(format=tarfile.USTAR_FORMAT))
(out/'compressed-over-limit.tar.gz').write_bytes(bytes(25*1024*1024+1))
(out/'stress.html').write_text('''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>IceLines archive boundary checks</title><link rel="stylesheet" href="./style.css"><main><h1>Archive boundary checks</h1><p>Local production client and worker. Deliberately invalid public synthetic inputs. Does not test file picking, persistence, physical phones or total browser peak memory.</p><button id="run">Run archive stress checks</button><p id="status" role="status">Ready.</p><pre id="report" style="white-space:pre-wrap;overflow-wrap:anywhere"></pre></main><script type="module" src="./stress.js"></script></html>''',encoding='utf-8')
(out/'stress.js').write_text('''import {EngineClient} from './src/engine-client.js';
const button=document.getElementById('run'),status=document.getElementById('status'),node=document.getElementById('report');
button.onclick=async()=>{button.disabled=true;const report={measured_at:new Date().toISOString(),user_agent:navigator.userAgent,samples:[],note:'Actual local browser worker rejection timing and data preservation. Excludes file picker, total peak memory and mobile hardware.'};let worker;
try {report.artifact_identity=await (await fetch('./stress-identity.json',{cache:'no-store'})).json();report.build_info=await (await fetch('./build-info.json',{cache:'no-store'})).json();if(report.build_info.commit!==report.artifact_identity.commit||report.build_info.working_tree_dirty!==false)throw Error('Fixture identity mismatch');const client=new EngineClient(message=>status.textContent=message,()=>{worker=new Worker('./src/worker.js',{type:'module'});return worker;});
const catalog=await (await fetch('./catalog.json')).json();const entry=catalog.packages.find(e=>e.season===20232024&&e.season_type==='regular');
const baseline=await (await fetch('./'+entry.url)).arrayBuffer();const loaded=await client.request('load',baseline);const query={filter:'p>=100',sort:'points',goalies:false,minimum_games:0,today:'2026-10-06'};const before=await client.request('query',query);if(before.rows.length!==9||loaded.revision!==entry.sha256)throw Error('Baseline mismatch');
for(const [file,expected] of [['expanded-over-limit.tar.gz','Expanded archive exceeds 100 MiB'],['file-over-limit.tar.gz','Expanded archive file exceeds limits'],['compressed-over-limit.tar.gz','Compressed archive exceeds 25 MiB']]){
status.textContent='Testing '+file;const bytes=await (await fetch('./'+file,{cache:'no-store'})).arrayBuffer();const start=performance.now();let error;
try{await client.request('importArchive',{bytes,filename:'data-20232024.tar.gz',seasonType:'regular'});}catch(failure){error=String(failure);}
const elapsed=performance.now()-start;if(!error?.includes(expected))throw Error('Unexpected rejection: '+error);const after=await client.request('query',query);if(JSON.stringify(after)!==JSON.stringify(before))throw Error('Failed import changed active data');report.samples.push({file,compressed_bytes:bytes.byteLength,rejection_ms:elapsed,error,previous_query_preserved:true,rows:after.rows.length});}
report.passed=true;status.textContent='Passed: 3 archive limits rejected; previous 9-player query preserved after each.';
}catch(error){report.error=String(error);status.textContent='Failed: '+error;}finally{worker?.terminate();node.textContent=JSON.stringify(report,null,2);}};
''',encoding='utf-8')
print(out)