function pause(milliseconds, signal) {
    return new Promise((resolve, reject) => {
        signal.throwIfAborted();
        const stop = () => { clearTimeout(timer); reject(signal.reason); };
        const timer = setTimeout(() => { signal.removeEventListener('abort', stop); resolve(); }, milliseconds);
        signal.addEventListener('abort', stop, { once: true });
    });
}
export function retryDelay(value, now) {
    if (value === null)
        return undefined;
    if (/^\d+$/.test(value.trim()))
        return Number(value) * 1000;
    const date = Date.parse(value);
    return Number.isFinite(date) ? Math.max(0, date - now) : undefined;
}
async function readBytes(response, signal, max, limitMessage) {
    if (Number(response.headers.get('content-length')) > max) {
        await response.body?.cancel();
        throw new Error(limitMessage);
    }
    const reader = response.body?.getReader();
    if (!reader)
        throw new Error('Source returned no body');
    let length = 0;
    const parts = [];
    const stop = () => { void reader.cancel(signal.reason); };
    signal.addEventListener('abort', stop, { once: true });
    try {
        for (;;) {
            signal.throwIfAborted();
            const { done, value } = await reader.read();
            signal.throwIfAborted();
            if (done)
                break;
            length += value.length;
            if (length > max)
                throw new Error(limitMessage);
            parts.push(value);
        }
        const bytes = new Uint8Array(length);
        let offset = 0;
        for (const part of parts) {
            bytes.set(part, offset);
            offset += part.length;
        }
        return bytes;
    }
    finally {
        signal.removeEventListener('abort', stop);
        await reader.cancel();
    }
}
// The caller supplies one foreground deadline shared by every report/page.
// Parse/schema failures are terminal; retry only transport/timeouts and safe HTTP reads.
async function readPublic(url, options, consume) {
    const now = options.now ?? Date.now;
    const request = options.fetch ?? fetch;
    const wait = options.wait ?? pause;
    for (let attempt = 0; attempt <= 3; attempt++) {
        options.signal.throwIfAborted();
        const remaining = options.deadline - now();
        if (remaining <= 0)
            throw new Error('Refresh deadline exceeded');
        const controller = new AbortController();
        const stop = () => controller.abort(options.signal.reason);
        options.signal.addEventListener('abort', stop, { once: true });
        const timer = setTimeout(() => controller.abort(new Error('Source request timed out')), Math.min(options.timeoutMs ?? 20000, remaining));
        let response;
        let failure;
        let delay = Math.min(1000 * 2 ** attempt, 8000);
        try {
            try {
                response = await request(url, { signal: controller.signal, cache: 'no-store', credentials: 'omit', redirect: 'error' });
            }
            catch (error) {
                failure = error;
            }
            if (options.signal.aborted && response)
                await response.body?.cancel(options.signal.reason);
            options.signal.throwIfAborted();
            if (response?.ok) {
                try {
                    const value = await consume(response, controller.signal);
                    options.signal.throwIfAborted();
                    if (now() >= options.deadline)
                        throw new Error('Refresh deadline exceeded');
                    return value;
                }
                catch (error) {
                    options.signal.throwIfAborted();
                    if (!controller.signal.aborted)
                        throw error;
                    failure = controller.signal.reason;
                }
            }
            else if (response) {
                await response.body?.cancel();
                if (![408, 429, 500, 502, 503, 504].includes(response.status))
                    throw new Error(`Source request failed (${response.status})`);
                delay = retryDelay(response.headers.get('retry-after'), now()) ?? delay;
                failure = new Error(`Source request failed (${response.status})`);
            }
        }
        finally {
            clearTimeout(timer);
            options.signal.removeEventListener('abort', stop);
        }
        if (attempt === 3)
            throw failure ?? new Error('Source unavailable');
        if (!Number.isFinite(delay) || delay >= options.deadline - now())
            throw new Error(`Source unavailable; retry allowed after ${Math.ceil(delay / 1000)} seconds`);
        await wait(delay, options.signal);
    }
    throw new Error('Source unavailable');
}
export function readPublicJSON(url, options) {
    return readPublic(url, options, async (response, signal) => {
        const bytes = await readBytes(response, signal, 2 * 1024 * 1024, 'Source response exceeds 2 MiB');
        return JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    });
}
export function readPublicBytes(url, maximumBytes, options) {
    if (!Number.isSafeInteger(maximumBytes) || maximumBytes <= 0 || maximumBytes > 100 * 1024 * 1024)
        throw new Error('Invalid public package byte limit');
    return readPublic(url, options, (response, signal) => readBytes(response, signal, maximumBytes, 'Package response exceeds its advertised byte limit'));
}
