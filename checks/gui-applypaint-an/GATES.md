# GATES — Wave AN-APPLYPAINT (checks/gui-applypaint-an)

Scope: `crates/gui/src/app.rs` + `checks/gui-applypaint-an/**` only.
KEVL files (views.rs/viewmodel.rs/tree.rs/views_table.rs/views_hex.rs/home.rs/gutter.rs/theme.rs/screenshot.rs) mat chhoona — apply model fns read-only use ok. NO push (orchestrator pushes). Dusre agent ki files kabhi touch mat karo. `git stash/push` kabhi nahi.

## Task

Gap #8 apply paint wiring (state half): merge decisions ka Apply action App state me.
AM wave ne headless apply path de diya (`merge_resolved_lines`, `join_merge_output`,
`merge_apply_text`, `merge_dry_run_summary/line`, `describe_merge_apply_report`,
`write_merge_output`, `run_merge_apply`, `merge_dry_run/output_path/last_report`).
Is wave ka delta Apply button intent + progress/result state machine + last-apply
summary (structured, real counts) hai. Paint adoption agle wave ka hai — is wave me
koi `views.rs` change nahi, sirf App headless state + summary strings.

## API (App, headless, no egui)

- `MergeApplyState` enum (Copy, Default=Idle): `Idle`, `DryRunReported`, `Wrote`,
  `NeedsPath`, `NoModel`, `Failed`. Pure state; paint agle wave me isi se
  Apply section banayega.
- `MergeApplySummary` struct (Clone, Default): `dry_run: bool`,
  `resolved_lines: usize`, `decided/total/pending: usize`,
  `wrote_lines: usize`, `out: String`. Har `run_merge_apply` branch isi se
  honest ginti rakhta hai.
- State: `merge_apply_requested: bool` (default false, Apply click intent),
  `merge_apply_state: MergeApplyState` (default Idle),
  `merge_apply_last_summary: Option<MergeApplySummary>` (default None).
  `merge_apply_last_report: Option<String>` (AM) rehta hai — string line,
  summary structured judvaan.
- `request_merge_apply()` — intent true + status note. `clear_merge_apply_request()`
  — intent false. `take_merge_apply_request() -> bool` — consume karke false.
- `merge_apply_enabled() -> bool` — model maujood (paint Apply enable gate).
- `merge_apply_summary_line() -> String` — state + summary se real line:
  no-model par compare-first, Idle par dry-run line ya idle note, baaki par
  last report reuse. Kabhi jhoothi ginti nahi.
- `clear_merge_apply_state()` — requested/state/summary/report sab reset.
  Clear points: `swap_sides` Merge, `run_merge` validation-fail, result landing
  Ok/Err (fresh compare purani intent inherit nahi karta).
- `run_merge_apply` update: har branch (no-model/dry-run/needs-path/write-ok/
  write-err) report ke saath state + summary bhi set kare, request consume kare.
  Model untouched (retry safe), never panics.

## Gates (sab PASS chahiye)

1. `cargo test -p autocompare-gui` green (naye unit tests samait).
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings` zero warnings.
3. `rustfmt --check crates/gui/src/app.rs` clean
   (sirf OWNS file par `cargo fmt -- <files>` pattern; bare `cargo fmt` nahi).
4. Banned grep clean: non-test me `unwrap()/expect()/panic!/todo!/
   unimplemented!/unreachable!/unsafe` nahi (tests me bhi nahi — `unwrap_or`
   allowed). `forbid(unsafe_code)` kept. Koi indexing nahi (`.get()` only).
5. `tools/gui-polish/run-geo-typo.sh checks/gui-applypaint-an/round-1` PASS
   (geometry first, typography second, colors untouched — is wave me
   paint change nahi, headless proof).
6. `REPORT.md` + `round-1/` artifacts, phir `git add` sirf OWNS paths +
   `git commit` (conventional English). No push, no stash, dusre agent
   ki files untouched.
