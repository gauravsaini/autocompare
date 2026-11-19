# GATES — Wave AO-APPLYUI (checks/gui-applyui-ao)

Gap #8 apply paint adoption: merge UI me Apply button + progress/result display
ko App apply state machine (AN precedent `checks/gui-applypaint-an/REPORT.md`)
se connect karna — real summary + result paint. State half AN me ho chuka hai
(`MergeApplyState`, `MergeApplySummary`, `request/take/clear`, `run_merge_apply`);
is wave me koi state change nahi, sirf paint adoption.

## OWNS / KEVL

- OWNS: `crates/gui/src/views.rs` + `checks/gui-applyui-ao/**`
- KEVL (chhoona mana, read bhi avoid — sirf App apply state model read-only use ok):
  `app.rs`, `viewmodel.rs`, `tree.rs`, `views_table.rs`, `views_hex.rs`, `home.rs`,
  `gutter.rs`, `theme.rs`, `screenshot.rs`
- App call-site wiring (merge tab me `paint_merge_apply` ka call) App owner wave ka
  hai; is wave me `views.rs` me call-ready signature + pure helpers + tests.

## Design (geometry first, typography second, colors frozen)

- Apply row: dry-run checkbox + output path field + Apply button, ek `HEADER_H`
  (28px) row me — paging-bar rhythm reuse, koi naya height token nahi.
- Path field width: `(available - BROWSE_RESERVE).max(MIN_HALF)` — existing tokens.
- Spacing: `TIGHT_GAP` andar, section ke baad `TIGHT_GAP`; koi naya gap nahi.
- Typography: section title small-strong (hunk-card precedent), summary/result
  `chrome_status_role` (Small 11 weak proportional) — mono nahi, koi naya size nahi.
- Colors: zero naya color; sirf existing roles/tones.
- Honesty: no-model par button disabled + `summary_line` already "compare first";
  result display `summary_line` (App real line) + structured counts
  (`format_merge_apply_result`); fake progress/percentage kuch nahi.
- Pure helpers (headless-tested): `merge_apply_section_title`,
  `merge_apply_path_hint`, `merge_apply_state_label` (6 states, distinct),
  `format_merge_apply_result` (dry-run / needs-path / wrote branches).
- Paint fn: `paint_merge_apply(ui, enabled, dry_run, out_path, state, summary,
  summary_line) -> bool` — returns Apply clicked; caller `request + run` chalata hai.
  Existing `paint_merge`/`paint_merge_three_pane` signatures unchanged (KEVL compat).

## Gates (sab PASS chahiye)

1. `cargo test -p autocompare-gui` — full suite green (naye tests samait).
2. `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — zero warnings.
3. `cargo fmt -- <sirf apne OWNS files>` — `views.rs` clean
   (`cargo fmt --check` repo-wide dusre agent ki files ke karan fail ho sakta hai —
   sirf OWNS file check/format karo).
4. Banned grep non-test + test: `unwrap()/expect()/panic!/todo!/unimplemented!/
   unreachable!/unsafe` — zero hits OWNS me; non-test me indexing nahi (`.get()` only).
5. `tools/gui-polish/run-geo-typo.sh checks/gui-applyui-ao/round-1` — `RESULT: PASS`.
6. `REPORT.md` + `round-1/` artifacts, phir `git add <sirf OWNS paths>` + `git commit`
   (conventional English). No push, no stash.

## Banned (MS rust guidelines)

- Non-test code me `unwrap/expect/panic/todo` mana; `forbid(unsafe_code)` kept.
- Repo ke bahar copy/sandbox nahi; dusre agent ki files mat chhoona.
- `git stash/push` kabhi nahi.
