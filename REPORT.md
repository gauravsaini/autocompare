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
