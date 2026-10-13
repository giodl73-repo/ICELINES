// Opt-in real-browser regression for the viewport renderer; no live NHL reads.
import { chromium, expect } from '@playwright/test';
import { createServer } from 'node:http';
import { readFile, writeFile, mkdtemp, stat } from 'node:fs/promises';
import { resolve, dirname, extname, sep } from 'node:path';
import assert from 'node:assert/strict';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
const repository = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const index = process.argv.indexOf('--artifact');
if (index < 0 || !process.argv[index + 1])
    throw new Error('Provide --artifact <retained distribution>. This test uses the curated 20252026 regular-season fixture.');
const root = resolve(repository, process.argv[index + 1]);
const info = JSON.parse(await readFile(resolve(root, 'build-info.json'), 'utf8'));
const shell = JSON.parse(await readFile(resolve(root, 'shell-manifest.json'), 'utf8'));
const verification = spawnSync('python', [resolve(repository, 'scripts/verify-browser-artifact.py'), root, info.commit, shell.build], { encoding: 'utf8' });
if (verification.status !== 0)
    throw new Error(verification.stderr || String(verification.error));
const out = await mkdtemp(resolve(repository, 'target/viewport-browser-'));
const report = { verified: false, artifact: JSON.parse(verification.stdout), scope: 'Local owned browser; complete result/export parity, continuous scrolling, keyboard, detail and failure/retry. Not phone or memory acceptance.' };
const server = createServer(async (req, res) => { try {
    const p = decodeURIComponent(new URL(req.url, 'http://localhost').pathname).replace('/ICELINES/workbench/', '');
    const f = resolve(root, p || 'index.html');
    if (!f.startsWith(root + sep) || !(await stat(f)).isFile())
        throw Error();
    res.writeHead(200, { 'Content-Type': ({ '.html': 'text/html', '.js': 'application/javascript', '.css': 'text/css', '.json': 'application/json', '.wasm': 'application/wasm' })[extname(f)] || 'application/octet-stream' }).end(await readFile(f));
}
catch {
    res.writeHead(404).end();
} });
await new Promise(r => server.listen(0, '127.0.0.1', r));
const browser = await chromium.launch({ channel: 'chromium', headless: true });
let failure;
let currentPage;
try {
    const page = currentPage = await browser.newPage({ viewport: { width: 1280, height: 800 } });
    await page.goto(`http://127.0.0.1:${server.address().port}/ICELINES/workbench/`);
    await expect(page.locator('#catalog option').first()).toBeAttached();
    await page.locator('#data-library > summary').click();
    await page.locator('#catalog').selectOption('20252026-regular');
    await page.locator('#load').click();
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '933', { timeout: 60000 });
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
    const download = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);
    const exported = JSON.parse(await readFile(await download[0].path(), 'utf8'));
    assert.equal(exported.rows.length, 933);
    const initialMounted = await page.locator('#rows tr[data-result-index]').count();
    assert.ok(initialMounted <= 44);
    await page.locator('#table-view').selectOption('all');
    await expect(page.locator('#rows tr[data-result-index]')).toHaveCount(933);
    const allNames = await page.locator('#rows td:nth-child(2) button').allTextContents();
    assert.deepEqual(allNames, exported.rows.map(r => r.name));
    const axSession=await page.context().newCDPSession(page);
    const axTree=await axSession.send('Accessibility.getFullAXTree');
    const nameSet=new Set(exported.rows.map(r=>r.name));
    const axNames=axTree.nodes.filter(n=>n.role?.value==='button' && nameSet.has(n.name?.value)).map(n=>n.name.value);
    assert.deepEqual(axNames,exported.rows.map(r=>r.name));
    await axSession.detach();
    await page.locator('#table-view').selectOption('scroll');
    await page.evaluate(()=>window.addEventListener('beforeprint',()=>{window.__printedRows=document.querySelectorAll('#rows tr[data-result-index]').length;}));
    await writeFile(resolve(out,'print-reference.json'),JSON.stringify(exported.rows.map((row,index)=>({rank:index+1,name:row.name,pace:row.pace_82==null?'—':row.pace_82.toFixed(1)})))+'\n');
    await page.pdf({path:resolve(out,'all-players.pdf'),format:'A4',printBackground:false,margin:{top:'12mm',bottom:'12mm',left:'12mm',right:'12mm'}});
    assert.equal(await page.evaluate(()=>window.__printedRows),933);
    await expect(page.locator('#table-view')).toHaveValue('scroll');
    assert.ok(await page.locator('#rows tr[data-result-index]').count()<=44);
    const top = await page.locator('#rows').evaluate(e => e.getBoundingClientRect().top + scrollY);
    const seen = new Set();
    let maxMounted = 0;
    for (let i = 0; i < 933; i += 20) {
        await page.evaluate(y => scrollTo(0, y), top + i * 72);
        const expectedStart = await page.locator("#rows").evaluate(e => Math.max(0, Math.min(933 - 44, Math.floor(Math.max(0, -e.getBoundingClientRect().top) / 72) - 16)));
        await expect(page.locator("#rows tr[data-result-index]").first()).toHaveAttribute("data-result-index", String(expectedStart));
        const rows = await page.locator('#rows tr[data-result-index]').evaluateAll(es => es.map(e => ({ index: Number(e.dataset.resultIndex), rank: Number(e.cells[0].textContent), name: e.cells[1].textContent, height: e.getBoundingClientRect().height })));
        maxMounted = Math.max(maxMounted, rows.length);
        for (const r of rows) {
            assert.equal(r.rank, r.index + 1);
            assert.equal(r.name, exported.rows[r.index].name);
            assert.equal(r.height, 72);
            seen.add(r.index);
        }
    }
    if (seen.size !== 933)
        console.log(JSON.stringify({ missing: exported.rows.map((_, i) => i).filter(i => !seen.has(i)), geometry: await page.locator('#rows').evaluate(e => ({ top: e.getBoundingClientRect().top + scrollY, height: e.getBoundingClientRect().height, y: scrollY, documentHeight: document.documentElement.scrollHeight, indices: Array.from(e.querySelectorAll('tr[data-result-index]'), e => e.dataset.resultIndex) })) }));
    assert.equal(seen.size, 933);
    assert.ok(maxMounted <= 44);
    await page.locator('tr[data-result-index="932"] button').click();
    await expect(page.locator('#player-name')).toHaveText(exported.rows[932].name);
    await page.locator('#close-player').click();
    await page.locator('tr[data-result-index="932"] button').focus();
    await page.keyboard.press('Tab');
    await expect(page.locator('#schedule-date')).toBeFocused();
    await page.keyboard.press('Shift+Tab');
    await expect(page.locator('tr[data-result-index="932"] button')).toBeFocused();
    await page.evaluate(y => scrollTo(0, y), top);
    await page.waitForTimeout(50);
    await page.locator('tr[data-result-index="0"] button').focus();
    for (let i = 0; i < 120; i++) {
        const active = await page.evaluate(() => document.activeElement?.closest('tr')?.dataset.resultIndex);
        assert.equal(Number(active), i);
        await page.keyboard.press('Tab');
        await page.waitForTimeout(15);
    }
    for (let i = 120; i > 0; i--) {
        const active = await page.evaluate(() => document.activeElement?.closest('tr')?.dataset.resultIndex);
        assert.equal(Number(active), i);
        await page.keyboard.press('Shift+Tab');
        await page.waitForTimeout(15);
    }
    await page.keyboard.press('Shift+Tab');
    await expect(page.locator('#analysis .table-scroll')).toBeFocused();
    await page.keyboard.press('Tab');
    await expect(page.locator('tr[data-result-index="0"] button')).toBeFocused();
    await page.locator('#filter').fill('p>=1');
    await page.locator('#query button[type="submit"]').click();
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '835', { timeout: 60000 });
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
    await page.locator('#filter').fill('nonsense ???');
    await page.locator('#query button[type="submit"]').click();
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '0');
    await expect(page.locator('#rows tr')).toHaveCount(0);
    await page.locator('#filter').fill('');
    await page.locator('#query button[type="submit"]').click();
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '933');
    await page.locator('#kind').selectOption('goalies');
    await expect(page.locator('#query-summary')).toContainText('goalies');
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
    const goalieDownload = await Promise.all([page.waitForEvent('download'), page.locator('#export-json').click()]);
    const goalies = JSON.parse(await readFile(await goalieDownload[0].path(), 'utf8'));
    await page.locator('#table-view').selectOption('all');
    await expect(page.locator('#rows tr[data-result-index]')).toHaveCount(goalies.rows.length);
    assert.deepEqual(await page.locator('#rows td:nth-child(2) button').allTextContents(), goalies.rows.map(r => r.name));
    await page.locator('#table-view').selectOption('scroll');
    await page.locator('#kind').selectOption('skaters');
    await expect(page.locator('#rows')).toHaveAttribute('data-result-count', '933');
    await expect(page.locator('#rows')).toHaveJSProperty('inert', false);
    await page.locator('#rows').evaluate(e => scrollTo(0, e.getBoundingClientRect().top + scrollY - 180));
    await page.waitForTimeout(50);
    await page.screenshot({ path: resolve(out, 'desktop.png'), fullPage: false });
    await page.setViewportSize({ width: 390, height: 844 });
    await page.waitForTimeout(50);
    assert.ok(await page.locator('#rows tr[data-result-index]').count() <= 44);
    await page.locator('#rows').evaluate(e => scrollTo(0, e.getBoundingClientRect().top + scrollY - 60));
    await page.waitForTimeout(50);
    await page.screenshot({ path: resolve(out, 'narrow.png'), fullPage: false });
    report.verified = true;
    report.evidence = { exported_rows: 933, all_table_names_match: true, all_ranks_scrolled: seen.size, max_mounted: maxMounted, exact_row_height: 72, last_player_detail: true, tab_steps: 120, reverse_tab_steps: 120, filtered_result_count: 835, failure_clear_retry: true, narrow_width: 390, terminal_tab_boundaries: true, ax_all_player_names: axNames.length, print_source_rows: 933, print_restores_scroll_mode: true, goalie_export_full_table_parity: goalies.rows.length };
}
catch (e) {
    failure = e;
    report.error = String(e);
    report.diagnostic = await currentPage.evaluate(() => ({ active: document.activeElement?.tagName, y: scrollY, top: document.querySelector('#rows').getBoundingClientRect().top, indices: Array.from(document.querySelectorAll('#rows tr[data-result-index]'), e => e.dataset.resultIndex) }));
}
finally {
    await browser.close();
    await new Promise(r => server.close(r));
    await writeFile(resolve(out, 'report.json'), JSON.stringify(report, null, 2));
    console.log(out, JSON.stringify(report));
}
if (failure)
    throw failure;
