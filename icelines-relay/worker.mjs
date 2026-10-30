// Public, read-only NHL relay. No browser credentials or arbitrary target URLs.
export const ORIGIN = 'https://giodl73-repo.github.io';
export const MAX_BYTES = 2 * 1024 * 1024;
export function upstreamURL(input) {
  const url = new URL(input);
  if (/^\/v1\/schedule\/\d{4}-\d{2}-\d{2}$/.test(url.pathname)) {
    const date = url.pathname.slice('/v1/schedule/'.length);
    const parsed = new Date(date + 'T00:00:00Z');
    if (url.search || !Number.isFinite(parsed.getTime()) || parsed.toISOString().slice(0, 10) !== date) throw new Error('Invalid date');
    return new URL(url.pathname, 'https://api-web.nhle.com');
  }
  if (!['/stats/rest/en/skater/bios', '/stats/rest/en/skater/summary', '/stats/rest/en/goalie/summary'].includes(url.pathname)) throw new Error('Unknown route');
  const allowed = ['isAggregate', 'isGame', 'start', 'limit', 'cayenneExp'];
  if ([...url.searchParams.keys()].some(key => !allowed.includes(key)) || allowed.some(key => url.searchParams.getAll(key).length !== 1)) throw new Error('Invalid parameters');
  if (url.searchParams.get('isAggregate') !== 'false' || url.searchParams.get('isGame') !== 'false' || url.searchParams.get('limit') !== '100') throw new Error('Invalid report mode');
  const start = url.searchParams.get('start');
  if (!/^(0|[1-9]\d*)$/.test(start) || Number(start) >= 10000 || Number(start) % 100) throw new Error('Invalid page');
  const expression = /^seasonId=(\d{8}) and gameTypeId=([23])$/.exec(url.searchParams.get('cayenneExp'));
  if (!expression) throw new Error('Invalid season');
  const season = Number(expression[1]); const year = Math.floor(season / 10000);
  if (year < 1917 || year > new Date().getUTCFullYear() + 1 || season % 10000 !== year + 1) throw new Error('Invalid season');
  const target = new URL(url.pathname, 'https://api.nhle.com');
  for (const key of allowed) target.searchParams.set(key, url.searchParams.get(key));
  return target;
}
export async function handle(request, transport = fetch, timeoutMs = 15000, limiter) {
  const origin = request.headers.get('Origin');
  const headers = new Headers({ 'Content-Type': 'application/json', 'Cache-Control': 'no-store', 'Vary': 'Origin', 'X-Content-Type-Options': 'nosniff' });
  if (origin === ORIGIN) {
    headers.set('Access-Control-Allow-Origin', ORIGIN);
    headers.set('Access-Control-Expose-Headers', 'Retry-After');
  }
  const error = (status, message) => new Response(JSON.stringify({ error: message }), { status, headers });
  if (origin && origin !== ORIGIN) return error(403, 'Origin not allowed');
  let target;
  try { target = upstreamURL(request.url); } catch { return error(400, 'Unsupported NHL request'); }
  if (request.method === 'OPTIONS') {
    if (request.headers.get('Access-Control-Request-Method') !== 'GET' || request.headers.get('Access-Control-Request-Headers')) return error(403, 'Preflight not allowed');
    headers.set('Access-Control-Allow-Methods', 'GET');
    return new Response(null, { status: 204, headers });
  }
  if (request.method !== 'GET') { headers.set('Allow', 'GET, OPTIONS'); return error(405, 'Read-only endpoint'); }
  if (limiter) {
    try {
      const { success } = await limiter.limit({ key: request.headers.get('CF-Connecting-IP') ?? 'unknown-client' });
      if (!success) { headers.set('Retry-After', '60'); return error(429, 'Refresh limit reached; try again in a minute'); }
    } catch { return error(503, 'Request limiting unavailable'); }
  }
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  const abort = () => controller.abort();
  request.signal.addEventListener('abort', abort, { once: true });
  let reader;
  try {
    if (request.signal.aborted) throw new Error('Request cancelled');
    // Workers supports manual/follow, whereas browser fetch also supports error.
    // A redirect is rejected below without issuing a second upstream request.
    const response = await transport(target, { signal: controller.signal, redirect: 'manual', headers: { 'Accept': 'application/json', 'User-Agent': 'IceLines/0.26 public NHL relay' } });
    if (!response.ok) {
      await response.body?.cancel();
      const status = [408, 429, 500, 502, 503, 504].includes(response.status) ? response.status : 502;
      const retry = response.headers.get('Retry-After');
      if (retry && retry.length <= 128 && (/^\d{1,10}$/.test(retry) || Number.isFinite(Date.parse(retry)))) headers.set('Retry-After', retry);
      return error(status, `NHL upstream returned ${response.status}`);
    }
    if (!/\bapplication\/json\b/i.test(response.headers.get('Content-Type') ?? '') || Number(response.headers.get('Content-Length')) > MAX_BYTES) {
      await response.body?.cancel(); return error(502, 'Invalid NHL response');
    }
    reader = response.body?.getReader();
    if (!reader) return error(502, 'Empty NHL response');
    const stop = () => { void reader.cancel().catch(() => {}); };
    controller.signal.addEventListener('abort', stop, { once: true });
    try {
      let length = 0; const parts = [];
      for (;;) {
        if (controller.signal.aborted) throw new Error('Request cancelled');
        const { done, value } = await reader.read();
        if (controller.signal.aborted) throw new Error('Request cancelled');
        if (done) break;
        length += value.byteLength;
        if (length > MAX_BYTES) return error(502, 'NHL response exceeds limit');
        parts.push(value);
      }
      const bytes = new Uint8Array(length); let offset = 0;
      for (const part of parts) { bytes.set(part, offset); offset += part.byteLength; }
      return new Response(bytes, { headers });
    } finally { controller.signal.removeEventListener('abort', stop); }
  } catch (failure) {
    console.warn('IceLines public NHL relay failure', failure?.name, String(failure?.message).slice(0, 200));
    return error(controller.signal.aborted ? 504 : 502, 'NHL request unavailable');
  }
  finally { clearTimeout(timer); request.signal.removeEventListener('abort', abort); await reader?.cancel().catch(() => {}); }
}
export default {
  fetch(request, env) {
    // Fail closed if the production rate-limit binding was not deployed.
    if (!env.NHL_RATE_LIMIT) return new Response('{"error":"Relay configuration incomplete"}', { status: 503, headers: { 'Content-Type': 'application/json', 'Cache-Control': 'no-store' } });
    return handle(request, fetch, 15000, env.NHL_RATE_LIMIT);
  }
};
