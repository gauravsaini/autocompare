# REPORT — Wave AM-APPLY (checks/gui-apply-am)

Wave: AM-APPLY. Gap #7/#8 apply wiring — App me persist merge decisions ko
real file apply tak le jaya gaya. Paint wiring agle wave ka hai (is wave
me koi paint change nahi).

## Kya bana (sirf OWNS: `crates/gui/src/app.rs`)

Merge Apply headless path, take-left/right/base/both decisions se output:

- `merge_resolved_lines()` — live `merge_model.lines` + staged
  `merge_picks` se `apply_merge_picks`. No model par
  `Err("merge apply: compare first")`, stale picks par engine error
  string me. Undecided hunks marker block bankar rehte hain.
- `join_merge_output()` (pure) — `"\n"` join + trailing newline
  (non-empty par), khali par khali string.
- `merge_apply_text()` — resolved lines ka file text.
- `merge_dry_run_summary()` (pure) + `merge_dry_run_line()` — honest
  counts (`resolved line(s) output, decided/total, pending`) + no-writes
  promise. No model par compare-first note.
- `describe_merge_apply_report()` (pure) — write outcome line.
- `write_merge_output()` — `std::fs::write`, har I/O error graceful
  `Err` string me, line count return. Kabhi panic nahi.
- State: `merge_dry_run` (default true), `merge_output_path` (default
  empty), `merge_apply_last_report` (default None). Naya compare / swap /
  error par picks ke saath clear hota hai.
- `set_merge_dry_run()`, `set_merge_output_path()`,
  `merge_should_write()` (dry-run off + model + output path),
  `run_merge_apply()` — dry-run par report + no writes, real par output
  path check + write, har failure graceful message, model untouched
  (retry safe). Returns true jab report line bani ho.

Tests (8 naye, `app.rs` tests module me, koi banned API nahi):

- take-left/right/base/both sahi output likhte hain
- undecided hunks markers rakhte hain, pending honest
- no-model par compare-first report
- dry-run default: report + `(no writes)`, file nahi banti
- real write end-to-end: Both pick se 4-line file + done report
- khali output path par graceful report, koi write nahi
- output path par dir hone par I/O error graceful (`merge apply failed`)
- pure summary/join format checks

## Gates proof

- `cargo test -p autocompare-gui` — 404 passed, 0 failed
- `cargo clippy --all-targets` — zero warnings
- `rustfmt --check crates/gui/src/app.rs` — clean
  (`cargo fmt -- crates/gui/src/app.rs` se formatted, sirf OWNS file)
- banned grep (`unwrap()/expect()/panic!/todo!/unimplemented!/
  unreachable!/unsafe`) — clean (`forbid(unsafe_code)` line ke siva
  koi hit nahi)
- `tools/gui-polish/run-geo-typo.sh checks/gui-apply-am/round-1` — PASS
  (PNG 9624 bytes, magic ok, distinct ok; geometry/typography tokens
  unchanged, colors untouched — paint change nahi tha)

## Files

- `crates/gui/src/app.rs` — apply path + state + tests
- `checks/gui-apply-am/GATES.md`, `REPORT.md`, `round-1/` — is wave ke
  artifacts
