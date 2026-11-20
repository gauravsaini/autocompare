# Gates: Exquisite GUI wave exq5-2 round-1 — dual-pane + virtualized + TypoRole + screenshot

OWNS: crates/gui/** + checks/ac-loop/round-exq5-2/**
FORBIDDEN: crates/diff/** crates/compare/** crates/sessions/** crates/reports/** crates/vfs/** crates/cli/**

Scope: extend-existing. Geometry first (alignment/positioning), typography second
(family/weight/line-height/letter-spacing). Colors FROZEN (zero color adds).

- [ ] G1: autocompare-gui test suite passes
  CHECK: CARGO_TARGET_DIR=/tmp/target-gui2 CARGO_BUILD_JOBS=2 cargo test -p autocompare-gui
  EXPECT: test result: ok
- [ ] G2: Clippy passes with zero warnings on autocompare-gui
  CHECK: CARGO_TARGET_DIR=/tmp/target-gui2 CARGO_BUILD_JOBS=2 cargo clippy -p autocompare-gui --all-targets -- -D warnings
  EXPECT: Finished
- [ ] G3: Format check clean on autocompare-gui
  CHECK: cargo fmt -p autocompare-gui -- --check && echo "fmt: ok"
  EXPECT: fmt: ok
- [ ] G4: Zero allow(dead_code) in OWNS paths
  CHECK: python3 -c "import subprocess; out=subprocess.run(['rg','-n','allow\\(dead_code\\)','crates/gui','checks/ac-loop/round-exq5-2','--glob','!*.md'],capture_output=True,text=True); print('dead-code: 0' if not [l for l in out.stdout.splitlines() if l.strip()] else out.stdout)"
  EXPECT: dead-code: 0
- [ ] G5: Zero banned (unwrap/expect/panic/todo/unimplemented/unreachable) in non-test OWNS code
  CHECK: python3 -c "import subprocess; out=subprocess.run(['rg','-n','(\\.unwrap\\(|\\.expect\\(|panic!|todo!|unimplemented!|unreachable!)','crates/gui/src'],capture_output=True,text=True); print('banned: 0' if not [l for l in out.stdout.splitlines() if not l.strip().startswith('//')] else out.stdout)"
  EXPECT: banned: 0
- [ ] G6: Colors frozen (no added Color32::from_rgb / Visuals:: in gui diff)
  CHECK: python3 -c "import subprocess; out=subprocess.run(['git','diff','HEAD','--','crates/gui'],capture_output=True,text=True); n=sum(1 for l in out.stdout.splitlines() if l.startswith('+') and ('Color32::from_rgb' in l or 'Visuals::' in l)); print(f'colors: {n}')"
  EXPECT: colors: 0
- [ ] G7: ac-loop harness round-1 passes with edge fixture + PNG magic valid
  CHECK: tools/ac-loop/run.sh checks/ac-loop/round-exq5-2/round-1
  EXPECT: RESULT: PASS
- [ ] G8: 960px + 720px narrow proof PNGs valid + --screenshot contract (dark/light/exit codes)
  CHECK: python3 -c "from pathlib import Path; b1=Path('checks/ac-loop/round-exq5-2/round-1/gui/after.png').read_bytes(); b2=Path('checks/ac-loop/round-exq5-2/round-1/comparison/narrow-720-proof.png').read_bytes(); assert b1[:8]==bytes.fromhex('89504e470d0a1a0a') and len(b1)>=8192 and len(set(b1))>64; assert b2[:8]==bytes.fromhex('89504e470d0a1a0a') and len(b2)>=6144 and len(set(b2))>64; print('screenshots: ok')"
  EXPECT: screenshots: ok
