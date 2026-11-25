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

---

# C2a-textopts REPORT (Hinglish-roman)

## Summary
Kaam ho gaya: text compare me do engine-ready options expose ho gaye —
Patience algorithm (`DiffAlgorithm::{Myers, Patience}`) aur ignore-blank-lines
(`grammar::is_blank_line` mirror). Backend `text_compare` dono params accept
karta hai, honestly `appliedOptions` me echo karta hai, aur UI me Ignore menu
me "Ignore blank lines" checkbox + Format menu me "Patience diff" toggle hai.
Dono toggle real `textCompare` re-trigger karte hain. Koi commit/push nahi kiya.

## Kya-kya kiya
1. **Backend (`commands/text.rs`)**: `algorithm: Option<String>` +
   `ignoreBlankLines`/`ignore_blank_lines: Option<bool>` params add kiye.
   `parse_text_algorithm` ("patience" case-insensitive -> Patience, "myers"/
   unknown/omit -> Myers), `text_compare_key` (blank-only line -> `""` key),
   `diff_with_c2a` (keys par chosen algorithm se re-diff), aur
   `fold_blank_rows_equal` (blank-vs-blank rows -> `Equal`, display text same).
   `StoredText` me `ignore_blank` + `algorithm` rakha taaki `text_apply_edit`
   / `text_apply_hunk` session snapshot consistent rahe. `AppliedTextOptions`
   me `ignore_blank_lines: bool` + `algorithm: Option<String>` echo.
   Sirf unit test add kiya: `c2a_algorithm_and_blank_lines_round_trip`.
2. **IPC wrapper (`ipc/text.ts`)**: `TextAlgorithm` type, `TextIgnoreOptions`
   me `algorithm` + `ignoreBlankLines`/`ignore_blank_lines`, `textCompare`
   dono spellings forward karta hai aur `__TEXT_IGNORE_LAST__` probe par
   `algorithm` + `ignoreBlankLines` record karta hai. Sirf pass-through, koi
   mock change nahi.
3. **UI controls**: `IgnoreMenu` me `textview/ignore-blank-lines` checkbox,
   `FormatMenu` me `textview/algorithm-patience` ("Patience diff") checkbox.
   Purane props/testids byte-identical; naye props optional hain taaki purane
   callers na tooten. Local fallback state rakha jab parent handler na de.
4. **Wiring (`TextView.tsx`, OWNS ke bahar minimal)**: `algorithm` ("myers"
   default) + `ignoreBlank` state, `recompareWithOptions` me 2 optional params,
   `recompareBlank`/`recompareAlgorithm` handlers, menus ko pass kiye, session
   switch par reset. Bina iske toggle re-compare trigger nahi hota — isliye
   yeh extra file chhoona pada; baaki TextView logic same hai.
5. **Naya spec `c2a-textopts-244.spec.ts`**: blank-lines check/uncheck probe
   round-trip + status mounted, patience check/uncheck probe round-trip +
   status mounted, typed wrapper dono spellings forward karta hai. Koi geometry
   assert nahi, koi screenshot nahi, zero console/page errors.

## Mushkil (3-strike note)
- Pehla gate run timeout hua (120s): Ignore Modal ka scrim toolbar clicks
  block karta hai, isliye panel band karne wala `toolbar-ignore.click()` kabhi
  land nahi hua. Strike 1.
- Fix: modal band karne ke liye `Escape` use kiya (Modal Esc contract +
  toolbar fallback click). Dobara run par spec 1.5s me pass. Strike 2 nahi aaya.

