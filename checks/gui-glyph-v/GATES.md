# Wave V GLYPH — GATES (write-first, one observable outcome per gate)

Scope: `crates/gui/src/theme.rs` + `checks/gui-glyph-v/**` ONLY.
`views.rs` / `app.rs` / `screenshot.rs` read-only (row-height call shape checked via grep only, no edits).
Colors FROZEN, geometry consts FROZEN (`SPACE_*`/`GEO_*`/`GUTTER_*` untouched). No new deps.

Early-feedback delta #3: raster glyphs read chunkier, row height 24px feels tall.
Fix is theme-owned knobs only: tighter line-height multipliers per role (mono/body),
tracking/tabular kept stable (documented why), zoom-clamp interplay via one new
helper reusing `zoomed_size` + `line_height`. Paint-neutral this wave (paint still
measures the shared mono row height; `rich()`/`rich_zoomed()` unchanged).

## G1 — Baseline green (before any edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `245 passed; 0 failed`

## G2 — Theme-only diff, no frozen drift, no out-of-scope edits

- CHECK: `git status --short && git diff --stat`
  + `git diff -- crates/gui/src/theme.rs | grep -E "^-.*(SPACE_|GEO_|GUTTER_|Color32::from_rgb|DARK_|LIGHT_|DELETED_|INSERTED_|CHANGED_|ACCENT_|ERROR_)" || echo NO-FROZEN-REMOVAL`
  + `git diff --name-only | grep -vE "^(crates/gui/src/theme.rs|checks/gui-glyph-v/)" || echo OWNS-CLEAN`
- EXPECT: only `crates/gui/src/theme.rs` + `checks/gui-glyph-v/**` touched;
  NO `-` lines on geometry/color tokens; `views.rs`/`app.rs`/`screenshot.rs` untouched;
  `rich()`/`rich_zoomed()` still `extra_letter_spacing(TYPO_LETTER_SPACING)` = 0.0

## G3 — Tighter line-heights + zoom interplay land, tests green

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `246 passed; 0 failed` (+1: `glyph_zoomed_line_height_clamps`);
  `TYPO_LINE_HEIGHT_BODY` 1.40, `TYPO_LINE_HEIGHT_MONO` 1.30, `TYPO_LINE_HEIGHT_CHROME` 1.2 kept;
  updated `typo_exquisite_tokens_carry_hierarchy` green;
  new `zoomed_line_height_px(role, level)` == `zoomed_size * line_height`,
  level 0 == `line_height_px`, monotonic per level, extremes clamp via existing resolver

## G4 — MS Rust guidelines + headless-first clean

- CHECK: `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `cargo fmt -- --check; echo fmt:$?`
  + `grep -nE "unwrap|expect|panic|todo!" crates/gui/src/theme.rs` hits only inside `#[cfg(test)]` (verify by line numbers >= test mod start) else ZERO-BANNED
- EXPECT: `clippy:0`, `fmt:0`, ZERO-BANNED in non-test code, `#![forbid(unsafe_code)]` kept,
  new helper is `pub fn` headless (no `Ui`/`Context`), reuses existing helpers

## G5 — Harness PASS + PNG magic-ok (headless proof)

- CHECK: `zsh tools/gui-polish/run-geo-typo.sh checks/gui-glyph-v/round-V-1; echo harness:$?`
  + `python3 -c "d=open('checks/gui-glyph-v/round-V-1/after.png','rb').read();print(len(d), d[:8].hex(), len(set(d)))"`
  + `cmp checks/gui-glyph-v/round-V-1/after.png checks/gui-typo-u/round-U-1/after.png && echo BIT-IDENTICAL`
- EXPECT: harness prints `RESULT: PASS`, `after.png` magic `89504e470d0a1a0a`,
  size >= 8192, distinct > 64, BIT-IDENTICAL to typo-u baseline (paint-neutral proof)

## G6 — Commit locally, NO push + REPORT done

- CHECK: `git log --oneline -1 && git status --short | head && git log origin/main..HEAD --oneline | head`
- EXPECT: one new `feat(gui): ...` English conventional commit on `main`,
  `checks/gui-glyph-v/REPORT.md` present, working tree clean except commit,
  NO push (orchestrator verifies+pushes)
