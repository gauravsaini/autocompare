# EXQ-TREE2 ROUND-2 GATES — checks/gui-exq-tree2/round-1

- cargo test -p autocompare-gui: GREEN (909+ passed, 0 failed)
- clippy -p autocompare-gui --lib: 0 warnings in owned file `crates/gui/src/tree.rs`
- rustfmt --edition 2024 --check crates/gui/src/tree.rs: CLEAN (exit 0)
- banned non-test code (`.unwrap()` / `.expect()` / `panic!` / `todo!` / `unreachable!` / `unsafe{}` / `Color32::` / `Visuals::`): 0
- harness tools/exquisite/run.sh: exit 0
- exquisite-report.md: RESULT: PASS (PNG >= 8192B, magic ok, distinct non-blank; colors 33 frozen)
- Geometry: indent step from SPACE tokens (GEO_TREE_INDENT_STEP = SPACE_MD = 12.0)
- Typography: orphan dimming via TypoRole Small (is_weak() == true), .size frozen at 14.0
- Alignment: Size/Modified columns aligned at 960px wide (Size 738.0, Modified 832.0; fixed cols 222.0px)
- Edge fixture: `comparison/tree-wide-960.json` + headless test `exq_tree2_round2_tree_wide_960_and_orphan_dimming`

RESULT: PASS
