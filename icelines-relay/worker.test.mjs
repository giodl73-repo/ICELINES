import test from 'node:test';
import assert from 'node:assert/strict';
import worker, { handle, upstreamURL, ORIGIN, MAX_BYTES } from './worker.mjs';
const path = '/stats/rest/en/skater/summary?isAggregate=false&isGame=false&start=0&limit=100&cayenneExp=seasonId%3D20252026+and+gameTypeId%3D2';
const request = (route = path, options = {}) => new Request('https://relay.example' + route, { headers: { Origin: ORIGIN }, ...options });
test('only fixed NHL hosts, reports, dates and bounded pages can be selected', () => {
  assert.equal(upstreamURL(request().url).hostname, 'api.nhle.com');
  assert.equal(upstreamURL(request('/v1/schedule/2026-10-04').url).hostname, 'api-web.nhle.com');
  for (const route of ['/proxy?url=https://evil.test', '/v1/schedule/2026-02-30', '/v1/schedule/2026-10-04?extra=1', path + '&start=100', path.replace('start=0', 'start=10000'), path.replace('limit=100', 'limit=1000'), path.replace('20252026', '20252027'), path + '&url=https://evil.test']) assert.throws(() => upstreamURL(request(route).url));
});
test('CORS, method and parameter rejection occur before upstream access', async () => {
  const noFetch = () => assert.fail('must not fetch');
  assert.equal((await handle(request(path, { headers: { Origin: 'https://evil.test' } }), noFetch)).status, 403);
  assert.equal((await handle(request(path, { method: 'POST' }), noFetch)).status, 405);
  assert.equal((await handle(request('/unknown'), noFetch)).status, 400);
  const preflight = await handle(request(path, { method: 'OPTIONS', headers: { Origin: ORIGIN, 'Access-Control-Request-Method': 'GET' } }), noFetch);
  assert.equal(preflight.status, 204); assert.equal(preflight.headers.get('Access-Control-Allow-Origin'), ORIGIN);
});
test('relay preserves public JSON, strips credentials and rejects redirects', async () => {
  const response = await handle(request(), async (url, options) => {
    assert.equal(url.hostname, 'api.nhle.com'); assert.equal(options.redirect, 'manual');
    assert.equal(options.headers.Origin, undefined); assert.equal(options.headers.Cookie, undefined);
    return new Response('{"data":[],"total":0}', { headers: { 'Content-Type': 'application/json', 'Set-Cookie': 'secret=1' } });
  });
  assert.equal(await response.text(), '{"data":[],"total":0}');
  assert.equal(response.headers.get('Set-Cookie'), null); assert.equal(response.headers.get('Cache-Control'), 'no-store');
  assert.equal(response.headers.get('Access-Control-Allow-Origin'), ORIGIN);
});
test('upstream failures, non-JSON and oversized streams never become success', async () => {
  for (const upstream of [new Response('', { status: 302, headers: { Location: 'https://evil.test' } }), new Response('blocked', { status: 403 }), new Response('<html>'), new Response(new Uint8Array(MAX_BYTES + 1), { headers: { 'Content-Type': 'application/json' } })]) {
    assert.equal((await handle(request(), async () => upstream)).status, 502);
  }
  const response = await handle(request(), async () => new Response('', { status: 429, headers: { 'Retry-After': '5' } }));
  assert.equal(response.status, 429); assert.equal(response.headers.get('Retry-After'), '5');
  const retryDate = 'Sun, 04 Oct 2026 23:00:00 GMT';
  const dated = await handle(request(), async () => new Response('', { status: 503, headers: { 'Retry-After': retryDate } }));
  assert.equal(dated.headers.get('Retry-After'), retryDate);
});
test('timeout covers a stalled body and cancels it', async () => {
  let cancelled = false;
  const response = await handle(request(), async () => new Response(new ReadableStream({ cancel() { cancelled = true; } }), { headers: { 'Content-Type': 'application/json' } }), 5);
  assert.equal(response.status, 504); assert.equal(cancelled, true);
});
test('rate limiting stops requests before upstream access and failures close access', async () => {
  const noFetch = () => assert.fail('must not fetch');
  const denied = await handle(request(), noFetch, 15000, { limit: async () => ({ success: false }) });
  assert.equal(denied.status, 429); assert.equal(denied.headers.get('Retry-After'), '60');
  assert.equal(denied.headers.get('Access-Control-Expose-Headers'), 'Retry-After');
  assert.equal((await handle(request(), noFetch, 15000, { limit: async () => { throw new Error('binding unavailable'); } })).status, 503);
  assert.equal((await worker.fetch(request(), {})).status, 503);
});
