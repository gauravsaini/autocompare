# GATES: exq4 exquisite GUI geometry+typography wave-4 (extend-existing)

OWNS: crates/gui/** + checks/ac-loop/round-exq4/**
Colors FROZEN (no color edits). Geometry FIRST, typography SECOND.
Baseline: HEAD builds clean; main dirty tree (parity/sessrep WIP) untouched.

- [x] G1: gui unit tests pass (staggered target dir, no lock clash)
  CHECK: CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-exq4 cargo test -p autocompare-gui
  EXPECT: test result: ok
  EVIDENCE: 714 passed 0 failed (incl merge_apply_chrome_covers_checkbox_and_button, sync_rich_routes_through_mono_role), worktree verified 2026-11-20

- [x] G2: clippy zero warnings on gui
  CHECK: CARGO_TARGET_DIR=target/agent-exq4 cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: Finished `dev` profile
  EVIDENCE: Finished dev profile, zero warnings, worktree verified 2026-11-20

- [x] G3: fmt clean on gui
  CHECK: cargo fmt -p autocompare-gui -- --check
  EXPECT: 0
  EVIDENCE: exit 0, worktree verified 2026-11-20

- [x] G4: zero allow(dead_code) in OWNS
  CHECK: rg -n "allow(dead_code)" crates/gui/src; test $? -eq 1
  EXPECT: 0
  EVIDENCE: rg exit 1 (zero hits), worktree verified 2026-11-20

- [x] G5: zero NEW unwrap/expect/panic/todo/unimplemented/unreachable/indexing in non-test gui code vs HEAD baseline (baseline legacy defs[i] out of scope)
  CHECK: python3 checks/ac-loop/round-exq4/new_violations.py
  EXPECT: NEW VIOLATIONS 0
  EVIDENCE: NEW VIOLATIONS 0, worktree verified 2026-11-20

- [x] G6: 960x600 + 720px narrow proof PNGs valid (magic + min bytes + non-blank)
  CHECK: python3 checks/ac-loop/round-exq4/png_check.py
  EXPECT: PNGS OK
  EVIDENCE: PNGS OK (after.png 9765 + narrow-720-proof.png 8162, both bit-identical to exq3), worktree verified 2026-11-20

- [x] G7: ac-loop harness RESULT PASS + >=1 new edge fixture in comparison/
  CHECK: tools/ac-loop/run.sh checks/ac-loop/round-exq4
  EXPECT: RESULT: PASS
  EVIDENCE: RESULT PASS (CLI PASS + GUI PASS), edge-03-merge-apply fixture added, round 1/6, worktree verified 2026-11-20
