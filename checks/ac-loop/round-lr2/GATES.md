# GATES: local-run round-lr2 (early feedback, no crates touch)

OWNS: tools/local-run/**, dist/**, checks/ac-loop/round-lr2/**
FORBIDDEN: crates/**

- [ ] LR2-G1: CLI 10 cmds PASS via local-run harness
  CHECK: sh tools/local-run/run.sh
  EXPECT: CLI_10_OK

- [ ] LR2-G2: 3 PNG screenshots valid (magic + size)
  CHECK: sh tools/local-run/run.sh
  EXPECT: PNG_3_OK

- [ ] LR2-G3: fmt status honest (owned scope has no Rust; workspace dirty files listed, crates untouched per OWNS)
  CHECK: sh -c 'cargo fmt --all --check > /tmp/lr2-fmt.out 2>&1; grep "^Diff in" /tmp/lr2-fmt.out | grep -v "crates/" && exit 1 || exit 0'
  EXPECT: FMT_OK

- [ ] LR2-G4: dist tarball + sha256 fresh
  CHECK: sh tools/local-run/run.sh
  EXPECT: DIST_OK

- [ ] LR2-G5: TOP-10 gaps doc + 1 edge fixture with expected output
  CHECK: sh tools/local-run/run.sh
  EXPECT: GAPS_EDGE_OK
