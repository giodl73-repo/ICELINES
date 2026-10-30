// Compare actual distributed WASM with full native-loader expectations.
import assert from 'node:assert/strict';
import {readFile,mkdir} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import init,{BrowserEngine} from '../icelines-browser/dist/pkg/icelines_wasm.js';
const root = fileURLToPath(new URL('../',import.meta.url));
const output = new URL('../target/browser-native-goldens.json',import.meta.url);
const temporary = fileURLToPath(new URL('../target/browser-test-tmp/',import.meta.url));
await mkdir(temporary,{recursive:true});
const generated = spawnSync('cargo',['run','-p','icelines-fetch','--example','browser_parity_goldens','--locked',
  ...(process.argv.includes('--offline') ? ['--offline'] : []),'--',fileURLToPath(output)],
  {cwd:root,stdio:'inherit',env:{...process.env,TMP:temporary,TEMP:temporary,TMPDIR:temporary}});
if (generated.error || generated.status !== 0) throw generated.error ?? new Error('Native expectation generation failed');
const dist = new URL('../icelines-browser/dist/',import.meta.url);
const cases = JSON.parse(await readFile(output,'utf8'));
const catalog = JSON.parse(await readFile(new URL('catalog.json',dist),'utf8'));
await init({module_or_path:await readFile(new URL('pkg/icelines_wasm_bg.wasm',dist))});
let comparisons = 0; let comparedRows = 0; let emptyCases = 0;
for (const kind of ['regular','playoff']) {
  const entry = catalog.packages.find(x=>x.season===20242025&&x.season_type===kind);
  assert.ok(entry,`Missing parity package: ${kind}`);
  const engine = new BrowserEngine();
  try {
    assert.equal(engine.load_package(await readFile(new URL(entry.url,dist))),entry.sha256);
    for (const expected of cases.filter(x=>x.season_type===kind)) {
      const result = JSON.parse(engine.query(JSON.stringify(expected.request)));
      const context = `${kind} ${JSON.stringify(expected.request)}`;
      assert.equal(result.season,expected.season,context);
      assert.equal(result.season_type,expected.season_type,context);
      assert.equal(result.minimum_games,expected.minimum_games,context);
      assert.deepEqual(result.rows,expected.rows,context);
      comparedRows += result.rows.length;
      if (!result.rows.length) emptyCases++;
      comparisons++;
    }
  } finally {engine.free();}
}
assert.equal(comparisons,270);
assert.ok(comparedRows > 10000, 'Parity corpus must exercise populated results');
assert.ok(emptyCases > 0, 'Parity corpus must exercise empty results');
console.log(`Verified ${comparisons} native-loader/distributed-WASM cases (${comparedRows} full rows, ${emptyCases} empty results): regular/playoff, nine sorts, five floors and three filters per player kind.`);
