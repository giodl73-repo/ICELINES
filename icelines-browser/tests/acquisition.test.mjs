import test from 'node:test';
import assert from 'node:assert/strict';
import { readPublicJSON, retryDelay } from '../dist/src/network.js';
import { refreshStats } from '../dist/src/acquisition.js';

const url = new URL('https://api.nhle.com/stats/rest/en/skater/summary');
function harness(responses) {
  let time = 0; const calls = []; const delays = [];
  return { calls, delays, signal: new AbortController().signal, deadline: 120000,
    now: () => time,
    wait: async delay => { delays.push(delay); time += delay; },
    fetch: async (target, options) => {
      calls.push({ target, options }); const response = responses.shift();
      if (response instanceof Error) throw response;
      assert.ok(response, 'Unexpected additional request'); return response;
    } };
}
const json = value => new Response(JSON.stringify(value));
test('safe reads retry transient failures, honor Retry-After, and omit credentials/cache', async () => {
  const h = harness([new TypeError('network'), new Response('', { status: 429, headers: { 'Retry-After': '5' } }), json({ ok: true })]);
  assert.deepEqual(await readPublicJSON(url, h), { ok: true });
  assert.deepEqual(h.delays, [1000, 5000]);
  for (const { options } of h.calls) assert.deepEqual([options.cache, options.credentials, options.redirect], ['no-store', 'omit', 'error']);
});
test('three retries is a hard per-request bound', async () => {
  const h = harness(Array.from({ length: 4 }, () => new Response('', { status: 503 })));
  await assert.rejects(readPublicJSON(url, h), /503/);
  assert.equal(h.calls.length, 4);
});
test('Retry-After beyond the foreground deadline stops without waiting', async () => {
  const h = harness([new Response('', { status: 429, headers: { 'Retry-After': '121' } })]);
  await assert.rejects(readPublicJSON(url, h), /retry allowed after 121 seconds/);
  assert.equal(h.delays.length, 0);
});
test('HTTP client errors and malformed JSON are terminal', async () => {
  for (const response of [new Response('', { status: 404 }), new Response('{broken')]) {
    const h = harness([response]); await assert.rejects(readPublicJSON(url, h)); assert.equal(h.calls.length, 1);
  }
});
test('oversized streamed responses are rejected even without Content-Length', async () => {
  const h = harness([new Response(new Uint8Array(2 * 1024 * 1024 + 1))]);
  await assert.rejects(readPublicJSON(url, h), /exceeds 2 MiB/);
});
test('cancellation stops before any request', async () => {
  const h = harness([]); const controller = new AbortController(); controller.abort();
  await assert.rejects(readPublicJSON(url, { ...h, signal: controller.signal }));
  assert.equal(h.calls.length, 0);
});
test('request timeout cancels pending transport and retries only within the bound', async () => {
  let calls = 0;
  const h = harness([]);
  h.fetch = async (_, { signal }) => { calls++; return await new Promise((_, reject) => signal.addEventListener('abort', () => reject(signal.reason), { once: true })); };
  await assert.rejects(readPublicJSON(url, { ...h, timeoutMs: 5 }), /timed out/);
  assert.equal(calls, 4);
});
test('Retry-After accepts HTTP dates and ignores invalid values', () => {
  assert.equal(retryDelay('Thu, 01 Jan 1970 00:00:05 GMT', 1000), 4000);
  assert.equal(retryDelay('invalid', 0), undefined);
});
test('body timeout cancels stalled streams and shares the same retry bound', async () => {
  let cancellations = 0;
  const h = harness([]);
  h.fetch = async () => new Response(new ReadableStream({ cancel() { cancellations++; } }));
  await assert.rejects(readPublicJSON(url, { ...h, timeoutMs: 5 }), /timed out/);
  assert.equal(cancellations, 4);
});
test('abort during a response body stops immediately without retrying', async () => {
  const controller = new AbortController(); const h = harness([]); let calls = 0; let cancellations = 0;
  h.fetch = async () => {
    calls++;
    return new Response(new ReadableStream({ start() { setTimeout(() => controller.abort(), 5); }, cancel() { cancellations++; } }));
  };
  await assert.rejects(readPublicJSON(url, { ...h, signal: controller.signal }));
  assert.equal(calls, 1); assert.equal(cancellations, 1);
});
test('refresh assembles all required reports with no more than two concurrent requests', async () => {
  let inFlight = 0; let peak = 0; const paths = [];
  const fetch = async target => {
    inFlight++; peak = Math.max(peak, inFlight); paths.push(target.pathname);
    await new Promise(resolve => setTimeout(resolve, 1)); inFlight--;
    return json({ data: [{ seasonId: 20242025, playerId: 1 }], total: 1 });
  };
  const bytes = await refreshStats(20242025, 'playoff', new AbortController().signal, { fetch });
  const data = JSON.parse(new TextDecoder().decode(bytes));
  assert.equal(peak, 2); assert.equal(paths.length, 3);
  assert.equal(data.season_type, 'playoff'); assert.equal(data.observed_at, null);
  assert.equal(data.goalies.length, 1);
});
test('incomplete/changing pagination never returns a replacement package', async () => {
  for (const first of [{ data: [], total: 1 }, { data: [{ playerId: 1 }], total: 2 }]) {
    const fetch = async target => target.searchParams.get('start') === '0' ? json(first) : json({ data: [{}], total: 3 });
    await assert.rejects(refreshStats(20242025, 'regular', new AbortController().signal, { fetch }), /pagination/);
  }
});
test('invalid season context is rejected before fetching', async () => {
  await assert.rejects(refreshStats(20242026, 'regular', new AbortController().signal, { fetch: async () => { assert.fail('must not fetch'); } }), /Invalid season/);
});

