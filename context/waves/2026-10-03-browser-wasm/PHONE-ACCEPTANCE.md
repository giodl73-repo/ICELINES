# Physical phone acceptance

Status: **not run**. This is a procedure, not device evidence. Desktop viewport
emulation and the Windows process-memory results cannot close this gate.

Open https://giodl73-repo.github.io/ICELINES/workbench/ in the phone's normal
browser. Record the actual deployed build; PR #85 is not automatically published
by a merge. Use a public test season that is not already in your Saved library.
Keep existing saved data intact. Record phone model, OS/browser versions, date,
orientation and whether a debugger was attached.

## Touch and display

1. In portrait, load the test season from **Data library**. Record its season/type
   and the full result count in the query summary. The viewport renderer mounts
   only nearby rows; record mounted rows separately from complete results. Scroll
   through to the final rank and use **Show all players** to compare every name
   with the complete JSON export. Verify that loading ends, the active context stays visible,
   and the library collapses to show analysis.
2. Try Night Game, Fresh Ice and Hockey Club. Read player names, numeric columns,
   selected controls and source/status text in each. Reload online and check that
   the selected appearance is restored.
3. Scroll the leaders table horizontally in portrait, then rotate to landscape.
   Verify table scrolling stays within its region, controls remain reachable,
   and a player tap opens the correct name/detail. Close the dialog using its
   visible control. Test with the phone keyboard open as well as closed.
4. Apply `p>=1 AND gp>=1`, change sort, clear the filter and switch to goalies.
   Check that rows/columns follow the current kind and the tapped player belongs
   to the displayed result. Return to skaters before the persistence test.
5. With VoiceOver or TalkBack, check control names, query/loading status,
   table/header relationships and dialog navigation. Record what was tested;
   do not claim a complete assistive-technology audit from a visual pass.

## File picker, saving and offline reopening

1. **Export package** for the test season, find the download in the phone's file
   app, then **Import package** using the actual phone picker. Record filename,
   size, context, row count and any changed provenance. Confirm the result is
   usable but unsaved until **Save locally** is selected.
2. **Save locally**. Record the saved revision/context from its library entry.
   Load a second season without saving; confirm it is only in the Session library.
   If that season was already saved, choose another unsaved season.
3. Open **Application options** and wait for offline preparation to report ready.
   Record the exact status. Disable cellular and Wi-Fi, close the browser through
   the phone's app switcher, reopen it and return to the same workbench address.
   Record whether the app opens and which saved season it restores.
4. Confirm the saved season is usable and the unsaved second season was not
   persisted by this test. Apply a *new* filter and open player detail offline.
   Record fresh counts and the opened name; seeing an old table alone is not proof.
5. Try **Refresh live stats** while offline. Confirm a clear failure leaves the
   saved/current data usable. Re-enable networking; refresh supported season
   data, observe its source/time and save policy, then explicitly save if desired.
   Do not compare live row counts to an older bundled snapshot as a fixed golden.
6. Repeat download/picker round-trip for **Export JSON** and **Export CSV**.
   These are query exports, not importable dataset packages. Inspect that the
   downloaded content has the expected query/context and readable names.
7. Choose a schedule date with known games, refresh, save the schedule and export
   its package. Import through the phone picker, reload and verify saved schedule
   context. Record the date and source/time; an empty schedule may be valid for
   another week and does not prove the nonempty-table path.
8. **Restart engine** with saved data available; verify recovery returns a usable
   saved context. Record any app/tab termination or failed recovery explicitly.

For an inspector-assisted offline run, an uncached request must fail while the
fresh saved-data query succeeds. Physical radios off are the test setup; if the
browser reports connectivity or the request succeeds, investigate before calling
the offline check passed. Do not clear browser storage to simulate being offline.

## Timing and resource evidence

Record load/import/query start and completion with an instrument and units.
Collect at least 20 warm single-season query/render samples and report median
and maximum with device/browser/build and instrument overhead. Record startup,
package bytes, decompression/import cost, resident count and recovery behavior.
A stopwatch can show gross stalls but does not establish a 500 ms timing target.

Collect platform-supported process/resource evidence when available. JS heap,
WASM linear memory and tab/browser process allocations are different scopes;
label each. If total process peak is unavailable, write **unmeasured**. Do not
substitute the Windows counters or infer mobile success from no visible crash.

Android Chrome can be inspected from this Windows workstation: enable the
phone's USB debugging, connect it, accept its authorization prompt, and select
the workbench tab at `chrome://inspect/#devices`. Disable screencasting during
timing because it affects performance. Follow the
[official Chrome remote debugging guide](https://developer.chrome.com/docs/devtools/remote-debugging/).

iPhone/iPad Safari remote inspection uses Web Inspector on the device and Safari
on a connected Mac. A physical touch/offline/picker check can be run without that
connection, but instrumented results remain incomplete if unavailable. Follow
[WebKit's enabling instructions](https://webkit.org/web-inspector/enabling-web-inspector/).

## Evidence to return

Record the actual build from `build-info.json` / `shell-manifest.json`, device and
browser versions, test start/end, each case's pass/fail/not-run status, observed
counts/context, exported file sizes, timing samples and resource scope. Include
the uncached offline-probe outcome when instrumented. Keep personal filters,
unrelated browsing data and account tokens out of the public record. Screenshots
or screen recordings may support interaction findings but do not prove memory.

Full phone acceptance remains open until these results are obtained on the real
device and reviewed against the plan. The public rollback rehearsal remains a
separate pending action; this procedure does not publish anything.
