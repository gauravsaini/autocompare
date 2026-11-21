# GATES: local-run round-lr6 (early feedback, no crates touch)

OWNS: checks/ac-loop/round-lr6/**, dist/VERSION, dist/autocompare-0.5.0-macos-arm64.tar.gz(+.sha256)
FORBIDDEN: crates/**, skills/**, tools/**, other checks/**

- [x] LR6-G1: CLI 10/10 parity via tools/ac-loop/run.sh
  CHECK: zsh tools/ac-loop/run.sh checks/ac-loop/round-lr6; cli/parity-report.md 10 exits match
  EXPECT: RESULT: PASS
  EVIDENCE: text=1 text-same=0 folder=1 hex=1 merge=1 table=1 sync=1 picture=1 folder-merge=1 audio=1

- [x] LR6-G2: 3 PNGs default/dark/light + exquisite harness PASS
  CHECK: GUI --screenshot x3 (default/dark/light); zsh tools/exquisite/run.sh checks/ac-loop/round-lr6/exq-default
  EXPECT: PNG magic ok, all >=8192B, exquisite RESULT: PASS
  EVIDENCE: default 11040B, dark 11236B, light 11040B; exq-default RESULT: PASS

- [x] LR6-G3: dist tarball + sha256 fresh + VERSION
  CHECK: cargo build --release; tar -czf dist/autocompare-0.5.0-macos-arm64.tar.gz (bins + VERSION); shasum -a 256
  EXPECT: DIST_OK
  EVIDENCE: 5816820B + .sha256 4e7f9b95...

- [x] LR6-G4: release LTO link check
  CHECK: Cargo.toml [profile.release] lto=true + codegen-units=1; file release bins; release CLI/GUI smoke
  EXPECT: LTO_LINK_OK
  EVIDENCE: lto=true, codegen-units=1; both Mach-O 64-bit arm64; CLI same-file exit=0; GUI smoke ok

- [x] LR6-G5: REPORT RESULT:PASS + fmt N/A clean + banned 0
  CHECK: REPORT.md contains RESULT: PASS; find round-lr6 -name '*.rs' empty; banned-pattern rg empty
  EXPECT: REPORT_PASS + FMT_NA + BANNED_0
  EVIDENCE: RESULT: PASS; 0 .rs files; rg exit=1 (no matches)
