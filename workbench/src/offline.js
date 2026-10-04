export async function registerOfflineShell() {
    const reload = document.getElementById('reload-app');
    if (reload)
        reload.onclick = () => location.reload();
    const state = document.getElementById('offline-state');
    const notice = document.getElementById('update-notice');
    const button = document.getElementById('update-app');
    const repair = document.getElementById('repair-offline');
    if (!state || !notice || !button || !repair)
        return;
    if (!('serviceWorker' in navigator) || !isSecureContext) {
        state.textContent = 'Offline shell unavailable in this browser context.';
        return;
    }
    let hadController = !!navigator.serviceWorker.controller;
    const workerUrl = new URL('../sw.js', import.meta.url);
    let reloading = false;
    navigator.serviceWorker.addEventListener('controllerchange', () => {
        if (!navigator.serviceWorker.controller)
            return;
        if (hadController && !reloading) {
            reloading = true;
            location.reload();
        }
        // The initial claim leaves this tab's work intact. A subsequent replacement
        // must reload it just like tabs opened after installation.
        hadController = true;
    });
    const showUpdate = () => { notice.hidden = false; };
    navigator.serviceWorker.addEventListener('message', event => {
        if (event.source?.scriptURL !== workerUrl.href)
            return;
        if (event.data?.type === 'SHELL_READY') {
            state.textContent = 'Application shell available offline. Saved datasets stay in your local library.';
            repair.hidden = true;
            repair.disabled = false;
        }
        if (event.data?.type === 'SHELL_UNAVAILABLE') {
            state.textContent = 'Cached application shell is missing or damaged. Keep this tab open and export your work; offline reopening is unavailable.';
            repair.hidden = false;
            repair.disabled = false;
        }
        if (event.data?.type === 'SHELL_REPAIRING') {
            state.textContent = 'Downloading and verifying this application’s offline files…';
            repair.hidden = false;
            repair.disabled = true;
        }
        if (event.data?.type === 'SHELL_REPAIR_FAILED') {
            state.textContent = 'Offline repair failed. Reconnect and retry. If the published build changed, save or export your work before updating. Offline reopening remains unavailable.';
            repair.hidden = false;
            repair.disabled = false;
        }
        if (event.data?.type === 'UPDATE_CONSENT_REQUIRED')
            showUpdate();
    });
    try {
        const registration = await navigator.serviceWorker.register(workerUrl, { updateViaCache: 'none' });
        repair.onclick = () => {
            if (!registration.active)
                return;
            repair.disabled = true;
            registration.active.postMessage({ type: 'REPAIR_SHELL' });
        };
        if (registration.active) {
            state.textContent = 'Checking the cached application shell…';
            registration.active.postMessage({ type: 'CHECK_SHELL' });
        }
        else
            state.textContent = 'Preparing the application for offline use…';
        if (registration.waiting)
            showUpdate();
        registration.addEventListener('updatefound', () => {
            const worker = registration.installing;
            worker?.addEventListener('statechange', () => {
                if (worker.state === 'installed' && registration.waiting && navigator.serviceWorker.controller)
                    showUpdate();
                if (worker.state === 'redundant')
                    state.textContent = 'Offline preparation failed. Reconnect and reload to try again.';
            });
        });
        button.onclick = () => {
            if (!registration.waiting) {
                notice.hidden = true;
                return;
            }
            registration.waiting.postMessage({ type: 'ACTIVATE_UPDATE' });
            state.textContent = 'Update requested. Other IceLines tabs must also choose update or close; click again after closing them. Saved datasets are retained.';
        };
    }
    catch (error) {
        state.textContent = `Offline preparation unavailable. You can continue online. ${error}`;
    }
}
