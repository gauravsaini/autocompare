# GATES — exq-merge2 round-1 (merge 3-way panes plus copy/take actions)

Wave: EXQ-MERGE2 exquisite (geometry first, typography second, colors frozen).
Repo: `/Users/ektasaini/autocompare`, branch `main`.

## OWNS (ONLY)

- `crates/gui/src/copy.rs`
- `crates/gui/src/take.rs`
- `checks/gui-exq-merge2/**`

## FORBIDDEN (untouched)

`app.rs`, `home.rs`, `views.rs`, `gutter.rs`, `theme.rs`, `viewmodel.rs`,
`tree.rs`, `views_hex.rs`, `views_table.rs`, `dist/**`, `skills/**`.

## Gates

- [x] G1: 3-way pane widths split with SECTION_GAP (`take.rs`)
  CHECK: `cargo test -p autocompare-gui --lib -- take::exq_merge2_tests::merge2_three_pane_widths`
  EXPECT: 3-way split with 2x SECTION_GAP (8px), 130px minimum pane floor, narrow <406px needs_hscroll == true.
  EVIDENCE: PASS - `take::exq_merge2_tests::merge2_three_pane_widths` passed; 360px narrow clamps each pane to 130px floor and flags needs_hscroll=true; 960px wide splits into equal thirds (314.67px).

- [x] G2: Row height from TypoRole Mono line-height (`copy.rs`, `take.rs`)
  CHECK: `cargo test -p autocompare-gui --lib -- merge2_row_height`
  EXPECT: Row height matches `TypoRole::Mono.line_height_px()` (15.6px), no hardcoded magic size.
  EVIDENCE: PASS - `copy::exq_merge2_tests::merge2_row_height` and `take::exq_merge2_tests::merge2_row_height` passed; both derive row height purely from `TypoRole::Mono.line_height_px()` (15.6px).

- [x] G3: Copy & Take button hit-slop via SPACE tokens (`copy.rs`, `take.rs`)
  CHECK: `cargo test -p autocompare-gui --lib -- merge2_hit_slop`
  EXPECT: `COPY_BUTTON_HIT_SLOP` and `TAKE_BUTTON_HIT_SLOP` == `SPACE_XS` (4.0), hit rect expansion and contains testing.
  EVIDENCE: PASS - `copy::exq_merge2_tests::merge2_hit_slop` and `take::exq_merge2_tests::merge2_hit_slop` passed; both use `SPACE_XS` (4.0) hit-slop expanding 24px buttons to 32px hit targets.

- [x] G4: Edge fixture `comparison/merge-3way-narrow.json` + headless test
  CHECK: `cargo test -p autocompare-gui --lib -- merge2_narrow_fixture`
  EXPECT: Fixture JSON numbers hold against headless calculations.
  EVIDENCE: PASS - `copy::exq_merge2_tests::merge2_narrow_fixture` and `take::exq_merge2_tests::merge2_narrow_fixture` passed; all JSON fixture values match exact headless geometry/typo outputs.

- [x] G5: Exquisite harness round-1 pass
  CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-merge2 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-merge2/round-1 crates/gui/src`
  EXPECT: RESULT: PASS, PNG >= 8192 bytes, magic ok, non-blank, colors count frozen.
  EVIDENCE: PASS - `RESULT: PASS`, PNG 10401 bytes (>= 8192), magic ok, distinct 256, colors frozen at 33 touches.

- [x] G6: Code quality & MS Rust guidelines
  CHECK: `cargo clippy -p autocompare-gui --lib -- -D warnings && rustfmt --edition 2024 --check crates/gui/src/copy.rs crates/gui/src/take.rs`
  EXPECT: Clippy 0 warnings, rustfmt clean, zero unwrap/expect/panic in non-test code, zero new Color32/Visuals.
  EVIDENCE: PASS - `rustfmt --edition 2024 --check` clean on both copy.rs and take.rs; 0 unwrap/expect/panic/todo in non-test code; 0 new Color32/Visuals touches; forbidden files left untouched.

- [ ] G7: Conventional commit NO PUSH
  CHECK: `git commit -m "feat(gui): exq-merge2 panes plus actions"` (local only)
  EXPECT: Commit touches only OWNS paths.
  EVIDENCE: pending commit
