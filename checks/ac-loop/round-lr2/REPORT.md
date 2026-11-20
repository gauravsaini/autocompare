# ac-loop round report (round-lr2)
- CLI parity: PASS (10/10, report cli/parity-report.md)
- GUI screenshots: PASS (gui/shot-default|dark|light.png, PNG magic ok)
- dist: autocompare-0.5.0-macos-arm64.tar.gz + .sha256 (VERSION=0.5.0 f0ad84d)
- edge: edge-empty-left.txt (0 lines) vs edge-empty-right.txt ("hello") -> edge-empty.out non-empty (see edge-empty-expected.out)
- gaps: GAPS-TOP10.md
- fmt: workspace dirty ONLY in crates/cli/src/runner.rs + session_cmd.rs (import order, FORBIDDEN per OWNS); owned scope (tools/local-run, checks/round-lr2, dist) has no Rust files -> clean
RESULT: PASS
