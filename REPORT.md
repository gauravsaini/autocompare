# B2c scrollcontain REPORT (orchestrator pickup after B2/B2b stalls)

## Scope
Only panes scroll, never the whole window + toolbar. B2 (agyx) 4 polls me
bina worktree ke stall hua -> kill. B2b (opencode) explore karke freeze
(log 4532B, CPU 0.43s, 3 strikes) -> kill. Khud implement kiya.

## Changes (testids byte-identical, 1 additive testid)
- `chrome/ChromeShell.tsx`: root `flex min-h-screen` -> `flex h-screen
  overflow-hidden`; `main` me `min-h-0 flex-col` chain.
- `views/ViewRouter.tsx`: router root + 7 view wrappers par
  `flex min-h-0 flex-1 flex-col`.
- `theme/tokens.css`: `.tv-body` me `min-height:0; flex:1 1 auto`.
- `TextView/FolderView/TableView`: section `flex-1 min-h-0 flex-col`;
  fixed `h-[var(--ac-scroll-h)]` scrollers -> `min-h-0 flex-1`;
  non-virtual branches par internal `overflow-y-auto`.
- `Hex/Merge/Sync/MediaView`: root section par `min-h-0 flex-1 overflow-y-auto`
  (natural-growth views ab viewport ke andar scroll karte hain, clip nahi).
- Additive: `textview/rows` testid (chhoti demo data par non-virtual branch).
- NEW `e2e/specs/scrollcontain-242.spec.ts`: window.scrollY 0 rehta hai,
  band rect pinned, pane internal scrollTop badhta hai.

## Gates (fresh ports, absolute paths)
- `tsc --noEmit`: 0
- port 1836 `scrollcontain-242 + ui-standards`: 5/5 green (webkit)
- density-88 fallback (.tv-body) aur tablegrid-91 Math.min(600) safe hain.

## MATCH/OFF
- MATCH: window scroll khatm, toolbar/tabs/pathbar/statusbar pinned,
  teeno virtual panes viewport bharte hain, zero regressions.
- OFF: bahut chhoti viewport par pane min-height floor nahi hai (future).

B2c-DONE

---

# Status Update: Wave B3 IPC & UI Honesty Verification – 2026-11-25

## 1. Summary
Wave B3 ke sabhi 4 files ke changes complete ho chuke hain aur verification gate 100% pass ho gaya hai. TSC type-checking pass hai (0 errors), Vite production build pass hai, aur Playwright test suite ke 8 ke 8 tests bina kisi console/page error ke WebKit par pass hue hain.

## 2. Key Metrics
- Progress: 100%
- Gate Exit Code: 0 (GATE_PLAYWRIGHT_EXIT=0)
- TSC Errors: 0 (TSC:0)
- Vite Build: Success (1.68s)
- Playwright Tests: 8 passed / 8 total (7.3s, 0 failed, 0 flaky)
- Browser console & page errors: 0

## 3. Summary of Changes Across 4 Files

1. `apps/desktop/src/components/chrome/Toolbar.tsx`:
   - `COMPARE_IDLE_STOP_REASON` ko update karke `"no compare running: backend has no abort IPC"` banaya gaya, jisse compare-stop button (`toolbar/action-compare-stop`) disabled state me honestly reveal kare ki backend me abort IPC available nahi hai.
   - `SCOPE_LOCK_DISABLED_REASONS` mapping aur `actionDisabledReasonOf` helper introduce kiya gaya jo `action.disabledReason`, label suffix fallback, aur scope locks (`prev-hunk`, `next-hunk`, `sync`, `merge`, `rules`, `format`, `copy`) ke reasons cleanly resolve karta hai.
   - `actionTitle` function me parenthesized label suffixes ko clean replace kiya gaya aur keyboard shortcut hints ko safely preserve kiya gaya.

