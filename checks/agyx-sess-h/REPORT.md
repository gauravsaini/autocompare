# AGYX-SESS-H SESSIONS HARDEN REPORT — Beyond Compare Parity in crates/sessions

Tareekh: 2026-11-20
Node: SWARM-SESS-H (leaf-2.1 parity harden)
Scope: `crates/sessions/src/session.rs`, `crates/sessions/src/settings.rs`, `crates/sessions/src/workspace.rs`, `crates/sessions/tests/**`, `checks/agyx-sess-h/**` ONLY.

## 1. Summary (Hinglish-Roman)

Beyond Compare 5 parity achieve karne ke liye `crates/sessions` me leaf-2.1 hardening complete kar di gayi hai:
- **Locked Sessions Parity**: `SessionDocument` aur `SessionTab` me `locked: bool` field, builder methods (`with_locked`), accessors (`is_locked`, `set_locked`), aur v0 legacy migration aliases (`locked`, `is_locked`, `read_only_session`) add kiye gaye. `save_to_file` par locked protection guard enforce kiya gaya hai jo locked hone par `PermissionDenied` error return karta hai, sath hi `save_to_file_force` bypass method provide kiya gaya hai.
- **Locked Workspaces Parity**: `WorkspaceDocument` me bhi `locked: bool` field, builder method (`with_locked`), accessors (`is_locked`, `set_locked`), `save_to_file` guard, aur v0 migration aliases (`locked`, `is_locked`, `read_only_workspace`) add kiye gaye.
- **Grammar Presets Parity**: `GrammarPreset` struct define kiya gaya jo line/block comment delimiters, associated extensions, aur case sensitivity capture karta hai. Standard built-in presets (`c_style`, `hash_style`, `sql_style`, `xml_style`, `default_preset`) expose kiye gaye. `AppSettings` me `grammar_presets: Vec<GrammarPreset>` roundtrip, extension lookup (`find_preset_for_extension`), lookup by name (`grammar_preset`), aur v0 auto-migration support implement kiya gaya.
- **ac-loop Verification**: `checks/agyx-sess-h/round-1` me edge comparison fixtures (`edge-session-locked.json`, `edge-workspace-locked.json`, `edge-grammar-presets.json`) aur expected outputs add kiye gaye. `tools/ac-loop/run.sh checks/agyx-sess-h/round-1` cleanly run hua with `RESULT: PASS`.

## 2. Key Metrics & Gates

- **Tests Passed**: 47/47 (18 lib unit, 9 corrupt_input, 12 migration, 8 round_trip).
- **Clippy**: 0 warnings (`cargo clippy -p autocompare-sessions --all-targets -- -D warnings`).
- **Rustfmt**: Clean formatting across all touched session source and test files.
- **Banned Patterns**: 0 violations in non-test src (MS Rust guidelines strictly enforced: unwrap/expect/panic/todo/unimplemented/unreachable/indexing_slicing denied, forbid unsafe_code).
- **ac-loop**: `RESULT: PASS` verified via `tools/ac-loop/run.sh checks/agyx-sess-h/round-1`.
- **OWNS Scope**: Staged with explicit pathspec `crates/sessions/src/session.rs crates/sessions/src/settings.rs crates/sessions/src/workspace.rs crates/sessions/tests checks/agyx-sess-h` only.

## 3. Files Changed in OWNS

- `crates/sessions/src/session.rs`: Added `locked: bool` on `SessionTab` and `SessionDocument`, `is_locked`, `set_locked`, `with_locked`, locked `save_to_file` protection guard, `save_to_file_force`, v0 migration parsing.
- `crates/sessions/src/workspace.rs`: Added `locked: bool` on `WorkspaceDocument`, `is_locked`, `set_locked`, `with_locked`, locked `save_to_file` protection guard, `save_to_file_force`, v0 migration parsing.
- `crates/sessions/src/settings.rs`: Added `GrammarPreset` struct and constructors, `grammar_presets` on `AppSettings`, lookup helpers (`find_preset_for_extension`, `grammar_preset`, `add_grammar_preset`), v0 migration mapping.
- `crates/sessions/tests/round_trip.rs`: Added locked session/workspace round-trip and save guard tests, custom grammar preset round-trip tests.
- `crates/sessions/tests/migration.rs`: Added v0 migration tests for locked sessions/workspaces and custom/default grammar presets.
- `crates/sessions/tests/corrupt_input.rs`: Added corrupt locked and grammar preset error verification tests.
- `checks/agyx-sess-h/GATES.md`: Unlazy acceptance ledger.
- `checks/agyx-sess-h/REPORT.md`: This verification report.
- `checks/agyx-sess-h/round-1/comparison/*`: Comparison edge fixtures and expected output files.

## 4. Verification Logs

```text
cargo test -p autocompare-sessions
running 18 tests in src/lib.rs ... ok
running 9 tests in tests/corrupt_input.rs ... ok
running 12 tests in tests/migration.rs ... ok
running 8 tests in tests/round_trip.rs ... ok
Total: 47 passed; 0 failed.

cargo clippy -p autocompare-sessions --all-targets -- -D warnings
exit code: 0 (zero warnings)

tools/ac-loop/run.sh checks/agyx-sess-h/round-1
RESULT: PASS
```
