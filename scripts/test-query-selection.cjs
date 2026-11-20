// Run against a local native server. An optional date-window URL exercises
// the same component with separately captured local evidence.
const { chromium } = require('../icelines-browser/node_modules/@playwright/test');
const assert = require('node:assert/strict');
(async () => {
    const browser = await chromium.launch({channel:'msedge',headless:true});
    try {
        const page = await browser.newPage();
        for (const url of [(process.env.ICELINES_BASE_URL || 'http://127.0.0.1:8000') + '/leaders?top=10&theme=fresh-ice', process.env.ICELINES_QUERY_URL].filter(Boolean)) {
            await page.goto(url, {waitUntil:'networkidle'});
            const ids = await page.locator('.range-table tbody tr').evaluateAll(rows => rows.slice(0,3).map(row=>row.dataset.nhlId));
            assert.equal(ids.length,3,'Local query needs at least three rows');
            const row = id => page.locator(`tr[data-nhl-id="${id}"]`);
            const selected = () => page.locator('tbody tr[aria-selected="true"]').evaluateAll(rows=>rows.map(row=>row.dataset.nhlId));
            if (await page.locator('#query-clear-selection').isEnabled()) await page.locator('#query-clear-selection').click();
            await row(ids[0]).locator('td').first().click();
            await row(ids[1]).locator('a').first().click({modifiers:['Shift']});
            assert.deepEqual(await selected(),ids.slice(0,2));
            assert.ok(await row(ids[0]).evaluate(r=>getComputedStyle(r).fontWeight >= 700));
            const saved = page.url();
            await page.reload({waitUntil:'networkidle'});
            assert.deepEqual(await selected(),ids.slice(0,2));
            assert.ok((await page.locator('[data-sort-key="ppg"]').getAttribute('href')).includes('selected='));
            await page.locator('[data-sort-key="ppg"]').click({modifiers:['Shift']});
            await page.waitForLoadState('networkidle');
            assert.equal(new URL(page.url()).searchParams.get('selected'), ids.slice(0,2).join(','));
            await row(ids[0]).focus(); await page.keyboard.press('Shift+Space');
            assert.deepEqual(await selected(),[ids[1]]);
            await row(ids[2]).locator('td').first().click();
            assert.deepEqual(await selected(),[ids[2]]);
            await page.locator('#query-controls button[type="submit"]').first().click();
            await page.waitForLoadState('networkidle');
            assert.deepEqual(await selected(),[ids[2]]);
            assert.ok((await page.locator('.query-selection-context').innerText()).includes(await row(ids[2]).getAttribute('data-player-name')));
            await page.emulateMedia({media:'print'});
            assert.ok(await row(ids[2]).evaluate(r=>getComputedStyle(r).fontWeight >= 700));
            assert.equal(await page.locator('.query-selection-controls').isVisible(),false);
            await page.emulateMedia({media:'screen'});
            await page.locator('#query-clear-selection').click();
            await page.reload({waitUntil:'networkidle'});
            assert.deepEqual(await selected(),[],'Cleared selection must override legacy highlight');
            assert.equal(new URL(page.url()).searchParams.get('selected'),'');
            await page.goto(saved, {waitUntil:'networkidle'});
            assert.deepEqual(await selected(),ids.slice(0,2),'Copied URL restores selection');
        }
        console.log('Shared row selection: additive/toggle, keyboard, URL restore, sorting, form submission, print and clear passed.');
    } finally { await browser.close(); }
})().catch(error=>{console.error(error);process.exitCode=1;});
