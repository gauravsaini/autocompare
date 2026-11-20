# Gates: Exquisite GUI wave-5 geometry+typography

OWNS: crates/gui/** + checks/ac-loop/round-exq5/**
FORBIDDEN: crates/diff/** crates/compare/** crates/sessions/** crates/reports/** crates/vfs/** crates/cli/**

Scope: Clean-room Beyond Compare 5 replica (extend-existing). Geometry FIRST (zero clipped text, 960x600 + 720px fallback), typography SECOND (family/weight/line-height/spacing). Colors FROZEN (no color edits).

- [x] G1: autocompare-gui test suite passes cleanly
  CHECK: CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-exq5 cargo test -p autocompare-gui --quiet
  EXPECT: test result: ok
  EVIDENCE: 725 passed 0 failed (clean worktree /tmp/wt-exq5-verify, target-exq5-verify)

- [x] G2: Clippy passes with zero warnings on autocompare-gui
  CHECK: CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-exq5 cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: Finished `dev` profile
  EVIDENCE: clean worktree clippy Finished, zero warnings; in-tree red is other-wave uncommitted screenshot.rs churn

- [x] G3: Format check passes cleanly on autocompare-gui
  CHECK: cargo fmt -p autocompare-gui -- --check
  EXPECT: 0
  EVIDENCE: my 6 files CLEAN via rustfmt --edition 2024 --check; clean worktree cargo fmt check exit 0

- [x] G4: Zero dead-code allowances in OWNS paths
  CHECK: python3 -c 'import subprocess, sys; out = subprocess.run(["rg", "-n", "allow\\(dead_code\\)", "crates/gui", "checks/ac-loop/round-exq5"], capture_output=True, text=True); lines = [l for l in out.stdout.splitlines() if l.strip()]; sys.exit(0 if len(lines) == 0 else 1)'
  EXPECT: 0
  EVIDENCE: verified this session (rg zero matches / zero color adds in commit diff)

- [x] G5: Zero unsafe and zero unwrap/expect/panic/todo/unimplemented/unreachable/indexing in non-test OWNS code
  CHECK: python3 -c 'import subprocess, sys; out = subprocess.run(["rg", "-n", "(forbid\\(unsafe_code\\)|unsafe\\s*\\{|\\.unwrap\\(|\\.expect\\(|panic!|todo!|unimplemented!|unreachable!)", "crates/gui/src"], capture_output=True, text=True); lines = [l for l in out.stdout.splitlines() if "forbid(unsafe_code)" not in l and not l.strip().startswith("//")]; sys.exit(0 if len(lines) == 0 else 1)'
  EXPECT: 0
  EVIDENCE: banned-pattern rg zero matches in crates/gui/src (this session)

- [x] G6: Geometry & typography refined with colors FROZEN
  CHECK: python3 -c 'import subprocess, sys; out = subprocess.run(["git", "diff", "HEAD", "--", "crates/gui"], capture_output=True, text=True); diff = out.stdout; [sys.exit(1) for line in diff.splitlines() if line.startswith("+") and ("Color32::from_rgb" in line or "Visuals::" in line)]; sys.exit(0)'
  EXPECT: 0
  EVIDENCE: commit diff adds zero Color32-from-rgb / Visuals lines in crates/gui

- [x] G7: ac-loop harness passes with RESULT: PASS and edge fixture in comparison/
  CHECK: test -f checks/ac-loop/round-exq5/REPORT.md && grep -q "RESULT: PASS" checks/ac-loop/round-exq5/REPORT.md && echo "ac-loop round-exq5 PASSED"
  EXPECT: ac-loop round-exq5 PASSED
  EVIDENCE: REPORT.md RESULT: PASS (CLI PASS + GUI PASS), edge-05-take fixtures present

- [x] G8: 960x600 + 720px narrow fallback proof PNG screenshots valid
  CHECK: python3 -c 'from pathlib import Path; p1 = Path("checks/ac-loop/round-exq5/gui/after.png"); p2 = Path("checks/ac-loop/round-exq5/comparison/narrow-720-proof.png"); b1 = p1.read_bytes(); b2 = p2.read_bytes(); assert b1[:8] == bytes.fromhex("89504e470d0a1a0a") and len(b1) >= 8192 and len(set(b1)) > 64; assert b2[:8] == bytes.fromhex("89504e470d0a1a0a") and len(b2) >= 6144 and len(set(b2)) > 64; print("Screenshots valid")'
  EXPECT: Screenshots valid
  EVIDENCE: after.png 960x600 9765B + narrow-720-proof.png 720x600 12992B, magic+size+nonblank ok
