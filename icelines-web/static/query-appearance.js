// Native query reports use the workbench's palette and saved appearance key.
(function () {
    'use strict';
    const themes = ['night-game', 'fresh-ice', 'hockey-club'];
    const densities = ['compact', 'comfortable'];
    const themeKey = 'icelines.appearance.v1';
    const densityKey = 'icelines.query-density.v1';
    const params = new URLSearchParams(location.search);
    const saved = key => { try { return localStorage.getItem(key); } catch { return null; } };
    const valid = (value, choices, fallback) => choices.includes(value) ? value : fallback;
    let theme = valid(params.get('theme') || saved(themeKey), themes, 'night-game');
    let density = valid(params.get('density') || saved(densityKey), densities, 'compact');
    function apply() {
        document.documentElement.dataset.theme = theme;
        document.documentElement.dataset.density = density;
        const picker = document.getElementById('appearance');
        const spacing = document.getElementById('density');
        if (picker) picker.value = theme;
        if (spacing) spacing.value = density;
        for (const [key, value] of [['theme', theme], ['density', density]]) {
            const input = document.querySelector(`input[name="${key}"]`);
            if (input) input.value = value;
        }
        for (const link of document.querySelectorAll('.range-sort-link')) {
            const url = new URL(link.href);
            url.searchParams.set('theme', theme);
            url.searchParams.set('density', density);
            link.href = url.href;
        }
        document.querySelector('meta[name="theme-color"]')?.setAttribute('content',
            getComputedStyle(document.documentElement).getPropertyValue('--bg').trim());
    }
    apply();
    document.addEventListener('DOMContentLoaded', () => {
        apply();
        function shiftSort(link) {
            const chain = Array.from(document.querySelectorAll('.range-sort-link'))
                .filter(node => Number(node.dataset.sortPriority) > 0)
                .sort((a,b) => Number(a.dataset.sortPriority)-Number(b.dataset.sortPriority))
                .map(node => ({ key: node.dataset.sortKey, order: node.dataset.sortOrder }));
            const existing = chain.find(term => term.key === link.dataset.sortKey);
            if (existing) existing.order = existing.order === 'desc' ? 'asc' : 'desc';
            else chain.push({key:link.dataset.sortKey,order:link.dataset.sortOrder});
            const url = new URL(link.href);
            url.searchParams.set('sort',chain[0].key);
            url.searchParams.set('order',chain[0].order);
            url.searchParams.set('then',chain.slice(1).map(t=>`${t.key}:${t.order}`).join(','));
            location.assign(url.href);
        }
        for (const link of document.querySelectorAll('.range-sort-link')) {
            link.addEventListener('click', event => {
                if (event.shiftKey && !event.ctrlKey && !event.metaKey && !event.altKey && event.button === 0) {
                    event.preventDefault(); shiftSort(link);
                }
            });
            link.addEventListener('keydown',event=>{
                if (event.key==='Enter' && event.shiftKey) {event.preventDefault();shiftSort(link);}
            });
        }
        const sortPicker=document.querySelector('select[name="sort"]');
        sortPicker?.addEventListener('change',()=>{
            const rest=document.querySelector('input[name="then"]');
            if(rest) rest.value=rest.value.split(',').filter(t=>t && t.split(':')[0]!==sortPicker.value).join(',');
        });
        const form=document.getElementById('query-controls');
        const refresh=form?.querySelector('button[formmethod="post"]');
        function refreshAvailability(){if(refresh){const from=form.querySelector('[name="date-from"]').value;const to=form.querySelector('[name="date-to"]').value;const roster=form.querySelector('[name="roster"]')?.value||'all';refresh.disabled=!((from&&to)||(!from&&!to&&roster!=='all'&&form.querySelector('[name="team"]').value));}}
        form?.addEventListener('input',refreshAvailability);refreshAvailability();
        function change() {
            theme = valid(document.getElementById('appearance').value, themes, 'night-game');
            density = valid(document.getElementById('density').value, densities, 'compact');
            apply();
            const url = new URL(location.href);
            url.searchParams.set('theme', theme);
            url.searchParams.set('density', density);
            history.replaceState(null, '', url);
            const notice = document.getElementById('appearance-status');
            try {
                localStorage.setItem(themeKey, theme);
                localStorage.setItem(densityKey, density);
                notice.textContent = 'Appearance saved on this device.';
            } catch {
                notice.textContent = 'Appearance changed for this tab; browser storage is unavailable.';
            }
        }
        document.getElementById('appearance').addEventListener('change', change);
        document.getElementById('density').addEventListener('change', change);
    });
    window.addEventListener('storage', event => {
        if (event.key === themeKey || event.key === null) theme = valid(saved(themeKey), themes, 'night-game');
        if (event.key === densityKey || event.key === null) density = valid(saved(densityKey), densities, 'compact');
        apply();
    });
})();
