export const SORT_OPTIONS = {
    skaters: [['points', 'Points'], ['goals', 'Goals'], ['assists', 'Assists'], ['gp', 'Games played'], ['pace', 'Pace per 82']],
    goalies: [['wins', 'Wins'], ['save_pct', 'Save percentage'], ['gaa', 'GAA'], ['gp', 'Games played']],
};
export function parseViewHash(hash) {
    if (!hash || hash === '#' || hash === '#analysis')
        return;
    if (!hash.startsWith('#/leaders?') || hash.length > 8192)
        throw new Error('Unsupported or oversized view link.');
    const query = new URLSearchParams(hash.slice('#/leaders?'.length));
    const allowed = ['season', 'type', 'kind', 'sort', 'gp', 'data', 'filter'];
    query.forEach((_value, key) => {
        if (!allowed.includes(key) || query.getAll(key).length !== 1)
            throw new Error('Unsupported or duplicate view parameter.');
    });
    const season = query.get('season') ?? '';
    if (!/^\d{8}$/.test(season))
        throw new Error('View link needs a YYYYZZZZ season.');
    const start = Number(season.slice(0, 4)), end = Number(season.slice(4));
    if (start < 1900 || start > 2999 || end !== start + 1 || season === '20042005')
        throw new Error('Unavailable or invalid season in view link.');
    const type = query.get('type') ?? 'regular';
    if (type !== 'regular' && type !== 'playoff')
        throw new Error('Unsupported season type in view link.');
    const kind = query.get('kind') ?? 'skaters';
    if (kind !== 'skaters' && kind !== 'goalies')
        throw new Error('Unsupported player view in link.');
    const sort = query.get('sort') ?? SORT_OPTIONS[kind][0][0];
    if (!SORT_OPTIONS[kind].some(option => option[0] === sort))
        throw new Error('Sort does not apply to this player view.');
    const gp = query.get('gp') ?? '';
    if (gp !== '' && (!/^\d{1,4}$/.test(gp) || Number(gp) > 1000))
        throw new Error('Minimum GP must be between 0 and 1000.');
    const data = query.get('data') ?? 'public';
    if (data !== 'public' && data !== 'local')
        throw new Error('Unsupported dataset reference in link.');
    const filter = query.get('filter') ?? '';
    if (filter.length > 4096 || (data === 'local' && filter))
        throw new Error('Filter text is unavailable or too long for this link.');
    return { season: Number(season), type, kind, sort, gp, data, filter };
}
export function serializeViewHash(state, includePublicFilter = false) {
    const query = new URLSearchParams({ season: String(state.season), type: state.type, kind: state.kind, sort: state.sort, gp: state.gp });
    if (state.data === 'local')
        query.set('data', 'local');
    if (includePublicFilter && state.data === 'public' && state.filter)
        query.set('filter', state.filter);
    const hash = '#/leaders?' + query;
    parseViewHash(hash);
    return hash;
}
export function isPublicDataset(dataset, catalog) {
    return catalog.some(entry => entry.id === dataset.id && entry.sha256 === dataset.revision
        && entry.season === dataset.metadata.season && entry.season_type === dataset.metadata.season_type);
}
export function localDatasetForView(saved, activeId, view, catalog) {
    const candidates = saved.filter(item => item.metadata.season === view.season
        && item.metadata.season_type === view.type && !isPublicDataset(item, catalog));
    return candidates.find(item => item.id === activeId) ?? (candidates.length === 1 ? candidates[0] : undefined);
}
