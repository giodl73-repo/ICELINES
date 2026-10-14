import { readFile, writeFile } from 'node:fs/promises';
import { build, version } from 'esbuild';
import { fileURLToPath } from 'node:url';

export async function bundleFrontend(dist) {
  if (version !== '0.28.2') throw new Error('Frontend bundler pin mismatch');
  const metadata = await readFile(new URL('src/catalog-metadata.js', dist), 'utf8');
  if (!metadata.includes('// BUILD_CATALOG_METADATA') || metadata.includes('const knownMetadata = {};'))
    throw new Error('Bundle only after authenticated catalog metadata injection');
  const result = await build({ entryPoints: [fileURLToPath(new URL('src/main.js', dist))],
    bundle: true, minify: true, format: 'esm', platform: 'browser', target: 'es2022',
    write: false, metafile: true });
  if (result.outputFiles.length !== 1 || !Object.keys(result.metafile.inputs).some(path => path.endsWith('/catalog-metadata.js')))
    throw new Error('Frontend bundle must contain the injected catalog metadata');
  await writeFile(new URL('src/main.js', dist), result.outputFiles[0].contents);
}
