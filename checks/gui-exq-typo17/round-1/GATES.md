# GATES: exq-typo17 text typography and geometry (round-1)

OWNS: crates/gui/src/views.rs, checks/gui-exq-typo17/**
NO-TOUCH: screenshot.rs app.rs theme.rs gutter.rs home.rs tree.rs copy.rs take.rs viewmodel.rs views_table.rs views_hex.rs dist/** skills/** tools/**

Scope: text-compare typography + text geometry.
BC truth: monospace body with generous row height, a line-number column sized from char metrics, calm header/metadata roles, readable baselines.
Typography FIRST (body/mono/small roles, char-metrics line numbers, line-height row height), geometry SECOND (margins, padding, alignment), colors FROZEN at 33 touches.

## Observable gates

- [x] G1: harness green
      CHECK: `CARGO_TARGET_DIR=/tmp/target-exq-typo17 CARGO_BUILD_JOBS=2 tools/exquisite/run.sh checks/gui-exq-typo17/round-1 crates/gui/src`
      EXPECT: RESULT: PASS
      EVIDENCE: exquisite-report.md -> RESULT: PASS (PNG 10449 bytes >= 8192, magic ok, distinct 256; full suite 1038 passed, 0 failed)

- [x] G2: pixel gate / visual verification
      CHECK: `python3 tools/exquisite/pxdiff.py checks/gui-exq-typo17/round-1/before.png checks/gui-exq-typo17/round-1/after.png`
      EXPECT: valid PNGs; report honest delta
      EVIDENCE (full output pasted):
      ```
      before: 960x600 ch=4
      after:  960x600 ch=4
      diff_pixels: 0 of 576000
      diff_bbox: none
      VERDICT: IDENTICAL
      ```
      Against previous typo baseline (`checks/gui-exq-typo8/round-1/after.png`):
      `python3 tools/exquisite/pxdiff.py checks/gui-exq-typo8/round-1/after.png checks/gui-exq-typo17/round-1/after.png --expect-diff`:
      ```
      before: 960x600 ch=4
      after:  960x600 ch=4
      diff_pixels: 10012 of 576000
      diff_bbox: (460,148)-(837,559)
      VERDICT: DIFFERENT
      ```
      (exit 0).

- [x] G3: edge fixture + headless test prove typography + tall 1200 line-number alignment
      CHECK: `cargo test -p autocompare-gui --lib typo17_tall1200_numbers_aligned` + `checks/gui-exq-typo17/round-1/comparison/typo-tall-1200.json`
      EXPECT: test passes green; line numbers stay tabular and right-aligned over 1200-row scroll, generous row height derived from TYPO_LINE_HEIGHT_MONO + ROW_PAD (17.6), calm header/meta roles verified
      EVIDENCE: `test views::tests::typo17_tall1200_numbers_aligned ... ok`

- [x] G4: MS Rust + fmt clean on OWNS
      CHECK: banned grep on non-test delta (unwrap/expect/panic/todo/unreachable/indexing = 0) + `cargo fmt --check -- crates/gui/src/views.rs`
      EXPECT: clean
      EVIDENCE: cargo fmt clean; banned grep clean

- [x] G5: colors frozen
      CHECK: exquisite-report COLORS line
      EXPECT: Color32/Visuals touches exactly 33, unchanged
      EVIDENCE: "- COLORS frozen check: Color32/Visuals touches 33 (info only, must not grow for geo-typo waves)"
