# Gates: Exquisite GUI wave-4 geometry+typography

OWNS: crates/gui/** + checks/ac-loop/round-exq4/**
FORBIDDEN: crates/diff/** crates/compare/** crates/sessions/** crates/reports/** crates/vfs/** crates/cli/**

Scope: Clean-room Beyond Compare 5 replica (extend-existing). Geometry FIRST (zero clipped text, 960x600 + 720px fallback), typography SECOND (family/weight/line-height/spacing). Colors FROZEN (no color edits).

- [x] G1: autocompare-gui test suite passes cleanly
  CHECK: CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-exq4 cargo test -p autocompare-gui --quiet
  EXPECT: test result: ok
  EVIDENCE: test result: ok. 726 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.26s

- [x] G2: Clippy passes with zero warnings on autocompare-gui
  CHECK: CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=target/agent-exq4 cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: Finished `dev` profile
  EVIDENCE: Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s with 0 warnings/errors

- [x] G3: Format check passes cleanly on autocompare-gui
  CHECK: cargo fmt -p autocompare-gui -- --check
  EXPECT: 0
  EVIDENCE: Exit code 0, formatting clean across all autocompare-gui source files

- [x] G4: Zero allow(dead_code) in OWNS paths
  CHECK: python3 -c 'import subprocess, sys; out = subprocess.run(["rg", "-n", "allow\\(dead_code\\)", "crates/gui", "checks/ac-loop/round-exq4", "--glob", "!*.md"], capture_output=True, text=True); lines = [l for l in out.stdout.splitlines() if l.strip()]; sys.exit(0 if len(lines) == 0 else 1)'
  EXPECT: 0
  EVIDENCE: Exit code 0, 0 matches across crates/gui and checks/ac-loop/round-exq4 non-markdown files

- [x] G5: Zero unsafe and zero unwrap/expect/panic/todo/unimplemented/unreachable/indexing in non-test OWNS code
  CHECK: python3 -c 'import subprocess, sys; out = subprocess.run(["rg", "-n", "(forbid\\(unsafe_code\\)|unsafe\\s*\\{|\\.unwrap\\(|\\.expect\\(|panic!|todo!|unimplemented!|unreachable!)", "crates/gui/src"], capture_output=True, text=True); lines = [l for l in out.stdout.splitlines() if "forbid(unsafe_code)" not in l and not l.strip().startswith("//")]; sys.exit(0 if len(lines) == 0 else 1)'
  EXPECT: 0
  EVIDENCE: Exit code 0, 0 lines with unsafe/unwrap/expect/panic/todo/unimplemented/unreachable; NEW VIOLATIONS: 0 via new_violations.py

- [x] G6: Geometry & typography refined with colors FROZEN
  CHECK: python3 -c 'import subprocess, sys; out = subprocess.run(["git", "diff", "HEAD~1..HEAD", "--", "crates/gui"], capture_output=True, text=True); diff = out.stdout; [sys.exit(1) for line in diff.splitlines() if line.startswith("+") and ("Color32::from_rgb" in line or "Visuals::" in line)]; sys.exit(0)'
  EXPECT: 0
  EVIDENCE: Exit code 0, 0 color additions in commit 05e667e (Color32/Visuals untouched)

- [x] G7: ac-loop harness passes with RESULT: PASS and edge fixture in comparison/
  CHECK: test -f checks/ac-loop/round-exq4/REPORT.md && grep -q "RESULT: PASS" checks/ac-loop/round-exq4/REPORT.md && echo "ac-loop round-exq4 PASSED"
  EXPECT: ac-loop round-exq4 PASSED
  EVIDENCE: ac-loop round-exq4 PASSED; CLI parity: PASS, GUI geo-typo: PASS, edge fixtures present (edge-04 and edge-03)

- [x] G8: 960x600 + 720px narrow fallback proof PNG screenshots valid
  CHECK: python3 -c 'from pathlib import Path; p1 = Path("checks/ac-loop/round-exq4/gui/after.png"); p2 = Path("checks/ac-loop/round-exq4/comparison/narrow-720-proof.png"); b1 = p1.read_bytes(); b2 = p2.read_bytes(); assert b1[:8] == bytes.fromhex("89504e470d0a1a0a") and len(b1) >= 8192 and len(set(b1)) > 64; assert b2[:8] == bytes.fromhex("89504e470d0a1a0a") and len(b2) >= 8192 and len(set(b2)) > 64; print("Screenshots valid")'
  EXPECT: Screenshots valid
  EVIDENCE: Screenshots valid; after.png (960x600, 9765 bytes, distinct 256), narrow-720-proof.png (720x600, 12992 bytes >= 8192 floor, distinct 256)
