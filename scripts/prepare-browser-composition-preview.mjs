// Local composition fixture; production assets are copied unchanged.
import { cp, mkdir, writeFile } from 'node:fs/promises';
const destination = new URL('../target/browser-composition-preview-37/', import.meta.url);
await mkdir(new URL('ICELINES/workbench/', destination), { recursive: true });
await cp(new URL('../icelines-browser/dist/', import.meta.url), new URL('ICELINES/workbench/', destination), { recursive: true });
await writeFile(new URL('narrow.html', destination), `<!doctype html><html lang="en"><meta charset="utf-8"><title>IceLines measured narrow composition</title><style>body{margin:0;background:#e5edf5}iframe{display:block;width:360px;height:800px;border:0}</style><iframe title="IceLines narrow composition" src="./ICELINES/workbench/"></iframe></html>`);
console.log('Prepared local composition preview: target/browser-composition-preview-37 (desktop subpath and 360x800 iframe).');
