# GATES — Wave BK-SYNCWIRE (Folder-tab sync wiring, P0 BI-S1)

Wave: BK-SYNCWIRE. OWNS: `crates/gui/src/app.rs` (Folder tab branch ONLY) +
`checks/gui-syncwire-bk/**`. KEVL (mat chhoona): `crates/gui/src/views.rs`
(callsite threading bhi nahi — signature unchanged tha, logic to bilkul nahi),
`tree.rs` / theme / screenshot / views_hex / views_table (read-only),
`checks/opencode-waves/STATUS.md` (UNTOUCHED). Colors FROZEN. NO push.
Note: working tree me `crates/gui/src/views.rs` ka uncommitted diff parallel
agent (BI-M2 three-way paint) ka hai — haath nahi lagaya, commit me shaamil nahi.

## G1 — BI-S1 wiring live (Folder tab calls paint_folder_sync)

- CHECK: `grep -n "paint_folder_sync" crates/gui/src/app.rs`
- EXPECT: Folder branch me call present; `views.rs:4657` signature unchanged
  (`git diff --name-only` me `views.rs` mere commit me nahi)
- EVIDENCE: `app.rs` Folder branch me naya block (sync_rows + sync_summary +
  sync_mode copy, phir `selection` + `expanded` + `pager` pass, baad me
  `folder_expanded` mirror). `#[allow(dead_code)]` ab live-call se cover hota hai.

## G2 — cargo test full green

- CHECK: `cargo test -p autocompare-gui`
- EXPECT: 595 passed, 0 failed
- EVIDENCE: `round-1/tests.out`

## G3 — clippy zero (mere change se zero naya warning)

- CHECK: `cargo clippy -p autocompare-gui --all-targets`
- EXPECT: `app.rs` me zero warning; jo 2 doc warnings `views.rs:5718-5719`
  me dikhen woh parallel agent ke uncommitted diff ke hain, mere commit se bahar
- EVIDENCE: gate command output (REPORT me note)

## G4 — fmt sirf OWNS par, bare cargo fmt MANA

- CHECK: `cargo fmt --check -p autocompare-gui` (read-only check, koi write nahi)
- EXPECT: exit 0, koi reformat nahi chahiye tha
- EVIDENCE: gate command output

## G5 — banned grep zero new

- CHECK: mere `app.rs` hunk me `unwrap|expect|panic|todo|unsafe|Color32::from_rgb|Visuals::`
- EXPECT: ZERO naya (hunk scan me koi match nahi)
- EVIDENCE: gate command output (`ZERO new banned in hunk`)

## G6 — PNG magic + distinct

- CHECK: `file round-1/*.png` + magic `89504e470d0a1a0a` + size >= 8192 + distinct > 64
- EXPECT: `after.png` (9695 B, distinct 255), `after-folder.png` (10068 B,
  distinct 256), `after-sync.png` (10732 B, distinct 256) — teeno magic-ok,
  teeno alag sizes (koi fallback-copy nahi)
- EVIDENCE: `round-1/after.png`, `round-1/after-folder.png`, `round-1/after-sync.png`

## G7 — Commit, no push (pathspec sirf OWNS)

- CHECK: `git log --oneline -1` + `git show --stat HEAD`
- EXPECT: `feat(gui): ...` conventional English message; fileset sirf
  `crates/gui/src/app.rs` + `checks/gui-syncwire-bk/`; `views.rs` commit me nahi;
  `git push` kabhi nahi chalaya
- EVIDENCE: log + show output

## DoD

- Folder tab me sync badges + selection live (`paint_folder_sync` wired, mode +
  selection + expanded + pager pass), tests green, clippy/fmt/banned/PNG gates
  pass, REPORT Hinglish-Roman, commit local only, NO push.
