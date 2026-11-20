# FOLDERTABLE10 GATES — checks/gui-exq-foldertable10/round-1

- cargo test -p autocompare-gui: GREEN (893 passed, 0 failed; tests.out me 3 suites ok)
- clippy -p autocompare-gui --lib: 0 warnings in owned file `crates/gui/src/tree.rs`
- rustfmt --edition 2024 --check crates/gui/src/tree.rs: CLEAN (exit 0)
- banned non-test code (`.unwrap()` / `.expect()` / `panic!` / `todo!` / `unreachable!` / `unsafe{}` / `Color32::` / `Visuals::`): 0
- harness tools/exquisite/run.sh: exit 0
- exquisite-report.md: RESULT: PASS (PNG 10401B >= 8192, magic ok, distinct 256 non-blank; colors 33 frozen)
- Typo: `.size` sirf `14.0`, koi naya size nahi; SPACE/MD/LG/SM tokens reuse
- Edge fixture: `comparison/folder-deep-nest.json` + headless test `exq_foldertable10_edge_deep_nest_and_dimmed_orphans`

RESULT: PASS