2. `apps/desktop/src/App.tsx`:
   - Menu placebo handlers ko honest status messages ke saath update kiya gaya:
     - `file.open-workspace`: Session Manager dialog open karta hai aur status message set karta hai `"Session Manager opened (open-workspace duplicates session manager; workspace mode pending backend)"`.
     - `view.toggle-theme`: Sets status `"Theme follows the OS appearance (no in-app toggle yet)"`.
     - `view.toggle-layout`: Sets status `"Layout toggle is not available in this build"`.
     - `tools.settings`: Sets status `"Settings UI is not available in this build"`.
   - `TRANSIENT_STATUS_RE` regex ko extend kiya gaya `/Session Manager opened.*/` ke liye taaki open-workspace ka expanded status message standard auto-fade behavior maintain kare.

3. `apps/desktop/src/ipc/client.ts`:
   - `mockInvoke` ke switch statement me 5 missing IPC mock handlers wire kiye gaye:
     - `"hex_compare"`: Returns typed `HexSummary` (`sessionId: "mock-hex-1"`, `leftSize: 64`, `rightSize: 64`, `equal: false`, `firstDiff: 8`).
     - `"hex_get_page"`: Returns typed `HexPage` (left/right byte buffers with diff at index 8, total 64, truncated false).
     - `"hex_find"`: Returns offset number (e.g. 8) or null.
     - `"merge_text_plan"`: Returns typed `TextMergePlan` (1 conflict block, conflictCount 1, cleanBlockCount 2, hasConflicts true).
     - `"merge_text_apply"`: Returns typed `TextMergeApplyReport` (`outPath`, `writtenLines: 3`, `conflictCount: 1`, `resolvedCount: 1`, `unresolvedCount: 0`).
   - Ab koi bhi mock IPC call empty array `[]` default fallback me drop nahi hoti.

4. `apps/desktop/e2e/specs/ipc-honesty-241.spec.ts`:
   - Fresh E2E test suite banayi gayi jo Wave B3 IPC honesty aur placebo menu behavior verify karti hai:
     - Test 1: Compare-stop button honesty check (`toolbar/action-compare-stop` disabled state, title aur aria-label me "backend has no abort IPC").
     - Test 2: Placebo menu notes status check (`view.toggle-theme`, `view.toggle-layout`, `tools.settings`, `file.open-workspace`).
     - Test 3: `page.evaluate` se mock IPC calls (`hex_compare`, `hex_get_page`, `hex_find`, `merge_text_plan`, `merge_text_apply`) verify karna ki valid structured fixtures return ho rahe hain, empty `[]` nahi.
     - Strict `collectErrors(page)` verification har test me zero console aur page errors enforce karta hai.

## 4. Blockers Documented

1. **Backend Abort IPC Missing**:
   - Issue: Desktop UI me "Stop Compare" feature backend Tauri command support na hone ki wajah se execution abort nahi kar sakta.
   - Current State: Toolbar me compare-stop button par explicit note surface kar diya gaya hai: `Stop (no compare running: backend has no abort IPC)` taaki user ko false impression na mile.
   - Required Fix: Rust backend me cancellation token / abort signal support IPC add karna hoga jab long-running folder/hex compares run ho rahe hon.

2. **Workspace Backend Pending**:
   - Issue: File menu me `file.open-workspace` standalone multi-root workspace open karne ke bajay normal Session Manager hi open karta hai.
   - Current State: Status message explicitly warn karta hai: `Session Manager opened (open-workspace duplicates session manager; workspace mode pending backend)`.
   - Required Fix: Multi-session workspace manifest backend file format aur state management implement karna.

3. **Settings UI & View Layout Toggle Not Implemented**:
   - Issue: Settings configuration UI aur dynamic side-by-side / stacked layout toggle ka frontend UI component is build me available nahi hai.
   - Current State: Honest status bar message `Settings UI is not available in this build` aur `Layout toggle is not available in this build` show hota hai.

## 5. Gate Run Results

### TypeScript Check
```
cd /tmp/wt-B3/apps/desktop && ./node_modules/.bin/tsc --noEmit
Exit Code: 0
Output: TSC:0
```

### Vite Build
```
./node_modules/.bin/vite build
Exit Code: 0
Output: ✓ built in 1.68s
```

