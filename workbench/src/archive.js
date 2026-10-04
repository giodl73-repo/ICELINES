export const MAX_ARCHIVE_BYTES = 25 * 1024 * 1024;
const MAX_FILE_BYTES = 32 * 1024 * 1024;
const MAX_EXPANDED_BYTES = 100 * 1024 * 1024;
const MAX_ENTRIES = 128;
const allowed = new Set(['manifest.json', 'bios.json', 'stats.json', 'goalie-stats.json', 'playoff-bios.json', 'playoff-stats.json', 'playoff-goalie-stats.json']);
const decoder = new TextDecoder('utf-8', { fatal: true });
class TarReader {
    reader;
    pending = new Uint8Array(0);
    expanded = 0;
    constructor(stream) { this.reader = stream.getReader(); }
    async read(size, allowEnd = false) {
        const result = new Uint8Array(size);
        let offset = 0;
        while (offset < size) {
            if (!this.pending.length) {
                const { value, done } = await this.reader.read();
                if (done) {
                    if (allowEnd && offset === 0)
                        return undefined;
                    throw new Error('Truncated archive');
                }
                this.expanded += value.length;
                // Includes headers/padding, bounding even malicious post-terminator streams.
                if (this.expanded > MAX_EXPANDED_BYTES)
                    throw new Error('Expanded archive exceeds 100 MiB');
                this.pending = value;
            }
            const count = Math.min(size - offset, this.pending.length);
            result.set(this.pending.subarray(0, count), offset);
            offset += count;
            this.pending = this.pending.subarray(count);
        }
        return result;
    }
    async close() { await this.reader.cancel(); }
}
function field(header, start, length) {
    const bytes = header.subarray(start, start + length);
    const end = bytes.indexOf(0);
    return decoder.decode(end < 0 ? bytes : bytes.subarray(0, end));
}
function octal(header, start, length) {
    const value = field(header, start, length).trim();
    if (!/^[0-7]+$/.test(value))
        throw new Error('Unsupported archive number encoding');
    const result = Number.parseInt(value, 8);
    if (!Number.isSafeInteger(result))
        throw new Error('Invalid archive size');
    return result;
}
// Reads only standard tar regular files/directories. Never extracts paths to disk.
export async function readTarFiles(stream) {
    const reader = new TarReader(stream);
    const files = new Map();
    let entries = 0;
    let total = 0;
    let ended = false;
    try {
        for (;;) {
            const header = await reader.read(512, true);
            if (!header) {
                if (!ended)
                    throw new Error('Archive is missing its end marker');
                return files;
            }
            if (header.every(byte => byte === 0)) {
                if (!ended) {
                    const second = await reader.read(512);
                    if (!second?.every(byte => byte === 0))
                        throw new Error('Invalid archive end marker');
                    ended = true;
                }
                continue;
            }
            if (ended)
                throw new Error('Unexpected data after archive end');
            if (++entries > MAX_ENTRIES)
                throw new Error('Archive exceeds 128 entries');
            const checksum = header.reduce((sum, byte, index) => sum + (index >= 148 && index < 156 ? 32 : byte), 0);
            if (octal(header, 148, 8) !== checksum)
                throw new Error('Archive header checksum mismatch');
            const prefix = field(header, 345, 155);
            const leaf = field(header, 0, 100);
            const name = prefix ? prefix + '/' + leaf : leaf;
            if (!name || name.startsWith('/') || name.includes('\\') || name.includes(':') || name.split('/').includes('..'))
                throw new Error('Unsafe archive path');
            const type = header[156];
            const size = octal(header, 124, 12);
            if (type === 53) {
                if (size !== 0)
                    throw new Error('Invalid archive directory size');
                continue;
            }
            if (type !== 0 && type !== 48)
                throw new Error('Unsupported archive entry: links and extended metadata are refused');
            const basename = name.split('/').at(-1);
            if (!allowed.has(basename))
                throw new Error('Unsupported archive file');
            if (files.has(basename))
                throw new Error('Duplicate archive file');
            total += size;
            if (size > MAX_FILE_BYTES || total > MAX_EXPANDED_BYTES)
                throw new Error('Expanded archive file exceeds limits');
            files.set(basename, (await reader.read(size)));
            const padding = (512 - size % 512) % 512;
            if (padding)
                await reader.read(padding);
        }
    }
    finally {
        await reader.close();
    }
}
function json(files, name) {
    const bytes = files.get(name);
    if (!bytes)
        throw new Error('Archive is missing ' + name);
    return JSON.parse(decoder.decode(bytes));
}
export function archivePackage(files, filename, seasonType) {
    if (!['regular', 'playoff'].includes(seasonType))
        throw new Error('Invalid archive season type');
    const filenameSeason = /^data-(\d{8})\.tar\.gz$/i.exec(filename)?.[1];
    let manifestSeason;
    if (files.has('manifest.json')) {
        const manifest = json(files, 'manifest.json');
        if (!manifest || typeof manifest !== 'object' || !/^\d{8}$/.test(String(manifest.season)))
            throw new Error('Invalid archive manifest season');
        manifestSeason = String(manifest.season);
    }
    if (filenameSeason && manifestSeason && filenameSeason !== manifestSeason)
        throw new Error('Archive season disagrees with filename');
    const season = Number(manifestSeason ?? filenameSeason);
    const start = Math.floor(season / 10000);
    if (!Number.isInteger(season) || start < 1900 || start > 2999 || season % 10000 !== start + 1 || season === 20042005)
        throw new Error('Invalid or lockout archive season');
    const prefix = seasonType === 'playoff' ? 'playoff-' : '';
    const bios = json(files, prefix + 'bios.json');
    const stats = json(files, prefix + 'stats.json');
    const goalies = files.has(prefix + 'goalie-stats.json') ? json(files, prefix + 'goalie-stats.json') : [];
    if (![bios, stats, goalies].every(Array.isArray) || !bios.length)
        throw new Error('Archive has no complete selected season data');
    for (const rows of [bios, stats, goalies])
        for (const row of rows) {
            if (!row || typeof row !== 'object' || ('seasonId' in row && row.seasonId != null && row.seasonId !== season))
                throw new Error('Archive contains malformed or wrong-season rows');
        }
    const data = { schema_version: 1, season, season_type: seasonType, source: 'Local archive', observed_at: null, fetched_at: null,
        bios: bios, stats: stats, goalies: goalies };
    const bytes = new TextEncoder().encode(JSON.stringify(data));
    if (bytes.length > MAX_EXPANDED_BYTES)
        throw new Error('Converted package exceeds 100 MiB');
    return bytes;
}
export async function importArchive(bytes, filename, seasonType) {
    if (bytes.length > MAX_ARCHIVE_BYTES)
        throw new Error('Compressed archive exceeds 25 MiB');
    if (typeof DecompressionStream === 'undefined')
        throw new Error('This browser cannot decompress archives. Import a browser-format JSON package instead.');
    const stream = new Blob([new Uint8Array(bytes).buffer]).stream().pipeThrough(new DecompressionStream('gzip'));
    return archivePackage(await readTarFiles(stream), filename, seasonType);
}