// Equal totals alone do not prove a complete offset-paginated report.
test('refresh requires ordered unique players and matching report coverage', async () => {
  const cases = [
    [{ playerId: 1 }, { playerId: 1 }],
    [{ playerId: 2 }, { playerId: 1 }],
    [{ playerId: 0 }],
    [{ playerId: '1' }],
  ];
  for (const rows of cases) {
    await assert.rejects(refreshStats(20242025, 'regular', new AbortController().signal, {
      fetch: async () => json({ data: rows, total: rows.length }),
    }), /pagination/);
  }
  await assert.rejects(refreshStats(20242025, 'regular', new AbortController().signal, {
    fetch: async target => json({ data: [{ playerId: target.pathname.endsWith('/bios') ? 1 : 2 }], total: 1 }),
  }), /coverage/);
});

test('duplicates across page boundaries cannot replace good data', async () => {
  await assert.rejects(refreshStats(20242025, 'regular', new AbortController().signal, {
    fetch: async target => {
      assert.deepEqual(JSON.parse(target.searchParams.get('sort')), [{ property: 'playerId', direction: 'ASC' }]);
      const start = Number(target.searchParams.get('start'));
      return json({ data: start === 0 ? Array.from({ length: 100 }, (_, i) => ({ playerId: i + 1 })) : [{ playerId: 100 }], total: 101 });
    },
  }), /duplicate or unordered pagination/);
});

test('ordered multiple pages retain every player exactly once', async () => {
  const bytes = await refreshStats(20242025, 'regular', new AbortController().signal, {
    fetch: async target => {
      assert.deepEqual(JSON.parse(target.searchParams.get('sort')), [{ property: 'playerId', direction: 'ASC' }]);
      const start = Number(target.searchParams.get('start'));
      return json({ data: Array.from({ length: start === 0 ? 100 : 1 }, (_, i) => ({ playerId: start + i + 1 })), total: 101 });
    },
  });
  const result = JSON.parse(new TextDecoder().decode(bytes));
  for (const name of ['bios', 'stats', 'goalies']) assert.deepEqual(result[name].map(r => r.playerId), Array.from({ length: 101 }, (_, i) => i + 1));
});
