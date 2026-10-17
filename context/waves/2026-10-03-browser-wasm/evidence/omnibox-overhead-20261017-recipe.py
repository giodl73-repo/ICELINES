from pathlib import Path
s=Path('icelines-browser/e2e/memory.mjs').read_text(encoding='utf-8')
s=s.replace("from '@playwright/test'","from '../icelines-browser/node_modules/@playwright/test/index.mjs'").replace("from './memory-diagnostics.mjs'","from '../icelines-browser/e2e/memory-diagnostics.mjs'").replace("from './memory-scope.mjs'","from '../icelines-browser/e2e/memory-scope.mjs'").replace("'../..'","'..'").replace('target/browser-memory-','target/omnibox-overhead-diagnostic-')
s=s.replace("const diagnostics = args.includes('--diagnose-gc');","const diagnostics = false;\nconst disableOmnibox=args.includes('--disable-omnibox-preload');")
needle="  browserServer = await chromium.launchServer({ channel: 'chromium', headless: true });"
assert s.count(needle)==1
s=s.replace(needle,"""  // Preserve every installed Playwright disabled feature in the overriding
  // flag. Never run/evaluate the bundled source; parse literal names only.
  const bundle=await readFile(resolve(repository,'icelines-browser/node_modules/playwright-core/lib/coreBundle.js'),'utf8');
  const list=bundle.match(/disabledFeatures = \\[([\\s\\S]*?)\\]\\.filter\\(Boolean\\);/)?.[1];
  if(!list)throw Error('Installed default feature list not found');
  const lines=list.split('\\n').map(line=>line.trim()).filter(line=>line&&!line.startsWith('//'));
  if(!lines.every(line=>/^"[A-Za-z0-9_]+",?$/.test(line)))throw Error('Default feature list is no longer literal-only');
  const defaults=lines.map(line=>line.match(/^"([A-Za-z0-9_]+)"/)[1]);
  if(defaults.length<5||new Set(defaults).size!==defaults.length)throw Error('Ambiguous feature defaults');
  const extra=['WebUIOmniboxPopup','WebUIOmniboxAimPopup'];
  if(extra.some(name=>defaults.includes(name)))throw Error('Omnibox already disabled by installed defaults');
  report.intervention={disabled: disableOmnibox,extra_features:extra,preserved_default_features:defaults,playwright_bundle_sha256:createHash('sha256').update(bundle).digest('hex')};
  browserServer = await chromium.launchServer({ channel: 'chromium', headless: true,...(disableOmnibox?{args:['--disable-features='+[...defaults,...extra].join(',')]}:{}) });""")
needle="  const rootProcessId = browserServer.process().pid;"
assert s.count(needle)==1
s=s.replace(needle,needle+"""
  {
    const command=spawnSync('powershell.exe',['-NoProfile','-Command',`(Get-CimInstance Win32_Process -Filter 'ProcessId=${rootProcessId}').CommandLine`],{encoding:'utf8',windowsHide:true});
    if(command.status!==0||!command.stdout.trim())throw Error('Owned root command line unavailable');
    report.launch_command_line=command.stdout.trim();
    const featureFlags=[...report.launch_command_line.matchAll(/(?:^|\\s)"?(--disable-features=[^"\\s]+)"?(?=\\s|$)/g)].map(match=>match[1]);
    if(!featureFlags.includes('--disable-features='+defaults.join(',')))throw Error('Parsed defaults differ from actual Playwright command');
    const effective=featureFlags.at(-1).slice('--disable-features='.length).split(',');
    const expected=disableOmnibox?[...defaults,...extra]:defaults;
    if(JSON.stringify(effective)!==JSON.stringify(expected))throw Error('Unexpected effective disabled features');
  }
""")
needle="  report.completed = true;"
assert s.count(needle)==1
s=s.replace(needle,"""  const inventory=await browser.newBrowserCDPSession();
  try{report.final_targets=(await inventory.send('Target.getTargets')).targetInfos;}finally{await inventory.detach();}
  const omniboxTargets=report.final_targets.filter(t=>t.url.startsWith('chrome://omnibox-popup.top-chrome'));
  if(disableOmnibox?omniboxTargets.length!==0:omniboxTargets.length!==2)throw Error('Omnibox target inventory does not confirm assigned condition');
  report.provisional_250_mib_target={sampled_private_bytes_below_target:null,sampled_sum_working_set_below_target:null,note:'Changed browser feature flags and extra command/target observations: diagnostic only, never original acceptance.'};
  report.target_scope='Full75/eight/two/20/restart diagnostic. Compare default with only two additional browser-owned omnibox preload features disabled; all original Playwright disabled features preserved. Official runner unchanged, no forced GC. No baseline subtraction or resource waiver.';
"""+needle)
Path('target/omnibox-overhead-diagnostic.mjs').write_text(s,encoding='utf-8')
print('Prepared scoped feature diagnostic; acceptance flags null in both conditions')