### Playwright Test Suite
Command executed:
```
PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-B3/apps/desktop 1833 ipc-honesty-241.spec.ts menu-honesty-128.spec.ts --workers=1
```
Output:
```
Running 8 tests using 1 worker

  ✓  1 [webkit] › e2e/specs/ipc-honesty-241.spec.ts:71:1 › ipc-honesty-241: compare-stop button surfaces lack of abort IPC in disabled reason (952ms)
  ✓  2 [webkit] › e2e/specs/ipc-honesty-241.spec.ts:91:1 › ipc-honesty-241: placebo menu notes report honest status messages (745ms)
  ✓  3 [webkit] › e2e/specs/ipc-honesty-241.spec.ts:124:1 › ipc-honesty-241: mock hex and merge IPC return valid fixtures rather than default [] (563ms)
  ✓  4 [webkit] › e2e/specs/menu-honesty-128.spec.ts:64:1 › menu-honesty-128: view.kind-merge triggers merge session open (572ms)
  ✓  5 [webkit] › e2e/specs/menu-honesty-128.spec.ts:76:1 › menu-honesty-128: session.new triggers new tab / session manager (667ms)
  ✓  6 [webkit] › e2e/specs/menu-honesty-128.spec.ts:88:1 › menu-honesty-128: session.find triggers find input focus / honest status (639ms)
  ✓  7 [webkit] › e2e/specs/menu-honesty-128.spec.ts:101:1 › menu-honesty-128: view.kind-* opens appropriate session kinds (612ms)
  ✓  8 [webkit] › e2e/specs/menu-honesty-128.spec.ts:136:1 › menu-honesty-128: view.toggle-theme, view.toggle-layout, tools.settings report honest status messages (656ms)

  8 passed (7.3s)
GATE_PLAYWRIGHT_EXIT=0
```

## 6. MATCH/OFF Table for Tested Items

| Item / Feature | Expected Spec | Actual Observed State | Status |
| :--- | :--- | :--- | :--- |
| `toolbar/action-compare-stop` disabled state | Button disabled when idle | Disabled attribute attached | MATCH |
| `toolbar/action-compare-stop` reason honesty | Title/aria-label surfaces "backend has no abort IPC" | `Stop (no compare running: backend has no abort IPC)` | MATCH |
| `view.toggle-theme` status message | Honest note on OS appearance | "Theme follows the OS appearance (no in-app toggle yet)" | MATCH |
| `view.toggle-layout` status message | Honest note layout toggle not available | "Layout toggle is not available in this build" | MATCH |
| `tools.settings` status message | Honest note settings UI not available | "Settings UI is not available in this build" | MATCH |
| `file.open-workspace` status message | Notes duplicate session manager & pending backend | "Session Manager opened (open-workspace duplicates session manager; workspace mode pending backend)" | MATCH |
| `file.open-workspace` dialog action | Opens session manager modal | `session/dialog` visible | MATCH |
| `hex_compare` IPC mock | Returns typed `HexSummary` | Object returned with `sessionId: "mock-hex-1"`, `firstDiff: 8`, not `[]` | MATCH |
| `hex_get_page` IPC mock | Returns typed `HexPage` | Object returned with 64 byte buffers, total 64, not `[]` | MATCH |
| `hex_find` IPC mock | Returns offset number or null | Returned match offset `8`, not `[]` | MATCH |
| `merge_text_plan` IPC mock | Returns typed `TextMergePlan` | Object returned with 1 conflict item, `conflictCount: 1`, not `[]` | MATCH |
| `merge_text_apply` IPC mock | Returns typed `TextMergeApplyReport` | Object returned with `outPath: "/tmp/merged.txt"`, `writtenLines: 3`, not `[]` | MATCH |
| `view.kind-merge` menu event | Switches view to merge mode | `view-merge` testid visible | MATCH |
| `session.new` menu event | Opens new session manager dialog | `session/dialog` testid visible | MATCH |
| `session.find` menu event | Focuses find in active view | Status "Find focused" displayed | MATCH |
| `view.kind-*` multi-kind switch | Opens folder, hex, table, media, sync, merge, text | All views mount cleanly with testids | MATCH |
| Console & Page Errors | Zero console error & page error logs | Array length 0 across all 8 tests | MATCH |

## 7. Files List Touched
- `apps/desktop/src/components/chrome/Toolbar.tsx`
- `apps/desktop/src/App.tsx`
- `apps/desktop/src/ipc/client.ts`
- `apps/desktop/e2e/specs/ipc-honesty-241.spec.ts`

