# REPORT — Wave AN-APPLYPAINT (checks/gui-applypaint-an)

Wave: AN-APPLYPAINT. Gap #8 apply paint wiring (state half) — merge decisions
ka Apply action App state me wired, summary real. Paint adoption agle wave ka
hai (is wave me koi paint change nahi).

## Kya bana (sirf OWNS: `crates/gui/src/app.rs`)

AM wave ke headless apply path (`run_merge_apply`, dry-run/report/write) ke upar
button-intent + progress/result state machine:

- `MergeApplyState` enum (Copy, Default=Idle): `Idle`, `DryRunReported`,
  `Wrote`, `NeedsPath`, `NoModel`, `Failed`. Har `run_merge_apply` branch ek
  honest terminal state deta hai.
- `MergeApplySummary` struct (Clone, Default): `dry_run`, `resolved_lines`,
  `decided/total/pending`, `wrote_lines`, `out`. Report string ka structured
  judvaan — paint agle wave me isi se real numbers dikhayega.
- State: `merge_apply_requested: bool` (default false),
  `merge_apply_state` (default Idle),
  `merge_apply_last_summary: Option<...>` (default None).
- `request_merge_apply()` (intent true + status), `clear_merge_apply_request()`,
  `take_merge_apply_request() -> bool` (one-shot consume),
  `merge_apply_enabled() -> bool` (model maujood = Apply dabane layak),
  `merge_apply_summary_line() -> String` (state + summary se real line;
  no-model par compare-first, Idle par dry-run line ya idle note, baaki par
  last report reuse), `clear_merge_apply_state()` (sab reset).
- `run_merge_apply` update: har branch (no-model/dry-run/needs-path/write-ok/
  write-err) report ke saath state + summary bhi set karta hai, intent consume
  karta hai. Model untouched (retry safe), never panics.
- Clear points: `swap_sides` Merge, `run_merge` validation-fail, result landing
  Ok/Err — teeno `clear_merge_apply_state()` reuse (fresh compare purani intent
  inherit nahi karta). `swap_sides`/`run_merge`/`poll` ke purane alag-alag
  `= None` lines isi ek helper me sama gaye (same behavior).

Tests (9 naye, `app.rs` tests module me, koi banned API nahi):

- defaults idle/request-false/summary-none + `merge_apply_summary_line`
  compare-first jab model nahi
- request/take roundtrip + clear
- enabled gate: bina model false, model par true
- dry-run: `DryRunReported` + summary (3 lines, 1/1/0, wrote 0) + report reuse +
  file nahi bani + intent consumed
- real write Both pick: `Wrote` + wrote 4 + file content exact
- missing path: `NeedsPath` + summary me out khali + output-path line
- no model: `NoModel` + intent consumed + summary `Some`
- dir-path I/O error: `Failed` + wrote 0 + out non-empty
- swap clear: reported state Idle/summary/report sab reset ho jata hai

## Gates proof

- `cargo test -p autocompare-gui` — 416 passed, 0 failed (baseline 404 + 9
  is wave ke; +3 concurrent wave ke `views.rs` tests, full suite green)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — OWNS file
  clean (`app.rs` ka koi diagnostic nahi). Note: full-workspace run is waqt
  concurrent wave ki `views.rs:153` doc-lint (`doc_lazy_continuation`) par
  fail hota hai — dusre agent ki file hai, chhooi nahi (evidence: clippy
  output me `app.rs` ka zero hit: `APP-RS-CLIPPY-CLEAN`)
- `rustfmt --edition 2024 --check crates/gui/src/app.rs` — clean
  (`cargo fmt -- crates/gui/src/app.rs` se formatted, sirf OWNS file)
- banned grep (non-test + test): `unwrap()/expect()/panic!/todo!/
  unimplemented!/unreachable!/unsafe` — zero hits; `forbid(unsafe_code)` kept;
  koi indexing nahi (`.get()` only)
- `tools/gui-polish/run-geo-typo.sh checks/gui-applypaint-an/round-1` — PASS
  (PNG 9624 bytes, magic ok, distinct 256; geometry/typography tokens
  unchanged, colors untouched — paint change nahi tha)

## Files

- `crates/gui/src/app.rs` — state machine + summary + tests
- `checks/gui-applypaint-an/GATES.md`, `REPORT.md`, `round-1/` — is wave ke
  artifacts (`round-1`: baseline-tests.out, after.png, geo-typo-report.md,
  screenshot.out, tests.out)
