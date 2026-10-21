// Reversible paint/layout intervention in the benchmark's isolated browser only.
// This is attribution evidence, never an application optimization or budget pass.
export async function diagnoseRendering(browser, context, page, changePhase, settle) {
  const browserSession = await browser.newBrowserCDPSession();
  const pageSession = await context.newCDPSession(page);
  const original = await page.evaluate(() => ({
    style: document.documentElement.getAttribute('style'), scrollY: window.scrollY,
    resultCount: document.getElementById('rows').getAttribute('data-result-count'),
    context: document.getElementById('context').textContent,
  }));
  const snapshots = [];
  const restore = async () => page.evaluate(state => {
    if (state.style === null) document.documentElement.removeAttribute('style');
    else document.documentElement.setAttribute('style', state.style);
    window.scrollTo(0, state.scrollY);
  }, original);
  try {
    for (const [index, hidden] of [false, true, false, true, false].entries()) {
      const phase = `diagnostic_rendering_${index}_${hidden ? 'hidden' : 'visible'}`;
      changePhase(phase);
      if (hidden) await page.evaluate(() => document.documentElement.style.setProperty('display', 'none', 'important'));
      else await restore();
      await settle();
      const state = await page.evaluate(() => ({
        display: getComputedStyle(document.documentElement).display,
        bodyRects: document.body.getClientRects().length,
        resultCount: document.getElementById('rows').getAttribute('data-result-count'),
        context: document.getElementById('context').textContent,
        style: document.documentElement.getAttribute('style'),
      }));
      if ((state.display === 'none') !== hidden || (state.bodyRects === 0) !== hidden
        || state.resultCount !== original.resultCount || state.context !== original.context
        || (!hidden && state.style !== original.style)) throw new Error('Rendering intervention/restoration invariant failed.');
      snapshots.push({ phase, hidden, state,
        process_types: (await browserSession.send('SystemInfo.getProcessInfo')).processInfo,
        main_heap: await pageSession.send('Runtime.getHeapUsage'),
        main_dom: await pageSession.send('Memory.getDOMCounters') });
    }
    return {
      scope: 'After the unchanged full workload, hide/restore the root twice while retaining DOM, JS and engine state. No forced GC or launch changes. Timing, allocator and browser caches can obscure effects; snapshots are not disjoint additive allocations. All post-intervention phases are excluded from normal budget acceptance.',
      original, snapshots, restored: true,
    };
  } finally {
    await restore();
    await pageSession.detach();
    await browserSession.detach();
  }
}