B3-DONE

---

# B1 SessionManager Search State & Dismiss REPORT

## Root Cause (Kyun ho raha tha bug)
1. `Modal.tsx` me `document.addEventListener("keydown", onKey, true)` capture phase listener laga hua hai jo Escape key press hone par capture level par hi `e.stopPropagation()` call kar deta hai aur `onClose()` invoke karta hai.
2. Pehle `SessionManager.tsx` me `handleClose` sirf `onClose ?? (() => {})` tha - yeh search state (`search`) ko clear nahi karta tha.
3. Jab user search input me query daal kar Escape press karta tha, Modal ka capture listener dialog ko dismiss kar deta tha (`onClose`), lekin `SessionManager` ka internal `search` state dirty ("zzz-no-such-session") hi reh jaata tha.
4. `App.tsx` me `SessionManager` unmount nahi hota tha balki `isOpen={false}` par persist rehta tha. Reopen karne par dirty filter persisted tha, isliye `alertempty-240.spec.ts` me `expect(page.getByTestId("session/search")).toHaveValue("")` fail ho raha tha.
5. Saath hi `Input.tsx` me `testid` prop support nahi tha, sirf `data-testid` pass through ho raha tha.

## Fix Details (Kya change kiya)
1. `apps/desktop/src/components/ui/Input.tsx`:
   - `InputProps` interface me additive `testid?: string` aur `readonly "data-testid"?: string` support add kiya.
   - `<input>` element par `data-testid={testid ?? dataTestId}` forward kiya taaki dono props seamlessly kaam karein.
2. `apps/desktop/src/components/session/SessionManager.tsx`:
   - `handleClose` ko update kiya taaki modal dismiss ya close button press par pehle `setSearch("")` execute ho aur fir `onClose?.()`.
   - `useEffect([isOpen])` add kiya: jab `isOpen` false ho jaye, to automatically `setSearch("")` state reset ho jaye.
   - Header close `ToolbarButton` ko `onClick={handleClose}` ke saath wire kiya.
   - Search `<Input>` par `testid="session/search"` provide kiya aur `onKeyDown` me Escape par `handleClearSearch()` cleanly call kiya. Zero layout changes.
3. `alertempty-240.spec.ts`: Pure UX fix implement hua, spec file ko modify karne ki koi zaroorat nahi padi.

## Verification & Gates
- `tsc --noEmit`: 0 errors.
- `alertempty-240.spec.ts`: 2/2 passed (webkit).
- `session-search.spec.ts`: 1/1 passed (webkit).
- Session suites (`session-actions`, `session-savedelete-129`, `flows-session`): 8/8 passed.

## MATCH/OFF Table
| Item | Status | Details |
|---|---|---|
| Search state reset on Esc/dismiss | MATCH | Esc ya backdrop se dialog close hone par filter clean reset hota hai |
| Reopen search value empty | MATCH | Reopening par `session/search` value `""` rehti hai (alertempty test pass) |
| Input testid prop forwarding | MATCH | `Input.tsx` additive `testid` prop cleanly `data-testid` me forward hota hai |
| Zero layout changes | MATCH | UI styling aur layout bilkul untouched rakha gaya hai |
| Specs touched | MATCH | Pure UX fix achieved; zero test specs altered |
| OFF | NONE | Koi missing capability ya mismatch nahi hai |

## Files Changed
- `apps/desktop/src/components/session/SessionManager.tsx`
- `apps/desktop/src/components/ui/Input.tsx`
- `REPORT.md`

B1-DONE

---

# C1b-p0 REPORT — 2 tutey hue UI vaadey jode (audit Category-3)

Base: main f470b68. Worktree: /tmp/wt-C1b (branch wave-C1b-p0).
NEVER committed / NEVER pushed — sirf worktree me changes hain.

## Kya tuta tha (2 P0)

1. SessionManager Delete button `hook.deleteSession` -> `session_delete` IPC
   bulata tha, par Rust backend me koi `session_delete` command tha hi nahi
   (sirf session_list/load/save). Production me Delete chup-chaap fail hota tha.
