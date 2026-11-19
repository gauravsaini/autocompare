# GATES — Wave AM-APPLY (checks/gui-apply-am)

Scope: `crates/gui/src/app.rs` + `checks/gui-apply-am/**` only.
KEVL files (views.rs, viewmodel.rs, tree.rs, views_table.rs, views_hex.rs,
home.rs, gutter.rs, theme.rs, screenshot.rs) read-only, no edits.

## Task

Gap #7/#8 apply wiring: App me persist merge decisions ko real file apply
tak le jao. `take-left/right/base/both` decisions se output likhne ka
headless apply path + dry-run/summary, I/O errors graceful (Result, no
panic). Paint wiring agle wave ka hai.

## API (App, headless, no egui)

- `merge_resolved_lines(&self) -> Result<Vec<String>, String>` — current
  `merge_model.lines` + `merge_picks` se `apply_merge_picks`. No model par
  `Err("merge apply: compare first")`, resolve fail par `Err` string.
- `join_merge_output(lines) -> String` (pure) — `"\n"` join + trailing
  newline jab non-empty, empty par empty.
- `merge_apply_text(&self) -> Result<String, String>` — resolved lines ka
  file text.
- `merge_dry_run_summary(resolved, decided, total, pending) -> String`
  (pure free fn) + `merge_dry_run_line(&self) -> String` — honest counts,
  `(no writes)` promise.
- `describe_merge_apply_report(wrote, out) -> String` (pure free fn).
- `write_merge_output(out, text) -> Result<usize, String>` (pure assoc fn)
  — `std::fs::write`, error string me, line count return.
- State: `merge_dry_run: bool` (default true), `merge_output_path: String`
  (default empty), `merge_apply_last_report: Option<String>` (default None).
- `set_merge_dry_run(bool)`, `set_merge_output_path(...)`,
  `merge_should_write() -> bool` (dry-run off + model + output path).
- `run_merge_apply(&mut self) -> bool` — dry-run par report, no writes;
  real par output path check + write; har I/O error graceful message,
  `true` jab report line bani.

## Gates (sab PASS chahiye)

1. `cargo test -p autocompare-gui` green (naye unit tests samait).
2. `cargo clippy --all-targets` zero warnings.
3. `cargo fmt` sirf OWNS file par + check clean.
4. Banned grep clean (`app.rs` me `unwrap()/expect()/panic!/todo!/
   unimplemented!/unreachable!/unsafe`, non-test me bhi nahi; tests me
   bhi nahi — `unwrap_or`/`expect` shabdon se alag hai, allowed).
5. `tools/gui-polish/run-geo-typo.sh checks/gui-apply-am/round-1` PASS
   (geometry first, typography second, colors untouched — is wave me
   paint change nahi, headless proof).
6. `REPORT.md` + `round-1/` artifacts, phir `git add` sirf OWNS paths +
   `git commit` (conventional English). No push, no stash, dusre agent
   ki files untouched.
