# GATES — Wave AQ-WRAP (text wrap parity)

OWNS: `crates/gui/src/views.rs` + `checks/gui-wrap-aq/**` only.
KEVL (no touch, read bhi avoid): `app.rs`, `viewmodel.rs`, `tree.rs`,
`views_table.rs`, `views_hex.rs`, `home.rs`, `gutter.rs`, `theme.rs`,
`screenshot.rs`.

Clean-room: Beyond Compare bahar se observe only, apne shabdon me.
`word_wrap` state read-only use ok. Colors frozen (koi naya hue/token nahi).

## Scope

`word_wrap` on par wrapped line paint parity:

- Geometry first: wrap width, indent continuity, row mapping.
- Typography second: wrapped continuation style (same mono, koi naya size/weight nahi).
- Wrap off path byte-identical (existing paint/tests untouched).

## Geometry gates

- G1 wrap width: wrap on par pane width cap 80 chars (`TYPO_WRAP_PANE_CHARS`
  reuse, 200 full-width precedent). Helper `wrap_width_for` /
  `effective_pane_chars_for` reuse — koi naya magic number nahi.
- G2 indent continuity: continuation visual lines same leading indent
  carry karti hain (`wrap_leading_indent` + `wrap_visual_lines`).
  Pehli sub-row gutter number dikhati hai, continuation blank gutter
  (same width) — column rhythm nahi toot-ta.
- G3 row mapping: model row <-> flat visual index mapping sahi
  (`wrap_visual_count`, `wrap_visual_to_model`, `wrap_model_to_visual_start`).
  Selection/scroll/minimap isi mapping par chalte hain.
- G4 row height: wrap on par har model row `row_height * visual_count`
  reserve karta hai (single `ROW_PAD` rhythm per visual line, `row_height_for_line`
  reuse). show_rows uniform-height path sirf wrap-off par; wrap-on par
  per-row variable height manual scroll me.

## Typography gates

- T1 continuation style: same mono `FontId`, same size, no strong, no naya
  size/weight. Accent inversion + 1px underline span rule unchanged
  (`span_text_format` reuse).
- T2 gutter/notes existing chrome roles reuse (`chrome_status_role` small weak,
  mono panes). Koi naya `.size()` / `.strong()` nahi.

## Quality gates (MS rust guidelines)

- Q1 `#![forbid(unsafe_code)]` intact; non-test code me `unwrap` / `expect` /
  `panic!` / `todo!` / `unreachable!` mana hai (`.unwrap_or*`, `if let`,
  `match` hi use hoga).
- Q2 `cargo test -p autocompare-gui` green (naye unit tests samahit).
- Q3 `cargo clippy -p autocompare-gui --all-targets` zero warnings.
- Q4 `cargo fmt --check` — sirf apne OWNS files par `cargo fmt -- <files>`.
- Q5 banned grep clean: `rg -n "unwrap\(|expect\(|panic!|todo!|unreachable!" crates/gui/src/views.rs`
  (test mod bahar; `unwrap_or` ok).
- Q6 `tools/gui-polish/run-geo-typo.sh checks/gui-wrap-aq/round-1` headless
  proof PASS (`RESULT: PASS`).

## Test plan (views.rs `mod tests` me)

1. `wrap_width_*`: on par cap 80, off par full pass-through.
2. `wrap_leading_indent_*`: spaces/tabs prefix, no-indent par empty.
3. `wrap_visual_lines_*`: short line single, long hard-split, word boundary
   prefer, indent continuity, width 0 guard.
4. `wrap_visual_count_*`: count == lines len, empty/width-0 guards.
5. `wrap_row_mapping_*`: visual->model + model->visual-start roundtrip,
   out-of-range `None`.
6. `wrap_continuation_*`: continuation plain mono (no strong/size change —
   helper contract), first-vs-continuation gutter shape.

## DoD

- [x] G1..G4 + T1..T2 implemented, sirf `views.rs` me.
- [x] Unit tests green + full `cargo test -p autocompare-gui` green (458 passed, 0 failed).
- [x] Clippy zero warnings, fmt clean, banned grep clean.
- [x] Harness PASS (`round-1/geo-typo-report.md` me `RESULT: PASS`).
- [x] `REPORT.md` + `round-1/` artifacts, phir `git add` sirf OWNS paths +
  `git commit` (conventional English, no push).

## Retry note (round-2, 2026-11-20)

- Pehle attempt me pure helpers (`wrap_chunks`, mapping, gutter) bane the,
  par `paint_text_with_prefs` me wrap-on branch wired nahi tha aur GATES
  test-plan tests aadhe the.
- Is round me joda: `wrap_reserved_height` (G4 reserve math), wrap-on
  variable-height manual list paint (G1 width cap reuse, G2 indent +
  blank-gutter continuity, G3 live mapping invariant + scroll guard, T1
  plain-mono continuation via `span_text_format`, T2 koi naya size/weight
  nahi), wrap-off path byte-identical else me.
- 10 naye tests (width/indent/lines/counts/mapping/spans/reserve/gutter/jobs).
