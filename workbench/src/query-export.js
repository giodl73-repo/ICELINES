const columns = ['player_id', 'name', 'team', 'position', 'gp', 'goals', 'assists', 'points', 'pace_82', 'wins', 'save_pct', 'gaa'];
const methodology = {
    pace: 'Descriptive pace per 82 games; values below pace_minimum_games are unavailable.',
    history: 'Historical comparisons are not era adjusted.',
    missing: 'Unavailable values remain null in JSON and empty in CSV; they are not zero.',
    precision: 'Numeric values retain engine precision; rows retain engine ordering.',
    csv_text: 'Potential spreadsheet formulas in text cells receive a leading apostrophe. JSON preserves original text.',
};
// Snapshot the request that produced these rows, rather than current UI controls.
export function queryExport(result, request) {
    return {
        ...result,
        export_schema_version: 1,
        query: {
            filter: request.filter, sort: request.sort, kind: request.goalies ? 'goalies' : 'skaters',
            requested_minimum_games: request.minimum_games, today: request.today,
        },
        methodology: { ...methodology },
    };
}
export function csvCell(value) {
    let text = String(value ?? '');
    // Quoting alone does not prevent spreadsheet formula evaluation. Only text
    // is guarded: signed numeric values remain numeric, and null stays empty.
    if (typeof value === 'string' && /^(?:[\u0000-\u001f\u007f]|[\s\u0000-\u001f]*[=+\-@])/u.test(value))
        text = "'" + text;
    return '"' + text.replaceAll('"', '""') + '"';
}
export function queryCsv(result, request) {
    const exported = queryExport(result, request);
    // Each comment is one CSV field. JSON encodes embedded line breaks so local
    // source/filter strings cannot introduce metadata lines or extra records.
    const metadata = Object.entries(exported).filter(([key]) => key !== 'rows')
        .map(([key, value]) => csvCell(`# ${key}=${JSON.stringify(value)}`));
    return [csvCell('# IceLines descriptive query'), ...metadata, columns.join(','),
        ...result.rows.map(row => columns.map(key => csvCell(row[key])).join(','))].join('\r\n');
}
