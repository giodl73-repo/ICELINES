// An embedded browsing context gives the application a measured 360px viewport
// when the host browser's viewport override is unavailable. This is layout
// evidence, not a physical mobile device or touch/screen-reader test.
import { mkdir, writeFile } from 'node:fs/promises';
const destination = new URL('../target/browser-schedule-preview/', import.meta.url);
await mkdir(destination, {recursive:true});
await writeFile(new URL('mobile-test.html', destination), `<!doctype html>
<html lang="en"><meta charset="utf-8"><title>IceLines 360px layout inspection</title>
<style>body{margin:0;padding:16px;font:14px system-ui;background:#e5edf5}h1{font-size:18px;margin:0 0 12px}p{max-width:600px}iframe{display:block;width:360px;height:800px;border:0;background:white}</style>
<h1>IceLines — 360px embedded viewport</h1>
<p>Production distribution in a 360px browsing context. This checks responsive layout and keyboard behavior; it does not simulate a physical phone or touch input.</p>
<iframe title="IceLines mobile layout" src="http://127.0.0.1:8057/#/leaders?season=20242025&amp;type=regular&amp;kind=skaters&amp;sort=points&amp;gp=&amp;filter=p%3E%3D100"></iframe></html>`);
console.log('Prepared mobile-test.html; serve production dist on localhost:8057 and the preview directory on localhost:8056.');
