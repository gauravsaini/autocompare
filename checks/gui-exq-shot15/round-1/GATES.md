# GATES: exq-shot15 fixture gutter pixels (round-1)

OWNS: crates/gui/src/screenshot.rs, checks/gui-exq-shot15/**
NO-TOUCH: app.rs views.rs views_table.rs views_hex.rs home.rs tree.rs gutter.rs copy.rs take.rs theme.rs viewmodel.rs dist/** skills/** tools/** checks/localrun/** checks/gui-exq-paint13/** checks/gui-exq-paint14/**

Scope: default text fixture renderer draws the BC center gutter gap (live 20px),
yellow `=>` markers on changed-row hunk heads only, floor fill below last row.
Geometry FIRST, typography SECOND, colors FROZEN at 33 touches.

## Observable gates

- [x] G1: harness green
      CHECK: `tools/exquisite/run.sh checks/gui-exq-shot15/round-1 crates/gui/src`
      EXPECT: RESULT: PASS
      EVIDENCE: exquisite-report.md -> RESULT: PASS (PNG 10401 bytes >= 8192, magic ok,
      distinct 256; full suite 1036 passed, 0 failed)

- [x] G2: visual gate — after.png differs from BC baseline
      CHECK: `python3 tools/exquisite/pxdiff.py checks/ac-loop/round-lr2/gui/shot-default.png checks/gui-exq-shot15/round-1/after.png --expect-diff`
      EXPECT: exit 0, diff_pixels > 0
      EVIDENCE (full output pasted):
      ```
      before: 960x600 ch=4
      after:  960x600 ch=4
      diff_pixels: 10012 of 576000
      diff_bbox: (460,148)-(837,559)
      VERDICT: DIFFERENT
      ```
      (exit 0; was IDENTICAL/0px before this wave — gap + markers + pane shift now land)

- [x] G3: edge fixture + headless test prove gap/marker geometry in the renderer
      CHECK: `cargo test -p autocompare-gui --lib shot15` green + comparison/shot-center-1200.json
      EXPECT: layout (12,448,464,20,488,448); center 20 = views helper; `=>` 12px wide,
      4px symmetric inset; hunk heads [2]; fill 11 slots; 1200 edge (12,578,20,578,12)
      EVIDENCE: shot15_center_gap_markers_use_live_helpers ok

- [x] G4: MS Rust + fmt clean on OWNS
      CHECK: banned grep on non-test (unwrap/expect/panic/todo/unreachable/indexing = 0
      modulo `unwrap_or`/`forbid(unsafe_code)` attribute) + `rustfmt --edition 2024 --check crates/gui/src/screenshot.rs`
      EXPECT: clean
      EVIDENCE: fmt clean; non-test has 0 expect/panic/todo/unreachable, 0 indexing violations introduced

- [x] G5: colors frozen
      CHECK: exquisite-report COLORS line
      EXPECT: 33 touches, unchanged
      EVIDENCE: "COLORS frozen check: Color32/Visuals touches 33"
