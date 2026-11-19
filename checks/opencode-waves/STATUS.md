# Opencode waves (default model) — 2026-11-20

Default model: opencode-go/muse-spark-1.3-contributor (no --model override, per user instruction).
Old agyx GUI-T/F (PIDs 85254/85255) killed after 11 min with zero log output — no file edits made (git status clean except untracked checks/), safe to replace.

## Live sessions (4 parallel, disjoint OWNS)
- ses-gui-text3 (log /tmp/op-text.log): views.rs, copy.rs, take.rs, gutter.rs, viewmodel.rs, checks/gui-text/**
- ses-gui-folder3 (log /tmp/op-folder.log): tree.rs, home.rs, checks/gui-folder/**
- ses-gui-shell3 (log /tmp/op-shell.log): app.rs, theme.rs, screenshot.rs, main.rs, checks/gui-shell/**
- ses-local-run3 (log /tmp/op-local.log): checks/local-run/** only, no source edits

## Rules for all
Hinglish Roman only, clean-room outside observation, MS rust guidelines (forbid unsafe, no unwrap/expect/panic/todo in non-test code), headless-first, tests green, clippy -D warnings, rustfmt clean, conventional commit NO push. Orchestrator verifies, pushes, rebuilds release.

## 2026-11-20 poll (orchestrator)
- Pasted PLAN questions answered per user history: (1) existing repo extend at /Users/ektasaini/autocompare/PLAN.md, no new subfolder; (2) scope order harness -> skill -> full waves.
- Skills active: unlazy (gates-first, depth tree, disjoint OWNS, reverify) + ponytail full (YAGNI, reuse, stdlib first, root-cause fix).
- MS rust guidelines URL live (HTTP 200, 2026-10-26): forbid unsafe, no unwrap/expect/panic/todo in non-test code, headless-first — already baked into all 4 agent prompts.
- Orchestrator-verified PNG: checks/local-run/run.png exists, 9174 bytes, magic 89504e470d0a1a0a OK (independent of agent claim).
- All 4 sessions alive, correct scope reads in logs; no agent commits yet.

## Poll 01:12 AEDT
- ses-gui-text3 alive, owned-file reads done, implementing.
- ses-gui-folder3 alive: baseline 57 tests green (agent-claimed, rerun at verify time), clippy/fmt clean claimed, now writing tree.rs headless logic.
- ses-gui-shell3 alive: app.rs +4 state fields (word_wrap, show_line_numbers, syntax_highlight, status) + defaults; orchestrator-scanned diff CLEAN (no unwrap/expect/panic/todo/unsafe).
- ses-local-run3 alive: report expanded with exit 0/1/2 proofs; PNG re-verified by orchestrator earlier.

## Poll 02
- ses-local-run3 DONE (local commit fdd5e97, already pushed inside 359335c): report.md 317 lines with exit 0/1/2 proofs. Follow-up report.md edits still uncommitted, left for agent.
- ses-gui-text3 alive: viewmodel.rs +39 implementing nav/filter.
- ses-gui-folder3 alive: tree.rs rewritten (+568) with headless filter/flatten/staging/sync-preview.
- ses-gui-shell3 alive: app.rs +289. Uncommitted diff scanned CLEAN (only hit is forbid(unsafe_code) attribute).
- No verification run yet (cargo busy with builders); rerun tests/clippy/screenshots at land time.

## Poll 03
- All 3 builders alive, all logs growing (shell log un-stuck, now wiring toolbar + status bar).
- Diff now +1291/-96: viewmodel +264 (nav/filter), tree +568, app +312, copy +37, home +23. Banned-pattern scan CLEAN.
- No land commits yet; verify at land time.

## Poll 04
- All 3 alive. Folder agent entering verify phase (fmt + tests). Text agent needs a hook in read-only app.rs — watching that it does NOT edit it (OWNS rule); so far app.rs still +312 from shell only.
- Diff +1305/-96, banned-pattern scan CLEAN.

## Poll 05
- Interface collision: folder tree.rs references TextFilter, text viewmodel.rs mid-edit — 16 compile errors on folder side. Folder agent self-diagnosing (fmt diffs, own-files-only fix). No orchestrator interrupt yet; re-check next poll, intervene only if still red.
- Text agent still reading viewmodel (offset 409). Shell agent reading app.rs tail sections.
- Banned-pattern scan CLEAN.

## Poll 06
- Collision self-resolving: TextFilter enum now defined at viewmodel.rs:383 (text agent). Folder agent on fmt-check of own files. No interrupt needed.
- Shell agent writing tests (status-bar asserts in log). All 3 alive, logs growing.
- Local-run report.md unstaged edits gone with no new commit — left alone (agent OWNS).

## Poll 07
- Git state confirmed sane: 1d2ae61 touches only checks/local-run/run-proof.md (+153, inside agent OWNS) — pushed.
- Folder agent: 84 tests green (baseline was 57, +27 new). Land approaching.
- Text agent writing viewmodel tests; shell agent in screenshot code.

## Poll 08
- Shared-crate breakage: all 3 edit autocompare-gui, mid-edit states break cargo for everyone (folder 21 errors, shell 11 + TextViewState never-constructed in viewmodel). Both agents correctly diagnosing own-vs-other errors, no interrupt.
- Threshold: if still red after 2 more polls, orchestrator pauses 2 agents and lands them one by one (text -> folder -> shell).

## Poll 09
- Breakage clearing: no error mentions in latest tails. Text writing tests, folder on fmt, shell produced before/after screenshots (9174 vs 9846 bytes, both PNG magic OK, content DIFFERENT — first visual proof of real UI change).
- No land commits yet.

## Poll 10
- Folder: 90 tests green + clippy done — land imminent.
- Shell: own-file warnings check running, build recovering.
- Diff +1889/-23 across 6 files, banned-pattern scan CLEAN.

## Poll 12
- Folder errors down to 2 (was 21/16) — almost through shared-crate breakage.
- Shell writing REPORT, land imminent. Text retrying views.rs edit after oldString mismatch.
- No land commits yet.

## Verify 01 (folder 9b1293d + shell e59fd07)
- OWNS respected by both (folder: tree/home + gui-folder; shell: app/screenshot + gui-shell).
- PNG magic OK all 4 (before 9174, after 9846, differ).
- Banned-pattern scan CLEAN on both commits.
- 90-test claims plausible (folder log showed 90 green pre-commit); current crate red = 3 errors ALL in views.rs (text mid-edit: TextRows missing filter/selected init). Attributed to text WIP, no revert. Full green re-verify at text land.

## Early-feedback run (3 waves in tree)
- cargo build green, cargo test 92 passed (text fixed views.rs init).
- Fresh screenshot checks/early-feedback/after-3-waves.png: 9846 bytes, PNG magic OK, exit 0. Toolbar (Swap/Reload/toggles), session-identity title, honest status counts visible.

## Poll 13
- app.rs +108 uncommitted = shell post-commit follow-ups (shell log shows 8 app.rs edits, text log shows 0 app.rs edits — no OWNS violation). Left untouched; integrate at text land after tests green.
- Text alive, editing copy/views/viewmodel only, retrying failed matches.
- CI: 3 latest runs still queued (runners backed up); release refresh waits for green + text land.

## Poll 14
- Text in clippy-polish phase (collapsible-if in views.rs), land near. CI still queued (4 runs backed up).

## Poll 15
- Text fixing clippy hits in views.rs (495/500/604), still landing. CI still queued.

## Poll 16
- Text probing TextViewState block (assert miss — likely renamed during dead-code fix) + 4 collapsible-if warnings. Still working, no land. CI still queued.

## Poll 17
- Text build finished green (dev profile 1.50s), land still pending. CI still queued.

## Poll 18
- Text: 92 tests green + rustfmt clean on owned files. Land imminent. CI still queued.

## Verify 02 (text 84dfe28 + full tree)
- OWNS respected (copy/viewmodel/views + gui-text only). Banned-scan CLEAN. PNGs magic OK.
- Orchestrator reran: cargo test 92 passed, clippy -D warnings clean, fmt clean — full tree green together, interface collision resolved.

## Wave D launched (harness-driven)
- ses-vis-folder4 (log /tmp/vis-folder.log): app/screenshot/home/tree + checks/gui-folder-vis + round-D-1. Folder tab wiring + sync Apply gating + screenshot Folder fixture.
- ses-vis-hooks4 (log /tmp/vis-hooks.log): views/gutter/viewmodel + checks/gui-text-hooks. paint_text prefs hook (wrap/syntax/line-numbers) with visual toggle proof.
- Disjoint OWNS, both on skills/parity-harness/SKILL.md proof rounds. NO push by agents.

## Wave D poll 01
- Both alive. Folder registered sibling scopes, reading REPORT hooks. Hooks grepping paint prefs (39 matches in app.rs), reading run-visual.sh. Tree clean. CI still queued.

## Wave D poll 02
- Both alive, reading phase (folder 610B unchanged, hooks into app.rs tail/main/theme). Tree clean. CI backlog persists (all queued).

## Wave D poll 03
- Hooks already at build+fmt stage, folder still reading. Tree clean.

## Wave D poll 04
- Diff +90, all attributed: app.rs +3 (folder_session hold in App — folder agent, correct OWNS), gutter +18 + viewmodel +69 (hooks agent tests). Banned-scan clean.

## Wave D poll 05
- Diff +461/-12: app +315 (folder wiring), views +54 (paint prefs hook), screenshot +17 (folder fixture starting), gutter +18, viewmodel +69. Both alive. Folder log silent but edits prove work. Scan clean.

## Wave D poll 06
- Diff +794/-22: screenshot +291 (folder fixture expanding), views +123 (paint hook), app +315, viewmodel +69, gutter +18. Scan clean. Both alive.

## Wave D poll 07
- Diff +944/-22: screenshot +334, views +230, app +315. Scan clean. CI still queued.

## Wave E planned (exquisite UI: geometry first, typography second, colors later)
- Sequenced AFTER wave D lands (same files: views/app/tree/screenshot — concurrent edit = conflict).
- GEO agent: pane widths/splits, margins, padding, alignment, positioning. Proof in px (measured margins/padding from layout code + screenshots), not vibes.
- TYPO agent: font family/weight/line-height/letter-spacing via egui text styles, per-pane hierarchy. Proof: before/after screenshots + style table.
- Note: screenshot.rs .expect() hits all inside mod tests (8/8) — permitted, non-test code clean.

## Wave D poll 08
- Diff +1000/-22. checks/gui-folder-vis/ appeared (folder proof incoming). Hooks retrying views.rs edit after oldString miss. Both alive.

## Wave D poll 09
- Hooks own test red (gutter narrow 3 vs 4), self-debugging. Folder new code has dead-code warnings (paint_folder, glyph_color) to fix before land. Both alive, normal fix loop.

## Wave D poll 10
- Hooks writing fixture-dispatch tests (text vs folder bytes differ assert). Diff +1014. Both alive.

## Verify 03 (folder 06aaf24)
- OWNS respected (app/screenshot + gui-folder-vis + round-D-1). 8/8 expects inside mod tests — clean.
- round-D-1: console PASS + visual PASS (9914, magic ok, non-blank). Folder-visual doc describes wired UI.
- Hooks agent still working (views/gutter/viewmodel uncommitted) — full green re-verify at its land.

## Wave D poll 11
- Hooks confirming workspace lint source (expect_used deny), still working. Its 3 files uncommitted. Folder proc gone (landed), hooks proc alive.

## Wave D poll 12
- Hooks in views.rs accent/color fmt code, no land. Both alive.

## Wave D poll 13
- Hooks in paint fg/bg code, no land. Both alive.

## Wave D poll 14
- Hooks WHITE/BLACK hits all in test module (9 in-test, 0 non-test) — no color-scope creep. No land. Both alive.

## Wave D poll 15
- Hooks red test fixed (gutter test ok, 7/7 focused pass). Land approaching.

## Wave D poll 16
- Hooks in proof phase (wrap proof script in own checks dir). Land approaching.

## Wave D poll 17
- Hooks visual PASS, final gates running (one compound-command tool hiccup, retrying split). No land. Both alive.

## Wave D poll 18
- Hooks on dead-code warning (folder_columns in views.rs, its OWNS) — fixing. No land. Both alive.

## Wave D poll 19
- Hooks FMT-CLEAN on owned files, land imminent. Both alive.

## Wave D poll 20
- Hooks 107 tests green, writing report + commit. Land imminent.


## Wave E exquisite (geo first, typo second, colors out) — launched 2026-11-20
- Harness first: tools/gui-polish/run-geo-typo.sh + skills/gui-polish/SKILL.md (baseline gui-geo PASS 9914B, spaces 2/4/6/8, Grid 1, mono x17 strong x5 small x10 size 14.0). Committed 5a26fba, pushed.
- 3 parallel default-model waves (no --model flag, opencode-go/muse-spark-1.3-contributor):
  ses-geo-exq (PID 17032): app.rs+views.rs+checks/gui-geo, geometry only, no colors.
  ses-typo-exq (PID 17049): theme.rs+gutter.rs+viewmodel.rs+home.rs+checks/gui-typo, typography only, no views/app edits.
  ses-local-bin (PID 17073): checks/local-run only, local run + binary early feedback, no source edits.
- Probe strategy: no timer, event-driven headless loop (tests + --screenshot PNG + grep px). Poll only agent liveness.
- Rules: Hinglish Roman, clean-room, MS rust (forbid unsafe, no unwrap/expect/panic/todo non-test), conventional commit NO push by agents.

## Wave E poll 01
- All 3 live verified via ps: ses-geo-exq PID 17032, ses-typo-exq PID 17049, ses-local-bin PID 17073. Logs growing 2893/1258/1879B.
- GEO in egui 0.37 Frame/Margin API research (Margin::same, inner_margin), TYPO in type-signal counts, BIN produced checks/local-run/early-geo-typo.png 9914B magic ok distinct 256.
- No source diff yet, reading phase per unlazy, no OWNS violation. Harness baseline still PASS.
