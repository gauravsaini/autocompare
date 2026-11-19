# REPORT — Wave AO-APPLYUI (checks/gui-applyui-ao)

Wave: AO-APPLYUI. Gap #8 apply paint adoption — merge UI ka Apply button +
progress/result display App apply state machine (AN precedent
`checks/gui-applypaint-an/REPORT.md`: `MergeApplyState`, `MergeApplySummary`,
`request/take/clear`, `run_merge_apply`) se connected. State me koi change nahi,
sirf paint adoption; App call-site wiring App owner wave ka hai.

## Kya bana (sirf OWNS: `crates/gui/src/views.rs`)

`paint_merge`/`paint_merge_three_pane` signatures unchanged (KEVL `app.rs`
call-site compat). Naya call-ready section:

- `paint_merge_apply(ui, enabled, dry_run, out_path, state, summary, summary_line)
  -> bool` — dry-run checkbox + output path field + Apply button ek `HEADER_H`
  (28px) row me; neeche progress/result display: `apply state: <label>` +
  App real `summary_line` + structured counts jab report ho. Returns Apply clicked
  (caller `request_merge_apply` phir `run_merge_apply` chalata hai; disabled button
  par kabhi true nahi). `enabled` = `merge_apply_enabled()`, baaki params App
  truth ke read-only view — koi naya state, token, color nahi.
- Pure helpers (headless-tested): `merge_apply_section_title` ("Apply"),
  `merge_apply_path_hint` (dry-run honest "not written" note),
  `merge_apply_state_label` (6 states, distinct, koi fake percent nahi),
  `format_merge_apply_result` (dry-run counts / needs-path note / wrote + path).
- Geometry first: `HEADER_H` row, `(available - BROWSE_RESERVE).max(MIN_HALF)`
  field width, `TIGHT_GAP` spacing. Typography second: small-strong title
  (hunk-card precedent) + `chrome_status_role` summary lines. Colors frozen.
- `#[allow(dead_code)]` 5 naye items par (file precedent `merge_pane_count`):
  App call-site wiring agle wave me judte hi live honge; tab tak tests live rakhte hain.

Tests (7 naye, `views.rs` tests module me, koi banned API nahi):

- title "Apply", path-hint dono modes, 6 state labels distinct/non-empty
- result None par None; dry-run exact line; needs-path exact line; wrote exact line

## Gates proof

- `cargo test -p autocompare-gui` — 431 passed, 0 failed (7 is wave ke; beech me
  KEVL `screenshot.rs` ke 3 fail concurrent agent ki in-progress file ke the —
  chhooi nahi, dobara run par full green; evidence `round-1/tests.out`)
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings` — zero warnings
- `cargo fmt -- --check crates/gui/src/views.rs` — clean (sirf OWNS file)
- banned grep: `unwrap()/expect()/panic!/todo!/unimplemented!/unreachable!/unsafe`
  — zero hits (diff + file); non-test me indexing nahi (`.get()` only)
- `tools/gui-polish/run-geo-typo.sh checks/gui-applyui-ao/round-1` — PASS
  (PNG 9624 bytes, magic ok, distinct 256; geometry/typography tokens unchanged,
  colors untouched)

## Files

- `crates/gui/src/views.rs` — apply section + 7 tests
- `checks/gui-applyui-ao/GATES.md`, `REPORT.md`, `round-1/` — is wave ke artifacts
  (`round-1`: after.png, geo-typo-report.md, screenshot.out, tests.out)
