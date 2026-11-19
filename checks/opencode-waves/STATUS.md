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
