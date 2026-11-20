# GATES: local-run round-lr3/round-1 (early feedback, no crates touch)

OWNS: dist/**, checks/ac-loop/round-lr3/**
FORBIDDEN: crates/**, tools/**

- [x] LR3-G1: CLI 10 cmds PASS via owned local-run wrapper
  CHECK: sh checks/ac-loop/round-lr3/round-1/run-lr3.sh
  EXPECT: CLI_10_OK

- [x] LR3-G2: 3 PNG screenshots valid (magic + size)
  CHECK: sh checks/ac-loop/round-lr3/round-1/run-lr3.sh
  EXPECT: PNG_3_OK

- [x] LR3-G3: fmt status honest (owned scope has no Rust; no Diff outside crates/)
  CHECK: sh -c 'cargo fmt --all --check > /tmp/lr3-fmt.out 2>&1; grep "^Diff in" /tmp/lr3-fmt.out | grep -v "crates/" && exit 1 || exit 0'
  EXPECT: FMT_OK

- [x] LR3-G4: dist tarball + sha256 fresh
  CHECK: sh checks/ac-loop/round-lr3/round-1/run-lr3.sh
  EXPECT: DIST_OK

- [x] LR3-G5: TOP-10 gaps doc + 1 NEW edge fixture (tail-insertion) with expected output + headless proof
  CHECK: sh checks/ac-loop/round-lr3/round-1/run-lr3.sh
  EXPECT: GAPS_EDGE_OK

- [x] LR3-G6: Overall report RESULT: PASS
  CHECK: grep '^RESULT: PASS' checks/ac-loop/round-lr3/round-1/REPORT.md
  EXPECT: RESULT: PASS

- [x] LR3-G7: Zero banned keywords in owned scope
  CHECK: sh -c '! grep -rnE "(unsafe|unwrap\(|expect\(|panic!|todo!|unreachable!)" checks/ac-loop/round-lr3/ --include="*.rs" && echo BANNED_GREP_0'
  EXPECT: BANNED_GREP_0
