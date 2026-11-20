# ac-loop round report (round-lr4)
- CLI parity: PASS (10/10, report cli/parity-report.md)
- GUI screenshots: PASS (gui/shot-default|dark|light.png, PNG magic ok)
- dist: autocompare-0.5.0-macos-arm64.tar.gz + .sha256 (VERSION=0.5.0 3d33e80)
- edge: edge-tail-left.txt (3 lines alpha/beta/gamma) vs edge-tail-right.txt (+delta-tail) -> edge-tail.out non-empty (see edge-tail-expected.out)
- gaps: GAPS-TOP10.md
- fmt: owned scope (checks/ac-loop/round-lr4, dist) has no Rust files -> clean
RESULT: PASS
