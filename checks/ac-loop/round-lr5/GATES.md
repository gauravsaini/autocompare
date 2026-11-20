# GATES: local-run round-lr5 (early feedback, no crates touch)

OWNS: checks/ac-loop/round-lr5/**, dist/VERSION, dist/*.tar.gz, dist/*.sha256
FORBIDDEN: crates/**, skills/**, tools/**

- [x] LR5-G1: CLI 10 cmds PASS via local-run equivalent
  CHECK: ROOT=/tmp/wt-localrun5; CLI=$ROOT/target/debug/autocompare; FX=$ROOT/tools/parity-harness/fixtures; $CLI --help lists 10 cmds + 10 parity runs exit 1/0 as expected
  EXPECT: CLI_10_OK + cli/parity-report.md RESULT:PASS
  EVIDENCE: help lists 10/10 (text folder hex merge table sync picture folder-merge audio session); parity 10/10 exits match

- [x] LR5-G2: 3 PNG screenshots valid (magic + size >=8192B)
  CHECK: GUI=$ROOT/target/debug/autocompare-gui; $GUI --screenshot gui/shot-default.png; --screenshot-mode dark/light; python3 PNG magic 89504e470d0a1a0a + len>=8192
  EXPECT: PNG_3_OK
  EVIDENCE: default 11059B ok, dark 11262B ok, light 11059B ok

- [x] LR5-G3: head-insertion edge fixture + expected output
  CHECK: comparison/edge-head-left.txt (3 lines) vs edge-head-right.txt (ZERO head + 3 lines); $CLI text -> edge-head.out non-empty, exit=1
  EXPECT: EDGE_HEAD_OK
  EVIDENCE: edge-head.out 320B non-empty, exit=1

- [x] LR5-G4: dist tarball + sha256 fresh + VERSION
  CHECK: cargo build --release -p autocompare-cli -p autocompare-gui; tar -czf dist/autocompare-VERSION-macos-ARCH.tar.gz; shasum -a 256
  EXPECT: DIST_OK
  EVIDENCE: autocompare-0.5.0-macos-arm64.tar.gz 5816822B + .sha256 aa08d008...

- [x] LR5-G5: release LTO link check
  CHECK: Cargo.toml [profile.release] lto=true + codegen-units=1; file release bins Mach-O arm64; release CLI text same-file exit=0; release GUI --screenshot ok
  EXPECT: LTO_LINK_OK
  EVIDENCE: lto=true, codegen-units=1; both Mach-O 64-bit arm64; CLI exit=0; GUI smoke 11059B

- [x] LR5-G6: TOP-10 gaps doc + REPORT RESULT:PASS + fmt owned-scope clean + banned 0
  CHECK: GAPS-TOP10.md exists; REPORT.md contains RESULT: PASS; cargo fmt --check Diff outside crates/ empty; banned-pattern grep in owned scope empty
  EXPECT: GAPS_EDGE_OK + FMT_OK + BANNED_0
