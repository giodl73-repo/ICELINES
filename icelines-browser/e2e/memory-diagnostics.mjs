// Diagnostic intervention on the benchmark's own browser, never a user profile.
export async function diagnoseMemory(browser, context, page, workerUrl, changePhase, settle) {
  changePhase('diagnostic_before_gc');
  const browserSession = await browser.newBrowserCDPSession();
  const pageSession = await context.newCDPSession(page);
  let workerSessionId;
  let sequence = 0;
  const workerCommand = (method, params = {}) => new Promise((accept, reject) => {
    const id = ++sequence;
    const finish = (error, result) => {
      clearTimeout(timer);
      browserSession.off('Target.receivedMessageFromTarget', receive);
      error ? reject(error) : accept(result);
    };
    const receive = event => {
      if (event.sessionId !== workerSessionId) return;
      const message = JSON.parse(event.message);
      if (message.id !== id) return;
      finish(message.error ? new Error(`${method}: ${message.error.message}`) : undefined, message.result);
    };
    const timer = setTimeout(() => finish(new Error(`${method} timed out`)), 20000);
    browserSession.on('Target.receivedMessageFromTarget', receive);
    browserSession.send('Target.sendMessageToTarget', { sessionId: workerSessionId, message: JSON.stringify({ id, method, params }) }).catch(error => finish(error));
  });
  try {
    const targets = (await browserSession.send('Target.getTargets')).targetInfos.filter(target => target.type === 'worker' && target.url === workerUrl);
    if (targets.length !== 1) throw new Error(`Expected exactly one benchmark engine worker, got ${targets.length}.`);
    workerSessionId = (await browserSession.send('Target.attachToTarget', { targetId: targets[0].targetId, flatten: false })).sessionId;
    const snapshot = async () => {
      const wasm = await workerCommand('Runtime.evaluate', { expression: '(async () => { const mod = await import("../pkg/icelines_wasm.js"); const wasm = await mod.default(); return {linear_memory_bytes: wasm.memory.buffer.byteLength}; })()', awaitPromise: true, returnByValue: true });
      if (wasm.exceptionDetails || !Number.isFinite(wasm.result?.value?.linear_memory_bytes)) throw new Error('Worker WASM memory inspection failed.');
      return {
        process_types: (await browserSession.send('SystemInfo.getProcessInfo')).processInfo,
        main_heap: await pageSession.send('Runtime.getHeapUsage'),
        main_dom: await pageSession.send('Memory.getDOMCounters'),
        worker_heap: await workerCommand('Runtime.getHeapUsage'),
        worker_wasm: wasm.result.value,
      };
    };
    const result = { scope: 'Forced main/worker garbage collection is a diagnostic intervention, not normal app behavior or proof that the memory budget passes.' };
    changePhase('diagnostic_before_gc');
    result.before = await snapshot();
    await settle();
    changePhase('diagnostic_main_gc');
    await pageSession.send('HeapProfiler.collectGarbage');
    result.after_main_gc = await snapshot();
    await settle();
    changePhase('diagnostic_worker_gc');
    await workerCommand('HeapProfiler.collectGarbage');
    result.after_worker_gc = await snapshot();
    await settle();
    return result;
  } finally {
    if (workerSessionId) await browserSession.send('Target.detachFromTarget', { sessionId: workerSessionId }).catch(() => {});
    await pageSession.detach();
    await browserSession.detach();
  }
}
