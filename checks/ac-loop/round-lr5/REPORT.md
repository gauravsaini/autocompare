# ac-loop round report (round-lr5)
- CLI parity: PASS (10/10, report cli/parity-report.md)
- GUI screenshots: PASS (gui/shot-default|dark|light.png, PNG magic ok, >=8192B: default 11059B, dark 11262B, light 11059B)
- dist: autocompare-0.5.0-macos-arm64.tar.gz (5816822B) + .sha256 (aa08d008..., VERSION=0.5.0 d7c3f29)
- edge: edge-head-left.txt (3 lines alpha/beta/gamma) vs edge-head-right.txt (ZERO head + 3 lines) -> edge-head.out 320B non-empty exit=1 (see edge-head-expected.out)
- gaps: GAPS-TOP10.md
- release LTO link: PASS ([profile.release] lto=true codegen-units=1; Mach-O 64-bit arm64 x2; release CLI same-file exit=0; release GUI smoke 11059B)
- fmt: owned scope (checks/ac-loop/round-lr5, dist) has no Rust files -> clean
- banned: 0 (owned scope has no Rust files; banned-pattern grep over round dir empty)
RESULT: PASS
