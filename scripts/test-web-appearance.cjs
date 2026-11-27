const {test} = require('node:test');
const assert = require('node:assert/strict');
const {readStyle, applyStyle, tableClipboard} = require('../icelines-web/static/appearance.js');

test('style preference survives navigation and unavailable storage remains usable', () => {
    const saved = new Map();
    const storage = {getItem: key => saved.get(key), setItem: (key, value) => saved.set(key, value)};
    const win = {document: {documentElement: {dataset: {}}}, localStorage: storage};
    assert.equal(readStyle(storage), 'icelines');
    applyStyle(win, 'nhl');
    assert.equal(win.document.documentElement.dataset.displayStyle, 'nhl');
    assert.equal(readStyle(storage), 'nhl');
    applyStyle(win, 'icelines');
    assert.equal(readStyle(storage), 'icelines');
    const blocked = {getItem() {throw new Error('blocked');}, setItem() {throw new Error('blocked');}};
    assert.equal(readStyle(blocked), 'icelines');
    win.localStorage = blocked;
    assert.doesNotThrow(() => applyStyle(win, 'nhl'));
    assert.equal(win.document.documentElement.dataset.displayStyle, 'nhl');
    applyStyle(win, 'unsupported');
    assert.equal(win.document.documentElement.dataset.displayStyle, 'icelines');
});

const cell = (tagName, textContent, numeric = false) => ({tagName, textContent, classList: {contains: name => numeric && name === 'numeric'}});
const row = (cells, hidden = false) => ({cells, hidden, getAttribute: () => null});
test('clean copy keeps displayed values/order and excludes source colors, links and hidden rows', () => {
    const table = {rows: [row([cell('TH', 'Player'), cell('TH', 'EV P', true)]), row([cell('TD', 'Sample & Player'), cell('TD', '0', true)]), row([cell('TD', 'Hidden Player'), cell('TD', '999')], true), row([cell('TD', '<script>not markup</script>'), cell('TD', '12', true)])]};
    const data = tableClipboard(table, '2026–27 · Even strength');
    assert.equal(data.plain, 'Player\tEV P\nSample & Player\t0\n<script>not markup</script>\t12');
    assert.ok(data.html.includes('2026–27 · Even strength'));
    assert.ok(data.html.includes('Sample &amp; Player'));
    assert.ok(data.html.includes('&lt;script&gt;not markup&lt;/script&gt;'));
    assert.ok(data.html.includes('text-align:right'));
    assert.ok(data.html.includes('background:#ffffff'));
    assert.ok(!data.html.includes('Hidden Player'));
    assert.ok(!/<script>|class=|href=|gradient|#[0-9a-f]*(?:ff0000|00ff00)/i.test(data.html));
});

test('clean copy uses the table applied-filter caption', () => {
    const table = {caption: {textContent: 'Age = 25 · 2026–27 regular · Points: Even strength'}, rows: [row([cell('TH', 'Player')])]};
    const data = tableClipboard(table, 'unfiltered fallback');
    assert.ok(data.html.includes('Age = 25'));
    assert.ok(data.html.includes('Points: Even strength'));
    assert.ok(!data.html.includes('unfiltered fallback'));
});
