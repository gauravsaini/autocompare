# ac-loop round report (round-1) - 3-Way Text Merge Parity

- CLI parity: PASS (log cli.log, report cli/parity-report.md)
- GUI geo-typo: PASS (log gui.log, report gui/geo-typo-report.md)
- BC live observe: BCompare (comparison/bc-process.txt)
- own sessions: text folder hex merge table sync picture folder-merge audio (comparison/own-help.txt)

## Is round me kya hua (Beyond Compare parity)
1. **3-Way Text Merge Parity**: Verified Myers & Patience-based diff scripts converted to base range replacement blocks.
2. **Clean Edit Merging**: Non-overlapping changes from left and right versions cleanly merge into base text without conflict.
3. **Conflict Detection & Formatting**: Overlapping or touching edit ranges generate standard conflict blocks with standard markers (`<<<<<<< left`, `=======`, `>>>>>>> right`).
4. **Context Preservation**: Unaltered base lines outside edit hunks are strictly preserved.

## Edge fixtures (comparison/)
- `clean_base.txt`, `clean_left.txt`, `clean_right.txt` -> `clean_expected.txt`: Clean non-conflicting 3-way merge scenario.
- `merge3_base.txt`, `merge3_left.txt`, `merge3_right.txt` -> `expected_conflict.txt`: Overlapping conflict scenario with standard conflict markers.
- `verify.sh`: Executable verification script validating clean vs conflict merges and exit codes.

## Verify
- `CARGO_BUILD_JOBS=2 cargo test -p autocompare-diff --target-dir target/agent-parity`: PASS
- `cargo clippy -p autocompare-diff -p autocompare-compare --target-dir target/agent-parity -- -D warnings`: PASS
- `rustfmt --edition 2024 --check`: PASS
- Zero `allow(dead_code)`, zero `unsafe`, zero `unwrap/expect/panic/todo` non-test: PASS

RESULT: PASS
