import test from 'node:test';
import assert from 'node:assert/strict';
import { gzipSync } from 'node:zlib';
import { importArchive, readTarFiles, archivePackage, MAX_ARCHIVE_BYTES } from '../dist/src/archive.js';

function header(name, size, type = '0') {
  const block = Buffer.alloc(512); block.write(name, 0, 100);
  block.write(size.toString(8).padStart(11, '0') + '\0', 124, 12);
  block.fill(32, 148, 156); block.write(type, 156, 1); block.write('ustar\0', 257, 6);
  const checksum = block.reduce((sum, byte) => sum + byte, 0);
  block.write(checksum.toString(8).padStart(6, '0') + '\0 ', 148, 8); return block;
}
function tar(entries) {
  const parts = [];
  for (const [name, content = '', type = '0'] of entries) {
    const bytes = Buffer.from(content); parts.push(header(name, bytes.length, type), bytes, Buffer.alloc((512 - bytes.length % 512) % 512));
  }
  parts.push(Buffer.alloc(1024)); return Buffer.concat(parts);
}
function stream(bytes, size = 31) {
  let offset = 0;
  return new ReadableStream({ pull(controller) { if (offset >= bytes.length) return controller.close(); controller.enqueue(bytes.subarray(offset, offset + size)); offset += size; } });
}
const entries = [['bundle-20242025/', '', '5'], ['bundle-20242025/manifest.json', '{"season":"20242025","bundled_at":"2025-07-01T00:00:00Z"}'],
  ['bundle-20242025/bios.json', '[{"playerId":1,"seasonId":20242025,"lastName":"Slafkovský"}]'], ['bundle-20242025/stats.json', '[{"seasonId":20242025}]']];
test('gzip/tar release layout is decoded locally with optional goalie absence and unknown observation time', async () => {
  const packageBytes = await importArchive(gzipSync(tar(entries)), 'data-20242025.tar.gz', 'regular');
  const data = JSON.parse(new TextDecoder().decode(packageBytes));
  assert.equal(data.season, 20242025); assert.equal(data.bios[0].lastName, 'Slafkovský');
  assert.deepEqual(data.goalies, []); assert.equal(data.observed_at, null); assert.equal(data.fetched_at, null);
  assert.equal(data.source, 'Local archive');
});
test('manifest supplies season for a renamed archive and playoffs are selected explicitly', async () => {
  const files = await readTarFiles(stream(tar([...entries, ['playoff-bios.json', '[{"seasonId":20242025}]'], ['playoff-stats.json', '[]']])));
  assert.equal(JSON.parse(new TextDecoder().decode(archivePackage(files, 'personal.tar.gz', 'playoff'))).season_type, 'playoff');
});
test('wrong-season, filename/manifest mismatch and empty playoffs are refused', async () => {
  const files = await readTarFiles(stream(tar(entries)));
  assert.throws(() => archivePackage(files, 'data-20232024.tar.gz', 'regular'), /disagrees/);
  assert.throws(() => archivePackage(files, 'data-20242025.tar.gz', 'playoff'), /missing/);
  files.set('playoff-bios.json', Buffer.from('[]')); files.set('playoff-stats.json', Buffer.from('[]'));
  assert.throws(() => archivePackage(files, 'data-20242025.tar.gz', 'playoff'), /complete/);
  files.set('stats.json', Buffer.from('[{"seasonId":20232024}]'));
  assert.throws(() => archivePackage(files, 'data-20242025.tar.gz', 'regular'), /wrong-season/);
});
test('paths, links, extended metadata and unrecognized files are refused before payload parsing', async () => {
  for (const [name, type] of [['../bios.json', '0'], ['/bios.json', '0'], ['C:/bios.json', '0'], ['dir\\bios.json', '0'], ['bios.json', '1'], ['bios.json', '2'], ['metadata', 'x'], ['program.js', '0']]) {
    await assert.rejects(readTarFiles(stream(tar([[name, '', type]]))), /Unsafe|Unsupported/);
  }
});
test('duplicate basenames cannot shadow one another across directories', async () => {
  await assert.rejects(readTarFiles(stream(tar([['one/bios.json', '[]'], ['two/bios.json', '[]']]))), /Duplicate/);
});
test('entry count and declared per-file expansion are bounded', async () => {
  const tooMany = tar(Array.from({ length: 129 }, (_, i) => [`dir${i}/`, '', '5']));
  await assert.rejects(readTarFiles(stream(tooMany)), /128 entries/);
  await assert.rejects(readTarFiles(stream(header('bios.json', 32 * 1024 * 1024 + 1))), /exceeds limits/);
});
test('header checksum, truncation and post-terminator data are refused', async () => {
  const corrupt = tar(entries); corrupt[0] ^= 1;
  await assert.rejects(readTarFiles(stream(corrupt)), /checksum/);
  await assert.rejects(readTarFiles(stream(tar(entries).subarray(0, 700))), /Truncated/);
  await assert.rejects(readTarFiles(stream(Buffer.concat([tar(entries), header('stats.json', 0)]))), /after archive end/);
});
test('missing tar end marker and malformed gzip cannot become a package', async () => {
  await assert.rejects(readTarFiles(stream(Buffer.alloc(0))), /end marker/);
  await assert.rejects(importArchive(Buffer.from('not gzip'), 'data-20242025.tar.gz', 'regular'));
});
test('compressed input above 25 MiB is refused before decompression', async () => {
  await assert.rejects(importArchive(new Uint8Array(MAX_ARCHIVE_BYTES + 1), 'data-20242025.tar.gz', 'regular'), /25 MiB/);
});
test('expanded streaming bytes are bounded independently of declared entry sizes', async () => {
  const oversized = new ReadableStream({ start(controller) { controller.enqueue(new Uint8Array(100 * 1024 * 1024 + 1)); controller.close(); } });
  await assert.rejects(readTarFiles(oversized), /100 MiB/);
});
test('gzip integrity failure is terminal even when tar contents are otherwise valid', async () => {
  const bytes = gzipSync(tar(entries)); bytes[bytes.length - 8] ^= 1;
  await assert.rejects(importArchive(bytes, 'data-20242025.tar.gz', 'regular'));
});
test('lockout, unsupported context and invalid report JSON are refused', async () => {
  const files = new Map([['bios.json', Buffer.from('[{}]')], ['stats.json', Buffer.from('[]')]]);
  assert.throws(() => archivePackage(files, 'data-20042005.tar.gz', 'regular'), /lockout/);
  assert.throws(() => archivePackage(files, 'data-20242025.tar.gz', 'unknown'), /season type/);
  files.set('stats.json', Buffer.from('{bad'));
  assert.throws(() => archivePackage(files, 'data-20242025.tar.gz', 'regular'));
});