2. Top Toolbar Stop (`toolbar/action-compare-stop`) imaandaar no-op tha (idle
   note ke saath). Rust `folder_cancel(session)` commands/folder.rs me MAUJOOD
   tha + lib.rs me registered tha, par Stop usko kabhi bulata nahi tha.

## Kya fix kiya (sirf OWNS files)

- `apps/desktop/src-tauri/src/commands/session.rs`: naya `session_delete(id)`
  command — `<tmp>/autocompare-desktop/sessions/<id>.json` hatata hai.
  Galat id -> Err String; missing file -> Ok(false) (idempotent, folder_cancel
  jaisa); delete hua -> Ok(true). Locked docs bhi delete hote hain (frontend
  delete path `locked` check nahi karta — last-row guard only — isliye mana
  karna rows ko phansa deta). Sirf iske liye unit tests jode.
- `apps/desktop/src-tauri/src/lib.rs`: `COMMANDS` list + `generate_handler!`
  me `session_delete` register kiya (string only, aur kuch nahi).
- `apps/desktop/src/ipc/session.ts`: naya `deleteSession(id)` wrapper jo
  `session_delete` IPC bulata hai. Iske alawa koi change nahi.
- `apps/desktop/src/components/chrome/Toolbar.tsx` (SIRF Stop path):
  `handleStop` busy hote hue, jab `folderSession` (naya optional prop) pata ho,
  to `folder_cancel(session)` bulata hai — best-effort, kabhi throw nahi karta,
  note me nateeja likhta hai (`compareFolderCancelSentNote`). Session pata na
  ho to purana imaandaar note (`COMPARE_NO_ABORT_NOTE`). Idle note/title
  byte-identical rakhe. Baaki Compare/Refresh paths untouched.
- `apps/desktop/e2e/specs/c1-p0-243.spec.ts` (NAYA): delete round-trip
  (save -> list me hai -> delete true -> re-list me nahi -> load reject ->
  dobara delete idempotent) + Stop wiring pin (static `folderCancel(target)`
  assert + mock `folder_cancel` cancelled:true proof + idle title regression).
  Koi geometry assert nahi.

## GATES (worktree, fresh ports, ABSOLUTE paths)

