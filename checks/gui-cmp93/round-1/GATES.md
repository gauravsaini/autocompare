# Gates: cmp93 round-1 folder compare orchestration (leaf-2.3)

OWNS: crates/compare/**, checks/gui-cmp93/**

Scope: Vfs-backed folder tree, Sync Update Both/Mirror dry-run/apply, Folder Merge, cancellable JobRunner; no GUI touch.

- [ ] cmp93-G1: Folder compare over LocalFs + Zip + Snapshot, Sync (Both/Mirror/dry-run/apply), Folder Merge, cancellable JobRunner tests pass.
  CHECK: cargo test -p autocompare-compare
  EXPECT: test result: ok
  EVIDENCE: checks/gui-cmp93/round-1/tests.out — 12 suite, 165 passed, 0 failed, 0 FAILED lines.

- [ ] cmp93-G2: Vfs-backed folder tree targets pass (LocalFs parity + Zip + Snapshot).
  CHECK: cargo test -p autocompare-compare --test vfs_compare
  EXPECT: test result: ok
  EVIDENCE: 8 passed (vfs_compare suite, tests.out me darj).

- [ ] cmp93-G3: rustfmt clean for the owned crate.
  CHECK: sh -c 'cargo fmt --check -p autocompare-compare && echo FMT_OK'
  EXPECT: FMT_OK
  EVIDENCE: FMT_OK (cargo fmt -p autocompare-compare ke baad check clean).

- [ ] cmp93-G4: strict clippy clean, banned tokens zero in non-test prod code (forbid unsafe, no unwrap/expect/panic/todo).
  CHECK: sh -c 'cargo clippy -p autocompare-compare --all-targets -- -D warnings && echo CLIPPY_OK'
  EXPECT: CLIPPY_OK
  EVIDENCE: CLIPPY_OK; grep se prod src me sirf forbid(unsafe_code) + "unsafe path" strings, koi unwrap/expect/panic/todo nahi (test files me file-level allow, pehle jaisa convention).

- [ ] cmp93-G5: OWNS-only dirt (no GUI touch, no sessionpaint strays).
  EVIDENCE: git status --short me sirf M crates/compare/src/{folder,jobs,lib}.rs + untracked crates/compare/src/vfs_compare.rs, crates/compare/tests/vfs_compare.rs, checks/gui-cmp93/; crates/gui/** aur checks/gui-sessionpaint-by/** me zero change.
