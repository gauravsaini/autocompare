# ac-loop round report (round-2) - Grammar & Whitespace Ignore Parity

- CLI parity: PASS (log cli.log, report cli/parity-report.md)
- GUI geo-typo: PASS (log gui.log, report gui/geo-typo-report.md)
- BC live observe: BCompare (comparison/bc-process.txt)
- own sessions: text folder hex merge table sync picture folder-merge audio (comparison/own-help.txt)

## Is round me kya hua (Beyond Compare parity)
1. **Whitespace Run Normalization**: Support for collapsing repeated whitespace sequences and trimming during text comparisons.
2. **Space-Change Ignore (`-b`)**: Strict diff correctly detects whitespace differences (exit code 1), while `-b` collapses whitespace runs and treats them as identical (exit code 0).
3. **Grammar & Comment Parsing**: Detection and handling of comments (line comments, block comments) and string literal boundaries without corrupting code diffs.
4. **Line Ending Invariance**: Normalization of CRLF vs LF differences across platforms.

## Edge fixtures (comparison/)
- `grammar_left.txt` vs `grammar_right.txt`: Whitespace run differences in function signatures.
- `verify.sh`: Executable verification script testing strict diff (exit 1) vs `-b` ignore-space-change (exit 0).

## Verify
- `CARGO_BUILD_JOBS=2 cargo test -p autocompare-diff --target-dir target/agent-parity`: PASS
- `cargo clippy -p autocompare-diff -p autocompare-compare --target-dir target/agent-parity -- -D warnings`: PASS
- `rustfmt --edition 2024 --check`: PASS
- Zero `allow(dead_code)`, zero `unsafe`, zero `unwrap/expect/panic/todo` non-test: PASS

RESULT: PASS
