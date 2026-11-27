/* Local display preference and deliberately neutral table clipboard output. */
(function (root, factory) {
    'use strict';
    const api = factory();
    if (typeof module === 'object' && module.exports) module.exports = api;
    if (root && root.document) api.install(root);
})(typeof window === 'undefined' ? null : window, function () {
    'use strict';
    const storageKey = 'icelines:display-style:v1';
    const normalized = value => value === 'nhl' ? 'nhl' : 'icelines';
    function readStyle(storage) {
        try { return normalized(storage.getItem(storageKey)); }
        catch (_) { return 'icelines'; }
    }
    function applyStyle(win, value, persist = true) {
        const style = normalized(value);
        win.document.documentElement.dataset.displayStyle = style;
        if (persist) {
            try { win.localStorage.setItem(storageKey, style); } catch (_) { /* Session still works. */ }
        }
        return style;
    }
    const text = value => String(value || '').replace(/\s+/g, ' ').trim();
    const escapeHtml = value => String(value).replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
    function tableClipboard(table, caption) {
        if (table.caption) caption = text(table.caption.textContent);
        const rows = Array.from(table.rows).filter(row => !row.hidden && row.getAttribute('aria-hidden') !== 'true');
        const cells = rows.map(row => Array.from(row.cells).filter(cell => !cell.hidden));
        const plain = cells.map(row => row.map(cell => text(cell.textContent)).join('\t')).join('\n');
        const htmlRows = cells.map(row => '<tr>' + row.map(cell => {
            const tag = cell.tagName.toLowerCase() === 'th' ? 'th' : 'td';
            const align = cell.classList.contains('numeric') ? 'right' : 'left';
            const bg = tag === 'th' ? '#f2f2f2' : '#ffffff';
            return '<' + tag + ' style="border:1px solid #cccccc;padding:5px 8px;color:#111111;background:' + bg + ';text-align:' + align + ';font-family:Arial,sans-serif;font-size:12px;">' + escapeHtml(text(cell.textContent)) + '</' + tag + '>';
        }).join('') + '</tr>').join('');
        const html = '<table style="border-collapse:collapse;background:#ffffff;color:#111111;font-family:Arial,sans-serif;font-size:12px;">' +
            '<caption style="text-align:left;color:#111111;background:#ffffff;font-weight:bold;">' + escapeHtml(caption) + '</caption>' + htmlRows + '</table>';
        return { plain, html };
    }
    function install(win) {
        const doc = win.document;
        let initial = 'icelines';
        try { initial = readStyle(win.localStorage); } catch (_) { /* Storage can be disabled. */ }
        applyStyle(win, initial, false);
        function ready() {
            doc.querySelectorAll('[data-rank-range]').forEach(form => {
                const first = form.querySelector('[name="rank-from"]');
                const last = form.querySelector('[name="rank-to"]');
                function validate() {
                    const start = Number(first.value), end = Number(last.value);
                    last.setCustomValidity(end < start ? 'Last rank must be at or above the first rank.' : end - start >= 500 ? 'Choose at most 500 rows per range.' : '');
                }
                first.addEventListener('input', validate);
                last.addEventListener('input', validate);
                form.querySelectorAll('[data-rank-step]').forEach(button => button.addEventListener('click', () => {
                    validate(); if (!form.reportValidity()) return;
                    const start = Number(first.value), end = Number(last.value), size = end - start + 1;
                    const next = Math.max(1, start + Number(button.dataset.rankStep) * size);
                    first.value = next; last.value = next + size - 1;
                    form.requestSubmit();
                }));
            });
            const selector = doc.getElementById('display-style');
            if (selector) {
                selector.value = doc.documentElement.dataset.displayStyle;
                selector.addEventListener('change', () => applyStyle(win, selector.value));
            }
            doc.querySelectorAll('main table').forEach((table, index) => {
                const tools = doc.createElement('div');
                tools.className = 'table-copy-tools';
                const button = doc.createElement('button');
                button.type = 'button';
                button.textContent = 'Copy clean table';
                button.setAttribute('aria-label', 'Copy clean table ' + (index + 1));
                const status = doc.createElement('span');
                status.setAttribute('role', 'status');
                tools.append(button, status);
                table.before(tools);
                button.addEventListener('click', async () => {
                    const season = doc.querySelector('.season-header > span');
                    const mode = doc.querySelector('select[name="points-mode"]');
                    const context = [doc.title, season && text(season.textContent), mode && ('Points: ' + mode.options[mode.selectedIndex].text)].filter(Boolean).join(' · ');
                    const data = tableClipboard(table, context);
                    try {
                        if (win.ClipboardItem && win.navigator.clipboard && win.navigator.clipboard.write) {
                            await win.navigator.clipboard.write([new win.ClipboardItem({
                                'text/plain': new win.Blob([data.plain], {type: 'text/plain'}),
                                'text/html': new win.Blob([data.html], {type: 'text/html'})
                            })]);
                        } else if (win.navigator.clipboard && win.navigator.clipboard.writeText) {
                            await win.navigator.clipboard.writeText(data.plain);
                        } else { throw new Error('Clipboard unavailable'); }
                        status.textContent = 'Copied';
                    } catch (_) { status.textContent = 'Copy unavailable. Select the table and copy, or paste as plain text.'; }
                });
            });
        }
        if (doc.readyState === 'loading') doc.addEventListener('DOMContentLoaded', ready, {once: true});
        else ready();
    }
    return {readStyle, applyStyle, tableClipboard, install};
});
