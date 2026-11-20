# HEXDENSE8 Gates (round-1)

- `cargo test -p autocompare-gui`: 858 passed, 0 failed — GREEN
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: 0 warnings — GREEN
- `rustfmt --edition 2024 --check` (views_hex.rs, views_table.rs): clean — GREEN
- Banned tokens non-test code me (diff me `unwrap()`/`expect(`/`panic!`/`todo!`): 0 — GREEN
- Harness `tools/exquisite/run.sh checks/gui-exq-hexdense8/round-1`: RESULT PASS — GREEN
  - PNG 10401 bytes (>= 8192 ok), magic ok, distinct 256 (non-blank ok)
  - Geometry: SPACE XS/SM/MD/LG/XL, margin/gap x320, Grid x14, row-h x125, width x69, gutter x969
  - Typo: TypoRole x755, monospace x26, strong x42, .size [14.0] only, weights/line-heights existing tokens only
  - Colors frozen: Color32/Visuals touches 33 (unchanged, info only)
- Evidence: `round-1/after.png`, `round-1/exquisite-report.md`, `round-1/tests.out`, `round-1/comparison/hexdense8-narrow-gutter.json`
- OWNS: `crates/gui/src/views_hex.rs`, `crates/gui/src/views_table.rs`, `checks/gui-exq-hexdense8/**` — bahar koi edit nahi
