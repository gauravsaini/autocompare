# Wave V GLYPH — REPORT (round-V-1)

Scope: `crates/gui/src/theme.rs` + `checks/gui-glyph-v/**` ONLY.
`views.rs` / `app.rs` / `screenshot.rs` read-only (row-height call shape checked via grep only, no edits).
Colors FROZEN, geometry consts FROZEN. No new deps. Clean-room: no strings/assets copied.

Early-feedback delta #3: raster glyphs read chunkier, row height 24px feels tall.
Fix is theme-owned knobs only — paint still measures the shared mono row height,
so this wave lands headless intent (PNG bit-identical by design); paint adopts later.

## Baseline -> After (isolated: siblings' in-flight views.rs/app.rs stashed)

- `cargo test -p autocompare-gui`: 245 passed -> 246 passed, 0 failed
  (+1: `glyph_zoomed_line_height_clamps`). Working-tree run with siblings'
  uncommitted code shows 248 — their +2, not mine; untouched by this commit.
- Harness `tools/gui-polish/run-geo-typo.sh -> round-V-1/geo-typo-report.md`: RESULT PASS.
- PNG: 9914 bytes, magic ok, distinct 256, BIT-IDENTICAL to
  `checks/gui-typo-u/round-U-1/after.png` (`cmp` clean — paint untouched by design).
- `cargo clippy -p autocompare-gui --all-targets -- -D warnings`: clean (exit 0) isolated.
  Full-tree run flags `views.rs:2813 assertions_on_constants` — sibling V-MINI
  in-flight code, out of scope, left for its owner.
- `cargo fmt -- --check`: clean (isolated and full-tree).
- MS guidelines: `#![forbid(unsafe_code)]` kept (theme.rs:49); zero banned hits in
  non-test code (`unwrap/expect/panic/todo!` grep above `#[cfg(test)]` = 0).

## Diff (theme.rs +52/-9, smallest that works)

- Line-height retune (the row-feel fix): `TYPO_LINE_HEIGHT_BODY` 1.45 -> 1.40
  (13px body row 18.85 -> 18.2px), `TYPO_LINE_HEIGHT_MONO` 1.35 -> 1.30
  (12px mono row 16.2 -> 15.6px). `TYPO_LINE_HEIGHT_CHROME` 1.2 kept (already tight).
  Doc comments record the Wave V rationale + old values.
- Zoom-clamp interplay: new `zoomed_line_height_px(role, level)` =
  `zoomed_size(role, level) * role.line_height()` — reuses both existing helpers,
  so zoom can never escape the MIN..MAX size clamp; level 0 == `line_height_px`.
- Tracking/tabular deliberately KEPT: headers 0.02em air + mono/small tabular
  already carry the hierarchy; chunkier-raster complaint is vertical rhythm, not
  tracking (and egui `extra_letter_spacing` stays 0.0 no-op in `rich()` x2).
- Tests: updated `typo_exquisite_tokens_carry_hierarchy` multipliers + px math;
  new `glyph_zoomed_line_height_clamps` (every role: level-0 passthrough, +1 step
  grows exactly one multiplier, extremes clamp via existing resolver, feel numbers).
- `rich()` / `rich_zoomed()` paint paths unchanged; no geometry/color token touched.

## Feel table (role -> before -> after, headless intent)

| Role | Size | Before | After |
|---|---|---|---|
| Mono (diff/gutter/hex/cells) | 12px | 16.2px | 15.6px |
| Body (labels/buttons) | 13px | 18.85px | 18.2px |
| Heading (chrome) | 18px | 21.6px | 21.6px (kept) |

Read-only shape refs (not edited): `views.rs:90-96` `row_height_for_line`
(line_h + ROW_PAD passthrough for non-finite), `views.rs:181-182`
`text_row_height` (measured mono), `screenshot.rs:204` mock `row_h = 24u32`
(headless mock renderer, owned outside this wave — theme intent tightens the
real rows when paint adopts the multipliers).

## Frozen proof

- `git diff -- crates/gui/src/theme.rs | grep "^-[^-].*(SPACE_|GEO_|GUTTER_|Color32|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_)"`: NO-FROZEN-REMOVAL.
- Staged for commit: only `crates/gui/src/theme.rs` + `checks/gui-glyph-v/**`
  (siblings' `views.rs`/`app.rs` + `checks/gui-mini-v/` + `checks/gui-toolbar-v/`
  left uncommitted for their owners).

## Files

- `crates/gui/src/theme.rs` (+52/-9: 2 retuned consts + 1 helper + doc lines + 1 new test).
- `checks/gui-glyph-v/GATES.md`, `REPORT.md`, `round-V-1/` (tests.out, screenshot.out, after.png, geo-typo-report.md).
- Untouched: views/app/gutter/home/hex/table/tree/viewmodel/main/screenshot, STATUS.md.
- No push (orchestrator verifies+pushes).