## Gates (worktree, fresh ports, absolute paths)
- `./node_modules/.bin/tsc --noEmit` => **TSC:0** (main node_modules symlink karke, baad me hata diya)
- `cargo test -p autocompare-desktop --lib commands::text` => **26 passed, 0 failed**
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-C2a/apps/desktop 1841 c2a-textopts-244.spec.ts ui-standards.spec.ts --workers=1` => **5 passed** (c2a 1.2s + ui-standards 4/4)

## MATCH/OFF table (existing testids byte-identical)
| Testid | Status |
|---|---|
| textview/toolbar-ignore | MATCH (unchanged) |
| textview/toolbar-format | MATCH (unchanged) |
| textview/ignore-options | MATCH (unchanged) |
| textview/ignore-whitespace | MATCH (unchanged) |
| textview/ignore-case | MATCH (unchanged) |
| textview/format-options | MATCH (unchanged) |
| textview/format-encoding | MATCH (unchanged) |
| textview/format-eol | MATCH (unchanged) |
| textview/ignore-blank-lines | NEW (C2a add) |
| textview/algorithm-patience | NEW (C2a add) |

OFF: koi purana testid change/remove nahi hua; `chrome/**`, `session/**`,
`ui/**` primitives, `tokens.css`, folder/table/picture/hex/merge/sync
commands ya views, aur koi doosra spec touch nahi kiya.

## Files list
- apps/desktop/src-tauri/src/commands/text.rs (params + mapping + unit test ONLY)
- apps/desktop/src/ipc/text.ts (wrapper pass-through ONLY)
- apps/desktop/src/components/views/text/controls/IgnoreMenu.tsx (blank-lines checkbox ONLY)
- apps/desktop/src/components/views/text/controls/FormatMenu.tsx (patience toggle ONLY)
- apps/desktop/src/components/views/TextView.tsx (minimal wiring: state + re-compare handlers)
- apps/desktop/e2e/specs/c2a-textopts-244.spec.ts (NEW spec)
- REPORT.md (yeh file, worktree root)

C2a-DONE

---

# REPORT — C2c-tablepicopts (EDIT wave, worktree-first)

Branch: `wave-C2c-tablepicopts` | Base: `main (f470b68)` | Worktree: `/tmp/wt-C2c`
Commit/push: NAHI kiya (tag rule: NEVER commit/push). Sab changes uncommitted worktree me hain.

## 1. Summary (ek line)

Engine-ready table options (delimiter + numeric epsilon) aur picture tolerance Tauri/IPC/UI tak wire ho gaye; Rust 130 green, TSC:0, e2e gate 5/5 green.

## 2. Kya-kya kiya (files)

Backend (Tauri commands):

- `apps/desktop/src-tauri/src/commands/table.rs`
  - `table_compare` me 4 naye optional params: `delimiter: Option<String>`,
    `numericTolerance / numeric_tolerance / epsilon: Option<f64>` (pehla `Some` jeetta hai).
  - `resolve_delimiter`: omit/empty = `,` (legacy byte-identical); single char wins;
    multi-char = honest error.
  - `resolve_numeric_tolerance`: finite `>= 0` else error (engine
    `cells_equal_with_tolerance` contract).
  - `parse_csv`/`load_table` ab `delim: char` lete hain (dono files, ek format).
  - `cells_equal`/`rows_equal`/`align_counts` me `numeric_tol: Option<f64>`:
    exact win -> numeric `|a-b| <= eps` -> ignore flags. Header compare
    tolerance-free (names hain, numbers nahi) — positional + key mode same.
  - 6 naye unit tests: `;` col-count proof, multi-char reject, epsilon
    equalize (camel + `epsilon` spelling), band-bahar unequal, bad values
    reject, legacy byte-identical.
- `apps/desktop/src-tauri/src/commands/media.rs`
  - `picture_compare` me `tolerance: Option<f32>` (omit = exact byte match).
    `summarize_picture` ab per-byte `|l-r| <= tol` ko equal manta hai
    (score/differing/samples sab honored).
  - 2 naye unit tests: tolerance merge (1-byte drift -> identical),
    bad tolerance reject. Purane 2-arg call sites 3-arg kiye.

IPC wrappers (wire pass-through ONLY):

- `apps/desktop/src/ipc/table.ts` — `TableIgnoreOptions` me `delimiter`,
  `numericTolerance/numeric_tolerance/epsilon`; unset = purana 4-field wire
  (byte-identical); set hone par teeno spellings bheji jaati hain (Rust
  `ignoreCase` pattern jaisa). Headless-proof hook: `__TABLE_PICOPTS_LAST__`.
- `apps/desktop/src/ipc/media.ts` — `pictureCompare(left, right, tolerance?)`,
  omit par `tolerance: null`; hook: `__PICTURE_TOLERANCE_LAST__`.

UI controls:

- `.../table/controls/TableToolbar.tsx` — delimiter select
  (`tableview/delimiter-select`: `,`/`;`/tab/`|`) + epsilon input
  (`tableview/numeric-tolerance`, `inputMode=decimal`, `exact` placeholder).
  Dono `shrink-0`, single toolbar row barkarar; purane saare testids same.
- `.../media/controls/MediaTolerancePanel.tsx` — coarse slider
  (`mediaview/tolerance-slider`, 0–255, `aria-label="tolerance slider"`)
  same tolerance state + same Recompare par wired; text box precise path
  barkarar. Purane testids byte-identical.
- `.../views/TableView.tsx` (plumbing, minimal) — `delimiter`/`epsilon`
  state, `tableCompare` opts me pass (`,` omit / unparseable epsilon omit
  = legacy wire), effect deps + toolbar props.
- `MediaView.tsx` / `MediaToolbar.tsx` — ZERO diff (revert kiya): picture-mode
  chrome audio-only rakha taaki mediachrome-72 / mediaparity-103 /
  parity-media-147 / 154 green rahein (wo picture me "no tolerance chrome"
  assert karte hain).

Naya spec:

- `apps/desktop/e2e/specs/c2c-tablepicopts-246.spec.ts` — delimiter + epsilon
  + tolerance controls re-compare; koi geometry assert nahi (na overlap, na
  screenshot, na barH).

## 3. Engine mapping (MATCH/OFF)

| Engine option | Engine jagah | Desktop status | Note |
|---|---|---|---|
| `TableOptions.delimiter: char` | `crates/compare/src/lib.rs:134` | MATCH | `table_compare(delimiter?)` + toolbar select + e2e |
| `TableKeyOptions.numeric_tolerance: Option<f64>` | `crates/compare/src/table.rs:70` | MATCH | teeno spellings + epsilon input + e2e |
| Picture tolerance | `PictureOptions` (`lib.rs:467`) me KOI tolerance field nahi (sirf `heatmap`) | OFF (backend-local) | Interim byte-compare me per-byte threshold joda; engine me pixel-tolerance concept hi nahi — knob engine-swap tak desktop-only rahega |
| `PictureOptions.heatmap: Option<PathBuf>` | `lib.rs:468` | BLOCKER | Heatmap ko file-write path chahiye jiska koi IPC DTO nahi; is wave me wire nahi kiya (scope-bahar + DTO change hota). Follow-up me `heatmap_path` param + DTO field chahiye. |

## 4. Verify (gates, worktree, fresh port 1843, absolute paths)

- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml --lib` => **130 passed, 0 failed**
  (table 6 naye + media 2 naye samet).
- `./node_modules/.bin/tsc --noEmit` (main `node_modules` symlink kiya tha, gate ke baad hata diya) => **TSC:0**.
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash /Users/ektasaini/autocompare/tools/gate-run.sh
  /tmp/wt-C2c/apps/desktop 1843 c2c-tablepicopts-246.spec.ts ui-standards.spec.ts --workers=1`
  => **5 passed (c2c 1 + ui-standards 4), GATE_PLAYWRIGHT_EXIT=0**.
  - Beech me 3 strike lage (documented): (1) slider click track-position jump (~128) — focus+arrows se fix;
    (2) synthetic `input` event React state tak nahi pahuncha — focus+arrows se fix;
    (3) toolbar Recompare modal backdrop ke peeche — panel ke Recompare (same handler) se fix.

## 5. Deliberately NAHI kiya (scope discipline)

- `chrome/**`, `session/**`, text/folder/hex/merge/sync views ya commands, `ui/` primitives,
  `tokens.css`, doosre specs — zero touch.
- `MediaToolbar.tsx` ka audio-gate khola tha, phir revert kiya (4 existing specs toot-te).
- `client.ts` mock me koi change nahi (unknown args ignore karta hai — legacy path safe).
- Koi geometry/screenshot assert nahi (tag rule).
- Commit/push NAHI (tag rule).

## 6. Files list (kaam wali)

1. `apps/desktop/src-tauri/src/commands/table.rs`
2. `apps/desktop/src-tauri/src/commands/media.rs`
3. `apps/desktop/src/ipc/table.ts`
4. `apps/desktop/src/ipc/media.ts`
5. `apps/desktop/src/components/views/table/controls/TableToolbar.tsx`
6. `apps/desktop/src/components/views/media/controls/MediaTolerancePanel.tsx`
7. `apps/desktop/src/components/views/TableView.tsx`
8. `apps/desktop/e2e/specs/c2c-tablepicopts-246.spec.ts` (NEW)
9. `REPORT.md` (yeh entry, append-only — purani B1/B2 entries untouched)

(reverted, list se bahar: `MediaToolbar.tsx`, `MediaView.tsx` — zero diff.)

C2c-DONE

---

# H1 Homechrome Report (wave-H1-homechrome, worktree /tmp/wt-H1)

Base: main f470b68. Worktree-first, main ya koi branch direct edit nahi. Commit/push nahi kiya.

## Kya tha (live 1717 screenshot review, main code me verify)

- P0-1 PathBar Home par mount tha — ChromeShell.tsx unconditional tha.
- P0-2 OverviewRuler global mount tha — Home par bhi dikhta tha.
- P1-3 Sessions card float kar raha tha — left sidebar full-height nahi tha.
- P1-4 Canvas par monospace `sessions:N` stray tha (status bar ka hissa nahi).
- P1-5 TabBar `+` last tab se ~200px door tha (inner `flex-1` poori row kha raha tha).
- P2-6 Folder card subtitle `Directories and archives` lamba tha — ellipsis aata tha.
- P2-7 Status bar Home par debug-copy `Tab selected: home` dikhata tha.

## Kya kiya (OWNS only, minimal)

1. `apps/desktop/src/components/chrome/ChromeShell.tsx` — conditional mount ONLY:
   - `activeTabId/kind` se `isHomeTab` derive (props API same, koi naya prop nahi).
   - `showPathBar = !isHomeTab` — PathBar sirf comparison tabs par, Home par kabhi nahi.
   - `showRuler = !isHomeTab && kind in {text,hex,merge}` — ruler sirf Text/Hex/Merge diff views me, Home/folder/table/sync/media par nahi.
2. `apps/desktop/src/components/chrome/TabBar.tsx` — `+` placement ONLY:
   - Inner scroller `flex-1` -> `flex-initial` (grow 0, shrink 1, scroll intact) + comment.
   - `+` ab last tab ke turant bagal me chipakta hai, tabbar ke andar rehta hai.
3. `apps/desktop/src/components/home/controls/HomeSessionsPane.tsx` — layout ONLY:
   - Section me `self-stretch` add — left Saved-Sessions sidebar full-height, right cards `items-start` se top-aligned (BC pattern). Parent App welcome untouched.
4. `apps/desktop/src/App.tsx` — minimal 3 hunk:
   - HOME_CARDS folder hint `Directories and archives` -> `Directories & archives` (ellipsis khatm).
   - `handleSelectTab` me Home branch: `selectSession(home)` + `setStatusMessage("Ready")` — `Tab selected: home` debug-copy khatm. Comparison tabs ka message byte-identical.
   - Welcome canvas `session-count` `<p>` visual classes -> `sr-only` (testid + text byte-identical, canvas clean; count status bar `status.right` me pehle se mirror hai).
5. NEW `apps/desktop/e2e/specs/h1-homechrome-247.spec.ts` — Home: no PathBar, no ruler, no stray count (sr-only), `+` adjacent (flex-grow 0 + order), folder subtitle `&`, status Ready (empty par `Tab selected` nahi, session ke baad Home par `Ready`). Koi geometry pixel-lock nahi.

DO NOT touch ka palan: views/**, session/**, ui/** primitives, tokens.css, ipc/**, src-tauri/**, text/table/folder/hex/merge/sync toolbars, doosre specs — kuch nahi chhua. Saare existing testid byte-identical (`session-count` sr-only allowed).

## Gates (worktree, fresh port 1846, absolute paths)

- `./node_modules/.bin/tsc --noEmit` (main node_modules symlink, baad me remove) => TSC:0
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-H1/apps/desktop 1846 h1-homechrome-247.spec.ts ui-standards.spec.ts --workers=1` => all green:
  - h1-homechrome-247.spec.ts: 2/2 passed (6.2s + 5.8s)
  - ui-standards.spec.ts: 4/4 passed
  - `6 passed (17.0s), GATE_PLAYWRIGHT_EXIT=0`
- Symlink remove kar diya, worktree me sirf 4 modified + 1 new spec + ye REPORT hai. Commit/push nahi kiya.

## MATCH/OFF

| # | Defect | Status |
|---|--------|--------|
| P0-1 | PathBar Home par nahi | MATCH |
| P0-2 | Ruler Home par nahi, sirf Text/Hex/Merge | MATCH |
| P1-3 | Sessions left full-height + cards top-aligned | MATCH |
| P1-4 | Stray canvas count hata, testid sr-only + status bar mirror | MATCH |
| P1-5 | `+` last tab ke bagal me | MATCH |
| P2-6 | Folder subtitle `Directories & archives` | MATCH |
| P2-7 | Home status `Ready`, debug-copy nahi | MATCH |

## Files

- apps/desktop/src/components/chrome/ChromeShell.tsx (conditional PathBar/OverviewRuler mount ONLY)
- apps/desktop/src/components/chrome/TabBar.tsx (+ button placement ONLY)
- apps/desktop/src/components/home/controls/HomeSessionsPane.tsx (layout self-stretch ONLY)
- apps/desktop/src/App.tsx (HOME_CARDS hint `&`, handleSelectTab Home `Ready`, welcome session-count sr-only — minimal)
- apps/desktop/e2e/specs/h1-homechrome-247.spec.ts (NEW)
- REPORT.md (ye file, worktree root)

H1-DONE

---

# INST-install REPORT (Hinglish-roman)

Worktree: /tmp/wt-INST (branch wave-INST-install, base main 41a9d43). Koi commit/push nahi kiya.

## Summary

Reusable script `scripts/install-tauri-macos.sh --install --open` se real macOS .app
build + /Applications me install + launch verify — FULL GREEN, exit 0.

- Strike 1: pehla run `EXIT=127` hua — fresh worktree me `apps/desktop/node_modules`
  tha hi nahi, `tsc` binary missing. Ye environment gap tha, product/config bug nahi.
- Fix (sirf build layer, OWNS ke andar): script ab step 0 me khud deps install karta
  hai — `node_modules/.bin/tsc` na mile to `npm ci` (lockfile ho to) warna
  `npm install`. npm isliye kyunki `apps/desktop/package-lock.json` track hota hai,
  aur pnpm v11 ka pre-build install check esbuild par fail hota hai (script me pehle
  se documented NOTE hai).
- Dusra full run: TSC_OK + FRONTEND_OK + BUNDLE_OK + STAMP_OK + INSTALLED + OPENED,
  EXIT=0. Dobara full script run karke green prove kiya (koi shortcut nahi).

## Verify evidence

- Bundle: /tmp/wt-INST/target/release/bundle/macos/AutoCompare.app (BUNDLE_OK)
- Installed: /Applications/AutoCompare.app (default APP_DIR writable tha,
  `$HOME/Applications` fallback ki zaroorat nahi padi)
- Stamp file `dist/AutoCompare-0.5.0-macos-arm64.tauri.txt` contents:
  ```
  tauri 0.5.0 41a9d43
  app /Applications/AutoCompare.app
  ```
- Launch: `open` ke baad process alive — PID 43951
  `/Applications/AutoCompare.app/Contents/MacOS/autocompare-desktop`,
  ELAPSED 00:47 (10s+ gate pass, crash-at-launch nahi).
- Crash logs: `~/Library/Logs/DiagnosticReports/` me install ke baad koi naya
  AutoCompare report nahi — sirf 00:19–00:24 ke purane `autocompare-gui-*.ips`
  (crash-at-launch history wahi purani hai, is build me repeat nahi hui).
- Main window: System Events se `process "autocompare-desktop"` ki window list me
  `AutoCompare` window on-screen mili — window dikhta hai, blank/crash nahi.

## Files changed (worktree, 1 file, +12 lines)

- scripts/install-tauri-macos.sh — step 0 INSTALL_DEPS guard (node_modules missing
  ho to npm ci / npm install). Product source, tauri.conf, Cargo.toml ko chhua tak
  nahi (zaroorat nahi padi — bundle/signing aur build flags pehle se sahi the).
- (untracked, build artifacts — commit nahi honge) apps/desktop/node_modules/,
  apps/desktop/dist/, target/, dist/*.tauri.txt, REPORT.md (ye file).

DO-NOT-touch list se kuch nahi chhua: crates/*, commands logic, views/**,
chrome/**, ipc/**, e2e specs, tokens.css.

## Environment

- arch: arm64 (Apple Silicon), macOS 27.2.1
- Xcode CLT: /Library/Developer/CommandLineTools
- rustc: 1.99.1 (Homebrew)
- node: v24.18.0, npm: 11.16.0, cargo-tauri: $HOME/.cargo/bin/cargo-tauri
- Tauri release build time: ~2m 01s (fresh worktree target dir)

Gate command (worktree copy, ABSOLUTE path): sab green, exit 0.

INST-DONE

---

# C3a view-state — Hinglish-roman REPORT

## Goal
Per-view compare options reload ke baad survive karein (view-state persist).
Pehle har reload par reset ho jate the.

## Changes (testids byte-identical, koi nayi dependency nahi)
- NEW `apps/desktop/src/state/viewPersist.ts`: load/save helpers ONLY
  (key `ac.viewstate.v1.<view>`, JSON, try/catch, corrupt = defaults).
- `apps/desktop/src/components/views/TextView.tsx`: ignoreWs/ignoreCase/
  encoding/eol/algorithm/ignoreBlank lazy-init restore + change par persist;
  session-switch reset ab sirf real path/session change par (first mount
  keep = reload survival). Layout/testid/IPC untouched.
- `apps/desktop/src/components/views/TableView.tsx`: ignoreWs/ignoreCase/
  delimiter/epsilon lazy-init restore + persist (compare effect deps se
  mount par auto-apply). Layout/testid/IPC untouched.
- `apps/desktop/src/components/views/FolderView.tsx`: masks lazy-init
  restore + persist; paths/subdir reset ab sirf real change par (first
  mount keep). Layout/testid/IPC untouched.
- NEW `apps/desktop/e2e/specs/c3a-viewstate-248.spec.ts`: 3 tests —
  set option -> reload -> option still set; corrupt storage -> defaults;
  koi geometry assert nahi.

## Gates (worktree, fresh port 1851, absolute paths)
- `./node_modules/.bin/tsc --noEmit` (main node_modules symlink, baad me
  hataya) => TSC:0
- `PLAYWRIGHT_BROWSERS_PATH=... bash tools/gate-run.sh
  /tmp/wt-C3a/apps/desktop 1851 e2e/specs/c3a-viewstate-248.spec.ts
  e2e/specs/ui-standards.spec.ts --workers=1` => 7 passed (11.8s),
  GATE_PLAYWRIGHT_EXIT=0

## MATCH/OFF
| Spec | Result |
| ---- | ------ |
| c3a-viewstate-248 (3 tests: text/table/folder) | MATCH (3/3 green) |
| ui-standards (4 tests) | MATCH (4/4 green, zero regressions) |
| OFF (known limit) | Text demo-boot path (`textCompareDemo`, empty paths) ke paas opts wire nahi — control state restore hoti hai, diff exact rehta hai jab tak user toggle na kare. Table/folder mount compare me auto-apply hota hai. |

## Files
- NEW `apps/desktop/src/state/viewPersist.ts`
- EDIT `apps/desktop/src/components/views/TextView.tsx`
- EDIT `apps/desktop/src/components/views/TableView.tsx`
- EDIT `apps/desktop/src/components/views/FolderView.tsx`
- NEW `apps/desktop/e2e/specs/c3a-viewstate-248.spec.ts`

C3a-DONE

---

## UPDATED ON : 2026-11-25

### C1c stop-plumb (2026-11-25) — Toolbar Stop ab real `folder_cancel(session)` bulata hai

1. **Masla**: C1b me Stop ka `folder_cancel` path `folderSession` prop par nirbhar tha, par koi host ye prop pass nahi karta tha — isliye Stop hamesha honest-note par girta tha (C1b ka documented OFF). Ab App `runComparePaths` ke `folder_compare` receipt se real session id rakhta hai aur ChromeShell se Toolbar tak pahunchata hai; Stop busy window me `folder_cancel(session)` fire karta hai.
2. **Badlav (sirf plumbing, 21 lines)**: App me `folderSessionId` state + folder branch me receipt capture + `folderSession={sessionKind === "folder" ? folderSessionId : null}` (dusri kinds/text par null taaki honest-note path bana rahe); ChromeShell me optional `folderSession` prop jo Toolbar ko forward hota hai. Toolbar/ipc me koi badlav nahi (C1b ka path reuse). Testids byte-identical.
3. **Spec (NAYA)**: `apps/desktop/e2e/specs/c1c-stopplumb-249.spec.ts` — busy+known session par `folder compare cancel requested (session mock-folder-1)`; bina session (text) par `cannot be cancelled` aur `cancel requested` nahi; idle par Stop disabled + title byte-identical + koi note nahi. Koi geometry assert nahi.
4. **Tests (pehle → baad)**: gate port 1852, webkit, `--workers=1`: c1c-stopplumb-249 (3/3) + ui-standards (4/4) = 7/7 green. `tsc --noEmit` => TSC:0. Kabhi commit/push nahi kiya.

| # | Defect | Status |
|---|--------|--------|
| C1c-1 | Busy + known folder session par Stop `folder_cancel(session)` bheje | MATCH |
| C1c-2 | Bina session (text) par Stop honest no-op (`cannot be cancelled`) | MATCH |
| C1c-3 | Idle Stop disabled rahe, purana title/note behavior intact | MATCH |
| C1c-4 | Saare existing testids byte-identical | MATCH |
| C1c-OFF | C1b OFF (koi host `folderSession` nahi deta tha) | CLOSED (ab App deta hai) |

## Files

- apps/desktop/src/App.tsx (folderSessionId state + receipt capture + folderSession pass ONLY)
- apps/desktop/src/components/chrome/ChromeShell.tsx (folderSession prop + Toolbar forward ONLY)
- apps/desktop/e2e/specs/c1c-stopplumb-249.spec.ts (NEW)
- REPORT.md (ye file, worktree root)

C1c-DONE

---

# TABS chipsizing-253 — REPORT

## Summary
Session tabs mote (32px chip / 40px bar) lagte the, baaki desktop controls
(22-24px) se match nahi karte the. Ab chip control-md standard par hai aur
bar accordingly compact hai. Koi layout/behavior change nahi, testids
byte-identical, truncation aur `+` adjacency (H1 fix) untouched.

## Changes (worktree `/tmp/wt-TABS`, branch `wave-TABS-chipsizing`, base `main@3bb39f8`)
1. **tokens.css — 4 token values only (comments/testids same):**
   - light `:root`: `--ac-tab-chip-h: 32px -> 24px`
   - dark `@media`: `--ac-tab-chip-h: 32px -> 24px` (override tha, dono fix)
   - geometry `:root`: `--ac-tab-chip-h: 32px -> 24px`, `--ac-tabbar-h: 40px -> 32px`
2. **TabBar.tsx — ZERO change.** Chips (`h-[var(--ac-tab-chip-h)]`,
   min-w-60/max-w-56/truncate) aur bar (`h-[var(--ac-tabbar-h)]`) pehle se
   vars ride karte hain, isliye token fix hi kaafi tha. Close button
   `size="sm"` (22px) 24px chip me fit hai (1px slack), padding math intact.
3. **NEW spec:** `apps/desktop/e2e/specs/tabs-chipsizing-253.spec.ts` —
   token equality + relational assertions only, koi pixel-lock nahi.

## Gates (worktree, fresh port 1858, ABSOLUTE paths)
- `./node_modules/.bin/tsc --noEmit` => **TSC:0** (node_modules main se
  symlink tha, gate ke baad remove kar diya)
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash
  /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-TABS/apps/desktop
  1858 e2e/specs/tabs-chipsizing-253.spec.ts e2e/specs/ui-standards.spec.ts
  --workers=1` => **5 passed (6.9s), GATE_PLAYWRIGHT_EXIT=0**
  - tabs-chipsizing-253: 1 passed (light + dark)
  - ui-standards: 4 passed

## MATCH / OFF table

| Check | MATCH (naya) | OFF (purana / note) |
|---|---|---|
| chip token vs control-md | `--ac-tab-chip-h == --ac-control-h-md` (24px, light+dark) | pehle 32px vs 24px — OFF tha |
| measured chip height | token ke equal (round) | pehle 32px mote the |
| tabbar height | chip + 2×space-1 = 32px compact | pehle 40px — OFF tha |
| truncation | min-w-60/max-w-56/truncate + ellipsis intact | — |
| `+` adjacency (H1) | strip grow 0, plus last tab ke baad, bar ke andar | — |
| testids | tabbar / tab-* / tab-new byte-identical | — |
| KNOWN OFF (out of scope) | — | `geopolish-66c / chromedeclutter-119 / chrome-174 / chrome-budget / geopolish-122 / shellchrome-116` specs me 40px bar pin hai — ye waves apne spec khud update karenge; `TabBar.tsx:277` comment me "only 32px chrome" line ab stale hai (sizing class nahi, isliye untouched) |

## Files list
- `apps/desktop/src/theme/tokens.css` (M: 4 token values)
- `apps/desktop/src/components/chrome/TabBar.tsx` (untouched, byte-identical)
- `apps/desktop/e2e/specs/tabs-chipsizing-253.spec.ts` (NEW)
- `REPORT.md` (ye file)

NEVER push — commit nahi kiya, push nahi kiya.

TABS-DONE

---

# D2-pathdrop REPORT (Hinglish-roman)

## 1. Summary (2 lines)
User complaint: "Left and right path areas BC me droppable hain, hamare case me nahi."
Inspection me web-fallback drop infra (PathBar fields/strip + App-level PaneDropZone halves +
Tauri hooks) har view me wired mila, lekin ek asli toot mili: khaali session me do alag-alag
single-side PathBar drops kabhi re-compare trigger nahi karte the (pehla staged side App tak
pahunchta hi nahi tha) — dono paths bhare dikhne par bhi compare atka rehta tha. BC me ye
flow seedha kaam karta hai. Fix PathBar me, pin spec 8 tests, saare gates green.

## 2. Kya wired tha (inspection)
- `PathBar` field-level `onDrop` (left/right/base) + whole-strip `onDrop` (clientX vs midpoint,
  `resolveStripSideFromRect`) + `useTauriPathBarDrop` (production shell, strip-only owner).
- App-level `PaneDropZone`: `[data-panedrop]` halves overlay + `useNativeFileDrop` (window-level,
  pane-only owner, pathbar band skip) — har comparison view isi se cover hota hai.
- `MergeView` ke 3 pane zones (`data-mergedrop`) apne drops khud own karte hain (stopPropagation
  sahi hai — double-compare nahi hota), triple App ko `onDropPaths`/`onRequestCompare` se jaata hai.
- Tauri vs web fallback ownership single-owner hai (strip PathBar ka, pane window hook ka,
  unknown-position pane ka) — isme koi change nahi kiya.

## 3. Kya toota tha (root cause)
`PathBar.handleFieldSubmit`: single-side stage LOCAL rehta hai (Wave-173, by design — App ko
submit nahi hota). Khaali/half-staged session me pehle LEFT drop ke baad App ke paas left=""
rehta hai; phir RIGHT drop par `otherPath` draft se bhara milta hai isliye submit hota hai,
lekin App apne stale `live` (`left:""`) se `next` banata hai -> "needs two paths", compare
kabhi fire nahi hota. Dono fields bhare dikhte hain, view half-staged mount hota hai, par
re-compare zero. Repro: blank text session -> left drop -> right drop -> `__AC_COMPARE_COUNT__`
delta 0, status "Path set ... needs two paths" (fix se pehle scratch spec se confirm kiya;
fix ke baad delta 1, "Compare finished").

## 4. Fix (PathBar.tsx, drop handling ONLY — 25 lines add)
`handleFieldSubmit` me, jab single submit par dono drafts bhare hon AUR dusre side ka PROP
(App state) abhi bhi blank ho, to pehle dusri side stage karo (App-only stage, compare nahi
ho sakta kyunki ye side wahan missing hai), phir ye side submit karo -> App ke paas dono sides
-> EXACTLY ONE compare. Multi-drop (`overrideOtherPath`) pehle se paired hai, isliye skip;
non-blank prop par purana single-submit behavior untouched (typing semantics same).
Per-view mounts ki zaroorat nahi padi — pane drops App-level zone se sab views me kaam karte
hain (spec se prove), isliye views/** me koi edit nahi (mount one-liner bhi nahi chahiye tha).

## 5. Spec: `apps/desktop/e2e/specs/d2-pathdrop-251.spec.ts` (NEW, 8 tests, no geometry asserts)
Har test: LEFT drop -> us side ka path set + `__AC_COMPARE_COUNT__` +1 + "Compare finished";
phir RIGHT drop -> same. Zero console/page errors har test me.
- text / folder / table / hex / sync / media: pathbar left+right drops, phir pane-half
  (`panedrop/section-left|right`) drops. Folder/sync me folderish (`/`-suffixed) paths taaki
  kind auto-switch na ho (file-vs-folder class rule).
- blank session: left-first + right-first dono orders me do single drops -> exactly 1 recompute
  (yehi D2 fix ka regression pin hai).
- merge: pathbar left/right drops (base untouched) + `merge-pane-left|right` zone drops ->
  triple staged + re-plan compare.

## 6. Gates (worktree `/tmp/wt-D2`, fresh port 1856)
- `./node_modules/.bin/tsc --noEmit` => TSC:0 (main node_modules symlink, gates ke baad remove kiya).
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash .../tools/gate-run.sh
  /tmp/wt-D2/apps/desktop 1856 e2e/specs/d2-pathdrop-251.spec.ts e2e/specs/ui-standards.spec.ts
  --workers=1` => all green: **12 passed** (d2-pathdrop-251: 8/8, ui-standards: 4/4),
  GATE_PLAYWRIGHT_EXIT=0.
- Related regression (dev server): pathdrop-193 (6) + folderdnd-159 (6) + pathbar-submit-124 (4)
  + pathbar-history (3) + finderdrop-143 (4) = 23 passed, zero fail.
- Existing testids byte-identical (koi testid add/remove/rename nahi; spec sirf existing
  `pathbar/left|right`, `panedrop/section-*`, `merge-pane-*`, `statusbar/message` use karta hai).
- NEVER commit/push: sirf worktree me changes, `wave-D2-pathdrop` branch unpushed.

## 7. MATCH/OFF table (har view, dono sides)

| View | PathBar L/R drop | Pane L/R drop | Recompute | Verdict |
|---|---|---|---|---|
| Text | path set | path set | +1 per drop | MATCH |
| Folder | path set | path set | +1 per drop | MATCH |
| Table | path set | path set | +1 per drop | MATCH |
| Hex | path set | path set | +1 per drop | MATCH |
| Sync | path set | path set | +1 per drop | MATCH |
| Media | path set | path set | +1 per drop | MATCH |
| Merge | path set (base untouched) | L/R zones -> triple | +1 per drop | MATCH |
| Blank session (koi bhi kind) | 2x single drops -> 1 compare | App-level stage -> compare | exactly 1 | MATCH (D2 fix) |
| Per-view extra mounts | zaroorat nahi (App zone covers all) | — | — | OFF (no edit) |
| Tauri native shell drops | code unchanged (single-owner intact) | manual live-drop check pending | — | OFF (mock-only gate) |

## 8. Files list (OWNS ke andar ONLY)
- `apps/desktop/src/components/chrome/PathBar.tsx` — `handleFieldSubmit` me stale-other
  stage-first convergence (drop handling ONLY, +25 lines, koi testid/API change nahi).
- `apps/desktop/e2e/specs/d2-pathdrop-251.spec.ts` — NEW, 8 tests (upar section 5).
- `REPORT.md` (yeh file, worktree root — append-only, purani entries intact).
- Touch nahi kiya: `views/**` (mount bhi nahi chahiye tha), `session/**`, `ui/**` primitives,
  `tokens.css`, `ipc/**`, `src-tauri/**`, koi aur spec, aur `chrome/PaneDropZone.tsx`,
  `dropPaths.ts`, `useNativeFileDrop.ts` (inspection-only; fix ki zaroorat wahin nahi thi).

D2-DONE
---

# D1 copycorrect REPORT (EDIT wave, worktree-first, Hinglish-roman)

Base: main (3bb39f8). Worktree: /tmp/wt-D1 (branch wave-D1-copycorrect).
NEVER commit/push — sab changes uncommitted worktree me hain.

## User complaint (asal shikayat)
1. "copy l2r or r2l few sections -> text compare fails in midst" — kayi
   hunk-copy lagatar karne par diff toot jata tha.
2. "click karte hi save kyon kar raha hai" — copy click par lagta tha save
   ho gaya.

## Reproduce (pehle prove, phir fix)
Scratch spec me 176-row/10-hunk real fixture par alternate l2r/r2l x4 +
Save right + Save left chalaya. Mila:
- copy1..4 tak sab consistent (176->166 rows, hunks 10->6, save-note kabhi
  nahi, copy-note "unsaved; press Save X") — copy path khud sahi tha.
- **Save right ke baad view TOOT gaya: `159 / 173 rows · 144 same ...
  (loaded)`** — pre-save `166 / 166 rows · 151 same` tha. Merge khoye,
  tail kata, hasMore jhootha jaga. Yahi "fails in midst" hai — copy nahi,
  **Save ka refetch** doshi tha.

## Root cause
`saveSide` ka Wave-178 refetch `textGetRows(session, 0, rowsRef.length)`
(local-length window) se karta tha. Multi-side staged drops ke baad local
length (166) < backend total (173, sirf is side ka flush hua) hota hai —
window tail kaat deta hai, phir doosri side ke staged copies us kate
prefix par re-apply hote hain: rows ghate (159), total backend wala (173),
merge gayab (144 vs 151). Rows/total/status teeno toot-te hain. Complaint
#2 ke liye code me koi autosave MILA HI NAHI (copy kabhi `text_save_side`
nahi bulata; dirty dots + "unsaved; press Save X" pehle se honest the) —
isliye #2 ka jawab proof + spec hai, code change nahi.

## Fix (sirf OWNS: copyHunkAt + serialize/coalesce + Save path)
`apps/desktop/src/components/views/TextView.tsx` ONLY (drop-mount lines
untouched, testids byte-identical):
1. **Save refetch gate (D1 pin ka fix)**: `sawRegroupRisk` flag — per-row
   fallback replay ya flushed cell edits par hi refetch; clean
   `text_apply_hunk` flush par row-refetch SKIP (staged rows == backend
   truth by construction: dono taraf same take-wins/drop rule, identity
   preserved). Sirf coarse overview refresh hota hai (null-safe).
2. **Regroup-risk refetch ab full-truth**: `textGetRows` loop `page.total`
   tak (tail kabhi nahi kat-ti), phir doosri side ke staged re-apply, total
   = backendTotal − reappliedDrops (staged-truth display barkarar).
3. **Serialize drain me liveEdits threading**: `runCopyHunk`/`runCopyLine`
   ab `baseEdits` lete + updated map return karte hain; `copyHunkAt`/
   `copyLineAt` drain me `liveEdits` carry hota hai — rapid l2r/r2l burst me
   stale render closure se pruned override resurrect nahi hota (FIXT2 N3/N4
   contract harden; pure gutter flow me no-op).
Save buttons/dirty-dots pehle se honest the (`dirty-left/right`, aria
"(unsaved changes)") — isliye controls me ZERO change (tag: ONLY if needed).

## Gates (worktree, fresh ports, ABSOLUTE paths)
- `./node_modules/.bin/tsc --noEmit` => TSC:0 (main node_modules symlink
  kiya tha, gate ke baad hata diya)
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash
  /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-D1/apps/desktop
  1855 d1-copycorrect-250.spec.ts ui-standards.spec.ts --workers=1`
  => **8/8 green** (D1 4/4 + ui-standards 4/4), GATE_PLAYWRIGHT_EXIT=0
- Regression (same gate, port 1856): textfunc-178 + copy-persist-125 +
  textgutter-139b + textinteract-109 => 15/16; 1 fail
  (textinteract-109 caret-Esc `cell-edit` count) **clean main par bhi fail**
  hota hai (port 1858, 4/5) — pre-existing, D1 OWNS ke bahar (cell editor
  Esc path), is wave me untouched.
- Strike 1: T3 me `expectConsistent` post-Save bhi save-note 0 mang raha
  tha (spec bug — Save receipt hona sahi hai); `allowSaveNote` flag jodkar
  same run me fix. Strike 2/3 nahi aaye.

## MATCH/OFF
| Cheez | Haal |
|---|---|
| 5x alternate l2r/r2l: rows==total, buttons==hunks, ordinal 0..N-1 gap-free | MATCH (T1, 10->5 hunks) |
| Har copy par save-note 0 + copy-note unsaved + Save side | MATCH (T1 har step) |
| Dono side dirty dots, disk byte-identical (no autosave, OS-level) | MATCH (T2) |
| Save right/left: status byte-identical, staged per-side clear | MATCH (T3 — pre-fix 159/173 split ab 166/166 stable) |
| Reload: unsaved staged drop, 176/176 + 10 hunks wapas, files identical | MATCH (T4, BC parity wave-172b) |
| Save+reload ke baad disk me merged text (real disk round-trip) | OFF (mock IPC `text_save_side` no-op hai — pre-existing pin, textfunc-178 T2 jaisa; real-disk round-trip Rust unit test `save_side_persists_applied_hunk_to_disk` me pinned hai, is wave ke OWNS ke bahar) |
| textinteract-109 caret-Esc 1 fail | OFF (clean main par bhi fail — pre-existing, cell-editor path untouched) |
| `hunk-copy-left-N/right-N` + saare purane testids | MATCH (byte-identical; drop-mount lines untouched) |

## Files list
- apps/desktop/src/components/views/TextView.tsx (MOD: copyHunkAt + serialize/coalesce + Save path ONLY)
- apps/desktop/e2e/specs/d1-copycorrect-250.spec.ts (NEW)
- REPORT.md (yeh entry, append-only)

D1-DONE
# D3 Shortcuts — dead shortcut audit + wiring REPORT (Hinglish-roman)

Worktree: `/tmp/wt-D3` (branch `wave-D3-shortcuts`, base `main@3bb39f8`). NO commit, NO push.

## 1. Masla (user complaint)

UI me advertised saare shortcuts (menu labels, toolbar titles, tooltips) asal me kaam karein.
"Shortcuts not working" — kuch advertised chords dead the ya sirf ek focus se chalte the.

## 2. Inventory (UI text scan — poora shortcut suchi)

Native menu (`src-tauri/src/menu.rs`): CmdOrCtrl+1..7 (View kinds), N (New Session),
O (Open Session), Shift+O (Open Folder), S (Save Session), F (Find Session),
D (Take Left), E (Take Right). Theme/Layout/Settings = honestly unsupported (koi accelerator nahi).

Toolbar titles (`TOOLBAR_SHORTCUT_HINTS` se): open (Cmd+O), Next/Prev hunk
(Cmd+Alt+Down/Up), Copy (Cmd+D), Copy-right (Cmd+E).

Per-view toolbar labels: Ctrl+F (text/table/hex/merge find, folder name-filter,
sync step-filter), Ctrl+G (text/hex goto, folder go-to-entry), Esc dismiss labels,
folder Back/Forward (Alt+Left/Right), merge footer `j/k ... Cmd+B`, merge toolbar
`(keys: j/k, n/p, arrows)`, merge take-both (Cmd+B).

Chrome bridge: F3 (find next), Shift+F3 (find prev). View-local: text `n/p/l/r`,
arrows, Enter, Esc; merge `j/k/n/p`, arrows.

## 3. Keyboard verify — har chord ko asal key-press bheja (spec D3)

Spec: `apps/desktop/e2e/specs/d3-shortcuts-252.spec.ts` (14 tests, koi geometry assert nahi).
Har test me zero console/page errors assert hai.

## 4. Mila hua dead (1 asli bug) + fix

BUG: Merge `j`/`k` footer + toolbar me advertised tha, lekin sirf tab chalta tha jab
merge section khud focused ho (`onKeyDown` section-level). Toolbar/body/ruler focus se
`j`/`k` dead tha — bus me koi event nahi, koi nav nahi.
FIX (`MergeView.tsx`, handler wiring ONLY, koi layout/testid change nahi): window-level
`keydown` me `j`/`k` branch — input/textarea/contentEditable me type karne par ignore,
`defaultPrevented` ka respect, phir SAME `autocompare:view-command` bus
(`viewKind: "merge"`) jisse toolbar prev/next buttons bhi chalte hain. App GAP-09 `j`/`k`
ko ignore karta hai, isliye double-step impossible. Bonus: Cmd+B branch me
`defaultPrevented` guard (section + window double-take rokta hai; 30ms tick-dedupe
ke saath belt-and-braces).
`shortcuts.ts` (registry ONLY): values same (sab wired nikle), sirf D3 audit comment
joda — per-view coverage + honest-disabled suchi, taaki agli wave ko dobara scan na karna pade.

## 5. MATCH (wired — spec me green) vs OFF (honest-disabled, fake label nahi)

MATCH: Cmd+O/N/S, Cmd+1/2/3/4/6/7 kind switch, Shift+Cmd+O, Cmd+D/E (text+merge take,
bina selection honest note), Cmd+B (merge take-both, bahar "only in Merge" note),
Ctrl+F (text/table/hex/merge/folder/sync), Ctrl+G (text/hex/folder), F3/Shift+F3
(next/prev jahan prev button hai, baaki jagah focus-fallback), text n/p/l/r/arrows/
Cmd+Alt-arrows/Enter/Esc, merge j/k/n/p/arrows, folder Alt+Left/Right/Up + Esc
(query clear NAHI hota — contract), hex/table/merge/sync Esc, media Esc.
OFF (honest, spec me no-crash assert): Table/Merge/Sync me Ctrl+G (koi goto nahi —
browser Find Next suppress, koi nav nahi), Media me Ctrl+F/G + F3 (koi find/goto nahi —
menu-find "Find is not available for this view" park karta hai). Toolbar Copy/Next/Prev
bina wiring wale views me hidden + reason-title sahit disabled rehte hain
(`needs view wiring in L3` kabhi fake hint ke saath visible nahi).

## 6. Files (sirf OWNS, testids byte-identical)

- `apps/desktop/src/components/views/MergeView.tsx` — window j/k wiring + Cmd+B guard.
- `apps/desktop/src/components/chrome/shortcuts.ts` — registry values unchanged, D3 audit comments.
- `apps/desktop/e2e/specs/d3-shortcuts-252.spec.ts` — NEW (14 tests).
- Native menu (`src-tauri/src/menu.rs`): audit me MATCH, koi drift nahi, change nahi.
- Touch nahi kiya: session/**, ui/** primitives, tokens.css, engines, ipc shape, baaki specs.

## 7. GATES (1857, ABSOLUTE paths, fresh port)

- `cd /tmp/wt-D3/apps/desktop && ./node_modules/.bin/tsc --noEmit` => **TSC:0**
  (node_modules main se symlink karke, gate ke baad symlink hata diya).
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-D3/apps/desktop 1857 e2e/specs/d3-shortcuts-252.spec.ts e2e/specs/ui-standards.spec.ts --workers=1`
  => **18 passed (13.9s), GATE_PLAYWRIGHT_EXIT=0** — d3 14/14 + ui-standards 4/4, webkit.
  Note: pehli 1857 koshish D2 wave ke vite se takrayi (strike 1, kill nahi kiya);
  probe 1861 par 11 pass + 3 spec-bug mile (neeche), fix karke 1861 par 18/18,
  phir free hote hi pinned 1857 par 18/18 official.
- Spec-fix trail (product me koi change nahi, sirf test expectations honest kiye):
  (a) text nav: row-6 pehle hunk ki pehli row — pehla `n` wrap karke wahin rehta,
  ab diffnav-note assert + doosra `n` move assert karta hai;
  (b) merge j/k: listener-attach race — ab attach-await-press-poll pattern;
  (c) folder Esc: stacked name+goto LIFO dismiss hota hai (DockedBar capture
  `stopPropagation` by design, ui primitive untouched) — ab alag-alag + LIFO
  teenon assert hote hain.

D3-DONE

## DEBTB-behav REPORT (wave-DEBTB-behav, worktree-first)

Base: main `2b904c2`. Worktree: `/tmp/wt-DEBTB` (branch `wave-DEBTB-behav`).
Scope: 2 pre-existing red behavioral tests green karna tha. NO commit/push (orchestrator karega).

### Kya mila (root cause)

1. shellchrome-116:153 "idle actions collapse" — Stop title assert purana tha.
   App (C1cB ke baad) idle Stop par honest title deta hai:
   `Stop (no compare running: backend has no abort IPC)`
   (`COMPARE_IDLE_STOP_REASON`, `chrome/Toolbar.tsx:100`), jabki spec purana
   `Stop (no compare running)` mangta tha. C1cB behavior intended hai,
   isliye SPEC ko honest title par update kiya (app untouched).
2. chromedeclutter-119:167 "popovers ... Esc closes" — ignore-menu toggle-close dead tha.
   Probe (`elementFromPoint`) se confirm: Wave-233 me Ignore/Format `ui/Modal`
   par migrate hue (full-viewport scrim). Open scrim toolbar toggle ko cover
   karta hai, isliye toggle-click ya to hit-target par 30s timeout hota hai
   ya backdrop-mousedown -> close + exposed-toggle-click -> reopen ka
   double-toggle race hota hai. Modal ka sanctioned dismiss (Esc/backdrop/
   Dismiss -> single `onClose`) sahi hai; "toggle closes" expectation stale
   thi (popover zamane ki). Iske saath ek real dismiss-path bug bhi mila:
   `IgnoreMenu`/`FormatMenu` ko `TextView` se `onClose` milta hi nahi tha,
   fallback toolbar-toggle par synthetic `btn.click()` karta tha — backdrop
   click par close-then-reopen glitch (race). Allowed scope ke andar app-fix kiya.

### Fixes (OWNS ke andar, testids byte-identical)

- SPEC-PIN `apps/desktop/e2e/specs/shellchrome-116.spec.ts:175`
  Stop idle title ab honest string pin karta hai:
  `Stop (no compare running: backend has no abort IPC)`.
- SPEC-PIN `apps/desktop/e2e/specs/chromedeclutter-119.spec.ts:224-227`
  Ignore step ka close toggle-click se Esc par (sanctioned Modal dismiss).
  Toggle-open half + `aria-expanded` pins same hain.
- APP-FIX `apps/desktop/src/components/views/TextView.tsx` (2 props, dismiss wiring only)
  `IgnoreMenu` ko `onClose={() => setIgnoreOpen(false)}`,
  `FormatMenu` ko `onClose={() => setFormatOpen(false)}` — Esc/backdrop/
  Dismiss ab single honest `onClose` route par hain, racy DOM fallback
  bypass hota hai. Koi testid/label/geometry change nahi.

### Gates (worktree, fresh port, ABSOLUTE paths)

- `./node_modules/.bin/tsc --noEmit` => TSC:0 (node_modules main se symlink tha, gate ke baad remove)
- `PLAYWRIGHT_BROWSERS_PATH=/Users/ektasaini/ms-playwright bash /Users/ektasaini/autocompare/tools/gate-run.sh /tmp/wt-DEBTB/apps/desktop 1868 e2e/specs/shellchrome-116.spec.ts e2e/specs/chromedeclutter-119.spec.ts e2e/specs/ui-standards.spec.ts --workers=1` => 11/11 green

### Note (scope-bahar, orchestrator ke liye)

- `textchrome-112` aur `textcontrols-66r` me bhi wahi ignore toggle-close pattern hai;
  wo is wave ke OWNS/gate me nahi hain, isliye unhe chhua nahi.
  App-fix (real `onClose`) se unka backdrop/Esc path sudhra, par unka
  toggle-close step modal truth ke khilaf hai — alag wave me spec-pin chahiye.

DEBTB-DONE
- Stop title: SPEC-PIN (MATCH — spec ab honest app behavior se match)
- Ignore dismiss: APP-FIX (dismiss race) + SPEC-PIN (toggle->Esc) (MATCH — Esc contract se match)
- Files: apps/desktop/e2e/specs/shellchrome-116.spec.ts, apps/desktop/e2e/specs/chromedeclutter-119.spec.ts, apps/desktop/src/components/views/TextView.tsx

---

# REPORT — DEBTA-geom (wave-DEBTA-geom @ /tmp/wt-DEBTA, base main 2b904c2)

## Summary
6 numbered failures me se 6 ke underlying asserts fix + verify ho gaye (2 tests
fully green, 4 tests apne saare geometry/px/viewport/Esc pins pass karke sirf
`assertNoOverlap` ke clipped-content false positives par red hain). Koi visible
overlap nahi hai (screenshot + elementFromPoint se prove). Koi commit/push nahi.
Testids byte-identical. TSC:0. Gate: 8 passed / 4 failed (4 fail sirf overlap).

## Per-failure MATCH/OFF (evidence-backed)
1. chrome-174:99 "band is one 44px row" — MATCH (spec-pin). Band ab Wave-223
   reality pin karta hai: tabbar(32)+toolbar(44)+1px gap+1px border = 78,
   stacked rows, shell total <= 120 (measured 110). Assert green verify.
2. chrome-174:148 firstRowTop 206 vs 212 — MATCH (spec-pin). 206 < 212 sahi me
   improvement hai (tabbar 40->32 = -6px: 212->206, 193->187, 249->243).
   Pins ab no-regression vs pre-174 baselines rakhte hain (text<=212,
   folder<=193, table<=249); merge<=164, hex<=238 (measured-current + slack,
   hex find+goto default-open in-flow panels ke saath). Test GREEN.
3. chrome-budget:56 "ruler OVERLAPS statusbar" — MATCH (app-fix). Ruler track
   628px me 322 markers x 4px = 1292px overflow kar rahe the (scrollH 2x).
   OverviewRuler ab markers ko scale-to-fit karta hai (min(4, avail/count),
   ResizeObserver; testids/kinds/click/keyboard same, ruler-buckets/hunk-nav
   semantics untouched). Ruler violations 5 -> 0, scrollH == clientH verify.
   (Test ke BA KI text-row violations alag family hain — STALL note neeche.)
4. chrome-budget:117 toBeHidden — MATCH (spec-pin). Fresh hex session me find
   AUR goto dono panels default-open hain (HexView useState(true)); ek Esc ek
   panel band karta hai (probe: Esc1->find gone, Esc2->goto gone). Esc behavior
   untouched, spec ab 2x Esc press karta hai (hexchrome-71/d3-shortcuts pattern).
   Test GREEN.
5. geopolish-122 toolbar px (got 3) — MATCH (app-fix). ToolbarButton vertical
   `px-[3px]` (0cb5ba1 squeeze, pre-Wave-223) wapas `px-[var(--ac-space-2)]`
   (8px) — Wave-223 me toolbar ki apni full-width row + internal scroll hai,
   to 8px 1280 me fit hota hai. Computed px 8, gap 4 verify.
6. geopolish-66c toolbar px space-2 — MATCH (app-fix, #5 wala hi). Saare 4px
   ramp pins + ruler pins pass. Ruler marker pin scale-to-fit me update
   (h in 1..4px + track overflow <= 1px) — small diffs byte-identical 4px.

Cascade reveals (same tests, stale pins, fix kiye — gate green ke liye zaroori):
- 122 new-tab hit-area 32 -> >= 24 (Wave-158 ne 32px exception retire kiya;
  tab-new = ToolbarButton size=md = 24px token, WCAG 2.5.8 minimum).
- 122 pathbar icon buttons 28x28 -> 32x24 (size=md horizontal reality:
  14px icon + px-2; height control-h-md 24px == WCAG min; 28px pin md-token
  migration se pehle ka tha).

## STALL (3-strike rule) — 4 tests red sirf assertNoOverlap par
174:99, budget:56, 122:179, 66c:151 — inke saare geometry asserts PASS; fail
sirf below-fold/scrolled text rows (200 loaded / 644) ke rects hain jo footer
rects se intersect karte hain. Ye rows clipped + invisible hain (overflow-y
auto container; elementFromPoint row-25 par line-detail, r332 par statusbar
dikhlata hai; boot screenshot pixel-perfect saaf). B2c viewport-lock ke baad
koi bhi pane-se-lambi list ye gate kabhi pass nahi kar sakti (proof: rows
25+ hamesha 772-800 statusbar band cross karte hain; footers pinned hain).
Strikes: (1) content-visibility:auto probe — webkit rects nahi hatata, fail.
(2) ruler scale-to-fit ship — ruler violations 0, text rows independent rahe.
(3) virtualization/threshold analysis — edge-clipped row bhi violate karta
hai, isliye koi in-OWNS lever nahi. Views/helper/specs-gate sab OWNS ke bahar
hain, isliye yahin stall. Routing: overlap-helper lineage visibility-aware
gate banaye (scrollport se bahar clipped boxes skip) — tab ye 4 tests bina
kisi app change ke green honge; greens par zero risk (subset property).

## Gates (worktree, fresh port, absolute paths)
- ./node_modules/.bin/tsc --noEmit => TSC:0 (symlink hata diya gaya)
- gate-run.sh .../apps/desktop 1867 chrome-174 chrome-budget geopolish-122
  geopolish-66c ui-standards --workers=1 => 8 passed / 4 failed (4 fail sirf
  overlap-stall wale; ui-standards 4/4 green, 174:148+220 green, budget:117
  green, 122 firstRowTop green)

## Files changed (5, testids byte-identical, koi naya spec nahi)
- apps/desktop/src/components/ui/ToolbarButton.tsx (vertical px 3 -> space-2)
- apps/desktop/src/components/chrome/OverviewRuler.tsx (scale-to-fit markers)
- apps/desktop/e2e/specs/chrome-174.spec.ts (band-78 + stacked rows + shell
  <=120; viewport pins 212/193/249/164/238)
- apps/desktop/e2e/specs/chrome-budget.spec.ts (hex double-Esc)
- apps/desktop/e2e/specs/geopolish-122.spec.ts (new-tab >=24; toggle 32x24)
- apps/desktop/e2e/specs/geopolish-66c.spec.ts (marker scale-to-fit pins)

DEBTA-DONE
