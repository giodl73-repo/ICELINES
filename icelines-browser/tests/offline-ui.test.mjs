import test from 'node:test';
import assert from 'node:assert/strict';
import { registerOfflineShell } from '../dist/src/offline.js';

for (const initiallyControlled of [false,true]) {
  test(`controller replacement reloads once after ${initiallyControlled ? 'existing control' : 'first installation'}`, async () => {
    const elements = new Map(['reload-app','offline-state','update-notice','update-app','repair-offline'].map(id => [id,{hidden:true}]));
    const listeners = new Map(); let reloads = 0;
    const active = {postMessage(){}};
    const registration = {active,waiting:null,addEventListener(){}};
    const serviceWorker = {controller:initiallyControlled ? active : null,
      addEventListener:(type,handler) => listeners.set(type,handler),register:async () => registration};
    const globals = {document:{getElementById:id => elements.get(id)},navigator:{serviceWorker},
      isSecureContext:true,location:{reload(){reloads++;}}};
    const originals = new Map(Object.keys(globals).map(key => [key,Object.getOwnPropertyDescriptor(globalThis,key)]));
    for (const [key,value] of Object.entries(globals)) Object.defineProperty(globalThis,key,{configurable:true,value});
    try {
      await registerOfflineShell();
      if (!initiallyControlled) {
        serviceWorker.controller = active;
        listeners.get('controllerchange')();
        assert.equal(reloads,0,'First installation must retain current in-memory work');
      }
      serviceWorker.controller = {postMessage(){}};
      listeners.get('controllerchange')();
      assert.equal(reloads,1,'A later approved update must reload the original installer tab too');
      listeners.get('controllerchange')();
      assert.equal(reloads,1,'Do not start duplicate reloads');
    } finally {
      for (const [key,descriptor] of originals) {
        if (descriptor) Object.defineProperty(globalThis,key,descriptor);
        else delete globalThis[key];
      }
    }
  });
}

test('offline UI requests verified readiness and exposes retryable repair without reloading work', async () => {
  const ids = ['reload-app','offline-state','update-notice','update-app','repair-offline'];
  const elements = new Map(ids.map(id => [id,{textContent:'',hidden:true,disabled:false}]));
  const listeners = new Map(); const sent = []; let registeredUrl; let reloads = 0;
  const active = {postMessage: message => sent.push(message)};
  const registration = {active,waiting:null,addEventListener(){}};
  const serviceWorker = {controller:active,addEventListener:(type,handler) => listeners.set(type,handler),register:async url => {registeredUrl = url.href; return registration;}};
  const globals = {document:{getElementById:id => elements.get(id)},navigator:{serviceWorker},isSecureContext:true,location:{reload(){reloads++;}}};
  const originals = new Map(Object.keys(globals).map(key => [key,Object.getOwnPropertyDescriptor(globalThis,key)]));
  for (const [key,value] of Object.entries(globals)) Object.defineProperty(globalThis,key,{configurable:true,value});
  try {
    await registerOfflineShell();
    const state = elements.get('offline-state'); const repair = elements.get('repair-offline');
    assert.match(state.textContent,/Checking/);
    assert.deepEqual(sent,[{type:'CHECK_SHELL'}]);
    const message = (type,scriptURL = registeredUrl) => listeners.get('message')({source:{scriptURL},data:{type}});
    message('SHELL_READY','https://other.test/sw.js'); assert.match(state.textContent,/Checking/);
    message('SHELL_UNAVAILABLE'); assert.equal(repair.hidden,false); assert.equal(repair.disabled,false);
    repair.onclick(); assert.equal(repair.disabled,true); assert.equal(sent.at(-1).type,'REPAIR_SHELL');
    message('SHELL_REPAIRING'); assert.match(state.textContent,/Downloading and verifying/);
    message('SHELL_REPAIR_FAILED'); assert.equal(repair.hidden,false); assert.equal(repair.disabled,false);
    assert.match(state.textContent,/Offline reopening remains unavailable/);
    repair.onclick(); message('SHELL_READY'); assert.equal(repair.hidden,true); assert.equal(repair.disabled,false);
    assert.match(state.textContent,/available offline/); assert.equal(reloads,0);
  } finally {
    for (const [key,descriptor] of originals) {
      if (descriptor) Object.defineProperty(globalThis,key,descriptor);
      else delete globalThis[key];
    }
  }
});