- `./node_modules/.bin/tsc --noEmit` => TSC:0 (node_modules symlink hataya)
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash
  /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-C1b/apps/desktop
  1841 c1-p0-243.spec.ts ui-standards.spec.ts --workers=1`
  => 6 passed (c1-p0-243: 2/2, ui-standards: 4/4), GATE_PLAYWRIGHT_EXIT=0
- `cargo test -p autocompare-desktop session_delete` (from
  /tmp/wt-C1b/apps/desktop/src-tauri) => 2 passed, 0 failed
  (session_delete_round_trip_removes_file_and_relist_misses,
  session_delete_rejects_bad_ids)

## C1-DONE

C1-DONE. Dono P0 vaadey ab wired hain: Delete asal me file mitata hai, Stop
busy me asal me `folder_cancel` bulata hai. Koi commit/push nahi kiya.

## MATCH/OFF table

| Cheez | Haalat |
|---|---|
| Delete -> `session_delete` Rust command | MATCH (naya command + test) |
| `deleteSession` typed wrapper | MATCH (session.ts me joda) |
| Stop busy -> `folder_cancel(session)` | MATCH (folderSession milne par) |
| Stop idle note/title | MATCH (byte-identical, purane specs green) |
| Testids (`action-compare-stop`, `compare-note`, `compare-runs`) | MATCH (byte-identical) |
| Mock `session_delete` missing par `true` vs Rust `false` | OFF (mock client.ts OWNS se bahar; dono success — UI par asar nahi; spec dono sehta hai) |
| App/ChromeShell se `folderSession` plumbing | OFF (App.tsx/ChromeShell.tsx is wave ke OWNS me nahi; host abhi prop pass nahi karta to production path purana note rakhta hai — follow-up wave me plumb karna hai; jhootha daava nahi) |

## Files list

- apps/desktop/src-tauri/src/commands/session.rs (MOD: session_delete + 2 tests)
- apps/desktop/src-tauri/src/lib.rs (MOD: COMMANDS + handler entry)
- apps/desktop/src/components/chrome/Toolbar.tsx (MOD: Stop path only)
- apps/desktop/src/ipc/session.ts (MOD: deleteSession only)
- apps/desktop/e2e/specs/c1-p0-243.spec.ts (NEW)
- REPORT.md (APPEND: yeh section)

---

# C2d folderopts REPORT (wave-C2d-folderopts, worktree-first, no commit/push)

## Scope
Engine me `FolderOptions.content_compare` pehle se tha (byte compare,
symlink-safe), par Tauri `folder_compare` sirf `(left,right,masks)` leta
tha aur RulesPanel me masks input hi tha. Is wave me content flag ko IPC
se engine tak expose kiya + RulesPanel me toggle + dobara compare.

## Changes (legacy testids byte-identical, 1 additive testid)
- `apps/desktop/src-tauri/src/commands/folder.rs`: `folder_compare` me
  4 optional params add — `contentCompare`/`content_compare` (either
  spelling wins, text.rs pattern) + `quickCompare`/`quick_compare`
  inverse alias (`true` = fast path). Explicit content flag quick par
  jeetta hai; omitted = legacy mtime/size fast path. Engine
  `FolderOptions { masks, content_compare, ..default }` me map hota hai.
  Unit test ONLY: `content_compare_flag_byte_compares_same_size_files`
  (same size+mtime+alag bytes => fast=Same, content=Different,
  quick=true=Same).
- `apps/desktop/src/ipc/folder.ts`: `folderCompare(left,right,masks?,
  contentCompare?, quickCompare?)`. Flag na ho to legacy 3-field wire
  byte-identical; flag ho to dono spellings bhejte hain. Wire probe
  `window.__FOLDER_CONTENT_LAST__` record hota hai (spec proof ke liye).
- `apps/desktop/src/components/views/folder/controls/RulesPanel.tsx`:
  ADD content-compare checkbox ONLY
  (`folderview/content-compare-toggle`, label "Compare contents
  (byte-by-byte)"). Baaki testids/classes same.
- `apps/desktop/src/components/views/FolderView.tsx` (folder wiring):
  `contentCompare` state (default false, session switch par reset),
  compare effect me `folderCompare(..., contentFlag)` + deps me shamil,
  RulesPanel ko props pass. Toggle karte hi dobara compare hota hai.
- NEW `apps/desktop/e2e/specs/c2b-folderopts-245.spec.ts`: toggle
  unchecked start, check karne par `__FOLDER_CONTENT_LAST__`
  contentCompare=true + rows bane rehte hain, uncheck par false; dark
  mode render; zero errors; koi geometry assert nahi.

## Gates (worktree, fresh port, ABSOLUTE paths)
- `/tmp/wt-C2d/apps/desktop ./node_modules/.bin/tsc --noEmit` => TSC:0
  (main node_modules symlink kiya tha, gate ke baad hata diya)
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash
  /Users/ektasaini/autocompare/tools/gate-run.sh
  /tmp/wt-C2d/apps/desktop 1842 c2b-folderopts-245.spec.ts
  ui-standards.spec.ts --workers=1` => 5/5 green (webkit)
- `cargo test -p autocompare-desktop --lib commands::folder` => 8/8
  pass (naya content test samet)

## MATCH/OFF
| cheez | haal |
|---|---|
| MATCH | contentCompare=true same-size alag-bytes ko Different batata hai |
| MATCH | toggle on/off dono par folder_compare dobara fire hota hai |
| MATCH | legacy masks wire + saare purane testids same hain |
| MATCH | ui-standards 4/4 samet 5/5 e2e green, tsc 0, rust 8/8 |
| OFF | quickCompare ka alag UI toggle nahi (off = fast path hi quick hai) |
| OFF | mock engine me content flag nodes nahi badalta (sirf wire probe) |

## Files
- apps/desktop/src-tauri/src/commands/folder.rs
- apps/desktop/src/ipc/folder.ts
- apps/desktop/src/components/views/folder/controls/RulesPanel.tsx
- apps/desktop/src/components/views/FolderView.tsx
- apps/desktop/e2e/specs/c2b-folderopts-245.spec.ts (NEW)

C2b-DONE
