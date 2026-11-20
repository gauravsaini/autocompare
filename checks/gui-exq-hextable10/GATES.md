# HEXTABLE10 GATES — checks/gui-exq-hextable10/round-1

- cargo test -p autocompare-gui: GREEN (886 passed, 0 failed; tests.out me 3 suites ok)
- clippy -p autocompare-gui --lib -- -D warnings: 0 (owned files views_hex.rs/views_table.rs me koi warning nahi; clean)
- rustfmt --edition 2024 --check (owned files): CLEAN (exit 0)
- banned (owned non-test code: `.unwrap()` / `.expect()` / `panic!` / `todo!` / `unimplemented!` / `unsafe{}` / `Color32::` / `Visuals::`): 0 (sirf `?` / `.get()` / safe patterns + `forbid(unsafe_code)`)
- harness tools/exquisite/run.sh: exit 0
- exquisite-report.md: RESULT: PASS (PNG 10401B >= 8192, magic ok, distinct 256 non-blank; colors 33 frozen)

RESULT: PASS
