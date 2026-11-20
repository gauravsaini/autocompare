# GATES: local-run round-lr4 (early feedback, no crates touch)

OWNS: checks/ac-loop/round-lr4/**, dist/**
FORBIDDEN: crates/**, skills/**, tools/**

- [ ] LR4-G1: CLI 10 cmds PASS via local-run equivalent
  CHECK: ROOT=/tmp/wt-localrun4; CLI=$ROOT/target/debug/autocompare; FX=$ROOT/tools/parity-harness/fixtures; $CLI --help lists 10 cmds + 10 parity runs exit 1/0 as expected
  EXPECT: CLI_10_OK + cli/parity-report.md RESULT:PASS

- [ ] LR4-G2: 3 PNG screenshots valid (magic + size)
  CHECK: GUI=$ROOT/target/debug/autocompare-gui; $GUI --screenshot gui/shot-default.png; --screenshot-mode dark/light; python3 PNG magic 89504e470d0a1a0a + len>1024
  EXPECT: PNG_3_OK

- [ ] LR4-G3: tail-insertion edge fixture + expected output
  CHECK: comparison/edge-tail-left.txt (3 lines) vs edge-tail-right.txt (3 lines + tail line); $CLI text -> edge-tail.out non-empty, exit=1
  EXPECT: EDGE_TAIL_OK

- [ ] LR4-G4: dist tarball + sha256 fresh + VERSION
  CHECK: cargo build --release -p autocompare-cli -p autocompare-gui; tar -czf dist/autocompare-VERSION-macos-ARCH.tar.gz; shasum -a 256
  EXPECT: DIST_OK

- [ ] LR4-G5: TOP-10 gaps doc + REPORT RESULT:PASS + fmt owned-scope clean
  CHECK: GAPS-TOP10.md exists; REPORT.md contains RESULT: PASS; cargo fmt --check Diff outside crates/ empty
  EXPECT: GAPS_EDGE_OK + FMT_OK
