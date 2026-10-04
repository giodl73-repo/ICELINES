// Shared local/CI build entry point. Generated artifacts stay under workspace roots.
import { spawnSync } from 'node:child_process';
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { createHash } from 'node:crypto';
const root = fileURLToPath(new URL('../', import.meta.url));
const config = JSON.parse(await readFile(new URL('../icelines-browser/toolchain.json', import.meta.url), 'utf8'));
// The repository's native `stable` override must not select a different compiler
// from the browser toolchain for which CI installed the WASM target.
const buildEnvironment = { ...process.env, RUSTUP_TOOLCHAIN: config.rust };
const offline = process.argv.includes('--offline') ? ['--offline'] : [];
const check = process.argv.includes('--check');
const temporary = fileURLToPath(new URL('../target/browser-test-tmp/', import.meta.url));
await mkdir(temporary, { recursive: true });
function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: 'inherit',
    env: { ...buildEnvironment, TMP: temporary, TEMP: temporary, TMPDIR: temporary } });
  if (result.error || result.status !== 0) throw result.error ?? new Error(`${command} failed (${result.status})`);
}
function capture(command, args) {
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', env: buildEnvironment });
  if (result.error || result.status !== 0) throw result.error ?? new Error(`${command} failed (${result.status})`);
  return result.stdout.trim();
}
const crate = await readFile(new URL('../icelines-wasm/Cargo.toml', import.meta.url), 'utf8');
if (!crate.includes(`wasm-bindgen = "=${config.wasm_bindgen}"`)) throw new Error('Binding crate/CLI pin mismatch');
const cli = fileURLToPath(new URL('../target/browser-tools/bin/wasm-bindgen' + (process.platform === 'win32' ? '.exe' : ''), import.meta.url));
let cliVersion;
try { cliVersion = capture(cli, ['--version']); } catch { /* Install into workspace tool output below. */ }
if (cliVersion !== `wasm-bindgen ${config.wasm_bindgen}`) {
  run('cargo', ['install', 'wasm-bindgen-cli', '--version', config.wasm_bindgen, '--locked', '--root', 'target/browser-tools', '--target-dir', 'target/browser-tools-build', ...offline]);
}
if (capture(cli, ['--version']) !== `wasm-bindgen ${config.wasm_bindgen}`) throw new Error('Wrong binding CLI version');
run('cargo', ['build', '-p', 'icelines-wasm', '--target', 'wasm32-unknown-unknown', '--profile', 'browser-release', '--target-dir', 'target/browser', '--locked', ...offline]);
run(cli, ['target/browser/wasm32-unknown-unknown/browser-release/icelines_wasm.wasm', '--target', 'web', '--out-dir', 'icelines-browser/public/pkg', '--out-name', 'icelines_wasm']);
run('python', ['scripts/build-browser-catalog.py']);
const info = { schema_version: 1, commit: capture('git', ['rev-parse', 'HEAD']),
  working_tree_dirty: capture('git', ['status', '--porcelain', '--untracked-files=normal']).length > 0,
  tools: { rust: capture('rustc', ['--version']), node: process.version, python: capture('python', ['--version']), wasm_bindgen: capture(cli, ['--version']) },
  wasm_sha256: createHash('sha256').update(await readFile(new URL('../icelines-browser/public/pkg/icelines_wasm_bg.wasm', import.meta.url))).digest('hex') };
await writeFile(new URL('../icelines-browser/public/build-info.json', import.meta.url), JSON.stringify(info, null, 2) + '\n');
run(process.execPath, ['icelines-browser/build.mjs']);
run(process.execPath, ['scripts/verify-browser-dist.mjs']);
if (check) {
  run(process.execPath, ['scripts/verify-browser-native-parity.mjs', ...offline]);
  run('cargo', ['test', '-p', 'icelines-sources', '--lib', 'nhl::schedule', '--locked', ...offline]);
  run('cargo', ['test', '-p', 'icelines-core', '--lib', 'stats_catalog::tests', '--locked', ...offline]);
  run('cargo', ['test', '-p', 'icelines-data', '-p', 'icelines-wasm', '--locked', ...offline]);
  run('cargo', ['test', '-p', 'icelines-fetch', '--test', 'stats_loader', '--test', 'browser_native_parity', '--locked', ...offline]);
  run('cargo', ['test', '-p', 'icelines-web', '--lib', 'handlers::leaders::tests', '--locked', ...offline]);
  run('python', ['-m', 'unittest', 'discover', '-s', 'scripts/tests', '-p', 'test_browser_catalog.py']);
  run('python', ['-m', 'unittest', 'discover', '-s', 'scripts/tests', '-p', 'test_browser_pages.py']);
  run('python', ['-m', 'unittest', 'discover', '-s', 'scripts/tests', '-p', 'test_browser_artifact.py']);
  // Explicit file paths work across Windows and Linux shells without glob expansion.
  const { readdir } = await import('node:fs/promises');
  const tests = (await readdir(new URL('../icelines-browser/tests/', import.meta.url))).filter(name => name.endsWith('.test.mjs')).sort().map(name => 'icelines-browser/tests/' + name);
  run(process.execPath, ['--test', ...tests]);
}
console.log('Browser distribution built and verified; publication is a separate workflow action.');
