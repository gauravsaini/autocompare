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
