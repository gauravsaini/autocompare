# ac-loop round report (round-lr3/round-1)
- CLI parity: PASS (10/10, report cli/parity-report.md)
- GUI screenshots: PASS (gui/shot-default|dark|light.png, PNG magic ok)
- dist: autocompare-0.5.0-macos-arm64.tar.gz + .sha256 (VERSION=0.5.0 7bdf2ea)
- edge: edge-tail-left.txt (3 lines) vs edge-tail-right.txt (3 + appended "delta") -> edge-tail.out non-empty, exit=1, appended line visible (see edge-tail-expected.out, edge-tail.exit)
- gaps: GAPS-TOP10.md
- fmt: owned scope (checks/round-lr3, dist) has no Rust files -> clean; workspace Diffs (if any) confined to crates/ (FORBIDDEN per OWNS, untouched)
RESULT: PASS
