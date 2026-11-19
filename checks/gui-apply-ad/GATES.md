# Wave AD-APPLY — GATES: folder Sync one-direction apply (Gap #8, kadam 1)

Scope: `crates/gui/src/app.rs` + `checks/gui-apply-ad/**` only.
KEVL read-only (chhoona mana hai): `tree.rs` / `views.rs` / `viewmodel.rs` /
`views_table.rs` / `views_hex.rs` / `gutter.rs` / `theme.rs` / `screenshot.rs` /
`crates/compare` (sirf `plan_sync` / `apply_sync` reuse, edit BANNED — read-only grep ok).
Geometry first, typography second, colors frozen. MS rust guidelines:
`forbid(unsafe_code)`, non-test code me `unwrap` / `expect` / `panic` / `todo` mana hai.

## Background (Beyond Compare se observed, apne shabdon me)
Folder sync ka matlab: do folder roots ki tulna karke antar ki suchi banana,
phir ek direction me antar mitana. Preview sirf batata hai kya hoga;
Apply tabhi chalta hai jab user ne saaf anumati di ho. Pehli wiring sirf
ONE safe direction karti hai: left se right ki taraf nayi/badli files ki
copy. Delete/prune is kadam me nahi — koi destructive default nahi. Dono
UI surfaces (Folder tab ka Mirror preview + Sync tab ka checklist) ek hi
safe core reuse karte hain.

## G1 — Safe one-direction core: plan_sync reuse + CopyToRight subset (headless)
- CHECK: `cargo test -p autocompare-gui mirror_copy`
- EXPECT: green; `mirror_copy_options()` LeftToRight + prune false;
  `mirror_copy_subset()` sirf `CopyToRight` steps rakhta hai (deletes /
  reverse copies bahar); `plan_mirror_copy` / `apply_mirror_copy`
  engine `plan_sync` / `apply_sync` reuse karte hain, errors `String` me,
  koi panic nahi.

## G2 — Temp-dir headless proof: left->right copy hota hai, right-only bachta hai
- CHECK: `cargo test -p autocompare-gui mirror_copy_applies` +
  `cargo test -p autocompare-gui sync_apply`
- EXPECT: green; temp dirs me left ki `new` + updated file right me
  pahunchti hai, right-only file na mit-ti hai na overwrite hoti hai,
  report me `copied >= 1`, `deleted == 0`, `failed` khali. Mtime fixtures
  deterministic hain (`touch_newer` / old-mtime, koi sleep nahi). Cleanup
  best-effort (`remove_dir_all` ignore).

## G3 — UI: explicit confirm + dry-run default, koi destructive default nahi
- CHECK: `cargo test -p autocompare-gui folder_mirror` +
  `cargo test -p autocompare-gui sync_dry_run` + Folder/Sync tab code padhna
- EXPECT: green; `App::default()` me dono dry-run flags true, dono
  confirms off; `folder_should_write()` / `sync_apply_enabled()` tabhi true
  jab dry-run OFF + confirm/consent + steps ho; dry-run Apply koi write nahi
  karta, sirf status batata hai; swap/recompare purani report + confirm saaf
  karta hai (stale report kabhi nahi). Paint me dry-run checkbox +
  last-report line, existing tokens (`TIGHT_GAP`, `monospace`, `small`,
  `strong`) reuse, koi `Color32` / `Visuals` touch nahi. Sync tab checklist
  wahi safe core (`copy_right_plan` + `apply_mirror_copy`) reuse karta hai.

## G4 — full gui green + clippy zero + fmt + banned grep
- CHECK: `cargo test -p autocompare-gui && cargo clippy -p autocompare-gui --all-targets`
  (zero warnings) `&& cargo fmt -- crates/gui/src/app.rs && cargo fmt --check -- crates/gui/src/app.rs`
  `&& grep -rn 'unwrap(\|\.expect(\|panic!\|todo!' crates/gui/src/app.rs` (empty)
- EXPECT: sab green; banned tokens zero (test blocks me bhi zero).

## G5 — harness proof + REPORT + commit (NO push)
- CHECK: `tools/gui-polish/run-geo-typo.sh checks/gui-apply-ad/round-1`
- EXPECT: `geo-typo-report.md` me `RESULT: PASS`, `REPORT.md` filed,
  `git add` sirf OWNS paths (`crates/gui/src/app.rs` + `checks/gui-apply-ad`),
  `git commit` (conventional English), NO push, `git stash/push` kabhi nahi.
