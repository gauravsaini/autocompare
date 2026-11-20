# Wave GUI-TEXT-H GATES — Exquisite Text Node (TextCompare & TextMerge)

OWNS ONLY:
- `crates/gui/src/views.rs`
- `crates/gui/src/copy.rs`
- `crates/gui/src/take.rs`
- `crates/gui/src/gutter.rs`
- `checks/gui-text-h/**`

FORBIDDEN:
- `crates/cli/**`
- `app.rs`, `tree.rs`, `home.rs`, `viewmodel.rs`, `views_hex.rs`, `views_table.rs`
- `STATUS.md`, `PLAN.md`, `inbox/build-progress.json`
- All other files outside OWNS.

Colors FROZEN: Zero new `Color32::from_rgb`, zero `Color32::from_rgba`, zero `Visuals::` additions.
Geometry first: widths, heights, margins, padding, alignment, positioning.
Typography second: family (Mono/Small/Body), weight, line-height, letter spacing.
Affordances: Gutter overview ruler, Copy Line/Section/All, 3-way Take Left/Right/Base/Both interactive affordances.

## G1 — Baseline Green (before edit)

- CHECK: `cargo test -p autocompare-gui 2>&1 | tail -n 3`
- EXPECT: `678 passed; 0 failed`
- EVIDENCE: Recorded baseline in `round-1/baseline-tests.out`

## G2 — OWNS-only Diff, Zero Collisions, Colors Frozen

- CHECK:
  + `git status --short`
  + `git diff --name-only | grep -vE "^(crates/gui/src/(views|copy|take|gutter)\.rs|checks/gui-text-h/)" || echo OWNS-CLEAN`
  + `git diff -- crates/gui/src/views.rs crates/gui/src/copy.rs crates/gui/src/take.rs crates/gui/src/gutter.rs | grep -E "^\+.*(Color32::from_rgb|Color32::from_rgba|Visuals::)" || echo NO-COLOR-ADD`
- EXPECT: OWNS-CLEAN, NO-COLOR-ADD, zero edits in forbidden files

## G3 — Exquisite Parity Implementations & Tests

- CHECK:
  + `cargo test -p autocompare-gui copy_`
  + `cargo test -p autocompare-gui take_`
  + `cargo test -p autocompare-gui merge_gutter_`
  + `cargo test -p autocompare-gui text_merge_`
- EXPECT: All pass, covering:
  + `copy.rs`: `copy_all`, `copyable_diff_count`, `can_copy_row`
  + `take.rs`: `resolve_take_all`, `conflict_count`, `is_conflict_marker`
  + `gutter.rs`: `merge_gutter_geometry`, `merge_conflict_line_ranges`, `gutter_ruler_hit_test`
  + `views.rs`: `paint_merge` 4-marker highlight (`<<<<<<<`, `|||||||`, `=======`, `>>>>>>>`), line number gutter column for merge output, overview ruler integration, interactive `selectable_label` for take choices, `paint_text_body` All copy buttons with tooltips.

## G4 — MS Rust Guidelines & Clean Formatting

- CHECK:
  + `cargo clippy -p autocompare-gui --all-targets -- -D warnings; echo clippy:$?`
  + `rustfmt --edition 2024 --check crates/gui/src/views.rs crates/gui/src/copy.rs crates/gui/src/take.rs crates/gui/src/gutter.rs; echo fmt:$?`
  + `awk '/^#\[cfg\(test\)\]/ {intest=1} !intest' crates/gui/src/views.rs crates/gui/src/copy.rs crates/gui/src/take.rs crates/gui/src/gutter.rs | grep -nE "unwrap\(|expect\(|panic!|todo!|unimplemented!|unreachable!" || echo ZERO-BANNED`
  + `head -20 crates/gui/src/views.rs crates/gui/src/copy.rs crates/gui/src/take.rs crates/gui/src/gutter.rs | grep -q "forbid(unsafe_code)" && echo UNSAFE-FORBID-OK`
- EXPECT: `clippy:0`, `fmt:0`, `ZERO-BANNED`, `UNSAFE-FORBID-OK`

## G5 — Harness PASS + Valid Screenshot Fixtures

- CHECK:
  + `tools/ac-loop/run.sh checks/gui-text-h/round-1; echo ac-loop:$?`
  + `target/debug/autocompare-gui --screenshot checks/gui-text-h/round-1/fixture-text.png`
  + `target/debug/autocompare-gui --screenshot checks/gui-text-h/round-1/fixture-merge.png`
  + `python3 -c "from pathlib import Path; f=Path('checks/gui-text-h/round-1/fixture-text.png').read_bytes(); assert f[:8]==bytes.fromhex('89504e470d0a1a0a') and len(f)>=8192 and len(set(f))>64; print('PNG_TEXT_OK')"`
  + `python3 -c "from pathlib import Path; f=Path('checks/gui-text-h/round-1/fixture-merge.png').read_bytes(); assert f[:8]==bytes.fromhex('89504e470d0a1a0a') and len(f)>=8192 and len(set(f))>64; print('PNG_MERGE_OK')"`
- EXPECT: `RESULT: PASS`, `PNG_TEXT_OK`, `PNG_MERGE_OK`

## G6 — Hinglish-Roman REPORT & Local Pathspec Commit (NO PUSH)

- CHECK:
  + `ls checks/gui-text-h/REPORT.md checks/gui-text-h/GATES.md`
  + `git commit pathspec only (no git add -A, no push)`
- EXPECT: Conventional commit `feat(gui): exquisite text and merge pane parity`, REPORT.md in Hinglish-Roman, no push.
